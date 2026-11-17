//! D-Mail (MailWindow 0x4AC6B0): a mail waits here until its recipient's mail window fetches it, after which the
//! client keeps it in its own files; kept in a file next to the accounts.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};

/// The most mail that waits for one player from other players (#39); the host's own mail always arrives.
pub const MAX_WAITING: u32 = 100;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mail {
    pub id: u32,
    /// 0 to a player, 3 to a team.
    pub kind: u32,
    pub from: (u32, String),
    pub to: (u32, String),
    pub title: String,
    pub body: String,
    /// "YYYY-MM-DD HH:MM:SS", Japan time like every server time.
    pub sent: String,
    pub team: (u32, String),
}

impl Mail {
    /// A 0x2586 item (0x4AD3C0).
    fn record(&self) -> Message {
        Message::record()
            .with("mail_id", Value::U32(self.id))
            .with("mail_type", Value::U32(self.kind))
            .with("from_id", Value::U32(self.from.0))
            .with("from_name", Value::text(&self.from.1))
            .with("to_id", Value::U32(self.to.0))
            .with("to_name", Value::text(&self.to.1))
            .with("title", Value::text(&self.title))
            .with("body", Value::text(&self.body))
            .with("send_date", Value::text(&self.sent))
            .with("team_id", Value::U32(self.team.0))
            .with("team_name", Value::text(&self.team.1))
    }

    fn to_json(&self) -> Json {
        json!({ "id": self.id, "kind": self.kind, "from": [self.from.0, self.from.1], "to": [self.to.0, self.to.1],
            "title": self.title, "body": self.body, "sent": self.sent, "team": [self.team.0, self.team.1] })
    }

    fn from_json(j: &Json) -> Option<Mail> {
        let pair = |k: &str| Some((j[k][0].as_u64()? as u32, j[k][1].as_str()?.to_string()));
        Some(Mail {
            id: j["id"].as_u64()? as u32,
            kind: j["kind"].as_u64()? as u32,
            from: pair("from")?,
            to: pair("to")?,
            title: j["title"].as_str()?.into(),
            body: j["body"].as_str()?.into(),
            sent: j["sent"].as_str()?.into(),
            team: pair("team").unwrap_or_default(),
        })
    }
}

#[derive(Clone, Default)]
pub struct Mailbox {
    last_id: u32,
    /// Mail not fetched yet, by recipient.
    waiting: BTreeMap<u32, Vec<Mail>>,
    file: Option<PathBuf>,
}

impl Mailbox {
    /// The mail kept in `file` (read now if it exists, written after each change).
    pub fn load(file: &Path) -> Mailbox {
        let saved = crate::store::read_or_keep(file, "mail");
        Self::from_saved(file, &saved)
    }

    pub(crate) fn try_load(file: &Path) -> std::io::Result<Mailbox> {
        Ok(Self::from_saved(file, &crate::store::read(file)?.unwrap_or_default()))
    }

    fn from_saved(file: &Path, saved: &Json) -> Mailbox {
        let mut mailbox = Mailbox { file: Some(file.to_path_buf()), ..Mailbox::default() };
        mailbox.last_id = saved["last_id"].as_u64().unwrap_or(0) as u32;
        for mail in saved["waiting"].as_array().into_iter().flatten().filter_map(Mail::from_json) {
            mailbox.waiting.entry(mail.to.0).or_default().push(mail);
        }
        mailbox
    }

    /// A deleted player's mail goes (what waits for them).
    pub fn forget(&mut self, player_id: u32) {
        if self.waiting.remove(&player_id).is_some() {
            self.save();
        }
    }

    /// A deleted account's mail goes from the mailboxes it was sent to, too.
    pub fn forget_from(&mut self, player_id: u32) {
        let before: usize = self.waiting.values().map(Vec::len).sum();
        self.waiting.values_mut().for_each(|m| m.retain(|mail| mail.from.0 != player_id));
        self.waiting.retain(|_, m| !m.is_empty());
        if self.waiting.values().map(Vec::len).sum::<usize>() != before {
            self.save();
        }
    }

    fn save(&self) {
        if let Some(file) = &self.file && let Err(e) = self.persist() {
            tracing::warn!("mail not saved to {}: {e}", file.display());
        }
    }

    /// Writes a prepared mailbox; callers publish it only after the enclosing transaction commits.
    pub(crate) fn persist(&self) -> std::io::Result<()> {
        let Some(file) = &self.file else { return Ok(()) };
        let waiting: Vec<Json> = self.waiting.values().flatten().map(Mail::to_json).collect();
        crate::store::write(file, &json!({ "last_id": self.last_id, "waiting": waiting }))
    }

    /// The existing Host BP letter, staged without saving or notifying a player (0x2581, front.rs).
    pub(crate) fn prepare_bp(&self, players: &[(u32, String)], amount: u32, total: u32) -> Self {
        let mut next = self.clone();
        let sent = (chrono::Utc::now() + chrono::Duration::hours(9)).format("%Y-%m-%d %H:%M:%S").to_string();
        for (id, name) in players {
            next.last_id += 1;
            let mail = Mail { id: next.last_id, from: (0, "Host".into()), to: (*id, name.clone()), title: "BP received".into(),
                body: format!("The host gave your account {amount} BP. You have {total} BP now."), sent: sent.clone(), ..Mail::default() };
            next.waiting.entry(*id).or_default().push(mail);
        }
        next
    }

    /// A staged Host gift letter, never saved or notified before the enclosing commit.
    pub(crate) fn prepare_host(&self, player: (u32, &str), title: &str, body: String) -> Result<Self, String> {
        let mut next = self.clone();
        next.last_id = next.last_id.checked_add(1).ok_or("The mail number limit has been reached.")?;
        let mail = Mail { id: next.last_id, from: (0, "Host".into()), to: (player.0, player.1.into()), title: title.into(), body,
            sent: (chrono::Utc::now() + chrono::Duration::hours(9)).format("%Y-%m-%d %H:%M:%S").to_string(), ..Mail::default() };
        next.waiting.entry(player.0).or_default().push(mail);
        Ok(next)
    }

    /// Mail waiting for a player.
    pub fn count(&self, player_id: u32) -> u32 {
        self.waiting.get(&player_id).map_or(0, Vec::len) as u32
    }

    /// Keeps a mail for its recipient; its id.
    pub fn post(&mut self, mut mail: Mail) -> u32 {
        self.last_id += 1;
        mail.id = self.last_id;
        self.waiting.entry(mail.to.0).or_default().push(mail);
        self.save();
        self.last_id
    }

    /// The fetch reply 0x2586 with a player's waiting mail, which is theirs to keep from then on.
    pub fn take(&mut self, player_id: u32) -> Message {
        let mail = self.waiting.remove(&player_id).unwrap_or_default();
        if !mail.is_empty() {
            self.save();
        }
        mail.iter().fold(Message::reply(0x2586), |m, mail| m.item(mail.record()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deleted_players_waiting_mail_goes() {
        let mut mailbox = Mailbox::default();
        let mail = |to: u32| Mail { id: to, to: (to, "x".into()), ..Mail::default() };
        mailbox.waiting.entry(7).or_default().push(mail(7));
        mailbox.waiting.entry(8).or_default().push(mail(8));
        mailbox.forget(7);
        assert!(!mailbox.waiting.contains_key(&7) && mailbox.waiting.contains_key(&8));
    }
}
