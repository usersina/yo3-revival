//! Anime-only cards, group 5 (rows 5367-5403 of engine/added/cards.tsv): Yami Marik's, Valon's, Maximillion Pegasus',
//! Dartz's, Jean-Claude Magnum's and Mai Valentine's cards. These cards exist only in the anime, so every duel plays with
//! anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models
//! checked after every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap here is Set first (a Trap
//! cannot be activated from the hand); neither player has a Battle Phase in turn 1 (Vampiric Leech's controller does), so
//! team 0's attacks wait for turn 3.
//! Covered: the Tokens (Mirror Knight, made by Orichalcos Mirror's Ritual Summon of Mirror Knight Calling; Copy, made by
//! Nighmare Tri-Mirror), the cards the per-card fuzz flagged that can be played (Extinction Fist, Class System, Vampiric
//! Leech, Great Kite of Ninja, Aero Nail, Harpie Lady Sparrow Formation, Trap Displacement), the signature cards
//! (Orichalcos Malevolence, Mirror Knight Calling, Nightmare Mirror, Harpie Lady Sparrow Formation) and a sample of the rest
//! (Orichalcos Mirror, Prophecy, Rose Whip, Twin-Bow Centaur).
//! Held back from the batch (Armor monsters: the client cannot show their type): Armored Gravitation, Data Brain, Full Armor
//! Gravitation, Magnet Armor and Phoenix Gravitation. Ninja Master Shogun, Ninja Commander Ikusa and Ninja Commando Kabuki are
//! tested with their partners (Ninja Soldier Katana, Kunoichi Ayame the Ninja Girl), Great Kite of Ninja with a Shogun.
//! Also: Giant's Training (batch 3), which still cannot be played (the game has no "Globerman"), and the Landstar monsters.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const BOOK_OF_MOON: u16 = 5432; // Quick-Play Spell: target 1 face-up monster; it is turned face-down
const POLYMERIZATION: u16 = 4837;
const MYSTICAL_SPACE_TYPHOON: u16 = 4909;
const DIMENSION_FUSION: u16 = 5906;
const RAIGEKI_BREAK: u16 = 5544; // Trap: discard 1 card; target 1 card on the field; destroy it

/// Nighmare Tri-Mirror's Copy Token (row 5368): batch 11's Copy Doll makes another card of that name (16120).
const COPY_TOKEN: u16 = 6462;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn banished(team: u32, cid: u16) -> Spawn { Spawn { team, place: Banished, cid } }
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

/// A duel with the Extra Decks given: `extra[team]`.
fn field_extra(extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], extra[0]), (&[FILLER; 40][..], extra[1])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
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

/// Shield Counters (0x1106) on a team's monsters in total, and on how many of them.
fn shields(s: &Scenario, team: usize, cid: u16) -> (i64, usize) {
    let tokens: Vec<&serde_json::Value> =
        s.state()[team]["mzone"].as_array().unwrap().iter().filter(|c| c["cid"] == cid).collect();
    let with = tokens.iter().filter(|t| t["counters"]["4358"].as_i64().unwrap_or(0) > 0).count();
    (tokens.iter().map(|t| t["counters"]["4358"].as_i64().unwrap_or(0)).sum(), with)
}

/// From a battle prompt on to the Main Phase 2 of the same turn.
fn to_main_phase_2(s: &mut Scenario) {
    if s.prompt()["type"] == msg::SELECT_BATTLECMD {
        s.respond(serde_json::json!({"type": 1, "action": battle::TO_M2, "index": null})).unwrap();
    }
    for _ in 0..8 {
        if s.prompt()["type"] == msg::SELECT_IDLECMD {
            break;
        }
        s.pass().unwrap();
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Orichalcos Mirror, Mirror Knight Calling and the Mirror Knight Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Orichalcos Mirror: "This card is used to Ritual Summon "Mirror Knight Calling" from your Deck. You must also Tribute
/// monsters whose total Levels equal 6 or more from the field or your hand." and Mirror Knight Calling: "When this card is
/// Ritual Summoned: Special Summon 4 "Mirror Knight Tokens" (Warrior-Type/DARK/Level 1/ATK 0/DEF 0), then place 1 Shield
/// Counter on each (max. 1)." Dark Magician (Level 7) is Tributed and Calling comes from the bottom of the Deck, with four
/// Tokens of those stats, each with one Shield Counter.
#[test]
fn orichalcos_mirror_ritual_summons_mirror_knight_calling_and_four_shielded_tokens() {
    let [mirror, calling, token] = ["Orichalcos Mirror", "Mirror Knight Calling", "Mirror Knight Token"].map(card);
    let Some(mut s) = field(&[calling], &[hand(0, mirror), mon(0, DARK_MAGICIAN)]) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Activate(mirror)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", calling), 1, "Mirror Knight Calling is on the field: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 1, "it came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "Dark Magician was Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", mirror), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", token), 4, "four Tokens: {}", s.context());
    let (t, blade) = (s.monster(0, token).unwrap().clone(), mon(0, DARK_BLADE));
    assert_eq!((t["attack"].as_i64(), t["defense"].as_i64(), t["level"].as_i64()), (Some(0), Some(0), Some(1)), "{t}");
    // Warrior and DARK, as Dark Blade (checked in a second duel for its race and attribute).
    let Some(s2) = field(&[], &[blade]) else { return };
    let b = s2.monster(0, DARK_BLADE).unwrap();
    assert_eq!((t["race"].clone(), t["attribute"].clone()), (b["race"].clone(), b["attribute"].clone()), "{t} {b}");
    assert_eq!(shields(&s, 0, token), (4, 4), "one Shield Counter on each Token: {}", s.context());
}

