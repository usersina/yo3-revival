"""Wire codec for Yu-Gi-Oh! ONLINE 3 (Konami "EasyMultiPlay").

Frame:   u32 BE (len(body) ^ 0x77777777) + body
Body:    swap32(Blowfish-ECB("12345678", swap32(plaintext zero-padded to 8)))
Message: u8 tag | u16 nfields | nfields x (u8 namelen, name, value) | u16 nitems | items
Values:  5/6 = u32 BE, 7/9 = text (u32 len; the client stops at the first NUL),
         8 = binary (u32 len; NULs kept), item = nested message.
Top-level messages use tag 4, records inside lists use tag 3.
The same cipher and layout are used by the client's .y3d/.y3a/.y3r data files.
"""
from __future__ import annotations

import struct
from dataclasses import dataclass, field

from Crypto.Cipher import Blowfish

KEY = b"12345678"
LEN_MASK = 0x77777777
MAX_FRAME = 1 << 20

T_U32, T_U32B, T_TEXT, T_BIN, T_TEXTB = 5, 6, 7, 8, 9
MSG, RECORD = 4, 3


def _swap32(data: bytes) -> bytes:
    return b"".join(data[i:i + 4][::-1] for i in range(0, len(data), 4))


def encrypt(plain: bytes) -> bytes:
    plain += b"\0" * (-len(plain) % 8)
    return _swap32(Blowfish.new(KEY, Blowfish.MODE_ECB).encrypt(_swap32(plain)))


def decrypt(body: bytes) -> bytes:
    if len(body) % 8:
        raise ValueError(f"encrypted body length {len(body)} is not a multiple of 8")
    return _swap32(Blowfish.new(KEY, Blowfish.MODE_ECB).decrypt(_swap32(body)))


def frame(plain: bytes) -> bytes:
    body = encrypt(plain)
    return struct.pack(">I", len(body) ^ LEN_MASK) + body


def frame_length(header: bytes) -> int:
    return struct.unpack(">I", header)[0] ^ LEN_MASK


@dataclass
class Value:
    tag: int
    data: int | bytes

    def __repr__(self) -> str:
        if self.tag in (T_U32, T_U32B):
            return f"{self.data}" if self.data < 0x10000 else f"{self.data:#x}"
        if self.tag in (T_TEXT, T_TEXTB):
            return repr(self.data.split(b"\0")[0].decode("latin1"))
        return f"bin[{len(self.data)}]:{self.data[:48].hex()}{'…' if len(self.data) > 48 else ''}"


@dataclass
class Message:
    """A key/value record with an ordered field list and nested items."""
    tag: int = MSG
    fields: dict[str, Value] = field(default_factory=dict)
    items: list["Message"] = field(default_factory=list)

    def get(self, name: str, default=None):
        v = self.fields.get(name)
        return default if v is None else v.data

    @property
    def request(self) -> int | None:
        return self.get("request")

    def encode(self) -> bytes:
        out = bytearray([self.tag]) + struct.pack(">H", len(self.fields))
        for name, val in self.fields.items():
            raw = name.encode("ascii")
            out += bytes([len(raw)]) + raw + bytes([val.tag])
            if val.tag in (T_U32, T_U32B):
                out += struct.pack(">I", val.data & 0xFFFFFFFF)
            else:
                out += struct.pack(">I", len(val.data)) + val.data
        out += struct.pack(">H", len(self.items))
        for item in self.items:
            out += item.encode()
        return bytes(out)

    def to_json(self) -> dict:
        return {
            "tag": self.tag,
            "fields": {k: (v.data if v.tag in (T_U32, T_U32B) else repr(v)) for k, v in self.fields.items()},
            "items": [i.to_json() for i in self.items],
        }


def u32(v: int) -> Value:
    return Value(T_U32, int(v))


def text(s: str | bytes) -> Value:
    return Value(T_TEXT, s.encode("utf-8") if isinstance(s, str) else bytes(s))


def blob(b: bytes) -> Value:
    return Value(T_BIN, bytes(b))


def u16be_array(values) -> Value:
    return blob(b"".join(struct.pack(">H", v & 0xFFFF) for v in values))


def message(request: int | None = None, fields: dict[str, Value] | None = None,
            items: list[Message] | None = None, tag: int = MSG) -> Message:
    f: dict[str, Value] = {}
    if request is not None:
        f["request"] = u32(request)
    f.update(fields or {})
    return Message(tag, f, list(items or []))


def record(fields: dict[str, Value] | None = None, items: list[Message] | None = None) -> Message:
    return Message(RECORD, dict(fields or {}), list(items or []))


class DecodeError(ValueError):
    pass


def decode(data: bytes) -> Message:
    if data[:1] == b"\0":
        # Bare u8-length string. The client opens each connection with its protocol
        # version this way: 00 12 "2010.03.23 skelton".
        n = data[1] if len(data) > 1 else 0
        if 2 + n > len(data):
            raise DecodeError("truncated version string")
        return Message(0, {"version": Value(T_TEXT, data[2:2 + n])})
    msg, _ = _decode_msg(data, 0)
    return msg


def _decode_msg(data: bytes, p: int) -> tuple[Message, int]:
    def need(n):
        if p + n > len(data):
            raise DecodeError(f"truncated at {p} (need {n}, have {len(data) - p})")

    need(3)
    tag = data[p]
    (nfields,) = struct.unpack_from(">H", data, p + 1)
    p += 3
    fields: dict[str, Value] = {}
    for _ in range(nfields):
        need(1)
        nl = data[p]
        need(1 + nl + 1)
        name = data[p + 1:p + 1 + nl].decode("latin1")
        vt = data[p + 1 + nl]
        p += 2 + nl
        need(4)
        (n,) = struct.unpack_from(">I", data, p)
        p += 4
        if vt in (T_U32, T_U32B):
            fields[name] = Value(vt, n)
        elif vt in (T_TEXT, T_BIN, T_TEXTB):
            need(n)
            fields[name] = Value(vt, data[p:p + n])
            p += n
        else:
            raise DecodeError(f"unknown value type {vt} for field {name!r} at {p - 5}")
    need(2)
    (nitems,) = struct.unpack_from(">H", data, p)
    p += 2
    items = []
    for _ in range(nitems):
        item, p = _decode_msg(data, p)
        items.append(item)
    return Message(tag, fields, items), p


def decrypt_file(blob_: bytes) -> Message:
    """Decode a .y3d/.y3a/.y3r data file (same framing as the wire)."""
    n = frame_length(blob_[:4])
    return decode(decrypt(blob_[4:4 + n]))
