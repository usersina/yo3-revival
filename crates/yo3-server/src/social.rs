//! Friends and ignore lists (the community agent 0x4A4EE0): friendships both ways, each player's ignore list, the names
//! they are listed under, and the friend requests waiting for an answer; kept in a file next to the accounts.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};

#[derive(Default)]
pub struct Social {
    friends: BTreeMap<u32, BTreeSet<u32>>,
    ignored: BTreeMap<u32, BTreeSet<u32>>,
    names: BTreeMap<u32, String>,
    /// Friend requests waiting for an answer: the player asked -> the player asking.
    pub requests: HashMap<u32, u32>,
    file: Option<PathBuf>,
}

impl Social {
    /// The friends and ignore lists kept in `file` (read now if it exists, written after each change).
    pub fn load(file: &Path) -> Social {
        let mut social = Social { file: Some(file.to_path_buf()), ..Social::default() };
        let saved = crate::store::read_or_keep(file, "friends");
        let pairs = |k: &str| -> Vec<(u32, u32)> {
            let pair = |p: &Json| Some((p[0].as_u64()? as u32, p[1].as_u64()? as u32));
            saved[k].as_array().into_iter().flatten().filter_map(pair).collect()
        };
        for (a, b) in pairs("friends") {
            social.friends.entry(a).or_default().insert(b);
            social.friends.entry(b).or_default().insert(a);
        }
        for (a, b) in pairs("ignored") {
            social.ignored.entry(a).or_default().insert(b);
        }
        for (id, name) in saved["names"].as_object().into_iter().flatten() {
            if let (Ok(id), Some(name)) = (id.parse(), name.as_str()) {
                social.names.insert(id, name.to_string());
            }
        }
        social
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        let pairs = |lists: &BTreeMap<u32, BTreeSet<u32>>, both_ways: bool| -> Vec<[u32; 2]> {
            let all = lists.iter().flat_map(|(&a, bs)| bs.iter().map(move |&b| [a, b]));
            all.filter(|[a, b]| !both_ways || a < b).collect()
        };
        let names: BTreeMap<String, &String> = self.names.iter().map(|(id, n)| (id.to_string(), n)).collect();
        let saved = json!({ "friends": pairs(&self.friends, true), "ignored": pairs(&self.ignored, false), "names": names });
        if let Err(e) = crate::store::write(file, &saved) {
            tracing::warn!("friends not saved to {}: {e}", file.display());
        }
    }

    /// A deleted player leaves every list: friends of theirs, who ignores them, what they ignore, requests.
    pub fn forget(&mut self, player_id: u32) {
        self.friends.remove(&player_id);
        self.friends.values_mut().for_each(|s| {
            s.remove(&player_id);
        });
        self.ignored.remove(&player_id);
        self.ignored.values_mut().for_each(|s| {
            s.remove(&player_id);
        });
        self.requests.retain(|from, to| *from != player_id && *to != player_id);
        self.names.remove(&player_id);
        self.save();
    }

    /// A player is online under this name: their friends list them by it.
    pub fn seen(&mut self, player_id: u32, name: &str) {
        if self.names.get(&player_id).map(String::as_str) != Some(name) {
            self.names.insert(player_id, name.to_string());
            if self.friends.contains_key(&player_id) || self.ignored.values().any(|s| s.contains(&player_id)) {
                self.save();
            }
        }
    }

    pub fn name(&self, player_id: u32) -> String {
        self.names.get(&player_id).cloned().unwrap_or_default()
    }

    pub fn friends(&self, player_id: u32) -> Vec<u32> {
        self.friends.get(&player_id).into_iter().flatten().copied().collect()
    }

    pub fn are_friends(&self, a: u32, b: u32) -> bool {
        self.friends.get(&a).is_some_and(|s| s.contains(&b))
    }

    /// Whether `other` is on `player_id`'s ignore list.
    pub fn ignores(&self, player_id: u32, other: u32) -> bool {
        self.ignored.get(&player_id).is_some_and(|s| s.contains(&other))
    }

    pub fn befriend(&mut self, a: u32, b: u32) {
        self.friends.entry(a).or_default().insert(b);
        self.friends.entry(b).or_default().insert(a);
        self.save();
    }

    pub fn unfriend(&mut self, a: u32, b: u32) {
        for (x, y) in [(a, b), (b, a)] {
            if let Some(s) = self.friends.get_mut(&x) {
                s.remove(&y);
            }
        }
        self.save();
    }

    /// Adds `other` to `player_id`'s ignore list, or takes it off.
    pub fn ignore(&mut self, player_id: u32, other: u32, add: bool) {
        let list = self.ignored.entry(player_id).or_default();
        if add { list.insert(other) } else { list.remove(&other) };
        self.save();
    }

    /// The friend list 0x251C {friend_id, friend_name, login_flag: 1 online} (0x475A20 mode 2).
    pub fn friend_list(&self, player_id: u32, online: impl Fn(u32) -> bool) -> Message {
        self.list(0x251C, "friend", self.friends.get(&player_id), online)
    }

    /// The ignore list 0x252A {black_id, black_name, login_flag} (0x475A20 mode 3).
    pub fn ignore_list(&self, player_id: u32, online: impl Fn(u32) -> bool) -> Message {
        self.list(0x252A, "black", self.ignored.get(&player_id), online)
    }

    fn list(&self, reply: u32, prefix: &str, ids: Option<&BTreeSet<u32>>, online: impl Fn(u32) -> bool) -> Message {
        ids.into_iter().flatten().fold(Message::reply(reply), |m, &id| {
            m.item(
                Message::record()
                    .with(&format!("{prefix}_id"), Value::U32(id))
                    .with(&format!("{prefix}_name"), Value::text(&self.name(id)))
                    .with("login_flag", Value::U32(online(id) as u32)),
            )
        })
    }
}