/// Orichalcos Mirror: "Tribute monsters whose total Levels equal 6 or more": with a Deck of Spells (no monster in hand) two
/// Thunder Kids (Level 2 each) are too little and the card is not offered; three (2 + 2 + 2) are exactly enough.
#[test]
fn orichalcos_mirror_needs_materials_with_a_total_level_of_6() {
    let [mirror, calling] = ["Orichalcos Mirror", "Mirror Knight Calling"].map(card);
    let spawns = [hand(0, mirror), mon(0, THUNDER_KID), mon(0, THUNDER_KID)];
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[calling], &spawns) else { return };
    let err = s.run(&[Activate(mirror)]).expect_err("Level 4 only");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", calling), 0, "{}", s.context());
    let spawns = [hand(0, mirror), mon(0, THUNDER_KID), mon(0, THUNDER_KID), mon(0, THUNDER_KID)];
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[calling], &spawns) else { return };
    s.run(&[Activate(mirror)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", calling), 1, "2 + 2 + 2 is 6: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 3, "all three were Tributed: {}", s.context());
}

/// Mirror Knight Calling: "If a "Mirror Knight Token" with a Shield Counter would be destroyed, remove its Shield Counter
/// instead." Team 1's two Dark Holes: the first destroys Calling (it has no counter) and costs each Token its counter; the
/// second destroys all four Tokens.
#[test]
fn a_shield_counter_saves_a_mirror_knight_token_from_destruction_once() {
    let [mirror, calling, token] = ["Orichalcos Mirror", "Mirror Knight Calling", "Mirror Knight Token"].map(card);
    let spawns = [hand(0, mirror), mon(0, DARK_MAGICIAN), hand(1, DARK_HOLE), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[calling], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(mirror), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", calling), 1, "Calling has no counter: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 4, "every Token survived: {}", s.context());
    assert_eq!(shields(&s, 0, token), (0, 0), "each paid its Shield Counter: {}", s.context());
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 0, "no counter, no protection: {}", s.context());
    assert_eq!(s.count(0, "grave", token), 0, "(a Token is gone, not in the Graveyard): {}", s.context());
}

