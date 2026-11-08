//! Wire format of Yu-Gi-Oh! ONLINE 3 (Konami "EasyMultiPlay").
//!
//! Frame: `u32 BE (len(body) ^ 0x77777777)` then the body.
//! Body:  Blowfish-ECB with key `"12345678"`, each 32-bit word read little-endian
//!        (Konami byte-swaps every word around a standard big-endian Blowfish), plaintext
//!        zero-padded to 8 bytes.
//! Message: `u8 tag | u16 nfields | nfields × (u8 namelen, name, value) | u16 nitems | items`,
//!        all integers big-endian. Values: 5/6 = u32, 7/9 = text, 8 = binary (each text/binary
//!        is `u32 len` + bytes). Items are nested messages. Requests and replies use tag 4,
//!        records inside lists tag 3.
//! The client opens every connection with a bare `00 <u8 len> <version>` string.
//!
//! The client's `.y3d/.y3a/.y3r` data files use the same frame and message layout.

use blowfish::BlowfishLE;
use blowfish::cipher::{Block, BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};

pub const KEY: &[u8; 8] = b"12345678";
pub const LEN_MASK: u32 = 0x7777_7777;
pub const MAX_FRAME: usize = 1 << 20;

pub const TAG_MESSAGE: u8 = 4;
pub const TAG_RECORD: u8 = 3;
/// The client builds a deck object (YO3Deck) for items with this tag (factory 0x531B90).
pub const TAG_DECK: u8 = 6;
/// The client builds a YO3DuelData for items with this tag, which finds its children "gameServer", "regulation",
/// "rule", "players" and "result" by name (0x5315C0).
pub const TAG_DUEL_DATA: u8 = 7;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("truncated at byte {at}: need {need} more")]
    Truncated { at: usize, need: usize },
    #[error("unknown value tag {tag:#x} at byte {at}")]
    UnknownValueTag { tag: u8, at: usize },
    #[error("encrypted body of {0} bytes is not a whole number of blocks")]
    BadBodyLength(usize),
    #[error("version string is not valid text")]
    BadVersion,
    #[error("messages nested deeper than {MAX_DEPTH} levels")]
    TooDeep,
}

/// How deep messages may nest on decode (the client's real ones nest about 3 deep). Decoding recurses, and so do
/// `Drop` and the packet log, so an unbounded depth lets one frame overflow the stack.
pub const MAX_DEPTH: usize = 16;

fn cipher() -> BlowfishLE {
    BlowfishLE::new_from_slice(KEY).expect("8-byte key is valid")
}

/// Encrypts `plain`, zero-padding it to the 8-byte block size.
pub fn encrypt(plain: &[u8]) -> Vec<u8> {
    let bf = cipher();
    let mut out = plain.to_vec();
    out.resize(plain.len().div_ceil(8) * 8, 0);
    for chunk in out.as_chunks_mut::<8>().0 {
        let mut block = Block::<BlowfishLE>::from(*chunk);
        bf.encrypt_block(&mut block);
        *chunk = block.into();
    }
    out
}

/// Decrypts a body; the result keeps any zero padding.
pub fn decrypt(body: &[u8]) -> Result<Vec<u8>, Error> {
    if !body.len().is_multiple_of(8) {
        return Err(Error::BadBodyLength(body.len()));
    }
    let bf = cipher();
    let mut out = body.to_vec();
    for chunk in out.as_chunks_mut::<8>().0 {
        let mut block = Block::<BlowfishLE>::from(*chunk);
        bf.decrypt_block(&mut block);
        *chunk = block.into();
    }
    Ok(out)
}

/// Encrypts `plain` and prefixes the masked length header.
pub fn frame(plain: &[u8]) -> Vec<u8> {
    let body = encrypt(plain);
    let mut out = ((body.len() as u32) ^ LEN_MASK).to_be_bytes().to_vec();
    out.extend(body);
    out
}

/// Body length announced by a frame header.
pub fn frame_len(header: [u8; 4]) -> usize {
    (u32::from_be_bytes(header) ^ LEN_MASK) as usize
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Tag 5: u32 (the usual integer).
    U32(u32),
    /// Tag 6: u32, seen in data files.
    U32Alt(u32),
    /// Tag 7: text; the client reads it as a C string (stops at the first NUL).
    Text(Vec<u8>),
    /// Tag 8: binary; NULs are kept.
    Bin(Vec<u8>),
    /// Tag 9: text flagged as binary; still read as a C string.
    TextAlt(Vec<u8>),
}

impl Value {
    pub fn text(s: &str) -> Self {
        Value::Text(s.as_bytes().to_vec())
    }

    pub fn u16_be_array(values: &[u16]) -> Self {
        Value::Bin(values.iter().flat_map(|v| v.to_be_bytes()).collect())
    }

