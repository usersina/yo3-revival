#!/usr/bin/env python3
"""UI monkey: random clicks, right clicks, drags and keys on the two real headless clients (Yugi on :78, Kaiba on :79),
weighted toward the game's real buttons, with a watchdog. Everything runs on the private Xvfb displays through
tools/client/vclient.sh and xdotool: it never touches the real screen or mouse.

    tools/monkey/ui-monkey.py --secs 1800 --seed 1            # roam, then Auto Duels, over and over

What it does
  * copies the user's client Documents into private prefixes; never changes the originals, starts the server on a scratch data dir and both clients, logs them in as
    Yugi and Kaiba (separate authenticated accounts, slot 0 each)
  * in rounds: ROAM (random actions on the lobby screens), DUEL (both clients enter Auto Duel by the scripted clicks,
    then the monkey plays both sides with actions weighted toward the duel screen's buttons and cards)
  * the watchdog, after every few actions: a client's window gone, a Wine page fault or winedbg, the "There is no
    response from the server. (N)" dialog (OCR of the screenshot), a server panic / ERROR / UNHANDLED request, and a
    stall (the screen unchanged and no new packets for a long time). On detection it saves screenshots, the last
    packets and the server log tail to SCRATCH/ui-failures/NNN-kind/, restarts what is needed and goes on.
  * a seed makes the sequence of actions repeatable (the game's own timing is not).
"""
import argparse, atexit, hashlib, json, os, random, re, shutil, signal, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VCLIENT = ROOT / "tools/client/vclient.sh"
DOCS = Path.home() / "Documents" / "Yu-Gi-Oh! ONLINE 3"
W, H = 1024, 768

# Known buttons (game-window coordinates in the 1024x768 window, read off screenshots and AGENTS.md): (x, y, weight).
def row(y, xs, w):
    return [(x, y, w) for x in xs]


LOBBY_SPOTS = (
    row(752, [60, 195, 330, 400, 470, 540, 610, 680, 750, 820, 900, 960], 2)   # the bottom bar: main menu, duel, ...
    + [(195, 752, 6), (60, 752, 3)]
    + [(503, 401, 4), (503, 441, 2), (500, 503, 2)]                             # Duel Menu: Auto Duel, Create Duel, Cancel
    + [(400, 351, 2), (656, 351, 2), (400, 379, 2), (400, 440, 1), (656, 440, 1), (400, 468, 1), (656, 468, 1)]  # Auto Duel form
    + [(420, 522, 4), (628, 523, 1), (620, 351, 1), (393, 533, 4)]              # Confirm, Cancel, waiting Cancel, Select Deck OK
    + [(370, 190, 3), (540, 150, 2), (510, 216, 2), (348, 225, 2), (430, 485, 2), (636, 530, 2), (402, 459, 2)]  # map, shop
    + [(656, 231, 1), (512, 384, 2), (512, 560, 2), (400, 560, 1), (620, 560, 1), (990, 30, 1), (30, 30, 1), (950, 700, 1)]
    + [(600, 723, 2), (330, 330, 1), (70, 380, 1), (120, 310, 1), (170, 310, 1), (225, 310, 1)]  # chat input, side panel tabs
)
DUEL_SPOTS = (
    row(540, [420, 518, 615, 713, 815], 4)                                      # the hand
    + row(443, [420, 518, 615, 713, 815], 1) + [(595, 443, 3), (575, 459, 2)]  # the Summon / Set menu above a card
    + [(435, 240, 2), (508, 240, 2), (580, 240, 3), (652, 240, 3), (724, 240, 2), (796, 240, 3)]  # DP SP M1 BP M2 EP
    + row(315, [360, 450, 540, 630, 720, 810], 2) + row(385, [355, 455, 555, 660, 760, 860], 2)  # my monster / spell zones
    + row(190, [400, 490, 580, 670, 760, 850], 1) + row(135, [400, 490, 580, 670, 760, 850], 1)  # theirs
    + [(930, 385, 1), (380, 130, 1), (60, 752, 1), (600, 723, 1)]               # my deck, their deck, duel menu, chat
    + [(512, 293, 2), (512, 381, 1), (352, 485, 1), (512, 485, 1), (672, 485, 1)]  # first/second to go, janken
    + [(512, 400, 2), (440, 450, 2), (580, 450, 2), (512, 500, 2), (512, 560, 1), (393, 533, 1), (420, 522, 1)]  # dialogs
)
KEYS = ["Return", "Escape", "space", "Tab", "Up", "Down", "Left", "Right", "BackSpace", "y", "n", "a", "1", "2", "F1"]
TEXTS = ["hello", "a" * 200, "ひらがな", "%s%n", "<script>", "../../x", "🂡"]