/// Mirror Knight Calling: "When a "Mirror Knight Token" with a Shield Counter battles an opponent's monster: Its ATK becomes
/// equal to that monster's ATK until the end of the Damage Step." and "Once per turn: You can place 1 Shield Counter on every
/// "Mirror Knight Token" you control without a Shield Counter (max. 1)." In turn 3 a Token (0 ATK) attacks Warwolf (2000):
/// its ATK becomes 2000, a tie, so Warwolf is destroyed and the Token, which would be, loses its Shield Counter instead; no
/// damage to anyone. In Main Phase 2 Calling's effect gives that Token a counter again.
#[test]
fn a_shielded_token_takes_its_opponents_atk_in_battle_and_calling_restores_its_counter() {
    let [mirror, calling, token] = ["Orichalcos Mirror", "Mirror Knight Calling", "Mirror Knight Token"].map(card);
    let spawns = [hand(0, mirror), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[calling], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(mirror), EndTurn, Attack(token)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed (a tie at 2000): {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 4, "no Token was destroyed: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "no battle damage: {}", s.context());
    assert_eq!(shields(&s, 0, token), (3, 3), "one Token paid its counter: {}", s.context());
    to_main_phase_2(&mut s);
    s.run(&[Activate(calling)]).unwrap();
    clean(&s);
    assert_eq!(shields(&s, 0, token), (4, 4), "the counter is back: {}", s.context());
}

/// Mirror Knight Calling, the control: a Token without a Shield Counter keeps its 0 ATK. Team 1's Dark Hole took the counters
/// away; its Warwolf comes after it, and in turn 3 a Token attacks it: the Token is destroyed and its controller takes 2000.
#[test]
fn a_token_without_a_shield_counter_fights_with_zero_atk() {
    let [mirror, calling, token] = ["Orichalcos Mirror", "Mirror Knight Calling", "Mirror Knight Token"].map(card);
    let spawns = [hand(0, mirror), mon(0, DARK_MAGICIAN), hand(1, DARK_HOLE), hand(1, WARWOLF)];
    let Some(mut s) = field(&[calling], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(mirror), EndTurn, Attack(token)], &[Activate(DARK_HOLE), Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 3, "the Token was destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "Warwolf lived: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "0 ATK against 2000: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Nighmare Tri-Mirror and the Copy Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Nighmare Tri-Mirror: "Special Summon "Copy Tokens" of 1 monster you control equal to the number of monsters your opponent
/// Summoned this turn, except by Normal Summon/Set. ("Copy Tokens" have the same ATK/DEF, Level, Type and Attribute as the
/// original monster). These Tokens cannot attack during your turn." Team 1's Dimension Fusion Special Summons two of its
/// banished monsters at once; in answer team 0 activates the Trap: two Copy Tokens of Dark Magician (2500/2100, Level 7,
/// Spellcaster, DARK).
#[test]
fn nighmare_tri_mirror_copies_a_monster_once_per_special_summon() {
    let (tri, copy) = (card("Nighmare Tri-Mirror"), COPY_TOKEN);
    let spawns = [set(0, tri), mon(0, DARK_MAGICIAN), hand(1, DIMENSION_FUSION), banished(1, WARWOLF), banished(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(tri)], &[Activate(DIMENSION_FUSION)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF) + s.count(1, "mzone", DARK_BLADE), 2, "two Special Summons: {}", s.context());
    assert_eq!(s.count(0, "mzone", copy), 2, "two Copy Tokens: {}", s.context());
    assert_eq!(s.count(0, "grave", tri), 1, "{}", s.context());
    let (original, token) = (s.monster(0, DARK_MAGICIAN).unwrap().clone(), s.monster(0, copy).unwrap().clone());
    for field in ["attack", "defense", "level", "race", "attribute"] {
        assert_eq!(token[field], original[field], "the Copy Token's {field}: {token} against {original}");
    }
    assert_eq!((token["attack"].as_i64(), token["defense"].as_i64(), token["level"].as_i64()), (Some(2500), Some(2100), Some(7)), "{token}");
    // They cannot attack during their controller's turn: in turn 3 only Dark Magician is offered.
    let err = s.run_both([&[Attack(copy)], &[EndTurn]]).expect_err("a Copy Token cannot attack");
    assert!(err.contains("can't attack") && err.contains("\"cid\":4041") && !err.contains("\"cid\":6462"), "{err}");
    clean(&s);
}

/// Nighmare Tri-Mirror: "equal to the number of monsters your opponent Summoned this turn, except by Normal Summon/Set": a
/// Normal Summon counts for nothing, so the Trap is never on offer.
#[test]
fn nighmare_tri_mirror_ignores_a_normal_summon() {
    let tri = card("Nighmare Tri-Mirror");
    let Some(mut s) = field(&[], &[set(0, tri), mon(0, DARK_MAGICIAN), hand(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(tri)], &[Summon(WARWOLF)]]).expect_err("only a Normal Summon");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", tri), 1, "still Set: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Extinction Fist
// ---------------------------------------------------------------------------------------------------------------

/// Extinction Fist: "When an opponent's Fusion or Ritual monster declares a direct attack: Destroy that monster, and Special
/// Summon the Material(s) that were used to Summon that monster from your opponent's Graveyard to their side of the field,
/// then end the Battle Phase." Team 1 Fusion Summons Gaia the Dragon Champion from Gaia The Fierce Knight and Curse of Dragon
/// and attacks directly: the Dragon is destroyed, both Materials come back, nothing is lost and the Battle Phase ends.
#[test]
fn extinction_fist_destroys_a_fusion_monster_and_returns_its_materials() {
    let [fist, champion, knight, dragon] =
        ["Extinction Fist", "Gaia the Dragon Champion", "Gaia The Fierce Knight", "Curse of Dragon"].map(card);
    let spawns = [set(0, fist), hand(1, POLYMERIZATION), hand(1, knight), hand(1, dragon)];
    let Some(mut s) = field_extra([&[], &[champion]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(fist)], &[Activate(POLYMERIZATION), Attack(champion)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", champion), 1, "the Fusion Monster was destroyed: {}", s.context());
    assert_eq!((s.count(1, "mzone", knight), s.count(1, "mzone", dragon)), (1, 1), "both Materials are back: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no damage: {}", s.context());
    assert_eq!(s.count(0, "grave", fist), 1, "{}", s.context());
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(msg::SELECT_IDLECMD), Some(1)), "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over: {}", s.context());
}

/// Extinction Fist: "Fusion or Ritual monster": a Ritual Monster counts too. Team 1 Ritual Summons Mirror Knight Calling with
/// Orichalcos Mirror, Tributing Dark Magician, and attacks directly with it: Calling is destroyed and Dark Magician comes back
/// (the four Tokens stay).
#[test]
fn extinction_fist_also_answers_a_ritual_monster() {
    let [fist, mirror, calling, token] = ["Extinction Fist", "Orichalcos Mirror", "Mirror Knight Calling", "Mirror Knight Token"].map(card);
    let spawns = [set(0, fist), hand(1, mirror), mon(1, DARK_MAGICIAN)];
    let dir = engine_dir();
    let Some(dir) = dir else { return };
    let (mine, theirs) = ([FILLER; 40], [calling].into_iter().chain([FILLER; 39]).collect::<Vec<u16>>());
    let decks = [(&mine[..], &[][..]), (&theirs[..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(fist)], &[Activate(mirror), Attack(calling)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", calling), 1, "Mirror Knight Calling was destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "the Tributed monster is back: {}", s.context());
    assert_eq!(s.count(1, "mzone", token), 4, "the Tokens stay: {}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over: {}", s.context());
}

/// Extinction Fist: only a Fusion or Ritual monster, and only a direct attack. Warwolf attacking directly is not answered
/// (2000 damage), and a Fusion Monster attacking a monster is not either (Thunder Kid dies).
#[test]
fn extinction_fist_needs_a_fusion_or_ritual_monster_attacking_directly() {
    let [fist, champion, knight, dragon] =
        ["Extinction Fist", "Gaia the Dragon Champion", "Gaia The Fierce Knight", "Curse of Dragon"].map(card);
    let Some(mut s) = field(&[], &[set(0, fist), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(fist)], &[Attack(WARWOLF)]]).expect_err("not a Fusion or Ritual monster");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "the attack landed: {}", s.context());
    let spawns = [set(0, fist), mon(0, THUNDER_KID), hand(1, POLYMERIZATION), hand(1, knight), hand(1, dragon)];
    let Some(mut s) = field_extra([&[], &[champion]], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(fist)], &[Activate(POLYMERIZATION), Attack(champion)]]).expect_err("not a direct attack");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the Fusion Monster attacked Thunder Kid: {}", s.context());
    assert_eq!(s.count(1, "mzone", champion), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Class System
// ---------------------------------------------------------------------------------------------------------------

/// Class System: "When an opponent's monster that attacks a monster you control activates its effect, and if that monster's
/// Level is lower than or equal to your monster's Level: negate that attack and effect." Team 1's Jirai Gumo (Level 4) declares
/// an attack on my Feral Imp (Level 4) and its effect (toss a coin and call it; wrong: lose half your LP) is chained to by this
/// Quick-Play Spell: the attack and the effect are negated, so in every seed the Imp lives and no LP change hands. (A Spell
/// cannot answer an effect at the start of the Damage Step, so Sasuke Samurai's destruction of a Set monster is not the
/// example.)
#[test]
fn class_system_negates_the_attack_and_the_effect_of_a_monster_of_the_same_level() {
    let (class, gumo) = (card("Class System"), card("Jirai Gumo"));
    for seed in 1..=8 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[set(0, class), mon(0, FILLER), mon(1, gumo)]) else { return };
        keep_hands(&mut s);
        s.run_both([&[Chain(class)], &[Attack(gumo)]]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "grave", class), 1, "seed {seed}: Class System was activated: {}", s.context());
        assert_eq!(s.count(0, "mzone", FILLER), 1, "seed {seed}: the attack was negated, the Imp lives: {}", s.context());
        assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "seed {seed}: no battle damage, no coin toss penalty: {}", s.context());
    }
}

/// Class System, the control: Jirai Gumo's coin toss is real. Without the Trap its attack on the Imp (2200 against 1300
/// ATK) costs 900 LP, and a wrong call costs its controller half of its LP first: among the seeds both happen.
#[test]
fn jirai_gumo_without_class_system_tosses_its_coin() {
    let gumo = card("Jirai Gumo");
    let (mut halved, mut kept) = (0, 0);
    for seed in 1..=8 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[mon(0, FILLER), mon(1, gumo)]) else { return };
        keep_hands(&mut s);
        s.run_both([&[], &[Attack(gumo)]]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.lp(0), 8000 - 900, "seed {seed}: {}", s.context());
        match s.lp(1) {
            4000 => halved += 1,
            8000 => kept += 1,
            other => panic!("seed {seed}: LP {other}: {}", s.context()),
        }
    }
    assert!(halved > 0 && kept > 0, "both calls in 8 seeds: wrong {halved}, right {kept}");
}

