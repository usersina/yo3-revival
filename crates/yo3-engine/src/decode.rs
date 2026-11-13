//! The core's messages and query answers as JSON, field for field as ocgcore-wasm 0.1.6 parses them (its
//! `messages.ts` and `queries.ts`), which is what the server's translator reads. 64-bit values are kept exact.
use serde_json::{Map, Value as Json, json};

pub const LOCATION_OVERLAY: u32 = 0x80;
pub const POSITION_FACEUP_ATTACK: u32 = 0x1;

/// Ran out of bytes.
#[derive(Debug)]
pub struct Eof;

impl std::fmt::Display for Eof {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Error: eof")
    }
}

pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes, at: 0 }
    }
    pub fn avail(&self) -> usize {
        self.bytes.len() - self.at
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Eof> {
        if self.avail() < n {
            return Err(Eof);
        }
        let out = &self.bytes[self.at..self.at + n];
        self.at += n;
        Ok(out)
    }
    pub fn u8(&mut self) -> Result<u8, Eof> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, Eof> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> Result<u32, Eof> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> Result<i32, Eof> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> Result<u64, Eof> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn bool(&mut self) -> Result<bool, Eof> {
        Ok(self.u8()? != 0)
    }
    /// `n` items read by `f`.
    fn list(&mut self, n: usize, mut f: impl FnMut(&mut Self) -> Result<Json, Eof>) -> Result<Json, Eof> {
        // A count can't promise more items than bytes left; reading stops at the end like the wrapper's.
        let mut out = Vec::with_capacity(n.min(self.avail()));
        for _ in 0..n {
            out.push(f(self)?);
        }
        Ok(Json::Array(out))
    }
    fn list32(&mut self, f: impl FnMut(&mut Self) -> Result<Json, Eof>) -> Result<Json, Eof> {
        let n = self.u32()? as usize;
        self.list(n, f)
    }
    fn list8(&mut self, f: impl FnMut(&mut Self) -> Result<Json, Eof>) -> Result<Json, Eof> {
        let n = self.u8()? as usize;
        self.list(n, f)
    }
}

/// A card's place (`parseInfoLocation`): an Xyz material's is its Xyz Monster's location, face-up attack, with the
/// material's index as `overlay_sequence`.
pub fn place(r: &mut Reader) -> Result<Map<String, Json>, Eof> {
    let controller = r.u8()?;
    let location = r.u8()? as u32;
    let sequence = r.u32()?;
    let position = r.u32()?;
    let mut m = Map::new();
    m.insert("controller".into(), controller.into());
    if location & LOCATION_OVERLAY != 0 {
        m.insert("location".into(), (location & !LOCATION_OVERLAY).into());
        m.insert("sequence".into(), sequence.into());
        m.insert("position".into(), POSITION_FACEUP_ATTACK.into());
        m.insert("overlay_sequence".into(), position.into());
    } else {
        m.insert("location".into(), location.into());
        m.insert("sequence".into(), sequence.into());
        m.insert("position".into(), position.into());
    }
    Ok(m)
}

/// `isInfoLocationEmpty`.
pub fn place_empty(m: &Map<String, Json>) -> bool {
    ["controller", "location", "sequence", "position", "overlay_sequence"].iter().all(|k| m.get(*k).and_then(Json::as_u64).unwrap_or(0) == 0)
}

