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

use std::path::{Path, PathBuf};

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
        format!("{:010}-{}-{}.dlog", self.end_time, self.duel_id, self.player_1)
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

    /// Writes the log into `dir`, returning its file name.
    pub fn save(&self, dir: &Path) -> std::io::Result<String> {
        std::fs::create_dir_all(dir)?;
        let name = self.file_name();
        std::fs::write(dir.join(&name), self.to_bytes())?;
        Ok(name)
    }
}

/// A saved log's path, if `name` is a plain file name of the directory (no path separators).
pub fn path_of(dir: &Path, name: &str) -> Option<PathBuf> {
    let plain = !name.is_empty() && name.ends_with(".dlog") && !name.contains(['/', '\\']) && !name.starts_with('.');
    plain.then(|| dir.join(name))
}
