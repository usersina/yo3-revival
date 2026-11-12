// Rules engine sidecar for the yo3-revival duel server: EDOPro's core (ocgcore-wasm) behind JSON lines.
//
// One duel per process. Each request is one line on stdin, each answer one line on stdout:
//   {"op":"new","seed":7,"lp":8000,"decks":[{"main":[cid…],"extra":[cid…]},{…}],"names":{"cid":"English name"}}
//                                        team 0 goes first; `names` finds other prints of a card (see resolve())
//   {"op":"respond","response":{…}}                                       an OcgResponse
//   {"op":"state"}                       where every card is now (read only; the server's duel check, see state())
//   {"op":"cards","cards":[{"cid":…,"name":…}]}   how the core loads each card (the server's card check)
// Answer: {"messages":[…],"status":"await"|"end","errors":[…]}: the core's messages up to the next prompt; "state"
// answers {"status":"state","teams":[…]}, "cards" {"status":"cards","cards":[…]}.
// Cards are given and reported by Konami card id (the client's "CID"): every `code` (or MOVE's `card`) in a
// message gets a `cid` next to it. MOVE also gets its `reason`, read back from the card. Needs `fetch.sh` run first
// (packages, card scripts, card database, Konami ids).
import { readFileSync, readdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { DatabaseSync } from "node:sqlite";
import { createInterface } from "node:readline";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import createCore, { OcgDuelMode, OcgLocation, OcgPosition, OcgProcessResult, OcgQueryFlags } from "ocgcore-wasm";

const here = dirname(fileURLToPath(import.meta.url));
const vendor = join(here, "vendor");
let identityCache: Record<string, string> | undefined;
/** Exact dependencies for diagnostic replay, including uncommitted card-script changes. Only requested in checks. */
function identity() {
  if (identityCache) return identityCache;
  const digest = (path: string) => createHash("sha256").update(readFileSync(path)).digest("hex");
  const scripts = createHash("sha256");
  for (const name of readdirSync(join(vendor, "CardScripts"), { recursive: true }).map(String).filter(n => n.endsWith(".lua")).sort()) {
    scripts.update(name.replaceAll("\\", "/") + "\0").update(readFileSync(join(vendor, "CardScripts", name)));
  }
  identityCache = { node: process.version, sidecar: digest(fileURLToPath(import.meta.url)),
    core: digest(join(here, "node_modules/ocgcore-wasm/lib/ocgcore.sync.wasm")),
    bindings: digest(join(here, "node_modules/ocgcore-wasm/dist/index.js")), scripts: scripts.digest("hex"),
    database: digest(join(vendor, "BabelCDB/cards.cdb")), card_ids: digest(join(vendor, "cardinfo.json")) };
  return identityCache;
}

const db = new DatabaseSync(join(vendor, "BabelCDB", "cards.cdb"), { readOnly: true });
const cardQuery = db.prepare("select * from datas where id = ?");
const nameQuery = db.prepare("select name from texts where id = ?");
const aliasQuery = db.prepare("select alias from datas where id = ?");
const byNameQuery = db.prepare("select d.id from datas d join texts t on t.id = d.id where lower(t.name) = lower(?) and d.alias = 0 order by d.id");
const artworksQuery = db.prepare("select id from datas where alias = ? and id > ? - 20 and id < ? + 20 order by id");

// Konami id <-> passcode (YGOProDeck's misc_info.konami_id). A card lists its artworks' passcodes too; the one the card
// database holds as the original (alias 0) is the passcode, so that the core finds the card's data and script. Every
// artwork the database holds maps back to the Konami id.
const passcodeOf = new Map<number, number>();
const cidOf = new Map<number, number>();
for (const card of JSON.parse(readFileSync(join(vendor, "cardinfo.json"), "utf-8")).data) {
  const ids: number[] = [...new Set<number>([card.id, ...(card.card_images ?? []).map((i: { id: number }) => i.id)])];
  const known = ids.filter((id) => aliasQuery.get(id));
  const original = known.find((id) => !(aliasQuery.get(id) as { alias: number }).alias) ?? known[0];
  for (const misc of card.misc_info ?? []) {
    const cid = misc.konami_id;
    if (!cid || passcodeOf.has(cid) || original === undefined) continue;
    passcodeOf.set(cid, original);
    for (const id of known) if (!cidOf.has(id)) cidOf.set(id, cid);
  }
}

/** The passcode of each Konami id in this duel, and the Konami id of each passcode in it, for the cards the table
 * above doesn't know: another print of a card under its own Konami id (the client has two Dark Magicians, 4041 and
 * 5165), found by its English name. Each print gets an artwork passcode of its own (the core plays an artwork as the
 * original: the same name and script), so the messages still tell the prints apart. */
let duelPasscodes = new Map<number, number>();
let duelCids = new Map<number, number>();

/** The passcode for Konami id `cid` (its client name `name` for another print), claiming it for this duel. */
function resolve(cid: number, name?: string): number | undefined {
  const known = duelPasscodes.get(cid) ?? passcodeOf.get(cid);
  if (known !== undefined) {
    duelPasscodes.set(cid, known);
    if (!duelCids.has(known)) duelCids.set(known, cid);
    return known;
  }
  const original = name ? (byNameQuery.get(name) as { id: number } | undefined)?.id : undefined;
  if (original === undefined) return undefined;
  // An artwork no Konami id claims, so the result doesn't depend on which print the deck lists first.
  const taken = (code: number) => duelCids.has(code) && duelCids.get(code) !== cid;
  const artworks = (artworksQuery.all(original, original, original) as { id: number }[]).map((r) => r.id);
  const code = artworks.find((c) => !cidOf.has(c) && !taken(c)) ?? [...artworks, original].find((c) => !taken(c)) ?? original;
  duelPasscodes.set(cid, code);
  duelCids.set(code, cid);
  return code;
}

/** The passcode the core wants for a declared card name (Konami id `cid`): the original's, not an artwork's. */
function declared(cid: number): number {
  const code = passcodeOf.get(cid) ?? duelPasscodes.get(cid) ?? 0;
  return (aliasQuery.get(code) as { alias: number } | undefined)?.alias || code;
}

/** A passcode's Konami id: this duel's print first. */
const cidFor = (code: number) => duelCids.get(code) ?? cidOf.get(code);

/** A card's English name, for the cards with no Konami id (tokens), which the server finds among the client's own. */
function nameOf(code: number): string | undefined {
  return (nameQuery.get(code) as { name?: string } | undefined)?.name;
}

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

/** The folders a card's script (`c<passcode>.lua`) may be in, in order: a few cards' current scripts sit outside
 * `official/` (Red-Eyes Darkness Metal Dragon's in `pre-errata/`). Any other script is at the top. */
const CARD_SCRIPT_FOLDERS = ["official", "pre-errata", "pre-release"];

function readScript(name: string) {
  const folders = /^c\d+\.lua$/.test(name) ? CARD_SCRIPT_FOLDERS : [""];
  for (const folder of folders) {
    try {
      return readFileSync(join(vendor, "CardScripts", folder, name), "utf-8");
    } catch {
      // the next folder
    }
  }
  return null;
}

/** Adds `cid` next to every numeric `code` (or `card`, as MOVE names it) and `cids` next to a list of codes, so the
 * server never sees passcodes (and `name` when there is no Konami id, as for tokens), and `description_cid` next to a
 * `description` that is a card's script string (aux.Stringid: code << 20 | n). */
function withCids(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(withCids);
  if (value && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value)) out[k] = withCids(v);
    const code = typeof out.code === "number" ? out.code : out.card;
    if (typeof code === "number") out.cid = cidFor(code) ?? 0;
    if (typeof code === "number" && code && !out.cid) {
      const name = nameOf(code);
      if (name) out.name = name;
    }
    // A bare list of codes (SHUFFLE_HAND's `cards`): `cids` beside it, in the same order.
    if (Array.isArray(out.cards) && out.cards.every((c) => typeof c === "number")) {
      out.cids = (out.cards as number[]).map((c) => cidFor(c) ?? 0);
    }
    if (typeof out.description === "bigint" || typeof out.description === "number") {
      const owner = cidOf.get(Number(BigInt(out.description) >> 20n));
      if (owner) out.description_cid = owner;
    }
    // SELECT_OPTION's options are descriptions too: `option_cids` beside them, each its card's id (0: a system string).
    if (Array.isArray(out.options) && out.options.every((o) => typeof o === "bigint" || typeof o === "number")) {
      out.option_cids = (out.options as (bigint | number)[]).map((o) => cidOf.get(Number(BigInt(o) >> 20n)) ?? 0);
    }
    return out;
  }
  return value;
}

