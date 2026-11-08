//! Card rental between players (CardRental.cpp, handler 0x41C7E0; the records RentalRecord.cpp, 0x482090): a lender
//! (the player who sent the request) puts up cards, sets the conditions (Free: any cards, Deck: a main, extra and side
//! deck; 1, 3, 7 or 14 days), and the borrower confirms. The rental is then kept here until its end: the client
//! counts the cards itself from the records (0x2D6C, collection builder 0x4F6340): a record's cards are added to the
//! collection of its `to_id` and taken from the one of its `from_id`, while the record's confirm flag is 0.
//! The server therefore never moves cards between the collections: it keeps the records.
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset, Utc};
use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};

/// The rental terms the lender's dialog offers (texts 14212-14215), in days.
pub const TERMS: [u32; 4] = [1, 3, 7, 14];
/// How many rentals of each side the history lists carry (the screen scrolls; nothing asks for a next page).
const HISTORY: usize = 50;

/// A rental under way in the scene: its two players (the lender first), whether their scenes are ready, the
/// conditions and cards the lender chose, and whether the borrower is being asked to confirm.
#[derive(Debug)]
pub struct Session {
    pub players: [(u32, String); 2],
    pub ready: [bool; 2],
    /// 0 Free (any cards, `main` only), 1 Deck (main, extra and side).
    pub kind: u32,
    pub days: u32,
    /// The cards offered, as CARD_Prop rows: main, extra, side.
    pub lists: [Vec<u16>; 3],
    pub offered: bool,
}

impl Session {
    pub fn new(lender: (u32, String), borrower: (u32, String)) -> Session {
        Session { players: [lender, borrower], ready: [false; 2], kind: 0, days: TERMS[0], lists: Default::default(), offered: false }
    }

    /// The seat of a player in it (0 the lender).
    pub fn seat(&self, player_id: u32) -> Option<usize> {
        self.players.iter().position(|(p, _)| *p == player_id)
    }

    /// Every card offered.
    pub fn cards(&self) -> impl Iterator<Item = u16> + '_ {
        self.lists.iter().flatten().copied()
    }
}

/// A rental made: the lender's cards are the borrower's until `end`.
#[derive(Clone, Debug, PartialEq)]
pub struct Rental {
    pub id: u32,
    pub lender: (u32, String),
    pub borrower: (u32, String),
    pub kind: u32,
    /// Unix seconds.
    pub start: i64,
    pub end: i64,
    /// Over: the time ran out (the records' confirm flags are 1 then).
    pub over: bool,
    pub lists: [Vec<u16>; 3],
}

/// "YYYY-MM-DD HH:MM:SS" in Japan time, like every server time.
fn jst(unix: i64) -> String {
    let at: DateTime<FixedOffset> = DateTime::<Utc>::from_timestamp(unix, 0).unwrap_or_default().with_timezone(&FixedOffset::east_opt(9 * 3600).expect("valid offset"));
    at.format("%Y-%m-%d %H:%M:%S").to_string()
}

impl Rental {
    /// The record both lists carry (0x2D6C items, read by the collection builder 0x4F6340 and the shop; the rows of the
    /// Rental Card List, 0x482A90 for the borrowed and 0x483100 for the lent, and its card view 0x483620).
    pub fn record(&self) -> Message {
        let over = self.over as u32;
        let [main, extra, side] = &self.lists;
        Message::record()
            .with("from_id", Value::U32(self.lender.0))
            .with("from_name", Value::text(&self.lender.1))
            .with("from_type", Value::U32(0))
            .with("to_id", Value::U32(self.borrower.0))
            .with("to_name", Value::text(&self.borrower.1))
            .with("rent_end", Value::text(&jst(self.end)))
            .with("deck_type", Value::U32(self.kind))
            .with("confirm_from", Value::U32(over))
            .with("confirm_to", Value::U32(over))
            .with("main", Value::u16_be_array(main))
            .with("extra", Value::u16_be_array(extra))
            .with("side", Value::u16_be_array(side))
    }

    fn to_json(&self) -> Json {
        json!({ "id": self.id, "lender": [self.lender.0, self.lender.1], "borrower": [self.borrower.0, self.borrower.1],
            "kind": self.kind, "start": self.start, "end": self.end, "over": self.over, "lists": self.lists })
    }

