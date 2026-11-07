//! D-Mail (MailWindow 0x4AC6B0): a mail waits here until its recipient's mail window fetches it, after which the
//! client keeps it in its own files; kept in a file next to the accounts.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};

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

#[derive(Default)]
pub struct Mailbox {
    last_id: u32,
    /// Mail not fetched yet, by recipient.
    waiting: BTreeMap<u32, Vec<Mail>>,
    file: Option<PathBuf>,
}

impl Mailbox {
    /// The mail kept in `file` (read now if it exists, written after each change).
    pub fn load(file: &Path) -> Mailbox {
        let mut mailbox = Mailbox { file: Some(file.to_path_buf()), ..Mailbox::default() };
        let saved = match crate::store::read(file) {
            Ok(saved) => saved.unwrap_or(Json::Null),
            Err(e) => {
                tracing::warn!("mail not read: {e}");
                Json::Null
            }
        };
        mailbox.last_id = saved["last_id"].as_u64().unwrap_or(0) as u32;
        for mail in saved["waiting"].as_array().into_iter().flatten().filter_map(Mail::from_json) {
            mailbox.waiting.entry(mail.to.0).or_default().push(mail);
        }
        mailbox
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        let waiting: Vec<Json> = self.waiting.values().flatten().map(Mail::to_json).collect();
        if let Err(e) = crate::store::write(file, &json!({ "last_id": self.last_id, "waiting": waiting })) {
            tracing::warn!("mail not saved to {}: {e}", file.display());
        }
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