const lib = await createCore({ sync: true });
let handle: ReturnType<typeof lib.createDuel> = null;
const errors: string[] = [];

const MSG_WIN = 5;
const MSG_ANNOUNCE_CARD = 142;

// EDOPro's card-name filter (ANNOUNCE_CARD's `opcodes`, card_data::is_declarable): a stack program over a card's data.
const OP = 0x4000000000000000n;
const OPS = { ADD: OP, SUB: OP | 0x100000000n, MUL: OP | 0x200000000n, DIV: OP | 0x300000000n, AND: OP | 0x400000000n,
  OR: OP | 0x500000000n, NEG: OP | 0x600000000n, NOT: OP | 0x700000000n, BAND: OP | 0x800000000n, BOR: OP | 0x900000000n,
  BNOT: OP | 0x1000000000n, BXOR: OP | 0x1100000000n, LSHIFT: OP | 0x1200000000n, RSHIFT: OP | 0x1300000000n,
  ALLOW_ALIASES: OP | 0x1400000000n, ALLOW_TOKENS: OP | 0x1500000000n, ISCODE: OP | 0x10000000000n,
  ISSETCARD: OP | 0x10100000000n, ISTYPE: OP | 0x10200000000n, ISRACE: OP | 0x10300000000n,
  ISATTRIBUTE: OP | 0x10400000000n, GETCODE: OP | 0x10500000000n, GETTYPE: OP | 0x10700000000n,
  GETRACE: OP | 0x10800000000n, GETATTRIBUTE: OP | 0x10900000000n };