    fn from_json(j: &Json) -> Option<Rental> {
        let pair = |k: &str| Some((j[k][0].as_u64()? as u32, j[k][1].as_str()?.to_string()));
        let list = |i: usize| -> Vec<u16> { j["lists"][i].as_array().into_iter().flatten().filter_map(|v| v.as_u64()).map(|v| v as u16).collect() };
        Some(Rental {
            id: j["id"].as_u64()? as u32,
            lender: pair("lender")?,
            borrower: pair("borrower")?,
            kind: j["kind"].as_u64()? as u32,
            start: j["start"].as_i64()?,
            end: j["end"].as_i64()?,
            over: j["over"].as_bool().unwrap_or(false),
            lists: [list(0), list(1), list(2)],
        })
    }
}

/// Every rental, kept in a file next to the accounts.
#[derive(Default)]
pub struct Rentals {
    last_id: u32,
    all: Vec<Rental>,
    file: Option<PathBuf>,
}

impl Rentals {
    /// The rentals kept in `file` (read now if it exists, written after each change).
    pub fn load(file: &Path) -> Rentals {
        let mut rentals = Rentals { file: Some(file.to_path_buf()), ..Rentals::default() };
        let saved = match crate::store::read(file) {
            Ok(saved) => saved.unwrap_or(Json::Null),
            Err(e) => {
                tracing::warn!("rentals not read: {e}");
                Json::Null
            }
        };
        rentals.all = saved["rentals"].as_array().into_iter().flatten().filter_map(Rental::from_json).collect();
        rentals.last_id = rentals.all.iter().map(|r| r.id).max().unwrap_or(0).max(saved["last_id"].as_u64().unwrap_or(0) as u32);
        rentals
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        let all: Vec<Json> = self.all.iter().map(Rental::to_json).collect();
        if let Err(e) = crate::store::write(file, &json!({ "last_id": self.last_id, "rentals": all })) {
            tracing::warn!("rentals not saved to {}: {e}", file.display());
        }
    }

    /// Starts a rental of a session's cards at `now` (unix seconds); its id.
    pub fn start(&mut self, s: &Session, now: i64) -> u32 {
        self.last_id += 1;
        let [lender, borrower] = s.players.clone();
        let end = now + i64::from(s.days) * 86_400;
        self.all.push(Rental { id: self.last_id, lender, borrower, kind: s.kind, start: now, end, over: false, lists: s.lists.clone() });
        self.save();
        self.last_id
    }

    /// Ends the rentals whose time is up at `now`; the players concerned (lender, borrower) of each.
    pub fn expire(&mut self, now: i64) -> Vec<(u32, u32)> {
        let mut ended = vec![];
        for r in self.all.iter_mut().filter(|r| !r.over && r.end <= now) {
            r.over = true;
            ended.push((r.lender.0, r.borrower.0));
        }
        if !ended.is_empty() {
            self.save();
        }
        ended
    }

    /// The rentals not over.
    pub fn active(&self) -> impl Iterator<Item = &Rental> {
        self.all.iter().filter(|r| !r.over)
    }

    /// The cards a player has lent out now (they can't offer them again: the trade scene's collection takes them
    /// away, flag 8 of 0x4F6340).
    pub fn lent_by(&self, player_id: u32) -> Vec<u16> {
        self.active().filter(|r| r.lender.0 == player_id).flat_map(|r| r.lists.iter().flatten().copied()).collect()
    }

    /// The cards a player has borrowed now: the client adds them to the player's collection (0x4F6340).
    pub fn borrowed_by(&self, player_id: u32) -> Vec<u16> {
        self.active().filter(|r| r.borrower.0 == player_id).flat_map(|r| r.lists.iter().flatten().copied()).collect()
    }

    /// The records pushed to a player (0x2D6C, kept in dataset slot 8): the rentals not over that they lent or
    /// borrowed. The client works its collection out from them.
    pub fn records_for(&self, player_id: u32) -> Message {
        self.active().filter(|r| r.lender.0 == player_id || r.borrower.0 == player_id).fold(Message::reply(0x2D6C), |m, r| m.item(r.record()))
    }

    /// The Rental Card List's history (0x2D6E for 0x2C35 {rental_kind}): 0 the rentals the player lent (the client
    /// keeps that reply in the vector `GetLoanCount` counts, +0x78), 1 the ones they borrowed (`GetBorrowCount`'s,
    /// +0x90), the oldest of the newest few first (the client reverses what it gets).
    pub fn history_for(&self, player_id: u32, kind: u32) -> Message {
        let mine: Vec<&Rental> = self.all.iter().filter(|r| if kind == 1 { r.borrower.0 == player_id } else { r.lender.0 == player_id }).collect();
        let skip = mine.len().saturating_sub(HISTORY);
        mine.into_iter().skip(skip).fold(Message::reply(0x2D6E), |m, r| m.item(r.record()))
    }
}
