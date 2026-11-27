//! Anime-only cards, group 10 (rows 5552-5584 of engine/added/cards.tsv): Lyman Banner's and Sartorius' cards (the Alchemy
//! Beasts and the spells that bring them, the Processes and the Helios cards; the Arcana Force spells, the tarot and the
//! fortune-telling). These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and Quick-Play Spell answering the
//! opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3. A coin's
//! result comes from the duel's seed: each coin card runs over seeds until both faces have come up, and each face's
//! effect is checked.
//! Covered: no Token row in this batch; the cards the per-card fuzz flagged (Black Process - Negledo, Yellow Process -
//! Kitolenics and Divination of Fate: the fuzz's random duels never held their conditions; Ace of Sword's COIN and Knight of
//! Pentacles' DESTROY: a coin and a destruction the fuzz does not see, each face checked here); the card with a script note
//! (Intervention of Fate: its cost can send the only Spell it could have activated away, which stopped the effect on a Lua
//! error, so it has an override, `engine/overrides/c511000600.lua`); the signature cards (Reverse Reborn, Vision, Decisive
//! Power of Absolute Destiny, Selection of Fate, the Alchemy Beast spells and Black Process - Negledo) and the seeded 30% of
//! the rest (Ace of Wand, Alchemy Beast - Aretos the Tin, Alchemy Beast - Ekenas the Mercury, Bronze Scale, Emperor's Staff,
//! Tin Spell Circle, Vision, White Process - Albedo), and in the end every other card of the batch too, since each was
//! cheap: the six Alchemy Beast spells and the Albedo in one loop, the Helios Processes, Planet Alignment, Arcanatic
//! Doomscythe, Future Vision, Magician's Scales, Necro Sacrifice, Pentacle of Ace, Sowing of the Fool.
//! The six Alchemy Beasts have no effect but "cannot be Normal Summoned/Set" and "can attack directly".
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // Normal Spell, gain 1000 LP
const DARK_HOLE: u16 = 4342;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: team 0's has `bottom` at its very
/// bottom (out of the opening hand and the first draws), for a card an effect fetches from the Deck.
fn duel(seed: u64, lp: u32, fillers: [u16; 2], bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(fillers[0], 40 - bottom.len())).collect();
    let decks = [(&mine[..], &[][..]), (&vec![fillers[1]; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [FILLER, FILLER], bottom, spawns) }

/// A duel with both Decks given, bottom first (the opening hand is the last five cards).
fn decks(seed: u64, mine: &[u16], theirs: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(Scenario::start_anime_field(&dir, seed, 8000, [(mine, &[][..]), (theirs, &[][..])], spawns).unwrap())
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn deck_size(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap()
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// A card in the Graveyard or banished (Chaos Distill banishes what is sent to the Graveyard).
fn gone(s: &Scenario, team: usize, cid: u16) -> usize {
    s.count(team, "grave", cid) + s.count(team, "removed", cid)
}

/// Whether the card's effect is offered at team 0's first idle prompt (the duel opens on a chain prompt: the first
/// `run` moves it to the Main Phase).
fn playable(s: &mut Scenario, cid: u16) -> bool {
    s.run(&[]).unwrap();
    offered(s, "activates").contains(&cid.into())
}

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

// ---------------------------------------------------------------------------------------------------------------
// The Alchemy Beasts and the spells that bring them
// ---------------------------------------------------------------------------------------------------------------

/// Each of these Spells: "While "Chaos Distill" is face-up on the field: Special Summon 1 "<Alchemy Beast>" from your hand or
/// Deck." Chaos Distill face-up, the Beast at the bottom of the Deck: it comes face-up in Attack Position, a Rock of Level 3
/// and 500/500, and the Spell leaves the hand (to the banished cards: Chaos Distill). Aretos the Tin (Tin Spell Circle), Ouroboros the Bronze (Bronze Scale),
/// Leon the Lead (Lead Compass), Ekenas the Mercury (Mercury Hourglass), Moonface the Silver (Silver Key) and Salamandra
/// the Steel (Steel Lamp); White Process - Albedo's "Golden Homunculus" (a Level 6 Warrior gaining 300 ATK/DEF for each banished card of mine: the Spell) the same way.
#[test]
fn each_alchemy_spell_summons_its_beast_from_the_deck_while_chaos_distill_is_up() {
    let distill = card("Chaos Distill");
    let pairs = [
        ("Tin Spell Circle", "Alchemy Beast - Aretos the Tin"),
        ("Bronze Scale", "Alchemy Beast - Ouroboros the Bronze"),
        ("Lead Compass", "Alchemy Beast - Leon the Lead"),
        ("Mercury Hourglass", "Alchemy Beast - Ekenas the Mercury"),
        ("Silver Key", "Alchemy Beast - Moonface the Silver"),
        ("Steel Lamp", "Alchemy Beast - Salamandra the Steel"),
        ("White Process - Albedo", "Golden Homunculus"),
    ];
    for (spell_name, beast_name) in pairs {
        let (card_spell, beast) = (card(spell_name), card(beast_name));
        let Some(mut s) = field(&[beast], &[hand(0, card_spell), spell(0, distill)]) else { return };
        s.run(&[Activate(card_spell)]).unwrap_or_else(|e| panic!("{spell_name}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "mzone", beast), 1, "{spell_name} brings {beast_name}: {}", s.context());
        assert_eq!(s.position_of(0, beast), Some(1), "{beast_name} face-up Attack: {}", s.context());
        assert_eq!(s.count(0, "hand", card_spell), 0, "{spell_name} is played: {}", s.context());
        assert_eq!(gone(&s, 0, card_spell), 1, "{spell_name} leaves the field: {}", s.context());
        let stats = (s.attack_of(0, beast), s.level_of(0, beast));
        if beast_name == "Golden Homunculus" {
            // Its anime text: "Gains 300 ATK/DEF for each of your banished cards" (the Spell, banished by Chaos Distill).
            assert_eq!(stats, (Some(300), Some(6)), "{}", s.context());
        } else {
            assert_eq!(stats, (Some(500), Some(3)), "{beast_name} is a Level 3 of 500: {}", s.context());
        }
        assert_eq!(s.count(0, "szone", distill), 1, "Chaos Distill stays: {}", s.context());
    }
}

/// Tin Spell Circle: the Beast also comes from the hand, and the Spell cannot be activated without Chaos Distill face-up on
/// the field (the beast is in the Deck and in the hand all the same).
#[test]
fn tin_spell_circle_needs_chaos_distill_and_also_takes_the_beast_from_the_hand() {
    let [circle, aretos, distill] = ["Tin Spell Circle", "Alchemy Beast - Aretos the Tin", "Chaos Distill"].map(card);
    let Some(mut s) = field(&[], &[hand(0, circle), hand(0, aretos), spell(0, distill)]) else { return };
    s.run(&[Activate(circle)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", aretos), 1, "from the hand: {}", s.context());
    assert_eq!(s.count(0, "hand", aretos), 0, "{}", s.context());
    // Without Chaos Distill the Spell is not offered, Beast in the hand or in the Deck.
    let Some(mut s) = field(&[aretos], &[hand(0, circle), hand(0, aretos)]) else { return };
    assert!(!playable(&mut s, circle), "not offered: {}", s.context());
    assert_eq!(s.count(0, "mzone", aretos), 0, "{}", s.context());
}

/// Bronze Scale and White Process - Albedo, the same cost of Chaos Distill: a face-down Chaos Distill is not enough
/// ("face-up on the field"); neither can be activated.
#[test]
fn bronze_scale_and_albedo_need_chaos_distill_face_up() {
    let [scale, albedo, ouroboros, homunculus, distill] =
        ["Bronze Scale", "White Process - Albedo", "Alchemy Beast - Ouroboros the Bronze", "Golden Homunculus", "Chaos Distill"].map(card);
    for (spell_card, beast) in [(scale, ouroboros), (albedo, homunculus)] {
        let Some(mut s) = field(&[beast], &[hand(0, spell_card), set(0, distill)]) else { return };
        assert!(!playable(&mut s, spell_card), "not offered: {}", s.context());
        assert_eq!(s.count(0, "mzone", beast), 0, "{}", s.context());
        assert_eq!(s.count(0, "hand", spell_card), 1, "{}", s.context());
    }
}

/// "Cannot be Normal Summoned/Set": a Beast in the hand is offered neither as a Normal Summon nor as a Set, nor as a
/// Special Summon.
#[test]
fn alchemy_beasts_cannot_be_normal_summoned_or_set() {
    let beasts = ["Aretos the Tin", "Ekenas the Mercury", "Leon the Lead", "Moonface the Silver", "Ouroboros the Bronze", "Salamandra the Steel"]
        .map(|n| card(&format!("Alchemy Beast - {n}")));
    let spawns: Vec<Spawn> = beasts.iter().map(|&b| hand(0, b)).collect();
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[]).unwrap();
    for list in ["summons", "monster_sets", "special_summons"] {
        for b in beasts {
            assert!(!offered(&s, list).contains(&b.into()), "{b} must not be in {list}: {}", s.context());
        }
    }
    // The Feral Imps of the hand are offered all the same.
    assert!(offered(&s, "summons").contains(&FILLER.into()), "{}", s.context());
}

/// "This card can attack directly": in turn 3 Aretos attacks past the opponent's Warwolf (2000 ATK, in Attack Position) and
/// hits the opponent for its 500; the Warwolf stays, and nothing attacks Aretos.
#[test]
fn an_alchemy_beast_attacks_directly_past_a_monster() {
    let aretos = card("Alchemy Beast - Aretos the Tin");
    let Some(mut s) = field(&[], &[mon(0, aretos), mon(1, WARWOLF)]) else { return };
    // The Battle Phase's attack list offers the direct attack (a target list holding only Warwolf would not).
    s.run(&[EndTurn]).unwrap();
    s.run_to(&[Attack(aretos)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    let attack = s.prompt()["attacks"].as_array().unwrap().iter().find(|a| a["cid"] == aretos).cloned().expect("Aretos can attack");
    assert_eq!(attack["can_direct"], true, "a direct attack is offered: {attack}");
    s.run(&[Attack(aretos)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
    assert_eq!(s.count(0, "mzone", aretos), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Black Process - Negledo
// ---------------------------------------------------------------------------------------------------------------

/// Black Process - Negledo: "While "Chaos Distill" is face-up on the field and you have no cards in your hand: Banish all
/// face-up "Alchemy Beast" monsters you control; draw 2 cards for each banished card." The fuzz never holds Chaos Distill
/// and an Alchemy Beast and an empty hand together. Team 0's Deck is all Dian Keto: the six cards of its hand are played
/// (1000 LP each), and the Spell is the last card in it: with the Spells still in the hand it is not offered. Two Beasts on
/// the field are banished and four cards drawn; the Spell itself is banished by Chaos Distill too.
#[test]
fn negledo_banishes_the_beasts_and_draws_two_for_each() {
    let [negledo, distill, aretos, ekenas] = ["Black Process - Negledo", "Chaos Distill", "Alchemy Beast - Aretos the Tin", "Alchemy Beast - Ekenas the Mercury"].map(card);
    let spawns = [hand(0, negledo), spell(0, distill), mon(0, aretos), mon(0, ekenas)];
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &spawns) else { return };
    assert!(!playable(&mut s, negledo), "six cards in the hand besides it: {}", s.context());
    s.run(&[Activate(DIAN_KETO); 6]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), 1, "only Negledo is left: {}", s.context());
    assert_eq!(s.lp(0), 8000 + 6000, "{}", s.context());
    let deck = deck_size(&s, 0);
    s.run(&[Activate(negledo)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", aretos), s.count(0, "removed", ekenas)), (1, 1), "both Beasts banished: {}", s.context());
    assert_eq!(s.count(0, "mzone", aretos) + s.count(0, "mzone", ekenas), 0, "{}", s.context());
    assert_eq!(hand_size(&s, 0), 4, "2 cards for each of the 2 banished cards: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 4, "{}", s.context());
}

/// Negledo's condition: with its own hand emptied it needs Chaos Distill face-up and an Alchemy Beast of mine face-up: with
/// the Beast but no Chaos Distill, or Chaos Distill but no Beast, it is not offered.
#[test]
fn negledo_needs_chaos_distill_and_a_beast() {
    let [negledo, distill, aretos] = ["Black Process - Negledo", "Chaos Distill", "Alchemy Beast - Aretos the Tin"].map(card);
    for spawns in [vec![hand(0, negledo), mon(0, aretos)], vec![hand(0, negledo), spell(0, distill)], vec![hand(0, negledo), spell(0, distill), mon(1, aretos)]] {
        let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &spawns) else { return };
        s.run(&[Activate(DIAN_KETO); 6]).unwrap();
        assert_eq!(hand_size(&s, 0), 1, "only Negledo is left: {}", s.context());
        assert!(!playable(&mut s, negledo), "not offered: {}", s.context());
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The Helios cards: Yellow Process - Kitolenics, Red Process - Rubedo and Planet Alignment
// ---------------------------------------------------------------------------------------------------------------

/// Yellow Process - Kitolenics: "Tribute 1 "Helios - The Primordial Sun", Special Summon 1 "Helios Duo Megistus" from your
/// hand or Deck." The fuzz never held the Sun on the field. The Sun is Tributed (to the Graveyard), the Duo comes from the
/// Deck; without the Sun on the field the Spell is not offered.
#[test]
fn kitolenics_tributes_the_sun_for_helios_duo_megistus() {
    let [kito, sun, duo] = ["Yellow Process - Kitolenics", "Helios - The Primordial Sun", "Helios Duo Megistus"].map(card);
    let Some(mut s) = field(&[duo], &[hand(0, kito), mon(0, sun)]) else { return };
    s.run(&[Activate(kito)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", duo), 1, "the Duo comes: {}", s.context());
    assert_eq!(s.count(0, "mzone", sun), 0, "the Sun is Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", sun), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", kito), 1, "{}", s.context());
    // From the hand, too.
    let Some(mut s) = field(&[], &[hand(0, kito), hand(0, duo), mon(0, sun)]) else { return };
    s.run(&[Activate(kito)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", duo), s.count(0, "hand", duo)), (1, 0), "{}", s.context());
    // No Sun, no Spell.
    let Some(mut s) = field(&[duo], &[hand(0, kito)]) else { return };
    assert!(!playable(&mut s, kito), "not offered: {}", s.context());
    // Nor with the Duo instead of the Sun.
    let Some(mut s) = field(&[duo], &[hand(0, kito), mon(0, duo)]) else { return };
    assert!(!playable(&mut s, kito), "not offered: {}", s.context());
}

/// Red Process - Rubedo: "Tribute 1 "Helios Duo Megistus", Special Summon 1 "Helios Trice Megistus" from your hand or Deck."
#[test]
fn rubedo_tributes_the_duo_for_helios_trice_megistus() {
    let [rubedo, duo, trice, sun] = ["Red Process - Rubedo", "Helios Duo Megistus", "Helios Trice Megistus", "Helios - The Primordial Sun"].map(card);
    let Some(mut s) = field(&[trice], &[hand(0, rubedo), mon(0, duo)]) else { return };
    s.run(&[Activate(rubedo)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", trice), 1, "the Trice comes: {}", s.context());
    assert_eq!((s.count(0, "mzone", duo), s.count(0, "grave", duo)), (0, 1), "the Duo is Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", rubedo), 1, "{}", s.context());
    let Some(mut s) = field(&[trice], &[hand(0, rubedo), mon(0, sun)]) else { return };
    assert!(!playable(&mut s, rubedo), "not offered: {}", s.context());
}

/// Planet Alignment: "Activate only if you control a face-up "Helios - The Primordial Sun": Destroy all monsters your opponent
/// controls and inflict 300 damage to your opponent." Both of the opponent's monsters are destroyed and 300 damage dealt; my
/// Sun stays. Without the Sun the Spell is not offered.
#[test]
fn planet_alignment_destroys_the_opponents_monsters_and_deals_300() {
    let [planet, sun] = ["Planet Alignment", "Helios - The Primordial Sun"].map(card);
    let Some(mut s) = field(&[], &[hand(0, planet), mon(0, sun), mon(1, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(planet)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", THUNDER_KID)), (1, 1), "both destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "{}", s.context());
    assert_eq!(s.count(0, "mzone", sun), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, planet), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    assert!(!playable(&mut s, planet), "not offered: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The coin cards
// ---------------------------------------------------------------------------------------------------------------

/// Ace of Sword: "Target 1 face-up monster on the field; then, toss a coin. Then, apply the following effect: Heads: Inflict
/// damage to your opponent equal to the targeted monster's ATK. Tails: You take damage equal to the targeted monster's
/// ATK." The opponent's Warwolf (2000) is targeted: over ten seeds heads hit the opponent for 2000 and tails me.
#[test]
fn ace_of_sword_damages_by_the_coin() {
    let ace = card("Ace of Sword");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=10 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[hand(0, ace), mon(1, WARWOLF)]) else { return };
        s.prefer(&[(1, WARWOLF)]);
        s.run(&[Activate(ace)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(1, "mzone", WARWOLF), 1, "seed {seed}: the monster stays: {}", s.context());
        assert_eq!(gone(&s, 0, ace), 1, "seed {seed}: {}", s.context());
        match (s.lp(0), s.lp(1)) {
            (8000, 6000) => heads += 1,
            (6000, 8000) => tails += 1,
            other => panic!("seed {seed}: one side takes 2000, got {other:?}: {}", s.context()),
        }
    }
    assert!(heads > 0 && tails > 0, "both faces in 10 seeds: heads {heads}, tails {tails}");
}

/// Ace of Sword may also target my own monster: Dark Blade (1800) targeted, the damage is its ATK all the same.
#[test]
fn ace_of_sword_may_target_my_own_monster() {
    let ace = card("Ace of Sword");
    let Some(mut s) = field(&[], &[hand(0, ace), mon(0, DARK_BLADE)]) else { return };
    s.prefer(&[(0, DARK_BLADE)]);
    s.run(&[Activate(ace)]).unwrap();
    clean(&s);
    assert!(matches!((s.lp(0), s.lp(1)), (8000, 6200) | (6200, 8000)), "1800 to one side: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
}

/// Ace of Wand (Set, activated by a monster destroyed by a card effect): "Toss a coin and apply the appropriate effect.
/// Heads: The controller of the destroyed monster(s) gains LP equal to the total ATK of the destroyed monster(s). Tails:
/// Inflict damage to the controller of the destroyed monster(s) equal to the total ATK of the destroyed monster(s)."
/// Dark Hole destroys my Dark Blade (1800) and the opponent's Warwolf (2000): each controller gains or takes his own.
#[test]
fn ace_of_wand_pays_or_hurts_each_controller_by_the_coin() {
    let wand = card("Ace of Wand");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=10 {
        let spawns = [set(0, wand), hand(0, DARK_HOLE), mon(0, DARK_BLADE), mon(1, WARWOLF)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        keep_hands(&mut s);
        s.run(&[EndTurn, Activate(DARK_HOLE), Chain(wand)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(1, "grave", WARWOLF)), (1, 1), "seed {seed}: {}", s.context());
        assert_eq!(s.count(0, "grave", wand), 1, "seed {seed}: the Spell is used: {}", s.context());
        match (s.lp(0), s.lp(1)) {
            (9800, 10000) => heads += 1,
            (6200, 6000) => tails += 1,
            other => panic!("seed {seed}: heads (9800, 10000) or tails (6200, 6000), got {other:?}: {}", s.context()),
        }
    }
    assert!(heads > 0 && tails > 0, "both faces in 10 seeds: heads {heads}, tails {tails}");
}

/// Pentacle of Ace: "Toss a coin and apply the appropriate effect. Heads: Draw 1 card, also, gain 500 LP. Tails: Your
/// opponent draws 1 card, also they gain 500 LP."
#[test]
fn pentacle_of_ace_draws_and_heals_one_side_by_the_coin() {
    let pentacle = card("Pentacle of Ace");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=10 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[hand(0, pentacle)]) else { return };
        let before = (hand_size(&s, 0), hand_size(&s, 1));
        s.run(&[Activate(pentacle)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        let after = (hand_size(&s, 0), hand_size(&s, 1));
        match (s.lp(0), s.lp(1)) {
            (8500, 8000) => {
                assert_eq!(after, (before.0, before.1), "seed {seed}: I drew one, the Spell left: {}", s.context());
                heads += 1;
            }
            (8000, 8500) => {
                assert_eq!(after, (before.0 - 1, before.1 + 1), "seed {seed}: the opponent drew: {}", s.context());
                tails += 1;
            }
            other => panic!("seed {seed}: 500 LP to one side, got {other:?}: {}", s.context()),
        }
    }
    assert!(heads > 0 && tails > 0, "both faces in 10 seeds: heads {heads}, tails {tails}");
}

/// Knight of Pentacles: "If this card is Summoned: Toss a coin and apply the appropriate effect. Heads: This card cannot be
/// destroyed by battle. Tails: This card cannot attack. If this card is attacked: Destroy this card at the end of the Damage
/// Step." Summoned in turn 1 (1000/1000), attacked in turn 2 by the opponent's Thunder Kid (700): heads, the Knight stays
/// and the Kid dies; tails, the Knight is destroyed all the same. In turn 3 a heads Knight attacks the Kid; a tails Knight
/// is not offered an attack.
#[test]
fn knight_of_pentacles_is_unbreakable_on_heads_and_destroyed_when_attacked_on_tails() {
    let knight = card("Knight of Pentacles");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=12 {
        let spawns = [hand(0, knight), mon(1, THUNDER_KID)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        s.run_both([&[Summon(knight)], &[Attack(THUNDER_KID)]]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        let survived = s.count(0, "mzone", knight) == 1;
        // The Kid was destroyed by the Knight's 1000 either way, or by nothing: it is gone on both faces.
        assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "seed {seed}: Thunder Kid (700) dies against 1000: {}", s.context());
        assert_eq!(s.lp(1), 8000 - 300, "{}", s.context());
        // Turn 3, a second duel with the same seed and the opponent not attacking: can the Knight attack?
        let spawns = [hand(0, knight), mon(1, WARWOLF)];
        let Some(mut t) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        let outcome = t.run(&[Summon(knight), EndTurn, Attack(knight)]);
        if survived {
            heads += 1;
            outcome.unwrap_or_else(|e| panic!("seed {seed}: heads, the Knight attacks: {e}"));
            clean(&t);
            // Warwolf (2000) against the unbreakable Knight (1000): the Knight stays, I take 1000.
            assert_eq!(t.count(0, "mzone", knight), 1, "seed {seed}: {}", t.context());
            assert_eq!(t.lp(0), 8000 - 1000, "seed {seed}: {}", t.context());
        } else {
            tails += 1;
            let err = outcome.expect_err("tails: the Knight cannot attack");
            assert!(err.contains("can't attack"), "seed {seed}: {err}");
            assert_eq!(t.count(0, "mzone", knight), 1, "seed {seed}: not attacked, not destroyed: {}", t.context());
        }
    }
    assert!(heads > 0 && tails > 0, "both faces in 12 seeds: heads {heads}, tails {tails}");
}

/// Sowing of the Fool: "Once per turn: Target 1 "Arcana Force" monster you control; note its ATK in multiples of 300
/// (rounded down), then toss a coin and apply an appropriate effect. Heads: Send cards from the top of your Deck to the
/// Graveyard equal to that number. Tails: ... your opponent's Deck." Arcana Force VI - The Lovers (1600): 5 cards.
#[test]
fn sowing_of_the_fool_mills_by_the_coin() {
    let [sowing, lovers] = ["Sowing of the Fool", "Arcana Force VI - The Lovers"].map(card);
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=10 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[spell(0, sowing), mon(0, lovers)]) else { return };
        let (mine, theirs) = (deck_size(&s, 0), deck_size(&s, 1));
        s.run(&[Activate(sowing)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        match (deck_size(&s, 0), deck_size(&s, 1)) {
            (m, t) if m == mine - 5 && t == theirs => {
                assert_eq!(s.count(0, "grave", FILLER), 5, "seed {seed}: {}", s.context());
                heads += 1;
            }
            (m, t) if m == mine && t == theirs - 5 => {
                assert_eq!(s.count(1, "grave", FILLER), 5, "seed {seed}: {}", s.context());
                tails += 1;
            }
            other => panic!("seed {seed}: 5 cards off one Deck, got {other:?} of ({mine}, {theirs}): {}", s.context()),
        }
    }
    assert!(heads > 0 && tails > 0, "both faces in 10 seeds: heads {heads}, tails {tails}");
}

// ---------------------------------------------------------------------------------------------------------------
// Sartorius' Spells
// ---------------------------------------------------------------------------------------------------------------

/// Arcanatic Doomscythe: "Send 1 "Arcana Force" monster from your Deck to the Graveyard. This turn, when an opponent's
/// monster is destroyed by battle, inflict damage to your opponent equal to the ATK of the destroyed monster." The Magician
/// goes to the Graveyard; Warwolf (2000) destroys the opponent's Feral Imp (1300) in turn 3: 700 battle damage and 1300 more.
#[test]
fn arcanatic_doomscythe_deals_the_atk_of_a_monster_destroyed_by_battle() {
    let [scythe, magician] = ["Arcanatic Doomscythe", "Arcana Force I - The Magician"].map(card);
    let Some(mut s) = field(&[magician], &[hand(0, scythe), mon(0, WARWOLF), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(scythe), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", magician), 1, "the Arcana Force goes to the Graveyard: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 700 - 1300, "battle damage and the Imp's ATK: {}", s.context());
}

/// Arcanatic Doomscythe without an Arcana Force in the Deck cannot be activated.
#[test]
fn arcanatic_doomscythe_needs_an_arcana_force_in_the_deck() {
    let scythe = card("Arcanatic Doomscythe");
    let Some(mut s) = field(&[], &[hand(0, scythe)]) else { return };
    assert!(!playable(&mut s, scythe), "not offered: {}", s.context());
}

/// Decisive Power of Absolute Destiny (a Normal Trap, Set first): "Declare 1 monster name; If a card with the declared name is in your opponent's Deck,
/// Special Summon it in Attack Position to their side of the field." Dark Blade is declared and is in the opponent's Deck:
/// it comes to their side in Attack Position, from the Deck. Declaring Warwolf, which it does not hold, brings nothing.
#[test]
fn decisive_power_summons_the_declared_card_from_the_opponents_deck() {
    let decisive = card("Decisive Power of Absolute Destiny");
    let mut theirs = vec![DARK_BLADE];
    theirs.extend([FILLER; 39]);
    let Some(mut s) = decks(7, &[FILLER; 40], &theirs, &[set(0, decisive)]) else { return };
    keep_hands(&mut s);
    s.declare_card(Some(DARK_BLADE));
    s.run(&[EndTurn]).unwrap();
    let deck = deck_size(&s, 1);
    s.run(&[Activate(decisive)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "on the opponent's side: {}", s.context());
    assert_eq!(s.position_of(1, DARK_BLADE), Some(1), "in Attack Position: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 0, "{}", s.context());
    assert_eq!(deck_size(&s, 1), deck - 1, "out of their Deck: {}", s.context());
    // Warwolf is in my Deck (the declaration's list holds the cards of both Decks first) and not in theirs.
    let mut mine = vec![WARWOLF];
    mine.extend([FILLER; 39]);
    let Some(mut s) = decks(7, &mine, &theirs, &[set(0, decisive)]) else { return };
    keep_hands(&mut s);
    s.declare_card(Some(WARWOLF));
    s.run(&[EndTurn, Activate(decisive)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF) + s.count(1, "mzone", DARK_BLADE) + s.count(1, "mzone", FILLER), 0, "nothing comes: {}", s.context());
}

/// Divination of Fate: "If a monster you control did not destroy an opponent's monster by battle this turn: Special Summon 1
/// Level 4 or lower "Arcana Force" monster from your hand." Thunder Kid (700) attacks Warwolf in turn 3 and is destroyed:
/// in Main Phase 2 the Empress (Level 4) is Special Summoned from the hand. Before any attack the Spell is not offered, and
/// after Warwolf (2000) destroys the opponent's Thunder Kid it is not either.
#[test]
fn divination_of_fate_needs_a_battle_that_destroyed_nothing() {
    let [divination, empress] = ["Divination of Fate", "Arcana Force III - The Empress"].map(card);
    let Some(mut s) = field(&[], &[hand(0, divination), hand(0, empress), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn]).unwrap();
    assert!(!offered(&s, "activates").contains(&divination.into()), "no battle yet: {}", s.context());
    s.run(&[Attack(THUNDER_KID), Activate(divination)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", empress), 1, "the Empress is Special Summoned: {}", s.context());
    assert_eq!(s.count(0, "hand", empress), 0, "{}", s.context());
    assert_eq!(gone(&s, 0, divination), 1, "{}", s.context());
    // Control: the attacker destroys the opponent's monster.
    let Some(mut s) = field(&[], &[hand(0, divination), hand(0, empress), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[EndTurn, Attack(WARWOLF), Activate(divination)]).expect_err("Warwolf destroyed a monster");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", empress), 1, "{}", s.context());
}

/// Emperor's Staff (a Normal Trap, Set first): "Your opponent draws 1 card. This turn, cards you control cannot be destroyed by your opponent's card
/// effects." Warwolf attacks in turn 3 into the opponent's Set Mirror Force: after the Staff the Mirror Force destroys
/// nothing and Warwolf hits for 2000, and the opponent holds a card more than without it. Without the Staff Warwolf is
/// destroyed.
#[test]
fn emperors_staff_draws_for_the_opponent_and_protects_my_cards_this_turn() {
    let [staff, mirror] = ["Emperor's Staff", "Mirror Force"].map(card);
    let spawns = [set(0, staff), mon(0, WARWOLF), set(1, mirror)];
    let Some(mut with) = field(&[], &spawns) else { return };
    keep_hands(&mut with);
    with.run_both([&[EndTurn, Activate(staff), Attack(WARWOLF)], &[ChainOnAttack(mirror)]]).unwrap();
    clean(&with);
    // The Trap answered the attack with its effect and destroyed nothing.
    assert_eq!(with.count(1, "grave", mirror), 1, "Mirror Force was used: {}", with.context());
    assert_eq!(with.count(0, "mzone", WARWOLF), 1, "Warwolf is protected: {}", with.context());
    assert_eq!(with.lp(1), 8000 - 2000, "{}", with.context());
    let Some(mut without) = field(&[], &[mon(0, WARWOLF), set(1, mirror)]) else { return };
    keep_hands(&mut without);
    without.run_both([&[EndTurn, Attack(WARWOLF)], &[ChainOnAttack(mirror)]]).unwrap();
    clean(&without);
    assert_eq!(without.count(0, "grave", WARWOLF), 1, "without the Staff Warwolf is destroyed: {}", without.context());
    assert_eq!(hand_size(&with, 1), hand_size(&without, 1) + 1, "the opponent drew one: {}", with.context());
}

/// Future Vision: "While your opponent controls a monster: Excavate cards from the top of your opponent's Deck until you
/// excavate a monster. If it is the same Type as a monster they control, your opponent gains 1000 LP, also, all cards
/// excavated return in the same order." Their Deck's top three cards are Dark Holes, the fourth a Feral Imp (a Fiend) and
/// they control a Feral Imp: 4 cards are excavated (the client's view of them is checked after every step), 1000 LP gained, and the Deck is the size it was. With a Warwolf instead
/// (a Beast-Warrior) there is no gain; without a monster of theirs the Spell is not offered.
#[test]
fn future_vision_heals_when_the_excavated_monster_shares_a_type() {
    let vision = card("Future Vision");
    let mut theirs = vec![FILLER; 30];
    theirs.extend([DARK_HOLE; 3]);
    theirs.extend([FILLER; 5]);
    let Some(mut s) = decks(7, &[FILLER; 40], &theirs, &[hand(0, vision), mon(1, FILLER)]) else { return };
    let deck = deck_size(&s, 1);
    s.run(&[Activate(vision)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 + 1000, "the same Type: {}", s.context());
    assert_eq!(deck_size(&s, 1), deck, "all cards excavated return: {}", s.context());
    let Some(mut s) = decks(7, &[FILLER; 40], &theirs, &[hand(0, vision), mon(1, WARWOLF)]) else { return };
    s.run(&[Activate(vision)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000, "another Type: {}", s.context());
    let Some(mut s) = decks(7, &[FILLER; 40], &theirs, &[hand(0, vision)]) else { return };
    assert!(!offered(&s, "activates").contains(&vision.into()), "no monster on their side: {}", s.context());
    assert!(!playable(&mut s, vision), "not offered: {}", s.context());
}

/// Intervention of Fate: "When your opponent Normal Summons a monster: You can send this card from your hand to the
/// Graveyard; activate 1 Normal Spell Card from your hand." Dian Keto is activated from the hand when the opponent Normal
/// Summons a Feral Imp: 1000 LP.
#[test]
fn intervention_of_fate_activates_a_normal_spell_from_the_hand() {
    let intervention = card("Intervention of Fate");
    let Some(mut s) = field(&[], &[hand(0, intervention), hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(intervention)], &[Summon(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 9000, "Dian Keto was activated: {}", s.context());
    assert_eq!((s.count(0, "grave", intervention), s.count(0, "grave", DIAN_KETO)), (1, 1), "{}", s.context());
}

/// Intervention of Fate's script note: its cost sends itself away, and a Normal Spell that the cost made impossible to
/// activate leaves nothing to choose. Lightning Vortex ("Discard 1 card") is the only other card in the hand, and can be
/// activated only while Intervention of Fate can be its discard. Upstream's script asked the core's CheckActivateEffect, which
/// tested the Vortex's cost for the opponent (the Summoner), so once Intervention was in the Graveyard the Vortex still
/// resolved with its discard unpaid: it destroyed the opponent's monster for free. The override
/// (`engine/overrides/c511000600.lua`) tests the Spell's own cost for me, after this card's cost, and stops when nothing
/// is left to choose: the Vortex stays in the hand and the Summoned monster stays.
#[test]
fn intervention_of_fate_stops_cleanly_when_its_cost_leaves_no_spell_to_activate() {
    let [intervention, vortex] = ["Intervention of Fate", "Lightning Vortex"].map(card);
    let spawns = [hand(0, intervention), hand(0, vortex)];
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &spawns) else { return };
    s.run_both([&[Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Chain(intervention)], &[Summon(FILLER)]])
        .unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", intervention), 1, "the cost was paid: {}", s.context());
    assert_eq!(s.count(0, "hand", vortex), 1, "Lightning Vortex was not activated: {}", s.context());
    assert_eq!(s.count(1, "mzone", FILLER), 1, "the opponent's monster stays: {}", s.context());
}

/// Magician's Scales: "Tribute 1 "Arcana Force" monster; add 1 Spell from your Deck to your hand." The Magician is Tributed,
/// and a Dian Keto, the only Spell in the Deck, comes to the hand. With no Arcana Force on the field the Spell is not offered.
#[test]
fn magicians_scales_tributes_an_arcana_force_to_search_a_spell() {
    let [scales, magician] = ["Magician's Scales", "Arcana Force I - The Magician"].map(card);
    let Some(mut s) = field(&[DIAN_KETO], &[hand(0, scales), mon(0, magician)]) else { return };
    let keto = s.count(0, "hand", DIAN_KETO);
    s.run(&[Activate(scales)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DIAN_KETO), keto + 1, "the Spell comes to the hand: {}", s.context());
    assert_eq!((s.count(0, "mzone", magician), s.count(0, "grave", magician)), (0, 1), "the Tribute: {}", s.context());
    let Some(mut s) = field(&[DIAN_KETO], &[hand(0, scales), mon(0, FILLER)]) else { return };
    assert!(!playable(&mut s, scales), "not offered: {}", s.context());
}

/// Necro Sacrifice: "Target 1 or 2 monsters in your Graveyard; Special Summon them to your opponent's field (Your opponent
/// chooses the battle position(s)). During this turn, if you Tribute Summon 1 monster, you can decrease the number of Tributes
/// needed by the number of monsters Special Summoned with this card's effect." Dark Blade and Warwolf go to the opponent's
/// side, and Dark Magician (Level 7, two Tributes) is Summoned with none. With only one target it still needs one Tribute,
/// and with none on my field it is not offered.
#[test]
fn necro_sacrifice_gives_monsters_away_and_decreases_the_tributes() {
    let necro = card("Necro Sacrifice");
    let spawns = [hand(0, necro), grave(0, DARK_BLADE), grave(0, WARWOLF), hand(0, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "two Tributes needed: {}", s.context());
    s.prefer(&[(0, DARK_BLADE), (0, WARWOLF)]);
    s.prefer_most(true);
    s.run(&[Activate(necro)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", DARK_BLADE), s.count(1, "mzone", WARWOLF)), (1, 1), "both on their side: {}", s.context());
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(0, "grave", WARWOLF)), (0, 0), "{}", s.context());
    assert!(offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "no Tribute needed now: {}", s.context());
    s.run(&[Summon(DARK_MAGICIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    // One target: one Tribute is still needed, and there is none.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DARK_BLADE)]);
    s.run(&[Activate(necro)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", DARK_BLADE) + s.count(1, "mzone", WARWOLF), 1, "one monster given: {}", s.context());
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "one Tribute is still needed: {}", s.context());
}

/// Reverse Reborn: "Target 1 "Arcana Force" monster in your Graveyard; Special Summon it in face-up Attack Position, and equip
/// it with this card. The equipped monster has its effect treated as its Tails result. When this card is destroyed, destroy
/// the equipped monster." The Empress (Tails: when the opponent Summons, send 1 card from my hand to the Graveyard; Heads:
/// Special Summon an Arcana Force from my hand) is brought back and equipped; over eight seeds the opponent's Normal Summon
/// always costs me a card, never brings an Arcana Force. Heavy Storm then destroys the Reverse Reborn, and the Empress too.
#[test]
fn reverse_reborn_equips_the_monster_and_forces_tails() {
    let [reborn, empress] = ["Reverse Reborn", "Arcana Force III - The Empress"].map(card);
    for seed in 1..=8 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[hand(0, reborn), grave(0, empress)]) else { return };
        s.run(&[Activate(reborn)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "mzone", empress), 1, "seed {seed}: {}", s.context());
        assert_eq!(s.position_of(0, empress), Some(1), "seed {seed}: face-up Attack: {}", s.context());
        assert_eq!(s.count(0, "szone", reborn), 1, "seed {seed}: equipped: {}", s.context());
        let before = hand_size(&s, 0);
        s.run_both([&[EndTurn], &[Summon(FILLER)]]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(hand_size(&s, 0), before - 1, "seed {seed}: tails sent a card from my hand to the Graveyard: {}", s.context());
    }
    let heavy = card("Heavy Storm");
    let Some(mut s) = field(&[], &[hand(0, reborn), grave(0, empress), hand(0, heavy)]) else { return };
    s.run(&[Activate(reborn), Activate(heavy)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", reborn), 1, "the equip card is destroyed: {}", s.context());
    assert_eq!((s.count(0, "mzone", empress), s.count(0, "grave", empress)), (0, 1), "and the Empress with it: {}", s.context());
}

/// Selection of Fate: "Your opponent chooses 1 random card in your hand. If it is a monster, Special Summon it." With a Feral
/// Imp and a Dian Keto in hand the pick is either: over twelve seeds a monster is Special Summoned in some and the hand is
/// left alone in the others.
#[test]
fn selection_of_fate_summons_the_chosen_card_only_if_it_is_a_monster() {
    let selection = card("Selection of Fate");
    let (mut summoned, mut kept) = (0, 0);
    for seed in 1..=12 {
        let Some(mut s) = duel(seed, 8000, [DIAN_KETO, FILLER], &[], &[hand(0, selection), hand(0, FILLER)]) else { return };
        s.run(&[Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(selection)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(gone(&s, 0, selection), 1, "seed {seed}: {}", s.context());
        match (s.count(0, "mzone", FILLER), s.count(0, "hand", FILLER)) {
            (1, 0) => {
                assert_eq!(hand_size(&s, 0), 1, "seed {seed}: the Dian Keto stays: {}", s.context());
                summoned += 1;
            }
            (0, 1) => {
                assert_eq!(hand_size(&s, 0), 2, "seed {seed}: {}", s.context());
                kept += 1;
            }
            other => panic!("seed {seed}: the Imp is summoned or kept, got {other:?}: {}", s.context()),
        }
    }
    assert!(summoned > 0 && kept > 0, "both picks in 12 seeds: summoned {summoned}, kept {kept}");
}

/// Vision: "Draw 1 card and reveal it. Then, shuffle it into your Deck. Once during this Duel, if that card or a card with
/// the same name is played, inflict 1000 damage to your opponent." Team 0's Deck is all Dian Keto: the card drawn is shuffled
/// back (my hand keeps its size), and the first Dian Keto I activate afterwards costs the opponent 1000, besides my own
/// 1000 LP gain; the second one costs nothing.
#[test]
fn vision_shuffles_the_card_back_and_punishes_its_first_play() {
    let vision = card("Vision");
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &[hand(0, vision)]) else { return };
    let (hand_before, deck_before) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run(&[Activate(vision)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), hand_before - 1, "drew one, shuffled one back, the Spell left: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck_before, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "the played copy costs the opponent 1000: {}", s.context());
    assert_eq!(s.lp(0), 8000 + 1000, "{}", s.context());
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "once: {}", s.context());
    assert_eq!(s.lp(0), 8000 + 2000, "{}", s.context());
}

/// Vision with a Feral Imp Deck: a Normal Summon plays the card too, once.
#[test]
fn vision_punishes_a_summoned_copy() {
    let vision = card("Vision");
    let Some(mut s) = field(&[], &[hand(0, vision)]) else { return };
    s.run(&[Activate(vision), Summon(FILLER)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "the summoned copy costs the opponent 1000: {}", s.context());
}