const TYPE_MONSTER = 0x1n, TYPE_TOKEN = 0x4000n;
const MARINE_DOLPHIN = 78734254, TWINKLE_MOSS = 13857930;
type Declarable = { code: number; alias: number; setcodes: bigint[]; type: bigint; race: bigint; attribute: bigint };

function declarable(card: Declarable, opcodes: bigint[]): boolean {
  const stack: bigint[] = [];
  let aliases = false, tokens = false;
  const two = (f: (l: bigint, r: bigint) => bigint) => { if (stack.length >= 2) { const r = stack.pop()!, l = stack.pop()!; stack.push(f(l, r)); } };
  const one = (f: (v: bigint) => bigint) => { if (stack.length >= 1) stack.push(f(stack.pop()!)); };
  const bool = (b: boolean) => (b ? 1n : 0n);
  const setcard = (set: bigint) => card.setcodes.some((s) => (s & 0xfffn) === (set & 0xfffn) && (s & 0xf000n & set) === (set & 0xf000n));
  for (const op of opcodes) {
    switch (op) {
      case OPS.ADD: two((l, r) => l + r); break;
      case OPS.SUB: two((l, r) => l - r); break;
      case OPS.MUL: two((l, r) => l * r); break;
      case OPS.DIV: two((l, r) => (r ? l / r : 0n)); break;
      case OPS.AND: two((l, r) => bool(!!l && !!r)); break;
      case OPS.OR: two((l, r) => bool(!!l || !!r)); break;
      case OPS.NEG: one((v) => -v); break;
      case OPS.NOT: one((v) => bool(!v)); break;
      case OPS.BAND: two((l, r) => l & r); break;
      case OPS.BOR: two((l, r) => l | r); break;
      case OPS.BNOT: one((v) => ~v); break;
      case OPS.BXOR: two((l, r) => l ^ r); break;
      case OPS.LSHIFT: two((l, r) => l << r); break;
      case OPS.RSHIFT: two((l, r) => l >> r); break;
      case OPS.ALLOW_ALIASES: aliases = true; break;
      case OPS.ALLOW_TOKENS: tokens = true; break;
      case OPS.ISCODE: one((v) => bool(BigInt(card.code) === v)); break;
      case OPS.ISSETCARD: one((v) => bool(setcard(v))); break;
      case OPS.ISTYPE: one((v) => card.type & v); break;
      case OPS.ISRACE: one((v) => card.race & v); break;
      case OPS.ISATTRIBUTE: one((v) => card.attribute & v); break;
      case OPS.GETCODE: stack.push(BigInt(card.code)); break;
      case OPS.GETTYPE: stack.push(card.type); break;
      case OPS.GETRACE: stack.push(card.race); break;
      case OPS.GETATTRIBUTE: stack.push(card.attribute); break;
      default: stack.push(op);
    }
  }
  if (stack.length !== 1 || stack[0] === 0n) return false;
  const token = (card.type & (TYPE_MONSTER | TYPE_TOKEN)) === (TYPE_MONSTER | TYPE_TOKEN);
  return card.code === MARINE_DOLPHIN || card.code === TWINKLE_MOSS || ((aliases || !card.alias) && (tokens || !token));
}

