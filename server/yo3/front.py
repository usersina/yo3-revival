"""Front server request handlers (login -> player select -> world entry).

Request/reply pairs confirmed so far (client: LoginWindow.cpp dispatcher at 0x469C00):
  first packet           -> 0x219F  version valid
  0x1F47 auth            -> 0x219A  {regulation_id, user_id, uname, dpoint}
  0x1F48 login           -> 0x2201  {regulation_id} + item {user_id, uname, dpoint}
  0x1F49 player list     -> 0x2260  items = player records
  0x1F4C select player   -> 0x2266
  0x1FA4 trunk data      -> 9000    item[0] is copied wholesale into the user's trunk
                                    dataset (app+0x8C8, a YO3Trunk). An empty item list
                                    would crash the client (it dereferences item[0]).
"""
from __future__ import annotations

from dataclasses import dataclass, field

from .codec import Message, blob, message, record, text, u32

VERSION_VALID = 0x219F


@dataclass
class Player:
    player_id: int
    name: str
    index: int = 0
    level: int = 1
    avatar: bytes = b"\0" * 34  # 17 BE u16 slots, read as tag 8; meaning of each slot unmapped
    # Per-player flag blob. The loader keeps a pointer to its bytes (player+0x40) and the
    # main screen reads/sets bit 0 of byte 0x4B on first entry (0x4727DE); a missing pbin
    # is a NULL pointer and crashes. 4096 bits, matching the 4096 entries of
    # data/extra/yo3pbininfo.bin (exact size not yet confirmed).
    pbin: bytearray = field(default_factory=lambda: bytearray(512))


@dataclass
class Account:
    user_id: int = 1001
    name: str = "test"
    dpoint: int = 0
    regulation_id: int = 1
    players: list[Player] = field(default_factory=lambda: [Player(1001, "test")])


def player_record(p: Player) -> Message:
    # Fields read by the client's player list / hover code (0x46C1E4) and PlayerData loader
    # (0x51C260: pname, title, avatar1|avatar, pbin, speech00-22, comment00-01, n_win/lose/draw).
    fields = {
        "pname": text(p.name),
        "pindex": u32(p.index),
        "player_id": u32(p.player_id),
        "dlv": u32(p.level),
        "title": u32(0),
        "n_win": u32(0),
        "n_lose": u32(0),
        "n_draw": u32(0),
        "team_level": u32(0),
        "team_name": text(""),
        "team_authority": u32(0),
        "avatar1": blob(p.avatar),
        "pbin": blob(bytes(p.pbin)),
    }
    fields.update({f"speech{i:02d}": text("") for i in range(23)})
    fields.update({f"comment{i:02d}": text("") for i in range(2)})
    return record(fields)


def trunk_record(account: Account, player: Player) -> Message:
    # Unknown so far: the 9000 handler copies every field of this record into the
    # trunk dataset; later scenes read from it. Start empty and fill in as the client
    # trace and later requests reveal what is needed.
    return record({})


class FrontSession:
    def __init__(self, account: Account, log):
        self.account = account
        self.log = log
        self.count = 0
        self.player: Player | None = None

    def handle(self, msg: Message) -> list[Message]:
        self.count += 1
        if self.count == 1:
            return [message(VERSION_VALID)]
        rid = msg.request
        handler = HANDLERS.get(rid)
        if handler is None:
            self.log("front", f"UNHANDLED request {rid:#x}" if isinstance(rid, int) else "request without id")
            return []
        return handler(self, msg)

    def _find_player(self, msg: Message) -> Player | None:
        pid = msg.get("player_id")
        return next((p for p in self.account.players if p.player_id == pid), None)

    def on_auth(self, msg):
        a = self.account
        return [message(0x219A, {"regulation_id": u32(a.regulation_id), "user_id": u32(a.user_id),
                                 "uname": text(a.name), "dpoint": u32(a.dpoint)})]

    def on_login(self, msg):
        a = self.account
        return [message(0x2201, {"regulation_id": u32(a.regulation_id)},
                        [record({"user_id": u32(a.user_id), "uname": text(a.name), "dpoint": u32(a.dpoint)})])]

    def on_player_list(self, msg):
        return [message(0x2260, items=[player_record(p) for p in self.account.players])]

    def on_select_player(self, msg):
        self.player = self._find_player(msg)
        if self.player is None:
            self.log("front", f"select: unknown player_id {msg.get('player_id')!r}")
            return []
        return [message(0x2266)]

    def on_trunk(self, msg):
        player = self._find_player(msg) or self.player
        if player is None:
            self.log("front", f"trunk: unknown player_id {msg.get('player_id')!r}")
            return []
        return [message(9000, items=[trunk_record(self.account, player)])]


HANDLERS = {
    0x1F47: FrontSession.on_auth,
    0x1F48: FrontSession.on_login,
    0x1F49: FrontSession.on_player_list,
    0x1F4C: FrontSession.on_select_player,
    0x1FA4: FrontSession.on_trunk,
}