/// An object of `fields`, then `rest` merged in.
fn obj(fields: Vec<(&str, Json)>) -> Map<String, Json> {
    fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn with(mut m: Map<String, Json>, fields: Vec<(&str, Json)>) -> Map<String, Json> {
    for (k, v) in fields {
        m.insert(k.to_string(), v);
    }
    m
}

/// `{code, ...place}` then `fields`.
fn coded(r: &mut Reader, fields: impl FnOnce(&mut Reader) -> Result<Vec<(&'static str, Json)>, Eof>) -> Result<Json, Eof> {
    let code = r.u32()?;
    let mut m = obj(vec![("code", code.into())]);
    m.extend(place(r)?);
    Ok(Json::Object(with(m, fields(r)?)))
}

/// `{code, controller (u8), location (u8), sequence}` with a u32 or u8 sequence.
fn short_place(r: &mut Reader, wide_sequence: bool) -> Result<Json, Eof> {
    let code = r.u32()?;
    let controller = r.u8()?;
    let location = r.u8()?;
    let sequence = if wide_sequence { r.u32()? } else { r.u8()? as u32 };
    Ok(json!({ "code": code, "controller": controller, "location": location, "sequence": sequence }))
}

/// One message (`readMessage`); None for a type the wrapper doesn't know (it skips those).
pub fn message(r: &mut Reader) -> Result<Option<Json>, Eof> {
    let kind = r.u8()?;
    let t = || ("type", Json::from(kind));
    let m = match kind {
        1 | 34 | 37 | 61 | 63 | 65 | 74 | 112 | 113 | 114 | 121 | 122 | 123 | 180 => obj(vec![t()]),
        2 => {
            let hint_type = r.u8()?;
            let player = r.u8()?;
            let hint = if r.avail() > 4 { r.u64()? } else { r.u32()? as u64 };
            obj(vec![t(), ("hint_type", hint_type.into()), ("player", player.into()), ("hint", hint.into())])
        }
        5 => obj(vec![t(), ("player", r.u8()?.into()), ("reason", r.u8()?.into())]),
        10 => {
            let player = r.u8()?;
            let chains = r.list32(|r| {
                Ok(json!({ "code": r.u32()?, "controller": r.u8()?, "location": r.u8()?, "sequence": r.u32()?,
                    "description": r.u64()?, "client_mode": r.u8()? }))
            })?;
            let attacks = r.list32(|r| {
                Ok(json!({ "code": r.u32()?, "controller": r.u8()?, "location": r.u8()?, "sequence": r.u8()?, "can_direct": r.bool()? }))
            })?;
            obj(vec![t(), ("player", player.into()), ("chains", chains), ("attacks", attacks), ("to_m2", r.bool()?.into()), ("to_ep", r.bool()?.into())])
        }
        11 => {
            let player = r.u8()?;
            let summons = r.list32(|r| short_place(r, true))?;
            let special_summons = r.list32(|r| short_place(r, true))?;
            let pos_changes = r.list32(|r| short_place(r, false))?;
            let monster_sets = r.list32(|r| short_place(r, true))?;
            let spell_sets = r.list32(|r| short_place(r, true))?;
            let activates = r.list32(|r| {
                Ok(json!({ "code": r.u32()?, "controller": r.u8()?, "location": r.u8()?, "sequence": r.u32()?,
                    "description": r.u64()?, "client_mode": r.u8()? }))
            })?;
            obj(vec![t(), ("player", player.into()), ("summons", summons), ("special_summons", special_summons),
                ("pos_changes", pos_changes), ("monster_sets", monster_sets), ("spell_sets", spell_sets),
                ("activates", activates), ("to_bp", r.bool()?.into()), ("to_ep", r.bool()?.into()), ("shuffle", r.bool()?.into())])
        }
        12 => {
            let player = r.u8()?;
            let code = r.u32()?;
            let m = with(obj(vec![t(), ("player", player.into()), ("code", code.into())]), vec![]);
            let mut m = m;
            m.extend(place(r)?);
            with(m, vec![("description", r.u64()?.into())])
        }
        13 => obj(vec![t(), ("player", r.u8()?.into()), ("description", r.u64()?.into())]),
        14 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("options", r.list8(|r| Ok(r.u64()?.into()))?)])
        }
        15 | 20 => {
            let player = r.u8()?;
            let can_cancel = r.bool()?;
            let min = r.u32()?;
            let max = r.u32()?;
            let selects = if kind == 15 {
                r.list32(|r| coded(r, |_| Ok(vec![])))?
            } else {
                r.list32(|r| {
                    Ok(json!({ "code": r.u32()?, "controller": r.u8()?, "location": r.u8()?, "sequence": r.u32()?, "release_param": r.u8()? }))
                })?
            };
            obj(vec![t(), ("player", player.into()), ("can_cancel", can_cancel.into()), ("min", min.into()), ("max", max.into()), ("selects", selects)])
        }
        16 => {
            let player = r.u8()?;
            let spe_count = r.u8()?;
            let forced = r.bool()?;
            let hint_timing = r.u32()?;
            let hint_timing_other = r.u32()?;
            let selects = r.list32(|r| coded(r, |r| Ok(vec![("description", r.u64()?.into()), ("client_mode", r.u8()?.into())])))?;
            obj(vec![t(), ("player", player.into()), ("spe_count", spe_count.into()), ("forced", forced.into()),
                ("hint_timing", hint_timing.into()), ("hint_timing_other", hint_timing_other.into()), ("selects", selects)])
        }
        18 | 24 => obj(vec![t(), ("player", r.u8()?.into()), ("count", r.u8()?.into()), ("field_mask", r.u32()?.into())]),
        19 => obj(vec![t(), ("player", r.u8()?.into()), ("code", r.u32()?.into()), ("positions", r.u8()?.into())]),
        21 | 25 => {
            let player = r.u8()?;
            let cards = r.list32(|r| Ok(json!({ "code": r.u32()?, "controller": r.u8()?, "location": r.u32()?, "sequence": r.u32()? })))?;
            obj(vec![t(), ("player", player.into()), ("cards", cards)])
        }
        22 => {
            let player = r.u8()?;
            let counter_type = r.u16()?;
            let count = r.u16()?;
            let cards = r.list32(|r| {
                Ok(json!({ "code": r.u32()?, "controller": r.u8()?, "location": r.u8()?, "sequence": r.u8()?, "count": r.u16()? }))
            })?;
            obj(vec![t(), ("player", player.into()), ("counter_type", counter_type.into()), ("count", count.into()), ("cards", cards)])
        }
        23 => {
            let player = r.u8()?;
            let select_max = r.u8()?;
            let amount = r.u32()?;
            let min = r.u32()?;
            let max = r.u32()?;
            let must = r.list32(|r| coded(r, |r| Ok(vec![("amount", r.u32()?.into())])))?;
            let selects = r.list32(|r| coded(r, |r| Ok(vec![("amount", r.u32()?.into())])))?;
            obj(vec![t(), ("player", player.into()), ("select_max", select_max.into()), ("amount", amount.into()), ("min", min.into()),
                ("max", max.into()), ("selects_must", must), ("selects", selects)])
        }
        26 => {
            let player = r.u8()?;
            let can_finish = r.bool()?;
            let can_cancel = r.bool()?;
            let min = r.u32()?;
            let max = r.u32()?;
            let select_cards = r.list32(|r| coded(r, |_| Ok(vec![])))?;
            let unselect_cards = r.list32(|r| coded(r, |_| Ok(vec![])))?;
            obj(vec![t(), ("player", player.into()), ("can_finish", can_finish.into()), ("can_cancel", can_cancel.into()),
                ("min", min.into()), ("max", max.into()), ("select_cards", select_cards), ("unselect_cards", unselect_cards)])
        }
        30 | 31 | 42 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("cards", r.list32(|r| short_place(r, true))?)])
        }
        32 | 40 => obj(vec![t(), ("player", r.u8()?.into())]),
        33 | 39 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("cards", r.list32(|r| Ok(r.u32()?.into()))?)])
        }
        35 => {
            let player = r.u8()?;
            let deck_size = r.u32()?;
            let n = r.u32()? as usize;
            let bytes = r.take(n)?;
            let returned: Vec<Json> = (0..deck_size as usize)
                .filter(|i| bytes.get(i / 8).is_some_and(|b| b & (1 << (i % 8)) != 0))
                .map(|i| Json::from(i as u64))
                .collect();
            obj(vec![t(), ("player", player.into()), ("deck_size", deck_size.into()), ("returned_to_extra", Json::Array(returned))])
        }
        36 => {
            let location = r.u8()?;
            let cards = r.list8(|r| Ok(json!({ "from": Json::Object(place(r)?), "to": Json::Object(place(r)?) })))?;
            obj(vec![t(), ("location", location.into()), ("cards", cards)])
        }
        38 => obj(vec![t(), ("player", r.u8()?.into()), ("count", r.u32()?.into()), ("code", r.u32()?.into()), ("position", r.u32()?.into())]),
        41 => obj(vec![t(), ("phase", r.u16()?.into())]),
        50 => {
            let card = r.u32()?;
            let from = place(r)?;
            let to = place(r)?;
            obj(vec![t(), ("card", card.into()), ("from", Json::Object(from)), ("to", Json::Object(to))])
        }
        53 => obj(vec![t(), ("code", r.u32()?.into()), ("controller", r.u8()?.into()), ("location", r.u8()?.into()), ("sequence", r.u8()?.into()),
            ("prev_position", r.u8()?.into()), ("position", r.u8()?.into())]),
        54 | 60 | 62 | 64 => {
            let code = r.u32()?;
            let mut m = obj(vec![t(), ("code", code.into())]);
            m.extend(place(r)?);
            m
        }
        55 => {
            let card1 = coded(r, |_| Ok(vec![]))?;
            let card2 = coded(r, |_| Ok(vec![]))?;
            obj(vec![t(), ("card1", card1), ("card2", card2)])
        }
        56 => obj(vec![t(), ("field_mask", r.u32()?.into())]),
        70 => {
            let code = r.u32()?;
            let mut m = obj(vec![t(), ("code", code.into())]);
            m.extend(place(r)?);
            with(m, vec![("triggering_controller", r.u8()?.into()), ("triggering_location", r.u8()?.into()),
                ("triggering_sequence", r.u32()?.into()), ("description", r.u64()?.into()), ("chain_size", r.u32()?.into())])
        }
        71 | 72 | 73 | 75 | 76 => obj(vec![t(), ("chain_size", r.u8()?.into())]),
        80 | 83 | 190 => obj(vec![t(), ("cards", r.list32(|r| Ok(Json::Object(place(r)?)))?)]),
        81 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("cards", r.list32(|r| Ok(Json::Object(place(r)?)))?)])
        }
        90 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("drawn", r.list32(|r| Ok(json!({ "code": r.u32()?, "position": r.u32()? })))?)])
        }
        91 | 92 | 100 => obj(vec![t(), ("player", r.u8()?.into()), ("amount", r.u32()?.into())]),
        94 => obj(vec![t(), ("player", r.u8()?.into()), ("lp", r.u32()?.into())]),
        93 | 96 | 97 => {
            let card = place(r)?;
            let target = place(r)?;
            obj(vec![t(), ("card", Json::Object(card)), ("target", Json::Object(target))])
        }
        101 | 102 => obj(vec![t(), ("counter_type", r.u16()?.into()), ("controller", r.u8()?.into()), ("location", r.u8()?.into()),
            ("sequence", r.u8()?.into()), ("count", r.u16()?.into())]),
        110 => {
            let card = place(r)?;
            let target = place(r)?;
            let target = if place_empty(&target) { Json::Null } else { Json::Object(target) };
            obj(vec![t(), ("card", Json::Object(card)), ("target", target)])
        }
        111 => {
            let side = |r: &mut Reader| -> Result<Json, Eof> {
                let m = place(r)?;
                Ok(Json::Object(with(m, vec![("attack", r.u32()?.into()), ("defense", r.u32()?.into()), ("destroyed", r.bool()?.into())])))
            };
            let card = side(r)?;
            let target = side(r)?;
            obj(vec![t(), ("card", card), ("target", target)])
        }
        120 => {
            let mut m = obj(vec![t()]);
            m.extend(place(r)?);
            with(m, vec![("code", r.u32()?.into())])
        }
        130 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("results", r.list8(|r| Ok(r.bool()?.into()))?)])
        }
        131 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("results", r.list8(|r| Ok(r.u8()?.into()))?)])
        }
        132 => obj(vec![t(), ("player", r.u8()?.into())]),
        133 => {
            let result = r.u8()?;
            obj(vec![t(), ("results", json!([result & 0b11, (result >> 2) & 0b11]))])
        }
        140 => obj(vec![t(), ("player", r.u8()?.into()), ("count", r.u8()?.into()), ("available", r.u64()?.into())]),
        141 => obj(vec![t(), ("player", r.u8()?.into()), ("count", r.u8()?.into()), ("available", r.u8()?.into())]),
        142 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("opcodes", r.list8(|r| Ok(r.u64()?.into()))?)])
        }
        143 => {
            let player = r.u8()?;
            obj(vec![t(), ("player", player.into()), ("options", r.list8(|r| Ok(r.u64()?.into()))?)])
        }
        160 => {
            let mut m = obj(vec![t()]);
            m.extend(place(r)?);
            with(m, vec![("card_hint", r.u8()?.into()), ("description", r.u64()?.into())])
        }
        161 => {
            let player = r.u8()?;
            let deck_size = r.u32()?;
            let extra_size = r.u32()?;
            let extra_faceup_count = r.u32()?;
            let hand_length = r.u32()?;
            let deck_top_card = r.u32()?;
            let hand = r.list(hand_length as usize, |r| Ok(json!({ "code": r.u32()?, "position": r.u32()? })))?;
            let extra = r.list(extra_size as usize, |r| Ok(json!({ "code": r.u32()?, "position": r.u32()? })))?;
            obj(vec![t(), ("player", player.into()), ("deck_size", deck_size.into()), ("extra_faceup_count", extra_faceup_count.into()),
                ("deck_top_card", if deck_top_card == 0 { Json::Null } else { deck_top_card.into() }), ("hand", hand), ("extra", extra)])
        }
        162 => {
            let mut m = obj(vec![t()]);
            m.extend(field(r)?);
            m
        }
        163 | 164 => {
            let n = r.u16()? as usize;
            let text = text(r.take(n)?);
            obj(vec![t(), (if kind == 163 { "name" } else { "hint" }, text.into())])
        }
        165 => obj(vec![t(), ("player", r.u8()?.into()), ("player_hint", r.u8()?.into()), ("description", r.u64()?.into())]),
        170 => obj(vec![t(), ("card", r.u32()?.into())]),
        _ => return Ok(None),
    };
    Ok(Some(Json::Object(m)))
}