let declarables: Declarable[] | undefined;
/** Every original card with a Konami id, for card-name filters. */
function allDeclarables(): Declarable[] {
  const query = db.prepare("select id, alias, setcode, type, race, attribute from datas where alias = 0");
  query.setReadBigInts(true); // setcode packs four 16-bit codes
  declarables ??= (query.all() as Record<string, number | bigint>[])
    .filter((r) => cidOf.has(Number(r.id)))
    .map((r) => {
      const setcodes: bigint[] = [];
      for (let s = BigInt(r.setcode); s > 0n; s >>= 16n) setcodes.push(s & 0xffffn);
      return { code: Number(r.id), alias: Number(r.alias), setcodes, type: BigInt(r.type), race: BigInt(r.race), attribute: BigInt(r.attribute) };
    });
  return declarables;
}

/** ANNOUNCE_CARD gets `fits`: Konami ids of cards its filter allows, this duel's cards first, at most 30, for the
 * server to name one for the CPU (and the duel check's players) that the core takes. */
function addFits(batch: { type: number | string; opcodes?: (bigint | number)[]; fits?: number[] }[]) {
  for (const m of batch) {
    if (m.type !== MSG_ANNOUNCE_CARD || !m.opcodes) continue;
    const opcodes = m.opcodes.map((o) => BigInt(o));
    const own = new Set([...duelCids.keys()]);
    const cards = allDeclarables();
    const fit = (c: Declarable) => declarable(c, opcodes);
    const first = cards.filter((c) => own.has(c.code) && fit(c));
    const rest = cards.filter((c) => !own.has(c.code) && fit(c)).slice(0, Math.max(0, 30 - first.length));
    m.fits = [...first, ...rest].map((c) => cidFor(c.code) ?? 0).filter((c) => c);
  }
}
const MSG_MOVE = 50;

type Move = { type: number | string; to?: { controller: 0 | 1; location: number; sequence: number }; reason?: number };

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

const MSG_SHUFFLE_SET_CARD = 36;
type Loc = { controller: 0 | 1; location: number; sequence: number; position?: number };

