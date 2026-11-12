// Writes engine/card-ids.json from YGOPRODeck's card info (`fetch.sh --update`): for every card with a Konami id,
// its passcode, its artworks' passcodes and its Konami ids, in the shape duelcore.ts reads (`vendor/cardinfo.json`).
// Usage: node card-ids.mjs CARDINFO.json OUT.json
import { readFileSync, writeFileSync } from "node:fs";
const [, , from, to] = process.argv;
const data = JSON.parse(readFileSync(from, "utf-8")).data
  .map((c) => ({
    id: c.id,
    card_images: (c.card_images ?? []).map((i) => ({ id: i.id })),
    misc_info: (c.misc_info ?? []).filter((m) => m.konami_id).map((m) => ({ konami_id: m.konami_id })),
  }))
  .filter((c) => c.misc_info.length);
writeFileSync(to, JSON.stringify({ data }).replaceAll('},{"id"', '},\n{"id"') + "\n");
console.log(`${data.length} cards with Konami ids -> ${to}`);