    pub fn tag(&self) -> u8 {
        match self {
            Value::U32(_) => 5,
            Value::U32Alt(_) => 6,
            Value::Text(_) => 7,
            Value::Bin(_) => 8,
            Value::TextAlt(_) => 9,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Value::U32(v) | Value::U32Alt(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Text(b) | Value::Bin(b) | Value::TextAlt(b) => Some(b),
            _ => None,
        }
    }

    /// Text up to the first NUL, as the client would read it.
    pub fn as_str(&self) -> Option<String> {
        match self {
            Value::Text(b) | Value::TextAlt(b) => {
                let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
                Some(String::from_utf8_lossy(&b[..end]).into_owned())
            }
            _ => None,
        }
    }
}

/// A key/value record with ordered fields and nested items.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Message {
    pub tag: u8,
    pub fields: Vec<(String, Value)>,
    pub items: Vec<Message>,
}

impl Message {
    /// A top-level message whose first field is `request = id`.
    pub fn reply(request: u32) -> Self {
        Message { tag: TAG_MESSAGE, fields: vec![("request".into(), Value::U32(request))], items: vec![] }
    }

    /// A record (tag 3) for use as a list item.
    pub fn record() -> Self {
        Message { tag: TAG_RECORD, ..Default::default() }
    }

    /// A deck (tag 6) for use as a list item.
    pub fn deck() -> Self {
        Message { tag: TAG_DECK, ..Default::default() }
    }

    /// Duel data (tag 7) for use as a list item.
    pub fn duel_data() -> Self {
        Message { tag: TAG_DUEL_DATA, ..Default::default() }
    }

    pub fn with(mut self, name: &str, value: Value) -> Self {
        self.fields.push((name.into(), value));
        self
    }

    pub fn item(mut self, item: Message) -> Self {
        self.items.push(item);
        self
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    pub fn request(&self) -> Option<u32> {
        self.get("request").and_then(Value::as_u32)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_into(&mut out);
        out
    }

    fn encode_into(&self, out: &mut Vec<u8>) {
        out.push(self.tag);
        out.extend((self.fields.len() as u16).to_be_bytes());
        for (name, value) in &self.fields {
            out.push(name.len() as u8);
            out.extend(name.as_bytes());
            out.push(value.tag());
            match value {
                Value::U32(v) | Value::U32Alt(v) => out.extend(v.to_be_bytes()),
                Value::Text(b) | Value::Bin(b) | Value::TextAlt(b) => {
                    out.extend((b.len() as u32).to_be_bytes());
                    out.extend(b);
                }
            }
        }
        out.extend((self.items.len() as u16).to_be_bytes());
        for item in &self.items {
            item.encode_into(out);
        }
    }

    /// Decodes one message; returns it and the number of bytes used.
    pub fn decode(data: &[u8]) -> Result<(Message, usize), Error> {
        let mut r = Reader { data, pos: 0 };
        let msg = r.message(0)?;
        Ok((msg, r.pos))
    }
}

/// What a client can send: its opening version string, or a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Version(String),
    Message(Message),
}

impl Packet {
    /// Decodes a decrypted body (trailing zero padding is ignored).
    pub fn decode(plain: &[u8]) -> Result<Packet, Error> {
        if plain.first() == Some(&0) {
            let mut r = Reader { data: plain, pos: 1 };
            let n = r.u8()? as usize;
            let s = r.take(n)?;
            return std::str::from_utf8(s).map(|s| Packet::Version(s.into())).map_err(|_| Error::BadVersion);
        }
        Message::decode(plain).map(|(m, _)| Packet::Message(m))
    }
}

/// Decodes a `.y3d/.y3a/.y3r` data file.
pub fn decode_file(blob: &[u8]) -> Result<Message, Error> {
    let header: [u8; 4] = blob.get(..4).ok_or(Error::Truncated { at: 0, need: 4 })?.try_into().unwrap();
    let n = frame_len(header);
    let body = blob.get(4..4 + n).ok_or(Error::Truncated { at: 4, need: n })?;
    Message::decode(&decrypt(body)?).map(|(m, _)| m)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let rest = self.data.len() - self.pos;
        if n > rest {
            return Err(Error::Truncated { at: self.pos, need: n - rest });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn message(&mut self, depth: usize) -> Result<Message, Error> {
        if depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        let tag = self.u8()?;
        let nfields = self.u16()?;
        let mut fields = Vec::with_capacity(nfields.min(64) as usize);
        for _ in 0..nfields {
            let n = self.u8()? as usize;
            let name = String::from_utf8_lossy(self.take(n)?).into_owned();
            let at = self.pos;
            let value = match self.u8()? {
                5 => Value::U32(self.u32()?),
                6 => Value::U32Alt(self.u32()?),
                t @ (7..=9) => {
                    let n = self.u32()? as usize;
                    let b = self.take(n)?.to_vec();
                    match t {
                        7 => Value::Text(b),
                        8 => Value::Bin(b),
                        _ => Value::TextAlt(b),
                    }
                }
                tag => return Err(Error::UnknownValueTag { tag, at }),
            };
            fields.push((name, value));
        }
        let nitems = self.u16()?;
        let mut items = Vec::with_capacity(nitems.min(64) as usize);
        for _ in 0..nitems {
            items.push(self.message(depth + 1)?);
        }
        Ok(Message { tag, fields, items })
    }
}