/// TextDecoder's UTF-8: invalid bytes replaced, a leading byte order mark dropped.
fn text(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    s.strip_prefix('\u{feff}').unwrap_or(&s).to_string()
}

/// The field (`readField`): the duel's flags, both players and the chain.
pub fn field(r: &mut Reader) -> Result<Map<String, Json>, Eof> {
    let flags = r.u32()?;
    let mut players = vec![];
    for _ in 0..2 {
        let lp = r.u32()?;
        let zone = |r: &mut Reader, n: usize| -> Result<Json, Eof> {
            r.list(n, |r| Ok(if r.u8()? != 0 { json!({ "position": r.u8()?, "materials": r.u32()? }) } else { Json::Null }))
        };
        let monsters = zone(r, 7)?;
        let spells = zone(r, 8)?;
        players.push(json!({ "lp": lp, "monsters": monsters, "spells": spells, "deck_size": r.u32()?, "hand_size": r.u32()?,
            "grave_size": r.u32()?, "banish_size": r.u32()?, "extra_size": r.u32()?, "extra_faceup_count": r.u32()? }));
    }
    let chain = r.list32(|r| {
        coded(r, |r| Ok(vec![("triggering_controller", r.u8()?.into()), ("triggering_location", r.u8()?.into()),
            ("triggering_sequence", r.u32()?.into()), ("description", r.u64()?.into())]))
    })?;
    Ok(obj(vec![("flags", flags.into()), ("players", Json::Array(players)), ("chain", chain)]))
}