def sh(cmd, env=None, timeout=60, capture=True):
    e = dict(os.environ)
    e.update(env or {})
    try:
        r = subprocess.run(cmd, env=e, capture_output=capture, text=True, timeout=timeout)
        return r.returncode, (r.stdout or "") + (r.stderr or "")
    except subprocess.TimeoutExpired:
        return 124, "timeout"


class Client:
    def __init__(self, name, display, instance, slot, logs):
        self.name, self.display, self.instance, self.slot, self.logs = name, display, instance, slot, logs
        self.env = {"DISPLAY": display, "YO3_INSTANCE": instance} if instance else {"DISPLAY": display}
        self.win = None
        self.wine_pos = 0
        self.last_hash, self.last_change = None, time.time()

    def vc(self, *a, timeout=120):
        return sh([str(VCLIENT), *a], env=self.env, timeout=timeout)

    def find_window(self):
        rc, out = sh(["xdotool", "search", "--name", "^Yu-Gi-Oh! ONLINE 3"], env=self.env)
        ids = out.split()
        self.win = ids[0] if rc == 0 and ids else None
        return self.win

    def xdo(self, *args):
        return sh(["xdotool", *args], env=self.env, timeout=20)

    def click(self, x, y, button=1):
        if not self.win and not self.find_window():
            return
        self.xdo("mousemove", "--window", self.win, str(x), str(y), "sleep", "0.12", "click", "--delay", "100", str(button))

    def drag(self, x1, y1, x2, y2):
        if not self.win and not self.find_window():
            return
        a = ["mousemove", "--window", self.win, str(x1), str(y1), "sleep", "0.2", "mousedown", "1"]
        for i in range(1, 6):
            a += ["mousemove", "--window", self.win, str(x1 + (x2 - x1) * i // 5), str(y1 + (y2 - y1) * i // 5), "sleep", "0.04"]
        a += ["mouseup", "1"]
        self.xdo(*a)

    def key(self, k):
        self.xdo("key", "--delay", "60", k)

    def type(self, text):
        self.xdo("type", "--delay", "40", text)

    def shot(self, path):
        if not self.win and not self.find_window():
            return False
        rc, _ = sh(["import", "-window", self.win, str(path)], env=self.env, timeout=30)
        return rc == 0

    def screen_hash(self):
        if not self.win and not self.find_window():
            return None
        r = subprocess.run(["import", "-window", self.win, "-depth", "8", "gray:-"], env={**os.environ, **self.env}, capture_output=True, timeout=30)
        return hashlib.md5(r.stdout).hexdigest() if r.returncode == 0 else None

    def ocr(self):
        if not self.win and not self.find_window():
            return ""
        p = subprocess.run(["import", "-window", self.win, "png:-"], env={**os.environ, **self.env}, capture_output=True, timeout=30)
        if p.returncode != 0:
            return ""
        t = subprocess.run(["tesseract", "-", "-", "--psm", "11"], input=p.stdout, capture_output=True, timeout=60)
        return t.stdout.decode(errors="replace")

    def alive(self):
        return self.find_window() is not None

    def crashed(self):
        """New Wine crash lines, or a client frozen in winedbg."""
        found = []
        log = self.logs / "client-wine.log"
        if log.exists():
            with open(log, "rb") as f:
                f.seek(self.wine_pos)
                data = f.read()
                self.wine_pos += len(data)
            for line in data.decode(errors="replace").splitlines():
                if re.search(r"page fault|unhandled exception|Unhandled exception|wine: Unhandled", line):
                    found.append(line.strip())
        return found

    def start(self):
        self.vc("xvfb")
        self.vc("stop")
        time.sleep(1)
        self.logs.mkdir(parents=True, exist_ok=True)
        rc, out = self.vc("start", timeout=120)
        rc2, out2 = self.vc("login", str(self.slot), timeout=240)
        self.wine_pos = (self.logs / "client-wine.log").stat().st_size if (self.logs / "client-wine.log").exists() else 0
        self.find_window()
        return rc == 0 and rc2 == 0, out + out2


class Server:
    def __init__(self, scratch, extra):
        self.scratch, self.extra, self.proc = scratch, extra, None
        self.log = scratch / "server.log"
        self.pos = 0
        self.unhandled = {}

    def start(self):
        (self.scratch / "data").mkdir(parents=True, exist_ok=True)
        out = open(self.log, "ab")
        binary = ROOT / "target/release/yo3-server"
        data = self.scratch / "data"
        def admin(*args):
            subprocess.run([str(binary), *args, "--data-dir", str(data)], check=True, stdout=subprocess.DEVNULL)
        if not (data / "server.crt").exists(): admin("init-auth", "localhost")
        if not (data / "accounts.json").exists():
            admin("add-account", "Yugi")
            admin("add-account", "Kaiba")
        for name in ("Yugi", "Kaiba"):
            profile = self.scratch / f"{name}.json"
            profile.unlink(missing_ok=True)
            admin("profile", name, str(profile), "--advertise", "localhost", "--auth-port", "17443")
        shutil.copyfile(data / "server.crt", self.scratch / "server.crt")
        env = dict(os.environ, RUST_LOG="warn", NO_COLOR="1", RUST_BACKTRACE="1")
        self.proc = subprocess.Popen(
            [str(ROOT / "target/release/yo3-server"), "--log-dir", str(self.scratch / "logs"), "--data-dir", str(self.scratch / "data"),
             "--auth-port", "17443", "--http-port", "18080", *self.extra], cwd=ROOT, env=env, stdout=out, stderr=out, stdin=subprocess.DEVNULL)
        for _ in range(100):
            if sh(["ss", "-ltn", "sport = :17443"])[1].count("LISTEN"):
                return
            time.sleep(0.1)

    def stop(self):
        if self.proc and self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(10)
            except subprocess.TimeoutExpired:
                self.proc.kill()

    def died(self):
        return self.proc is not None and self.proc.poll() is not None

    def trouble(self):
        """New panics and ERRORs; UNHANDLED requests are counted (the client sent something the server ignores)."""
        if not self.log.exists():
            return []
        with open(self.log, "rb") as f:
            f.seek(self.pos)
            data = f.read()
            self.pos += len(data)
        out = []
        for line in data.decode(errors="replace").splitlines():
            if "panicked at" in line or " ERROR " in line or "stack overflow" in line:
                out.append(line.strip())
            m = re.search(r"\[(\w+)\] UNHANDLED request (0x[0-9a-f]+)", line)
            if m:
                k = f"{m.group(1)} {m.group(2)}"
                self.unhandled[k] = self.unhandled.get(k, 0) + 1
        return out

    def tail(self, n=40):
        try:
            return "\n".join(self.log.read_text(errors="replace").splitlines()[-n:])
        except OSError:
            return ""

    def packets_tail(self, n=40):
        files = sorted((self.scratch / "logs").glob("packets-*.jsonl"), key=lambda p: p.stat().st_mtime)
        if not files:
            return ""
        rows = []
        for line in files[-1].read_text(errors="replace").splitlines()[-n:]:
            try:
                d = json.loads(line)
                m = d.get("msg") or {}
                rid = (m.get("fields") or {}).get("request")
                rows.append(f"{d['t'][11:23]} {d['server']} {d['dir']} {hex(rid) if isinstance(rid, int) else m.get('version', '?')} {json.dumps(m.get('fields', {}))[:160]}")
            except (ValueError, KeyError):
                pass
        return "\n".join(rows)

    def count_packets(self, needle):
        """How many logged packets contain `needle` (spaces removed) in the server's packet logs."""
        n = 0
        for p in (self.scratch / "logs").glob("packets-*.jsonl"):
            n += p.read_text(errors="replace").replace(" ", "").count(needle)
        return n

    def packet_bytes(self):
        return sum(p.stat().st_size for p in (self.scratch / "logs").glob("packets-*.jsonl"))


class Monkey:
    def __init__(self, a):
        self.a = a
        self.rng = random.Random(a.seed)
        self.scratch = Path(a.scratch)
        self.scratch.mkdir(parents=True, exist_ok=True)
        self.fail_dir = self.scratch / "ui-failures"
        self.fail_dir.mkdir(exist_ok=True)
        self.nfail = 0
        self.sigs = {}
        self.actions = 0
        self.kinds = {}
        self.duels = 0
        self.duels_not_started = 0
        self.clients = [Client("b", ":78", "b", 0, ROOT / "logs/b"), Client("c", ":79", "c", 0, ROOT / "logs/c")]
        for client, name in zip(self.clients, ["Yugi", "Kaiba"]):
            client.env.update(YO3_WINEPREFIX=str(self.scratch / f"prefix-{client.name}"),
                YO3_AUTH_PROFILE=str(self.scratch / f"{name}.json"), YO3_PACKET_LOG_DIR=str(self.scratch / "logs"))
        self.server = Server(self.scratch, ["--duel-lp", str(a.duel_lp)])
        self.last_packets = 0
        self.last_packet_time = time.time()
        self.backup = self.scratch / "docs-backup"

    # --- the user's Documents --------------------------------------------------------------------------------
    def backup_docs(self):
        # Only private prefix copies use the snapshots; the user's Documents are never changed/restored over.
        for client in self.clients:
            prefix = self.scratch / f"prefix-{client.name}"
            if not prefix.exists():
                source = ROOT / ".wine-b" if (ROOT / ".wine-b").exists() else Path.home() / ".wine"
                subprocess.run(["cp", "-a", "--reflink=auto", str(source), str(prefix)], check=True)
            documents = self.scratch / f"documents-{client.name}"
            if not documents.exists():
                documents.mkdir()
                if DOCS.exists(): shutil.copytree(DOCS, documents / DOCS.name, symlinks=True)
            link = prefix / "drive_c/users" / os.environ.get("USER", "sina") / "Documents"
            if link.is_symlink(): link.unlink()
            elif link.exists(): shutil.rmtree(link)
            link.symlink_to(documents)

    def restore_docs(self):
        pass  # source Documents were never written

    # --- failures ----------------------------------------------------------------------------------------------
    def fail(self, kind, detail, clients=()):
        sig = kind + ":" + re.sub(r"\d+", "", detail)[:100]
        self.sigs[sig] = self.sigs.get(sig, 0) + 1
        self.nfail += 1
        print(f"FAIL #{self.nfail} [{kind}] {detail.splitlines()[0] if detail else ''} (action {self.actions})", flush=True)
        if self.sigs[sig] > 3:
            return
        d = self.fail_dir / f"{self.nfail:03d}-{kind.replace(' ', '-')}"
        d.mkdir(parents=True, exist_ok=True)
        (d / "info.txt").write_text(f"{kind}\n{detail}\nseed {self.a.seed}, action {self.actions}\n")
        (d / "server-tail.txt").write_text(self.server.tail())
        (d / "packets-tail.txt").write_text(self.server.packets_tail())
        for c in clients or self.clients:
            c.shot(d / f"{c.name}.png")
            for name in ("client-stdout.log", "client-wine.log"):
                f = c.logs / name
                if f.exists():
                    (d / f"{c.name}-{name}").write_text("\n".join(f.read_text(errors="replace").splitlines()[-60:]))

    # --- actions -----------------------------------------------------------------------------------------------
    def pick_spot(self, spots):
        total = sum(sp[2] for sp in spots)
        r = self.rng.uniform(0, total)
        for x, y, w in spots:
            r -= w
            if r <= 0:
                return x + self.rng.randint(-6, 6), y + self.rng.randint(-5, 5)
        return spots[0][0], spots[0][1]

    def act(self, c, spots):
        r = self.rng.random()
        self.actions += 1
        if r < 0.55:
            x, y = self.pick_spot(spots)
            c.click(x, y)
            kind = "click"
        elif r < 0.75:
            c.click(self.rng.randint(0, W - 1), self.rng.randint(0, H - 1))
            kind = "click-random"
        elif r < 0.84:
            x, y = self.pick_spot(spots) if self.rng.random() < 0.6 else (self.rng.randint(0, W - 1), self.rng.randint(0, H - 1))
            c.click(x, y, 3)
            kind = "right-click"
        elif r < 0.92:
            x, y = self.pick_spot(spots)
            if self.rng.random() < 0.5:
                x2, y2 = self.pick_spot(spots)
            else:
                x2, y2 = self.rng.randint(0, W - 1), self.rng.randint(0, H - 1)
            c.drag(x, y, x2, y2)
            kind = "drag"
        elif r < 0.99:
            c.key(self.rng.choice(KEYS))
            kind = "key"
        else:
            c.type(self.rng.choice(TEXTS))
            kind = "type"
        self.kinds[kind] = self.kinds.get(kind, 0) + 1

    # --- the watchdog --------------------------------------------------------------------------------------------
    def watch(self, deep=False):
        problems = 0
        if self.server.died():
            self.fail("server died", f"exit status {self.server.proc.returncode}")
            problems += 1
            self.server.start()
            for c in self.clients:
                c.start()
        for line in self.server.trouble():
            self.fail("server log", line)
            problems += 1
        for c in self.clients:
            crash = c.crashed()
            if crash:
                self.fail("client crash", f"{c.name}: " + crash[0], [c])
                problems += 1
                c.start()
                continue
            if not c.alive():
                self.fail("client gone", f"{c.name}: no game window on {c.display}", [c])
                problems += 1
                c.start()
                continue
            if deep:
                text = c.ocr()
                m = re.search(r"no response from the server\.?\s*\(?(\d+)?", text, re.I)
                if m:
                    self.fail("no response dialog", f"{c.name}: There is no response from the server. ({m.group(1)})", [c])
                    problems += 1
                    c.click(512, 460)  # its OK button is near the center
                h = c.screen_hash()
                if h != c.last_hash:
                    c.last_hash, c.last_change = h, time.time()
        # A stall: screens frozen and no packets for a long time while the monkey kept acting.
        pb = self.server.packet_bytes()
        if pb != self.last_packets:
            self.last_packets, self.last_packet_time = pb, time.time()
        if deep and all(time.time() - c.last_change > self.a.stall_secs for c in self.clients) and time.time() - self.last_packet_time > self.a.stall_secs:
            self.fail("stall", f"screens unchanged and no packets for {self.a.stall_secs} s")
            for c in self.clients:
                c.last_change = time.time()
            self.last_packet_time = time.time()
        return problems

    # --- the rounds ----------------------------------------------------------------------------------------------
    def roam(self, secs):
        end = time.time() + secs
        n = 0
        while time.time() < end:
            c = self.rng.choice(self.clients)
            self.act(c, LOBBY_SPOTS)
            n += 1
            if n % 6 == 0:
                self.watch(deep=(n % 24 == 0))

    def enter_auto_duel(self, c):
        """Duel button -> Auto Duel -> Confirm (the Auto Duel form's defaults: Both, Any level)."""
        for x, y in [(195, 752), (503, 401), (420, 522)]:
            c.click(x, y)
            time.sleep(1.2)

    def start_duel(self, entered_before):
        """Both clients queue; then Select Deck's OK (clicked every 2 s until both duel connections open), the janken
        hand and First To Go, as a player does. True once both duel connections opened."""
        for c in self.clients:
            self.enter_auto_duel(c)
        opened = False
        for _ in range(20):
            time.sleep(2)
            for c in self.clients:
                c.click(393, 533)
            if self.server.count_packets('"request":8202') >= entered_before + 2:
                opened = True
                break
        if not opened:
            return False
        for _ in range(3):
            time.sleep(6)
            for c in self.clients:
                c.click(self.rng.choice([352, 512, 672]), 485)   # the janken (a draw asks again)
            time.sleep(4)
            for c in self.clients:
                c.click(512, 293)                                  # the winner picks First To Go
        return True

    def duel_round(self, secs):
        # Roaming leaves a client on any screen; a fresh login is the one state the scripted clicks know.
        self.restart_clients()
        print(f"duel round: both clients into Auto Duel ({self.actions} actions so far)", flush=True)
        before = self.server.count_packets('"request":8202')
        self.duels += 1
        if not self.start_duel(before):
            self.duels_not_started += 1
            self.fail("duel did not open", "Auto Duel from a fresh login: no duel connection after 40 s")
            self.restart_clients()
            return
        end = time.time() + secs
        n = 0
        while time.time() < end:
            c = self.rng.choice(self.clients)
            self.act(c, DUEL_SPOTS)
            n += 1
            if n % 6 == 0:
                self.watch(deep=(n % 24 == 0))

    def restart_clients(self):
        for c in self.clients:
            ok, out = c.start()
            if not ok:
                self.fail("client start", f"{c.name}: {out[-300:]}", [c])

    def run(self):
        a = self.a
        self.backup_docs()
        atexit.register(self.cleanup)
        for s in (signal.SIGINT, signal.SIGTERM):
            signal.signal(s, lambda *_: sys.exit(1))
        self.server.start()
        for c in self.clients:
            ok, out = c.start()
            print(f"client {c.name}: {'logged in' if ok else 'FAILED to start'}", flush=True)
        end = time.time() + a.secs
        while time.time() < end:
            self.roam(self.rng.randint(a.roam_min, a.roam_max))
            if not a.no_duel:
                self.duel_round(self.rng.randint(a.duel_min, a.duel_max))
                # Half the time the clients are killed in the middle of the duel (the server sees both leave) and
                # logged in again; otherwise they stay and roam wherever the duel left them.
                if self.rng.random() < 0.5:
                    self.restart_clients()
        self.watch(deep=True)

    def cleanup(self):
        if getattr(self, "cleaned", False):
            return
        self.cleaned = True
        for c in self.clients:
            c.vc("stop")
        self.server.stop()
        self.report()
        self.restore_docs()

    def report(self):
        print("\n=== UI monkey report ===")
        print(f"actions: {self.actions} {self.kinds}")
        print(f"Auto Duel rounds attempted: {self.duels} ({self.duels_not_started} did not open)")
        pk = list((self.scratch / "logs").glob("packets-*.jsonl"))
        ended = entered = reqs = 0
        for p in pk:
            for line in p.read_text(errors="replace").splitlines():
                if '"request":8201' in line.replace(" ", "") and '"dir":"->"' in line.replace(" ", ""):
                    ended += 1
                if '"request":8202' in line.replace(" ", ""):
                    entered += 1
                reqs += '"dir":"<-"' in line.replace(" ", "")
        print(f"packets from the real clients: {reqs}; duel connections opened: {entered}; duel results pushed: {ended}")
        print(f"UNHANDLED requests the real clients sent: {json.dumps(self.server.unhandled, indent=1)}")
        print(f"failures: {self.nfail}")
        for sig, n in sorted(self.sigs.items(), key=lambda x: -x[1]):
            print(f"  {n:>4}x {sig}")


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--secs", type=int, default=600)
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--scratch", default="/tmp/yo3-ui-monkey")
    p.add_argument("--roam-min", type=int, default=40)
    p.add_argument("--roam-max", type=int, default=120)
    p.add_argument("--duel-min", type=int, default=120)
    p.add_argument("--duel-max", type=int, default=300)
    p.add_argument("--duel-lp", type=int, default=1000)
    p.add_argument("--no-duel", action="store_true")
    p.add_argument("--stall-secs", type=int, default=240)
    a = p.parse_args()
    for display in (":78", ":79"):
        result = subprocess.run(["xdotool", "search", "--name", "^Yu-Gi-Oh! ONLINE 3"],
            env={**os.environ, "DISPLAY": display}, capture_output=True)
        if result.returncode == 0:
            sys.exit(f"private display {display} already has a game client; refusing to disturb it")
    for port in (17443, 18080):
        if f":{port} " in sh(["ss", "-ltn", f"sport = :{port}"])[1]:
            sys.exit(f"port {port} is in use: stop the server first")
    m = Monkey(a)
    try:
        m.run()
    finally:
        m.cleanup()
    sys.exit(1 if m.nfail else 0)
