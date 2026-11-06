"""Codec round-trip on real client data and a scripted login flow against the server."""
import asyncio
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

from yo3 import codec  # noqa: E402
from yo3.server import DEFAULT_GAME_DIR  # noqa: E402


def test_codec_roundtrip_on_client_data():
    """The client's own encrypted files decode and re-encode byte-for-byte."""
    ypk = DEFAULT_GAME_DIR / "package" / "data002.ypk"
    if not ypk.is_file():
        return
    import struct, zlib
    d = ypk.read_bytes()
    p, checked = 0, 0
    while d[p:p + 4] == b"YO\x03\x04" and checked < 40:
        _, _, meth, _, _, _, csz, _, nl, el = struct.unpack_from("<HHHHHIIIHH", d, p + 4)
        name = d[p + 30:p + 30 + nl].decode("latin1")
        off = p + 30 + nl + el
        if name.endswith((".y3d", ".y3a", ".y3r")):
            raw = d[off:off + csz]
            blob = raw if meth == 0 else zlib.decompress(raw, -15)
            n = codec.frame_length(blob[:4])
            plain = codec.decrypt(blob[4:4 + n])
            msg = codec.decode(plain)
            assert plain.startswith(msg.encode()), name
            assert plain[len(msg.encode()):].strip(b"\0") == b"", name
            checked += 1
        p = off + csz
    assert checked > 0


async def _flow():
    from yo3.server import main_async
    import argparse, tempfile
    args = argparse.Namespace(bind="127.0.0.1", advertise="127.0.0.1", gate_port=15730, front_port=11213,
                              http_port=18080, game_dir=str(DEFAULT_GAME_DIR), log_dir=tempfile.mkdtemp())
    task = asyncio.create_task(main_async(args))
    await asyncio.sleep(0.3)

    async def rpc(w, r, msg):
        w.write(codec.frame(msg.encode()))
        await w.drain()
        n = codec.frame_length(await r.readexactly(4))
        return codec.decode(codec.decrypt(await r.readexactly(n)))

    r, w = await asyncio.open_connection("127.0.0.1", 15730)
    gate = await rpc(w, r, codec.message(0x1F40))
    assert gate.request == 0x21FE and gate.get("port") == 11213
    w.close()

    r, w = await asyncio.open_connection("127.0.0.1", 11213)
    assert (await rpc(w, r, codec.message(0x1F41))).request == 0x219F
    assert (await rpc(w, r, codec.message(0x1F47))).request == 0x219A
    login = await rpc(w, r, codec.message(0x1F48))
    assert login.request == 0x2201 and login.items[0].get("user_id") == 1001
    plist = await rpc(w, r, codec.message(0x1F49))
    assert plist.request == 0x2260 and plist.items[0].fields["avatar1"].tag == codec.T_BIN
    assert (await rpc(w, r, codec.message(0x1F4C, {"player_id": codec.u32(1001)}))).request == 0x2266
    trunk = await rpc(w, r, codec.message(0x1FA4, {"player_id": codec.u32(1001)}))
    assert trunk.request == 9000 and len(trunk.items) == 1 and trunk.items[0].tag == codec.RECORD

    # main screen
    for req, rep in ((0x271A, 0x271B), (0x2710, 0x2711), (0x2AFB, 0x2B13), (0x2AFD, 0x2B17)):
        assert (await rpc(w, r, codec.message(req))).request == rep, hex(req)
    regs = await rpc(w, r, codec.message(0x2072))
    assert regs.request == 0x2398 and len(regs.fields["regulations"].data) == 40
    assert len((await rpc(w, r, codec.message(0x27D8))).items) == 1
    assert (await rpc(w, r, codec.message(0x2583))).get("count") == 0
    refresh = await rpc(w, r, codec.message(0x2CEE))
    assert refresh.request == 0x2CF3 and refresh.items[0].fields["pbin"].tag == codec.T_BIN
    info = await rpc(w, r, codec.message(0x1F41))
    assert info.request == 0x21FC and info.items[0].items[0].get("starttime")
    assert (await rpc(w, r, codec.message(0x2CF0, {"search_type": codec.u32(1)}))).request == 0x2CF7
    w.close()
    task.cancel()


def test_login_flow():
    asyncio.run(_flow())


if __name__ == "__main__":
    test_codec_roundtrip_on_client_data()
    test_login_flow()
    print("ok")