/// The core's messages from one `OCG_DuelGetMessage`: each a u32 length and the message. A message of a type the
/// wrapper doesn't know is skipped (with a note on stderr, as the wrapper warns).
pub fn messages(bytes: &[u8]) -> Result<Vec<Json>, Eof> {
    let mut r = Reader::new(bytes);
    let mut out = vec![];
    while r.avail() > 0 {
        let length = r.i32()?;
        let body = r.take(length.max(0) as usize)?;
        match message(&mut Reader::new(body))? {
            Some(m) => out.push(m),
            None => eprintln!("failed to parse a message: {}", body.first().copied().unwrap_or(0)),
        }
    }
    Ok(out)
}

// The query flags the engine asks for (ocgapi_constants.h).
pub const QUERY_CODE: u32 = 0x1;
pub const QUERY_POSITION: u32 = 0x2;
pub const QUERY_LEVEL: u32 = 0x10;
pub const QUERY_ATTRIBUTE: u32 = 0x40;
/// 64-bit in this core (card.cpp `CHECK_AND_INSERT_T(QUERY_RACE, get_race(), uint64_t)`); the low half is kept.
pub const QUERY_RACE: u32 = 0x80;
pub const QUERY_ATTACK: u32 = 0x100;
pub const QUERY_DEFENSE: u32 = 0x200;
pub const QUERY_BASE_ATTACK: u32 = 0x400;
pub const QUERY_BASE_DEFENSE: u32 = 0x800;
pub const QUERY_REASON: u32 = 0x1000;
pub const QUERY_COUNTERS: u32 = 0x20000;
const QUERY_END: u32 = 0x80000000;