/// Class System: "lower than or equal to your monster's Level": Jirai Gumo (Level 4) against my Skull Servant (Level 1) is
/// not covered, so the card is not offered and the attack lands.
#[test]
fn class_system_needs_the_attacker_not_to_outrank_the_defender() {
    let (class, gumo, servant) = (card("Class System"), card("Jirai Gumo"), card("Skull Servant"));
    let Some(mut s) = field(&[], &[set(0, class), mon(0, servant), mon(1, gumo)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(class)], &[Attack(gumo)]]).expect_err("Level 4 against Level 1");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "grave", servant), 1, "the Servant was destroyed: {}", s.context());
    assert_eq!(s.count(0, "szone", class), 1, "still Set: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Vampiric Leech
// ---------------------------------------------------------------------------------------------------------------

/// Vampiric Leech: "This card can attack during the first turn." (500 ATK; no other monster can) and "During the turn this
/// card attacked directly: You can discard 1 card; change this card to Defense Position during the End Phase." Turn 1: the
/// Leech attacks directly (500 damage), in Main Phase 2 a card is discarded, and at the End Phase it is in face-up Defense.
#[test]
fn vampiric_leech_attacks_in_the_first_turn_and_goes_to_defense_after_a_direct_attack() {
    let leech = card("Vampiric Leech");
    let Some(mut s) = field(&[], &[mon(0, leech)]) else { return };
    keep_hands(&mut s);
    let hand_before = hand_size(&s, 0);
    s.run(&[Attack(leech)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500, "a direct attack in turn 1: {}", s.context());
    to_main_phase_2(&mut s);
    s.run(&[Activate(leech)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), hand_before - 1, "one card was discarded: {}", s.context());
    assert_eq!(s.position_of(0, leech), Some(1), "still in Attack Position until the End Phase: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, leech), Some(4), "face-up Defense Position after the End Phase: {}", s.context());
}

/// Vampiric Leech: the Defense Position needs "the turn this card attacked directly": after an attack on a monster
/// (Thunder Kid in face-up Defense, 600 DEF against 500 ATK) the effect is not offered, and Warwolf, which has no such
/// text, cannot attack in turn 1.
#[test]
fn vampiric_leech_needs_a_direct_attack_and_is_the_only_monster_to_attack_in_turn_1() {
    let leech = card("Vampiric Leech");
    let Some(mut s) = field(&[], &[mon(0, leech), mon(0, WARWOLF), mon_def(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Attack(WARWOLF)]).expect_err("only the Leech may attack in turn 1");
    assert!(err.contains("can't attack"), "{err}");
    let Some(mut s) = field(&[], &[mon(0, leech), mon_def(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Attack(leech)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 100, "500 ATK against 600 DEF: {}", s.context());
    to_main_phase_2(&mut s);
    assert!(!offered(&s, "activates").contains(&leech.into()), "no effect after an attack on a monster: {:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Great Kite of Ninja, Ninja Master Shogun, Ninja Commander Ikusa and Ninja Commando Kabuki
// ---------------------------------------------------------------------------------------------------------------

/// Great Kite of Ninja: "Equip only to "Ninja Master Shogun"." With only Warwolf on the field it is not offered; with Shogun
/// (placed on the field: it cannot be Summoned, see below) it equips it.
#[test]
fn great_kite_equips_only_ninja_master_shogun() {
    let (kite, shogun) = (card("Great Kite of Ninja"), card("Ninja Master Shogun"));
    let Some(mut s) = field(&[], &[hand(0, kite), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(kite)]).expect_err("no Shogun");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    let Some(mut s) = field(&[], &[hand(0, kite), mon(0, shogun), mon(0, WARWOLF)]) else { return };
    s.run(&[Activate(kite)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", kite), 1, "equipped: {}", s.context());
    assert_eq!(s.count(0, "mzone", shogun), 1, "{}", s.context());
}

/// Great Kite of Ninja: "Negate other Spell/Trap effects that target the equipped monster." Team 1's Book of Moon (target 1
/// face-up monster: it is turned face-down) is activated on Shogun and negated; without the Kite Shogun is turned face-down.
#[test]
fn great_kite_negates_spells_and_traps_that_target_the_equipped_monster() {
    let (kite, shogun) = (card("Great Kite of Ninja"), card("Ninja Master Shogun"));
    let spawns = [hand(0, kite), mon(0, shogun), hand(1, BOOK_OF_MOON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(kite), EndTurn], &[Activate(BOOK_OF_MOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BOOK_OF_MOON), 1, "Book of Moon was activated: {}", s.context());
    assert_eq!(s.position_of(0, shogun), Some(1), "and negated, Shogun is still face-up Attack: {}", s.context());
    // The control, without the Kite.
    let Some(mut s) = field(&[], &[mon(0, shogun), hand(1, BOOK_OF_MOON)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(BOOK_OF_MOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, shogun), Some(8), "Book of Moon turned it face-down: {}", s.context());
}

/// Great Kite of Ninja: "The equipped monster cannot be targeted for attacks, but does not prevent your opponent from
/// attacking you directly." Shogun (1600) is my only monster, Warwolf (2000) attacks in turn 2: it attacks directly
/// (2000 damage) and Shogun is not destroyed. Without the Kite it would be.
#[test]
fn great_kite_makes_the_equipped_monster_untargetable_for_attacks() {
    let (kite, shogun) = (card("Great Kite of Ninja"), card("Ninja Master Shogun"));
    let Some(mut s) = field(&[], &[hand(0, kite), mon(0, shogun), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(kite), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "a direct attack: {}", s.context());
    assert_eq!(s.count(0, "mzone", shogun), 1, "Shogun was not the target: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, shogun), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", shogun), 1, "without the Kite Shogun is the target: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 400, "{}", s.context());
}

/// Great Kite of Ninja: "You can Tribute 1 monster, except the equipped monster; the equipped monster can attack directly this
/// turn." In turn 3 Thunder Kid is Tributed and Shogun (1600) attacks directly past Warwolf: 1600 damage, and Warwolf lives.
#[test]
fn great_kite_lets_the_equipped_monster_attack_directly_for_a_tribute() {
    let (kite, shogun) = (card("Great Kite of Ninja"), card("Ninja Master Shogun"));
    let spawns = [hand(0, kite), mon(0, shogun), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(kite), EndTurn, Activate(kite), Attack(shogun)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was Tributed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1600, "a direct attack past Warwolf: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", shogun), 1, "{}", s.context());
}

/// Ninja Master Shogun: "This card can only be Summoned while a "Ninja Commander Ikusa" and a "Ninja Soldier Katana" is in your
/// Graveyard. When this card is Summoned: you can Special Summon 1 "Ninja Soldier Katana" and 1 "Ninja Commando Kabuki" from
/// your Deck in face-up Attack Position." With both in the Graveyard Shogun is Tribute Summoned (Warwolf is the Tribute) and
/// brings a second Katana and Kabuki from the Deck.
#[test]
fn ninja_master_shogun_is_summoned_with_ikusa_and_katana_in_the_graveyard_and_fetches_two_ninjas() {
    let [shogun, ikusa, katana, kabuki] =
        ["Ninja Master Shogun", "Ninja Commander Ikusa", "Ninja Soldier Katana", "Ninja Commando Kabuki"].map(card);
    let spawns = [hand(0, shogun), grave(0, ikusa), grave(0, katana), mon(0, WARWOLF)];
    let Some(mut s) = field(&[katana, kabuki], &spawns) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Summon(shogun), Chain(shogun)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", shogun), 1, "Shogun was Summoned: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was the Tribute: {}", s.context());
    assert_eq!((s.count(0, "mzone", katana), s.count(0, "mzone", kabuki)), (1, 1), "from the Deck: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 2, "{}", s.context());
    assert_eq!((s.position_of(0, katana), s.position_of(0, kabuki)), (Some(1), Some(1)), "face-up Attack: {}", s.context());
    assert_eq!(s.count(0, "grave", katana), 1, "the Graveyard's Katana stays: {}", s.context());
}

/// Ninja Master Shogun: "a "Ninja Commander Ikusa" and a "Ninja Soldier Katana"": with only one of the two in the Graveyard it is
/// offered neither as a Tribute Summon nor a Special Summon.
#[test]
fn ninja_master_shogun_needs_both_ninjas_in_the_graveyard() {
    let [shogun, ikusa, katana] = ["Ninja Master Shogun", "Ninja Commander Ikusa", "Ninja Soldier Katana"].map(card);
    for graves in [[grave(0, ikusa), grave(0, ikusa)], [grave(0, katana), grave(0, katana)]] {
        let spawns = [hand(0, shogun), mon(0, WARWOLF), graves[0], graves[1]];
        let Some(mut s) = field(&[], &spawns) else { return };
        s.run(&[]).unwrap();
        assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "{}", s.context());
        assert!(!offered(&s, "summons").contains(&shogun.into()), "no Tribute Summon: {:?}", offered(&s, "summons"));
        assert!(!offered(&s, "special_summons").contains(&shogun.into()), "no Special Summon: {:?}", offered(&s, "special_summons"));
        clean(&s);
    }
}

/// Ninja Commander Ikusa and Ninja Commando Kabuki: "When this card is Summoned in face-up Attack Position: you can Special
/// Summon 1 "Ninja Soldier Katana" / "Kunoichi Ayame the Ninja Girl" from your hand in face-up Attack Position." Each brings its
/// own partner from the hand, in Attack Position, and not the other's.
#[test]
fn ninja_commander_and_commando_summon_their_partners_from_the_hand() {
    let [ikusa, kabuki, katana, ayame] =
        ["Ninja Commander Ikusa", "Ninja Commando Kabuki", "Ninja Soldier Katana", "Kunoichi Ayame the Ninja Girl"].map(card);
    for (ninja, partner, other) in [(ikusa, katana, ayame), (kabuki, ayame, katana)] {
        let Some(mut s) = field(&[], &[hand(0, ninja), hand(0, partner), hand(0, other)]) else { return };
        s.run(&[Summon(ninja), Chain(ninja)]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "mzone", partner), 1, "{ninja}'s partner came: {}", s.context());
        assert_eq!(s.position_of(0, partner), Some(1), "face-up Attack: {}", s.context());
        assert_eq!(s.count(0, "hand", other), 1, "the other ninja stayed in hand: {}", s.context());
        // Without the partner in hand (only the other one) nothing is offered.
        let Some(mut s) = field(&[], &[hand(0, ninja), hand(0, other)]) else { return };
        let err = s.run(&[Summon(ninja), Chain(ninja)]).expect_err("no partner in the hand");
        assert!(err.contains("never became possible"), "{err}");
        assert_eq!(s.count(0, "mzone", other), 0, "{}", s.context());
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Giant's Training and the Landstar monsters
// ---------------------------------------------------------------------------------------------------------------

/// Giant's Training (batch 3): "Tribute 1 "Tom Thumb" you control which has been face-up on the field for 1 or more turns;
/// Special Summon 1 "Globerman" from your hand or Deck." Tom Thumb is now a card of the game, Globerman still is not (no
/// card of that name), so the effect has nothing to Special Summon and can be shown only not offered: with a Tom Thumb that has
/// been on the field since turn 1, in turn 3, it is not on offer.
#[test]
fn giants_training_cannot_be_played_without_globerman() {
    let (training, thumb) = (card("Giant's Training"), card("Tom Thumb"));
    let Some(mut s) = field(&[], &[hand(0, training), mon(0, thumb)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(training)], &[EndTurn]]).expect_err("no Globerman");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", thumb), 1, "Tom Thumb was not Tributed: {}", s.context());
}

/// Landstar Forces (batch 2): "Special Summon as many Level 3 or lower "Landstar" monsters from your hand as possible in
/// face-up Attack Position." The three Landstar monsters of this batch are plain monsters (Level 3, Warrior, 1000/500,
/// 1000/1200 and 900/1200): all three come, in Attack Position, with those stats.
#[test]
fn landstar_forces_summons_the_three_vanilla_landstar_monsters() {
    let [forces, grappler, knight, brigadier] =
        ["Landstar Forces", "Grappler of Landstar", "Knight of Landstar", "Brigadier of Landstar"].map(card);
    let spawns = [hand(0, forces), hand(0, grappler), hand(0, knight), hand(0, brigadier)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(forces)]).unwrap();
    clean(&s);
    for (cid, atk, def) in [(grappler, 1000, 500), (knight, 1000, 1200), (brigadier, 900, 1200)] {
        let m = s.monster(0, cid).unwrap_or_else(|| panic!("{cid} is not on the field: {}", s.context()));
        assert_eq!((m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64()), (Some(atk), Some(def), Some(3)), "{m}");
        assert_eq!(m["position"], 1, "face-up Attack: {m}");
    }
    assert_eq!(s.count(0, "grave", forces), 1, "{}", s.context());
}
// ---------------------------------------------------------------------------------------------------------------
// Aero Nail, Rose Whip and Harpie Lady Sparrow Formation
// ---------------------------------------------------------------------------------------------------------------

/// Aero Nail and Rose Whip: "Equip only to "Harpie Lady". It gains 300 ATK." Harpie Lady (1300) has 1600 with either, and
/// Warwolf alone is no target for them.
#[test]
fn aero_nail_and_rose_whip_equip_to_harpie_lady_for_300_atk() {
    let harpie = card("Harpie Lady");
    for name in ["Aero Nail", "Rose Whip"] {
        let equip = card(name);
        let Some(mut s) = field(&[], &[hand(0, equip), mon(0, harpie), mon(0, WARWOLF)]) else { return };
        s.run(&[Activate(equip)]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "szone", equip), 1, "{name}: {}", s.context());
        assert_eq!(s.attack_of(0, harpie), Some(1600), "{name}: +300 ATK: {}", s.context());
        assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "{name}: only Harpie Lady: {}", s.context());
        let Some(mut s) = field(&[], &[hand(0, equip), mon(0, WARWOLF)]) else { return };
        let err = s.run(&[Activate(equip)]).expect_err("no Harpie Lady");
        assert!(err.contains("never became possible"), "{name}: {err}");
        clean(&s);
    }
}

/// Aero Nail: "When this card is sent from the field to the Graveyard: You can target 1 "Harpie Lady" you control; equip this
/// card to that target." Team 1's Mystical Space Typhoon destroys the Nail and it equips itself to the Harpie Lady again
/// (1600 ATK, on the field, not in the Graveyard).
#[test]
fn aero_nail_equips_itself_again_when_it_leaves_the_field() {
    let (nail, harpie) = (card("Aero Nail"), card("Harpie Lady"));
    let spawns = [hand(0, nail), mon(0, harpie), hand(1, MYSTICAL_SPACE_TYPHOON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(nail), EndTurn, Chain(nail)], &[Activate(MYSTICAL_SPACE_TYPHOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MYSTICAL_SPACE_TYPHOON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", nail), 0, "the Nail left the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "szone", nail), 1, "and is equipped again: {}", s.context());
    assert_eq!(s.attack_of(0, harpie), Some(1600), "{}", s.context());
}

/// Aero Nail, the control: when its Harpie Lady is destroyed (Dark Hole) there is no "Harpie Lady you control" to equip it to,
/// and it stays in the Graveyard.
#[test]
fn aero_nail_has_nothing_to_equip_when_its_harpie_lady_is_destroyed() {
    let (nail, harpie) = (card("Aero Nail"), card("Harpie Lady"));
    let spawns = [hand(0, nail), mon(0, harpie), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(nail), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", harpie), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", nail), 1, "the Nail stays in the Graveyard: {}", s.context());
}

/// Harpie Lady Sparrow Formation: "When your opponent declares an attack while you control 2 or more "Harpie Lady": Negate the
/// attack, and if you do, end the Battle Phase." Warwolf's attack is negated and the opponent is in its Main Phase 2: Thunder
/// Kid cannot attack either.
#[test]
fn harpie_lady_sparrow_formation_negates_the_attack_and_ends_the_battle_phase() {
    let (sparrow, harpie) = (card("Harpie Lady Sparrow Formation"), card("Harpie Lady"));
    let spawns = [set(0, sparrow), mon(0, harpie), mon(0, harpie), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(sparrow)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(msg::SELECT_IDLECMD), Some(1)), "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over (Thunder Kid cannot attack): {}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "mzone", harpie), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", sparrow), 1, "{}", s.context());
}

/// Harpie Lady Sparrow Formation: "2 or more": with one Harpie Lady the Trap is not offered and the attack lands.
#[test]
fn harpie_lady_sparrow_formation_needs_two_harpie_ladies() {
    let (sparrow, harpie) = (card("Harpie Lady Sparrow Formation"), card("Harpie Lady"));
    let Some(mut s) = field(&[], &[set(0, sparrow), mon(0, harpie), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(sparrow)], &[Attack(WARWOLF)]]).expect_err("only one Harpie Lady");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "grave", harpie), 1, "Warwolf destroyed it: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "2000 against 1300: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Trap Displacement
// ---------------------------------------------------------------------------------------------------------------

/// Trap Displacement: "Switch the effect of a Trap Card that targets 1 monster you control to another correct target your
/// opponent controls." Team 1's Raigeki Break (discard 1 card; destroy 1 targeted card on the field) targets my Warwolf; I
/// chain the Trap and it is switched to Thunder Kid, team 1's own monster: Warwolf lives and Thunder Kid is destroyed.
#[test]
fn trap_displacement_switches_a_targeting_trap_onto_the_opponents_monster() {
    let td = card("Trap Displacement");
    let spawns = [set(0, td), mon(0, WARWOLF), set(1, RAIGEKI_BREAK), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(td)], &[Activate(RAIGEKI_BREAK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", td), 1, "Trap Displacement was activated: {}", s.context());
    assert_eq!(s.count(1, "grave", RAIGEKI_BREAK), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf was spared: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "Thunder Kid took its place: {}", s.context());
}

/// Trap Displacement, the control: without it Raigeki Break destroys the Warwolf it targets.
#[test]
fn raigeki_break_destroys_its_target_without_trap_displacement() {
    let spawns = [mon(0, WARWOLF), set(1, RAIGEKI_BREAK), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[], &[Activate(RAIGEKI_BREAK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Orichalcos Malevolence, Nightmare Mirror, Twin-Bow Centaur and Prophecy
// ---------------------------------------------------------------------------------------------------------------

/// Orichalcos Malevolence: "Once per turn: You can target 1 Defense Position monster your opponent controls; change that
/// target to face-up Attack Position." A Set Dark Magician is chosen over a face-up Thunder Kid in Defense and turns face-up
/// Attack; Thunder Kid stays. A second activation the same turn is not offered.
#[test]
fn orichalcos_malevolence_turns_a_defense_position_monster_face_up_attack() {
    let malevolence = card("Orichalcos Malevolence");
    let spawns = [mon(0, malevolence), mon_def(1, THUNDER_KID), mon_set(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, DARK_MAGICIAN)]);
    s.run(&[Activate(malevolence)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, DARK_MAGICIAN), Some(1), "face-up Attack Position: {}", s.context());
    assert_eq!(s.position_of(1, THUNDER_KID), Some(4), "the other stays in Defense: {}", s.context());
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&malevolence.into()), "once per turn: {:?}", offered(&s, "activates"));
}

/// Orichalcos Malevolence: "Defense Position monster": with only Warwolf in Attack Position it is not offered.
#[test]
fn orichalcos_malevolence_needs_a_defense_position_target() {
    let malevolence = card("Orichalcos Malevolence");
    let Some(mut s) = field(&[], &[mon(0, malevolence), mon(1, WARWOLF)]) else { return };
    let err = s.run(&[Activate(malevolence)]).expect_err("no Defense Position monster");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Nightmare Mirror: "When an opponent's monster declares an attack: Discard 1 card; the attack becomes a direct attack to
/// your opponent, but the battle damage they would have taken from this battle becomes 1000." Warwolf (2000) attacks my
/// Thunder Kid: a card is discarded, Thunder Kid is not attacked, and the attacker's controller takes 1000, I take nothing.
#[test]
fn nightmare_mirror_turns_the_attack_around_for_1000_damage() {
    let mirror = card("Nightmare Mirror");
    let Some(mut s) = field(&[], &[set(0, mirror), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let hand_before = hand_size(&s, 0);
    s.run_both([&[ChainOnAttack(mirror)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", mirror), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before - 1, "one card was discarded: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid was not hit: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "the attacker takes 1000: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Twin-Bow Centaur: "Once per turn: You can target 1 monster you control and 1 monster your opponent controls; toss a coin
/// and call it. If you call it right: Banish the monster your opponent controls, and if you do, inflict damage to your
/// opponent equal to the original ATK of that monster. If you call it wrong: Banish the monster you control, and if you do,
/// take damage equal to the original ATK of that monster." Dark Blade (1800) against Warwolf (2000): in some seeds the
/// call is right (Warwolf banished, 2000 to the opponent), in others wrong (Dark Blade banished, 1800 to me).
#[test]
fn twin_bow_centaur_banishes_a_monster_by_the_coin_and_damages_for_its_original_atk() {
    let centaur = card("Twin-Bow Centaur");
    let (mut right, mut wrong) = (0, 0);
    for seed in 1..=10 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[spell(0, centaur), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
        s.run(&[Activate(centaur)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        if s.count(1, "removed", WARWOLF) == 1 {
            assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2000), "seed {seed}: right: {}", s.context());
            assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "seed {seed}: {}", s.context());
            right += 1;
        } else {
            assert_eq!(s.count(0, "removed", DARK_BLADE), 1, "seed {seed}: wrong: {}", s.context());
            assert_eq!((s.lp(0), s.lp(1)), (8000 - 1800, 8000), "seed {seed}: {}", s.context());
            assert_eq!(s.count(1, "mzone", WARWOLF), 1, "seed {seed}: {}", s.context());
            wrong += 1;
        }
    }
    assert!(right > 0 && wrong > 0, "both calls in 10 seeds: right {right}, wrong {wrong}");
}

/// Twin-Bow Centaur: "You cannot declare an attack the turn you activate this effect." In turn 3 the effect is used, and
/// Thunder Kid, which was not a target, then cannot attack.
#[test]
fn twin_bow_centaur_stops_attacks_the_turn_it_is_used() {
    let centaur = card("Twin-Bow Centaur");
    let spawns = [spell(0, centaur), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(centaur), Attack(THUNDER_KID)], &[EndTurn]]).expect_err("no attack after the effect");
    assert!(err.contains("can't attack"), "{err}");
    assert_eq!(s.count(0, "szone", centaur), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
}

/// Prophecy: "Choose 1 card in your opponent's hand and guess whether its ATK is above or below 2000. If you guess correctly,
/// add that card to your hand." (A Trap in the database: Set in turn 1, activated in turn 3.) Dark Magician (2500) is chosen:
/// "above" takes it, "below" leaves it. Warwolf's 2000 is neither above nor below: both guesses fail.
#[test]
fn prophecy_takes_the_card_only_when_the_guess_is_right() {
    let prophecy = card("Prophecy");
    let case = |pick: u16, guess: usize| -> Option<Scenario> {
        let mut s = field(&[], &[set(0, prophecy), hand(1, pick)])?;
        s.prefer(&[(1, pick)]);
        s.choose_options(&[guess]);
        // Team 1 Normal Summons a Feral Imp, so that its End Phase has no hand-size discard to take the chosen card.
        s.run_both([&[EndTurn, Activate(prophecy)], &[Summon(FILLER)]]).unwrap();
        clean(&s);
        Some(s)
    };
    let Some(s) = case(DARK_MAGICIAN, 0) else { return };
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "above 2000 was right: {}", s.context());
    assert_eq!(s.count(1, "hand", DARK_MAGICIAN), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", prophecy), 1, "{}", s.context());
    let Some(s) = case(DARK_MAGICIAN, 1) else { return };
    assert_eq!(s.count(1, "hand", DARK_MAGICIAN), 1, "below 2000 was wrong: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 0, "{}", s.context());
    for guess in [0, 1] {
        let Some(s) = case(WARWOLF, guess) else { return };
        assert_eq!(s.count(1, "hand", WARWOLF), 1, "2000 is neither above nor below (guess {guess}): {}", s.context());
        assert_eq!(s.count(0, "hand", WARWOLF), 0, "{}", s.context());
    }
}