/** MSG_SHUFFLE_SET_CARD as the core writes it: the shuffled cards' old places, then as many Xyz material places (zero
 * when none). ocgcore-wasm reads them as {from, to} pairs, which pairs old places with each other; and the new place of
 * each card is in neither list. So the message is given `places`: each shuffled zone and the card it holds now, read
 * back from the duel right after the step (the translator matches them to its cards by id). */
function fixShuffles(batch: { type: number | string; cards?: { from: Loc; to: Loc }[]; places?: unknown[] }[]) {
  for (const m of batch) {
    if (m.type !== MSG_SHUFFLE_SET_CARD || !m.cards) continue;
    const zones = m.cards.flatMap((c) => [c.from, c.to]).slice(0, m.cards.length).filter((l) => l.location);
    m.places = zones.map((l) => {
      const q = lib.duelQuery(handle!, { flags: OcgQueryFlags.CODE | OcgQueryFlags.POSITION, controller: l.controller, location: l.location, sequence: l.sequence, overlaySequence: 0 });
      return { controller: l.controller, location: l.location, sequence: l.sequence, position: Number(q?.position ?? 0), code: Number(q?.code ?? 0) };
    });
  }
}

const MSG_SWAP_GRAVE_DECK = 35;

/** MSG_SWAP_GRAVE_DECK (Exchange of the Spirit) says only how many cards the Deck holds: it is given the player's
 * `grave` and `extra` as they are after the step, in sequence order, for the translator to move each card. */
function fixGraveSwaps(batch: { type: number | string; player?: number; grave?: unknown[]; extra?: unknown[] }[]) {
  for (const m of batch) {
    if (m.type !== MSG_SWAP_GRAVE_DECK) continue;
    const codes = (location: OcgLocation) =>
      lib.duelQueryLocation(handle!, { flags: OcgQueryFlags.CODE, controller: (m.player ?? 0) as 0 | 1, location }).filter((c) => c).map((c) => ({ code: Number(c!.code) }));
    m.grave = codes(OcgLocation.GRAVE);
    m.extra = codes(OcgLocation.EXTRA);
  }
}

const MONSTER_STATS = OcgQueryFlags.POSITION | OcgQueryFlags.LEVEL | OcgQueryFlags.ATTACK | OcgQueryFlags.DEFENSE |
  OcgQueryFlags.BASE_ATTACK | OcgQueryFlags.BASE_DEFENSE;
let lastField = "";

/** Both players' monsters as they are now, `{type: "field", monsters: [{controller, sequence, position, level, attack,
 * defense, base_attack, base_defense}]}`, or null if nothing changed since the last report: effects change stats
 * without a message of their own. */
function fieldReport() {
  const monsters: Record<string, number>[] = [];
  for (const controller of [0, 1] as const) {
    lib.duelQueryLocation(handle!, { flags: MONSTER_STATS, controller, location: OcgLocation.MZONE }).forEach((card, sequence) => {
      if (!card) return;
      const { position, level, attack, defense, baseAttack, baseDefense } = card;
      monsters.push({ controller, sequence, position: Number(position), level: Number(level), attack: Number(attack),
        defense: Number(defense), base_attack: Number(baseAttack), base_defense: Number(baseDefense) });
    });
  }
  const json = JSON.stringify(monsters);
  if (json === lastField) return null;
  lastField = json;
  return { type: "field", monsters };
}

/** A card's counters as {counter type: count}. ocgcore-wasm reads each of the core's u32 entries (type | count << 16)
 * into `counters[count] = type`, the other way round (a card holding two types with the same count keeps one of
 * them there). Types left at 0 are dropped. */
function counterCounts(counters: Record<string, number> | undefined) {
  const out: Record<string, number> = {};
  for (const [count, type] of Object.entries(counters ?? {})) if (Number(count)) out[type] = Number(count);
  return out;
}