/// One card's answer to a query: the 32-bit fields the engine asks for by flag, and its counters as (type, count).
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Queried {
    pub fields: Vec<(u32, u32)>,
    pub counters: Vec<(u16, u16)>,
}

impl Queried {
    pub fn get(&self, flag: u32) -> Option<u32> {
        self.fields.iter().find(|(f, _)| *f == flag).map(|(_, v)| *v)
    }
}

/// One card's query (`readQuery`); None for an empty place.
pub fn query(r: &mut Reader) -> Result<Option<Queried>, Eof> {
    let mut out = Queried::default();
    while r.avail() > 0 {
        let size = r.u16()? as usize;
        if size == 0 {
            return Ok(None);
        }
        if size < 4 {
            break;
        }
        let flag = r.u32()?;
        let body = r.take(size - 4)?;
        if flag == QUERY_END {
            break;
        }
        let mut b = Reader::new(body);
        if flag == QUERY_COUNTERS {
            let n = b.u32()?;
            for _ in 0..n {
                let kind = b.u16()?;
                let count = b.u16()?;
                out.counters.push((kind, count));
            }
        } else if body.len() == 4 {
            out.fields.push((flag, b.u32()?));
        } else if body.len() == 8 {
            // A 64-bit value (the race): every race of the client's era is in the low half.
            out.fields.push((flag, b.u32()?));
        }
    }
    Ok(Some(out))
}

/// A location's queries (`readQueryLocation`): one per card or slot, None where a slot is empty.
pub fn query_location(bytes: &[u8]) -> Result<Vec<Option<Queried>>, Eof> {
    let mut r = Reader::new(bytes);
    if bytes.is_empty() {
        return Ok(vec![]);
    }
    r.u32()?;
    let mut out = vec![];
    while r.avail() > 0 {
        out.push(query(&mut r)?);
    }
    Ok(out)
}
