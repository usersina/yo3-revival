// Rules engine sidecar for the yo3-revival duel server: EDOPro's core (ocgcore-wasm) behind JSON lines.
//
// One duel per process. Each request is one line on stdin, each answer one line on stdout:
//   {"op":"new","seed":7,"lp":8000,"decks":[{"main":[cid…],"extra":[cid…]},{…}]}  team 0 goes first
//   {"op":"respond","response":{…}}                                       an OcgResponse
// Answer: {"messages":[…],"status":"await"|"end","errors":[…]}: the core's messages up to the next prompt.
// Cards are given and reported by Konami card id (the client's "CID"): every `code` (or MOVE's `card`) in a
// message gets a `cid` next to it. MOVE also gets its `reason`, read back from the card. Needs `fetch.sh` run first
// (packages, card scripts, card database, Konami ids).
import { readFileSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";
import { createInterface } from "node:readline";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import createCore, { OcgDuelMode, OcgLocation, OcgPosition, OcgProcessResult, OcgQueryFlags } from "ocgcore-wasm";

const here = dirname(fileURLToPath(import.meta.url));
const vendor = join(here, "vendor");

// Konami id <-> passcode (YGOProDeck's misc_info.konami_id).
const passcodeOf = new Map<number, number>();
const cidOf = new Map<number, number>();
for (const card of JSON.parse(readFileSync(join(vendor, "cardinfo.json"), "utf-8")).data) {
  for (const misc of card.misc_info ?? []) {
    if (misc.konami_id && !passcodeOf.has(misc.konami_id)) {
      passcodeOf.set(misc.konami_id, card.id);
      cidOf.set(card.id, misc.konami_id);
    }
  }
}

const db = new DatabaseSync(join(vendor, "BabelCDB", "cards.cdb"), { readOnly: true });
const cardQuery = db.prepare("select * from datas where id = ?");

function readCard(code: number) {
  const row = cardQuery.get(code) as Record<string, number> | undefined;
  if (!row) return null;
  const setcodes: number[] = [];
  for (let s = BigInt(row.setcode); s > 0n; s >>= 16n) setcodes.push(Number(s & 0xffffn));
  const level = Number(row.level);
  return {
    code,
    alias: Number(row.alias),
    setcodes,
    type: Number(row.type),
    level: level & 0xff,
    attribute: Number(row.attribute),
    race: BigInt(row.race),
    attack: Number(row.atk),
    defense: Number(row.def),
    lscale: (level >> 24) & 0xff,
    rscale: (level >> 16) & 0xff,
    link_marker: 0,
  };
}

function readScript(name: string) {
  const file = /^c\d+\.lua$/.test(name) ? join(vendor, "CardScripts", "official", name) : join(vendor, "CardScripts", name);
  try {
    return readFileSync(file, "utf-8");
  } catch {
    return null;
  }
}

/** Adds `cid` next to every numeric `code` (or `card`, as MOVE names it), so the server never sees passcodes, and
 * `description_cid` next to a `description` that is a card's script string (code << 4 | n). */
function withCids(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(withCids);
  if (value && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value)) out[k] = withCids(v);
    const code = typeof out.code === "number" ? out.code : out.card;
    if (typeof code === "number") out.cid = cidOf.get(code) ?? 0;
    if (typeof out.description === "bigint" || typeof out.description === "number") {
      const owner = cidOf.get(Number(BigInt(out.description) >> 4n));
      if (owner) out.description_cid = owner;
    }
    return out;
  }
  return value;
}

const lib = await createCore({ sync: true });
let handle: ReturnType<typeof lib.createDuel> = null;
const errors: string[] = [];

const MSG_WIN = 5;
const MSG_MOVE = 50;

type Move = { type: number; to?: { controller: 0 | 1; location: number; sequence: number }; reason?: number };

/** MSG_MOVE's `reason`, which ocgcore-wasm doesn't parse: read back from the card at its new place (0 when there is
 * none to ask, as for an Xyz material). */
function addReasons(batch: Move[]) {
  for (const m of batch) {
    if (m.type !== MSG_MOVE) continue;
    const to = m.to;
    const queried = to && to.location && !(to.location & OcgLocation.OVERLAY)
      ? lib.duelQuery(handle!, { flags: OcgQueryFlags.REASON, controller: to.controller, location: to.location, sequence: to.sequence, overlaySequence: 0 })
      : null;
    m.reason = Number(queried?.reason ?? 0);
  }
}

/** Runs the duel until it needs an answer or ends. The core keeps prompting after a win, so a win ends it here. */
function run() {
  const messages: { type: number }[] = [];
  for (;;) {
    const status = lib.duelProcess(handle!);
    const batch = lib.duelGetMessage(handle!) as { type: number }[];
    addReasons(batch);
    const win = batch.findIndex((m) => m.type === MSG_WIN);
    if (win >= 0) return { messages: [...messages, ...batch.slice(0, win + 1)], status: "end" };
    messages.push(...batch);
    if (status === OcgProcessResult.END) return { messages, status: "end" };
    if (status === OcgProcessResult.WAITING) return { messages, status: "await" };
  }
}

function newDuel(req: { seed: number; lp?: number; decks: { main: number[]; extra: number[] }[] }) {
  const team = { startingLP: req.lp ?? 8000, startingDrawCount: 5, drawCountPerTurn: 1 };
  const seed = BigInt(req.seed);
  handle = lib.createDuel({
    flags: OcgDuelMode.MODE_MR2,
    seed: [seed | 1n, seed ^ 0x9e3779b97f4a7c15n, 2n, 3n],
    team1: team,
    team2: team,
    cardReader: readCard,
    scriptReader: readScript,
    errorHandler: (type, text) => void errors.push(`${type}: ${text}`),
  });
  if (!handle) throw new Error("the core refused the duel");
  for (const name of ["constant.lua", "utility.lua"]) lib.loadScript(handle, name, readScript(name) ?? "");
  req.decks.forEach((deck, t) => {
    const put = (cids: number[], location: number) =>
      cids.forEach((cid) => {
        const code = passcodeOf.get(cid);
        if (!code) return void errors.push(`no passcode for cid ${cid}`);
        lib.duelNewCard(handle!, { team: t as 0 | 1, duelist: 0, code, controller: t as 0 | 1, location, sequence: 0, position: OcgPosition.FACEDOWN_DEFENSE });
      });
    put(deck.main, OcgLocation.DECK);
    put(deck.extra, OcgLocation.EXTRA);
  });
  lib.startDuel(handle);
  return run();
}

const bigintsAsNumbers = (_: string, v: unknown) => (typeof v === "bigint" ? Number(v) : v);

for await (const line of createInterface({ input: process.stdin })) {
  if (!line.trim()) continue;
  errors.length = 0;
  let answer;
  try {
    const req = JSON.parse(line);
    if (req.op === "new") answer = newDuel(req);
    else if (req.op === "respond") {
      lib.duelSetResponse(handle!, req.response);
      answer = run();
    } else throw new Error(`unknown op ${req.op}`);
    answer = { ...answer, messages: withCids(answer.messages) };
  } catch (e) {
    answer = { messages: [], status: "error", error: String(e) };
  }
  process.stdout.write(JSON.stringify({ ...answer, errors: [...errors] }, bigintsAsNumbers) + "\n");
}