/** Where every card is, by team: the hand, Graveyard, banished and Extra Deck in sequence order, the monster and
 * Spell/Trap zones by sequence (null where empty; Spell/Trap 5 is the Field Spell zone), and how many cards the Deck
 * holds. Each card {code, position} gets its `cid` (or `name`) like the messages. Xyz materials aren't listed. */
function state() {
  const flags = OcgQueryFlags.CODE | MONSTER_STATS | OcgQueryFlags.COUNTERS;
  const at = (controller: 0 | 1, location: OcgLocation) =>
    lib.duelQueryLocation(handle!, { flags, controller, location }).map((c) => (c ? {
      code: Number(c.code), position: Number(c.position), level: Number(c.level ?? 0), attack: Number(c.attack ?? 0),
      defense: Number(c.defense ?? 0), base_attack: Number(c.baseAttack ?? 0), base_defense: Number(c.baseDefense ?? 0),
      counters: counterCounts(c.counters),
    } : null));
  const list = (controller: 0 | 1, location: OcgLocation) => at(controller, location).filter((c) => c !== null);
  // ocgcore-wasm returns LP in queryField but omits it from OcgFieldPlayer's declaration (dist/index.js, readField).
  const field = lib.duelQueryField(handle!);
  const teams = ([0, 1] as const).map((t) => ({
    // The wrapper reads a signed core LP as u32; a lethal attack may leave a negative value before WIN.
    lp: Math.max(0, (field.players[t] as typeof field.players[0] & { lp: number }).lp | 0),
    hand: list(t, OcgLocation.HAND),
    mzone: at(t, OcgLocation.MZONE),
    szone: at(t, OcgLocation.SZONE),
    grave: list(t, OcgLocation.GRAVE),
    removed: list(t, OcgLocation.REMOVED),
    extra: list(t, OcgLocation.EXTRA),
    deck: lib.duelQueryCount(handle!, t, OcgLocation.DECK),
  }));
  return { status: "state", teams: withCids(teams) };
}

/** Runs the duel until it needs an answer or ends. The core keeps prompting after a win, so a win ends it here. */
function run() {
  const messages: { type: number | string }[] = [];
  for (;;) {
    const status = lib.duelProcess(handle!);
    const batch = lib.duelGetMessage(handle!) as { type: number | string }[];
    addReasons(batch as Move[]);
    fixShuffles(batch as Parameters<typeof fixShuffles>[0]);
    fixGraveSwaps(batch as Parameters<typeof fixGraveSwaps>[0]);
    addFits(batch as Parameters<typeof addFits>[0]);
    const waiting = status === OcgProcessResult.WAITING;
    // Before the prompt, which stays last.
    const field = waiting && !batch.length ? null : fieldReport();
    if (field) batch.splice(waiting ? batch.length - 1 : batch.length, 0, field);
    const win = batch.findIndex((m) => m.type === MSG_WIN);
    if (win >= 0) return { messages: [...messages, ...batch.slice(0, win + 1)], status: "end" };
    messages.push(...batch);
    if (status === OcgProcessResult.END) return { messages, status: "end" };
    if (status === OcgProcessResult.WAITING) return { messages, status: "await" };
  }
}

function newDuel(req: { seed: number; lp?: number; decks: { main: number[]; extra: number[] }[]; names?: Record<string, string> }) {
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
  lastField = "";
  duelPasscodes = new Map();
  duelCids = new Map();
  for (const name of ["constant.lua", "utility.lua"]) lib.loadScript(handle, name, readScript(name) ?? "");
  req.decks.forEach((deck, t) => {
    const put = (cids: number[], location: number) =>
      cids.forEach((cid) => {
        const code = resolve(cid, req.names?.[cid]);
        if (!code) return void errors.push(`no passcode for cid ${cid}`);
        lib.duelNewCard(handle!, { team: t as 0 | 1, duelist: 0, code, controller: t as 0 | 1, location, sequence: 0, position: OcgPosition.FACEDOWN_DEFENSE });
      });
    put(deck.main, OcgLocation.DECK);
    put(deck.extra, OcgLocation.EXTRA);
  });
  lib.startDuel(handle);
  return run();
}

