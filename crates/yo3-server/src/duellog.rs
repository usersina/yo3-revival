//! Duel logs: the `.dlog` file the client downloads after a duel (the `dlogURL` of the `0x2009` result) and keeps in
//! `players\<name>\duellogs\myduels\`, where Main Menu > Duel Log > Latest Duel Logs lists it.
//!
//! Layout (the loaders `0x538AB0` header only, `0x5384F0` everything), integers little-endian:
//!
//! | offset | size | field |
//! |---|---|---|
//! | 0 | u16 | magic `0x09E7`, then u16 version 1 |
//! | 4 | u32 | unknown (the loaders keep it; **guess** 0) |
//! | 8 | u32 | `duel_id` |
//! | 12 | u32 | `player_1` |
//! | 16 | u32 | `player_2` |
//! | 20 | u32 | `match_type` |
//! | 24 | u32 | `dlv_1`, then u32 `dlv_2` at 28 |
//! | 32 | u32 | `end_time` (epoch) |
//! | 36 | 32 | `pname_1`, then `pname_2` at 68, NUL padded |
//! | 100 | 256 | `dlog_comment` |
//! | 356 | u32 | kept by the comment rewrite (`0x538BD0`); **guess** the body's length |
//! | 360 | | the body, run-length coded as (value, count) byte pairs (`0x538360`) |
//!
//! The body (`0x5384F0`, then `0x524330` which makes a match push and a duel setup of it) is, before the coding: 20 u32s
//! (the third is `duel_id`, the fourth the end time (`0x17C`), the fifth `match_type`, the seventh `event_type`, the
//! eighth `official`; the rest unknown, 0), two player blocks of 8656 bytes each, then a u32 count and that many
//! commands of six u32s `{id, 0, param1..param4}` (the engine commands of the duel pump `0x522A50`; entries whose second
//! u32's low byte is negative are skipped by the loader). A block is 11 u32s `{player_id, dlv, MainDeckNum,
//! ExtraDeckNum, title, ?, PlayerType, card_protector, n_win, n_lose, n_draw}`, `pname` (32 bytes), the avatar (128
//! bytes), `country` (4), the 32 speech lines (256 bytes each) and 256 bytes (the deck's name).

use std::path::Path;

const MAGIC: u16 = 0x09E7;
const NAME: usize = 32;
const COMMENT: usize = 256;

/// What a duel's log holds.
#[derive(Clone, Debug, Default)]
pub struct Log {
    pub duel_id: u32,
    pub player_1: u32,
    pub player_2: u32,
    pub match_type: u32,
    pub dlv_1: u32,
    pub dlv_2: u32,
    pub end_time: u32,
    pub pname_1: String,
    pub pname_2: String,
    pub comment: String,
    /// The player whose copy this is (it is in the file's name).
    pub owner: u32,
    /// The body before run-length coding.
    pub body: Vec<u8>,
}

fn field(out: &mut Vec<u8>, text: &str, size: usize) {
    let mut bytes = text.as_bytes().to_vec();
    // Keep a NUL at the end: the client reads the field as a C string.
    bytes.truncate(size - 1);
    bytes.resize(size, 0);
    out.extend(bytes);
}

/// One player of a replay.
#[derive(Clone, Debug, Default)]
pub struct Side {
    pub player_id: u32,
    pub level: u32,
    pub title: u32,
    pub main: u32,
    pub extra: u32,
    pub player_type: u32,
    pub protector: u32,
    pub record: [u32; 3],
    pub name: String,
    pub avatar: Vec<u8>,
    pub speeches: Vec<String>,
}

/// A duel to replay: both players (the one who went first first) and the commands the client saw.
#[derive(Clone, Debug, Default)]
pub struct Replay {
    pub sides: [Side; 2],
    pub commands: Vec<[u32; 6]>,
}

const BLOCK: usize = 0x21D0;

impl Replay {
    /// The body of the file, before the run-length coding.
    pub fn body(&self, duel_id: u32, match_type: u32, end_time: u32) -> Vec<u8> {
        let mut out = vec![];
        let mut ints = [0u32; 20];
        ints[4] = duel_id;
        ints[5] = end_time;
        ints[6] = match_type;
        out.extend(ints.iter().flat_map(|n| n.to_le_bytes()));
        for side in &self.sides {
            let start = out.len();
            let [wins, losses, draws] = side.record;
            let head = [side.player_id, side.level, side.main, side.extra, side.title, 0, side.player_type, side.protector, wins, losses, draws];
            out.extend(head.iter().flat_map(|n| n.to_le_bytes()));
            field(&mut out, &side.name, NAME);
            let mut avatar = side.avatar.clone();
            avatar.resize(128, 0);
            out.extend(avatar);
            out.extend([0; 4]);
            for line in 0..32 {
                field(&mut out, side.speeches.get(line).map_or("", String::as_str), 256);
            }
            out.extend([0; 256]);
            debug_assert_eq!(out.len() - start, BLOCK);
        }
        out.extend((self.commands.len() as u32).to_le_bytes());
        out.extend(self.commands.iter().flatten().flat_map(|n| n.to_le_bytes()));
        out
    }
}

/// The client's run-length coding (`0x538360`): (value, count) pairs, runs up to 255.
pub fn pack(body: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    let mut at = 0;
    while at < body.len() {
        let value = body[at];
        let run = body[at..].iter().take(255).take_while(|&&b| b == value).count();
        out.extend([value, run as u8]);
        at += run;
    }
    out
}

impl Log {
    /// The file's name: sorted by name the logs come in time order. Each player gets their own file (the same duel's
    /// two logs differ in who is first).
    pub fn file_name(&self) -> String {
        format!("{:010}-{}-{}.dlog", self.end_time, self.duel_id, self.owner)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_le_bytes().to_vec();
        out.extend(1u16.to_le_bytes());
        let ints = [0, self.duel_id, self.player_1, self.player_2, self.match_type, self.dlv_1, self.dlv_2, self.end_time];
        out.extend(ints.iter().flat_map(|n| n.to_le_bytes()));
        field(&mut out, &self.pname_1, NAME);
        field(&mut out, &self.pname_2, NAME);
        field(&mut out, &self.comment, COMMENT);
        out.extend((self.body.len() as u32).to_le_bytes());
        out.extend(pack(&self.body));
        out
    }

    /// Keeps the log in the server's database (`store::put_duel_log`) in data folder `dir`, returning its name.
    pub fn save(&self, dir: &Path) -> std::io::Result<String> {
        let name = self.file_name();
        crate::store::put_duel_log(dir, &name, &self.to_bytes())?;
        Ok(name)
    }
}

/// Whether `name` can be a kept log's name: a plain `.dlog` file name, as the download URL carries it.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.ends_with(".dlog") && !name.contains(['/', '\\']) && !name.starts_with('.')
}

/// The engine commands among duel messages, as the log keeps them: `{id, 0, param1..param4}`.
pub fn commands(messages: &[yo3_proto::Message]) -> Vec<[u32; 6]> {
    use yo3_proto::Value;
    messages
        .iter()
        .filter(|m| m.get("duelRequestName") == Some(&Value::U32(0x11)))
        .map(|m| {
            let int = |k: &str| m.get(k).and_then(Value::as_u32).unwrap_or(0);
            [int("id"), 0, int("param1"), int("param2"), int("param3"), int("param4")]
        })
        .collect()
}
