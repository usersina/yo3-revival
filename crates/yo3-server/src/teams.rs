//! Teams (the community agent's modes 5-12, 0x4A6490 on): a leader, subleaders and members, and the invitations
//! waiting for an answer; kept in a file next to the accounts.
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};

/// A member's authority (0x4B6510; Flash teamMode 2 for the leader).
pub const MEMBER: u32 = 0;
pub const SUBLEADER: u32 = 1;
pub const LEADER: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub id: u32,
    pub name: String,
    pub authority: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Team {
    pub id: u32,
    pub name: String,
    /// When it was formed, "YYYY-MM-DD HH:MM:SS" (Japan time).
    pub created: String,
    pub members: Vec<Member>,
}

impl Team {
    pub fn member(&self, player_id: u32) -> Option<&Member> {
        self.members.iter().find(|m| m.id == player_id)
    }

    pub fn authority(&self, player_id: u32) -> Option<u32> {
        self.member(player_id).map(|m| m.authority)
    }

    pub fn ids(&self) -> Vec<u32> {
        self.members.iter().map(|m| m.id).collect()
    }

    /// 0x2B13, the team info (slot 17; 0x4BBEC0): level 1 (above 1 opens team titles and protectors).
    pub fn info(&self) -> Message {
        let leader = self.members.iter().find(|m| m.authority == LEADER).map_or("", |m| m.name.as_str());
        Message::reply(0x2B13)
            .with("team_id", Value::U32(self.id))
            .with("team_name", Value::text(&self.name))
            .with("level", Value::U32(1))
            .with("leader_name", Value::text(leader))
            .with("created", Value::text(&self.created))
            .with("season_score", Value::U32(0))
            .with("score", Value::U32(0))
    }

    /// 0x2B17, the members (slot 9; 0x475A20 mode 4, 0x4B6060), the player's own row among them.
    pub fn member_list(&self, online: impl Fn(u32) -> bool) -> Message {
        self.members.iter().fold(Message::reply(0x2B17), |list, m| {
            list.item(
                Message::record()
                    .with("member_id", Value::U32(m.id))
                    .with("member_name", Value::text(&m.name))
                    .with("login_flag", Value::U32(online(m.id) as u32))
                    .with("authority", Value::U32(m.authority)),
            )
        })
    }
}

/// Why a team can't be formed (0x2B10's error_code, 0x4A6490).
pub const NAME_TAKEN: u32 = 409;
pub const BAD_NAME: u32 = 808;

#[derive(Default)]
pub struct Teams {
    last_id: u32,
    teams: BTreeMap<u32, Team>,
    /// Invitations waiting for an answer: the player invited -> (the player inviting, their team).
    pub invites: HashMap<u32, (u32, u32)>,
    file: Option<PathBuf>,
}

impl Teams {
    /// The teams kept in `file` (read now if it exists, written after each change).
    pub fn load(file: &Path) -> Teams {
        let mut teams = Teams { file: Some(file.to_path_buf()), ..Teams::default() };
        let saved = crate::store::read_or_keep(file, "teams");
        teams.last_id = saved["last_id"].as_u64().unwrap_or(0) as u32;
        for t in saved["teams"].as_array().into_iter().flatten() {
            let member = |m: &Json| {
                Some(Member { id: m["id"].as_u64()? as u32, name: m["name"].as_str()?.into(), authority: m["authority"].as_u64()? as u32 })
            };
            let (Some(id), Some(name)) = (t["id"].as_u64(), t["name"].as_str()) else { continue };
            let members = t["members"].as_array().into_iter().flatten().filter_map(member).collect();
            let created = t["created"].as_str().unwrap_or_default().into();
            teams.teams.insert(id as u32, Team { id: id as u32, name: name.into(), created, members });
        }
        teams
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        let teams: Vec<Json> = self
            .teams
            .values()
            .map(|t| {
                let members: Vec<Json> = t.members.iter().map(|m| json!({ "id": m.id, "name": m.name, "authority": m.authority })).collect();
                json!({ "id": t.id, "name": t.name, "created": t.created, "members": members })
            })
            .collect();
        if let Err(e) = crate::store::write(file, &json!({ "last_id": self.last_id, "teams": teams })) {
            tracing::warn!("teams not saved to {}: {e}", file.display());
        }
    }

    /// The team a player is in.
    pub fn of(&self, player_id: u32) -> Option<&Team> {
        self.teams.values().find(|t| t.member(player_id).is_some())
    }

    pub fn get(&self, team_id: u32) -> Option<&Team> {
        self.teams.get(&team_id)
    }

    /// A new team led by `leader`; its id, or why not (NAME_TAKEN, BAD_NAME).
    pub fn form(&mut self, (id, name): (u32, &str), team_name: &str, created: String) -> Result<u32, u32> {
        let team_name = team_name.trim();
        if team_name.is_empty() || team_name.chars().any(char::is_control) {
            return Err(BAD_NAME);
        }
        if self.teams.values().any(|t| t.name.eq_ignore_ascii_case(team_name)) {
            return Err(NAME_TAKEN);
        }
        self.last_id += 1;
        let leader = Member { id, name: name.into(), authority: LEADER };
        self.teams.insert(self.last_id, Team { id: self.last_id, name: team_name.into(), created, members: vec![leader] });
        self.save();
        Ok(self.last_id)
    }

    pub fn join(&mut self, team_id: u32, (id, name): (u32, &str)) {
        if let Some(t) = self.teams.get_mut(&team_id) {
            t.members.push(Member { id, name: name.into(), authority: MEMBER });
            self.save();
        }
    }

    /// Takes a player out of their team; the team as it was.
    pub fn remove(&mut self, player_id: u32) -> Option<Team> {
        let team = self.of(player_id)?.clone();
        if let Some(t) = self.teams.get_mut(&team.id) {
            t.members.retain(|m| m.id != player_id);
        }
        self.save();
        Some(team)
    }

    pub fn disband(&mut self, team_id: u32) -> Option<Team> {
        let team = self.teams.remove(&team_id)?;
        self.invites.retain(|_, (_, t)| *t != team_id);
        self.save();
        Some(team)
    }

    pub fn set_authority(&mut self, team_id: u32, player_id: u32, authority: u32) {
        let member = self.teams.get_mut(&team_id).and_then(|t| t.members.iter_mut().find(|m| m.id == player_id));
        if let Some(m) = member {
            m.authority = authority;
            self.save();
        }
    }
}