/** What the core makes of each card `{cid, name}` (the server's card check): its passcode (null when the engine has
 * no such card), the card data it reads, and the errors and missing scripts loading it reported. Each card is made in
 * a throwaway duel of its own; the running duel, if any, is left alone. */
function checkCards(req: { cards: { cid: number; name?: string }[] }) {
  // One throwaway duel for them all: the core reports each card's script problems as it makes the card.
  let problems: string[] = [];
  let missing: string[] = [];
  const team = { startingLP: 8000, startingDrawCount: 0, drawCountPerTurn: 1 };
  const scratch = lib.createDuel({
    flags: OcgDuelMode.MODE_MR2, seed: [1n, 2n, 3n, 4n], team1: team, team2: team, cardReader: readCard,
    scriptReader: (file) => { const text = readScript(file); if (text === null && /^c\d+\.lua$/.test(file)) missing.push(file); return text; },
    errorHandler: (type, text) => void problems.push(`${type}: ${text}`),
  });
  if (!scratch) throw new Error("the core refused a duel");
  for (const file of ["constant.lua", "utility.lua"]) lib.loadScript(scratch, file, readScript(file) ?? "");
  const saved = [duelPasscodes, duelCids];
  const out = req.cards.map(({ cid, name }) => {
    duelPasscodes = new Map();
    duelCids = new Map();
    const code = resolve(cid, name);
    if (code === undefined) return { cid, passcode: null };
    problems = [];
    missing = [];
    lib.duelNewCard(scratch, { team: 0, duelist: 0, code, controller: 0, location: OcgLocation.DECK, sequence: 0, position: OcgPosition.FACEDOWN_DEFENSE });
    const row = cardQuery.get(code) as Record<string, number> | undefined;
    const data = row && { type: Number(row.type), level: Number(row.level) & 0xff, attack: Number(row.atk), defense: Number(row.def),
      attribute: Number(row.attribute), race: Number(row.race), alias: Number(row.alias), name: nameOf(code) };
    return { cid, passcode: code, data: data ?? null, missing_scripts: missing, errors: problems };
  });
  [duelPasscodes, duelCids] = saved;
  lib.destroyDuel(scratch);
  return out;
}

const bigintsAsNumbers = (_: string, v: unknown) => (typeof v === "bigint" ? Number(v) : v);

for await (const line of createInterface({ input: process.stdin })) {
  if (!line.trim()) continue;
  errors.length = 0;
  let answer;
  try {
    const req = JSON.parse(line);
    if (req.op === "identity") {
      process.stdout.write(JSON.stringify(identity()) + "\n");
      continue;
    } else if (req.op === "new") answer = newDuel(req);
    else if (req.op === "respond") {
      // A declared card comes as a Konami id (the client's DLGResult); the core wants its passcode. Races are 64-bit
      // values in the core (BigInts), JSON numbers here.
      const r = req.response;
      const response =
        r.type === 18 && "cid" in r ? { type: 18, card: declared(r.cid) }
        : r.type === 16 ? { ...r, races: r.races.map((x: number) => BigInt(x)) }
        : r;
      lib.duelSetResponse(handle!, response);
      answer = run();
    } else if (req.op === "cards") {
      process.stdout.write(JSON.stringify({ status: "cards", cards: checkCards(req) }) + "\n");
      continue;
    } else if (req.op === "state") {
      process.stdout.write(JSON.stringify({ ...state(), errors: [...errors] }, bigintsAsNumbers) + "\n");
      continue;
    } else throw new Error(`unknown op ${req.op}`);
    answer = { ...answer, messages: withCids(answer.messages) };
  } catch (e) {
    answer = { messages: [], status: "error", error: String(e) };
  }
  process.stdout.write(JSON.stringify({ ...answer, errors: [...errors] }, bigintsAsNumbers) + "\n");
}
