"""Gate, front and patch-HTTP servers with full packet logging.

Point the client at it with yo3.ini:
  GateServerAddress=127.0.0.1   GateServerPort=5730
  PatchUpdateUrl=http://127.0.0.1:8080/yo3/   PatchDataUrl=http://127.0.0.1:8080/ygoda/patch/
"""
from __future__ import annotations

import argparse
import asyncio
import datetime as dt
import json
import pathlib
import sys

from . import codec
from .front import Account, FrontSession

DEFAULT_GAME_DIR = pathlib.Path.home() / ".wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3"


class Logger:
    def __init__(self, log_dir: pathlib.Path):
        log_dir.mkdir(parents=True, exist_ok=True)
        stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
        self.text = open(log_dir / f"server-{stamp}.log", "a", encoding="utf-8")
        self.packets = open(log_dir / f"packets-{stamp}.jsonl", "a", encoding="utf-8")

    def __call__(self, who: str, line: str):
        s = f"{dt.datetime.now():%H:%M:%S.%f}"[:-3] + f"  [{who}] {line}"
        print(s, flush=True)
        self.text.write(s + "\n")
        self.text.flush()

    def packet(self, who: str, direction: str, plain: bytes, msg: codec.Message | None, err: str | None = None):
        rid = msg.request if msg else None
        summary = f"{direction} {rid:#x}" if isinstance(rid, int) else f"{direction} (no id)"
        if msg:
            fields = ", ".join(f"{k}={v!r}" for k, v in msg.fields.items() if k != "request")
            summary += f" {{{fields}}}" + (f" +{len(msg.items)} item(s)" if msg.items else "")
        self(who, summary + (f"  !! {err}" if err else ""))
        for i, item in enumerate(msg.items if msg else []):
            self(who, f"    item[{i}] tag={item.tag} " + ", ".join(f"{k}={v!r}" for k, v in item.fields.items()))
        self.packets.write(json.dumps({
            "t": dt.datetime.now().isoformat(timespec="milliseconds"), "server": who, "dir": direction,
            "msg": msg.to_json() if msg else None, "raw": plain.hex(), "error": err,
        }) + "\n")
        self.packets.flush()


async def serve_conn(reader, writer, who, log: Logger, on_message):
    peer = writer.get_extra_info("peername")
    log(who, f"connected {peer[0]}:{peer[1]}")
    try:
        while True:
            header = await reader.readexactly(4)
            n = codec.frame_length(header)
            if not 0 < n <= codec.MAX_FRAME:
                log(who, f"bad frame length {n}; closing")
                break
            plain = codec.decrypt(await reader.readexactly(n))
            try:
                msg, err = codec.decode(plain), None
            except codec.DecodeError as e:
                msg, err = None, str(e)
            log.packet(who, "<-", plain, msg, err)
            for reply in on_message(msg or codec.Message(-1)):
                data = reply.encode()
                log.packet(who, "->", data, reply)
                writer.write(codec.frame(data))
            await writer.drain()
    except asyncio.IncompleteReadError:
        log(who, "client closed")
    except ConnectionError as e:
        log(who, f"connection error: {e}")
    finally:
        writer.close()


async def serve_http(reader, writer, log: Logger, game_dir: pathlib.Path):
    try:
        request = (await reader.readuntil(b"\r\n\r\n")).decode("latin1")
    except (asyncio.IncompleteReadError, asyncio.LimitOverrunError):
        writer.close()
        return
    path = request.split(" ", 2)[1] if " " in request else "/"
    body, status = b"", "404 Not Found"
    # Serve only the client's own patch index, so it sees itself as up to date.
    if path.rstrip("/").endswith("/yo3.idx") and (game_dir / "yo3.idx").is_file():
        body, status = (game_dir / "yo3.idx").read_bytes(), "200 OK"
    elif path.startswith("/app/"):
        body, status = b"", "200 OK"
    log("http", f"GET {path} -> {status.split()[0]} ({len(body)} B)")
    writer.write(f"HTTP/1.1 {status}\r\nContent-Length: {len(body)}\r\nConnection: close\r\n\r\n".encode() + body)
    await writer.drain()
    writer.close()


async def main_async(args):
    log = Logger(pathlib.Path(args.log_dir))
    account = Account()

    def gate_handler(_msg):
        return [codec.message(0x21FE, {"host": codec.text(args.advertise), "port": codec.u32(args.front_port)})]

    async def on_gate(r, w):
        await serve_conn(r, w, "gate", log, gate_handler)

    async def on_front(r, w):
        session = FrontSession(account, log)
        await serve_conn(r, w, "front", log, session.handle)

    async def on_http(r, w):
        await serve_http(r, w, log, pathlib.Path(args.game_dir))

    servers = [
        await asyncio.start_server(on_gate, args.bind, args.gate_port),
        await asyncio.start_server(on_front, args.bind, args.front_port),
        await asyncio.start_server(on_http, args.bind, args.http_port),
    ]
    log("main", f"gate :{args.gate_port}  front :{args.front_port}  http :{args.http_port}  "
                f"(bind {args.bind}, advertise {args.advertise}); logs in {args.log_dir}")
    await asyncio.gather(*(s.serve_forever() for s in servers))


def main(argv=None):
    p = argparse.ArgumentParser(description="Yu-Gi-Oh! ONLINE 3 private server (gate/front/patch)")
    p.add_argument("--bind", default="127.0.0.1")
    p.add_argument("--advertise", default="127.0.0.1", help="front-server address handed out by the gate")
    p.add_argument("--gate-port", type=int, default=5730)
    p.add_argument("--front-port", type=int, default=1213)
    p.add_argument("--http-port", type=int, default=8080)
    p.add_argument("--game-dir", default=str(DEFAULT_GAME_DIR))
    p.add_argument("--log-dir", default="logs")
    try:
        asyncio.run(main_async(p.parse_args(argv)))
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    sys.exit(main())
