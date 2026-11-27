//! Anime-only cards, group 15 (rows 5693-5729 of engine/added/cards.tsv): Howard X Miller's, Sartyr's, Trapper's, Tyranno
//! Hassleberry's and Aster Phoenix's cards. They exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play Spell answering the
//! opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! Covered: the cards the per-card fuzz flagged (Elegant Light LV4, Curry Pot, Capture Snare, Dark Auction, Decoy Baby, Hunting
//! Net, Lure Phantom, Substitute Pain, Card of Reversal, Dino Stomp, New Ultra Evolution, Plasma Discharger, Plasma Roar),
//! Spell Spice Cinnamon (the fuzz's failing duel: today's override `engine/overrides/c511001223.lua` against a Spell Transfer),
//! Super Guard (and its case at infinite LP), the signature cards (Plasma Roar, D - Acceleration, Dino Blast, Dino Stomp, Curry
//! Fiend Roo and Curry Pot, the Prey Counter cards) and a seeded sample of the rest (Carrotman, Counter Spice Cumin, Curry
//! Fiend Roo, D - Acceleration, Level Pod, Mix Spice Garamasala, Monster Register), with the other cards of the batch that were
//! quick to try. This batch makes no Token.
//! Not coverable: Plasma Counter's "D - Force is face-up on top of your Deck" (no card of the game turns a Deck's top face-up
//! and a spawn can't place it there; only its absence is tested). Purse with a Hole's hand lock is tested in
//! `anime_fixes.rs` (`purse_with_a_hole_locks_the_opponents_hand_for_the_turn`).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 2, 700/600 vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MST: u16 = 4909; // Mystical Space Typhoon
const LEGENDARY_SWORD: u16 = 4307; // Equip Spell for a Warrior, +300 ATK and DEF
const SABERSAURUS: u16 = 6796; // Level 4 Dinosaur, 1900/500
const ULTIMATE_TYRANNO: u16 = 6777; // Level 8 Dinosaur, 3000/2200
const HARPIE_LADY: u16 = 4068; // Winged Beast, Level 4
const PLASMA: u16 = 7096; // Destiny HERO - Plasma, Level 8, 1900/600
const DIAMOND_DUDE: u16 = 6660; // Destiny HERO - Diamond Dude, Level 4, 1400/1600
const MIRROR_FORCE: u16 = 4887;
const KURIBOH: u16 = 4064;
const INFINITE: i64 = 19_999_998;
/// The counter the Prey cards place (0x1107), as the engine's field report names it.
const PREY: &str = "4359";

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn banished(team: u32, cid: u16) -> Spawn { Spawn { team, place: Banished, cid } }
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

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn pile(s: &Scenario, team: usize, place: &str) -> usize {
    s.state()[team][place].as_array().map_or(0, |v| v.iter().filter(|c| !c.is_null()).count())
}

fn deck_size(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap_or(-1)
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// The Prey Counters on a team's Spell & Trap Zone cards with this id (the first such card).
fn prey(s: &Scenario, team: usize, cid: u16) -> i64 {
    let card = s.state()[team]["szone"].as_array().and_then(|z| z.iter().find(|c| c["cid"] == cid));
    card.map_or(-1, |c| c["counters"][PREY].as_i64().unwrap_or(0))
}

fn in_spell_zone(s: &Scenario, team: usize, cid: u16) -> usize {
    s.count(team, "szone", cid)
}

/// A duel where team 1's Compulsory Evacuation Device returns its own `caught` in turn 2 and team 0's Hunting Net, Set
/// from the start, takes it into its Spell & Trap Zone with a Prey Counter instead. Returns at team 1's turn 2 idle prompt.
fn net_catches(caught: u16, spawns: &[Spawn]) -> Option<Scenario> {
    let [net, ced] = ["Hunting Net", "Compulsory Evacuation Device"].map(card);
    let mut all = vec![set(0, net), set(1, ced), mon(1, caught)];
    all.extend_from_slice(spawns);
    let mut s = field(&[], &all)?;
    s.prefer(&[(0, FILLER), (1, FILLER), (1, caught)]);
    s.run_both([&[Chain(net)], &[Activate(ced)]]).unwrap();
    keep_hands(&mut s);
    clean(&s);
    assert_eq!(in_spell_zone(&s, 0, caught), 1, "Hunting Net holds the returned monster: {}", s.context());
    Some(s)
}

// ---------------------------------------------------------------------------------------------------------------
// Trapper: Hunting Net, Capture Snare, Dark Auction, Decoy Baby, Substitute Pain and the Prey Counter
// ---------------------------------------------------------------------------------------------------------------

/// Hunting Net: "If a monster would return to the hand: Place it face-up in your Spell & Trap Zone, instead. Then, place 1
/// Prey Counter on it." Team 1's Compulsory Evacuation Device returns its Thunder Kid and Team 0's Hunting Net takes the
/// monster into team 0's Spell & Trap Zone with one Prey Counter: not team 1's hand, and the bounce does not happen.
#[test]
fn hunting_net_takes_a_monster_that_would_return_to_the_hand_and_marks_it() {
    let net = card("Hunting Net");
    let Some(s) = net_catches(THUNDER_KID, &[]) else { return };
    assert_eq!(prey(&s, 0, THUNDER_KID), 1, "one Prey Counter: {}", s.context());
    assert_eq!(s.count(1, "hand", THUNDER_KID), 0, "not returned to team 1's hand: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", net), 1, "{}", s.context());
    clean(&s);
}

/// Hunting Net answers the controller's own bounce as well ("a monster", not "an opponent's monster"): the same, with team 0's
/// own Compulsory Evacuation Device, Set in the same duel, on its own Warwolf.
#[test]
fn hunting_net_takes_its_controllers_own_monster_too() {
    let [net, ced] = ["Hunting Net", "Compulsory Evacuation Device"].map(card);
    let Some(mut s) = field(&[], &[set(0, net), set(0, ced), mon(0, WARWOLF)]) else { return };
    s.run(&[Activate(ced), Chain(net)]).unwrap();
    clean(&s);
    assert_eq!(in_spell_zone(&s, 0, WARWOLF), 1, "{}", s.context());
    assert_eq!(prey(&s, 0, WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", WARWOLF), 0, "{}", s.context());
}

/// Capture Snare: "When your opponent declares an attack while you control a card with at least 1 Prey Counter on it: Negate
/// the attack, then place the attacking monster face-up in your Spell & Trap Zone, and place 1 Prey Counter on it." Team 0's
/// Hunting Net caught Thunder Kid; team 1's Warwolf attacks in the same turn and Capture Snare takes it too: Warwolf is in
/// team 0's Spell & Trap Zone with a Prey Counter, and no damage is dealt.
#[test]
fn capture_snare_negates_an_attack_and_takes_the_attacker_with_a_prey_counter() {
    let [net, snare, ced] = ["Hunting Net", "Capture Snare", "Compulsory Evacuation Device"].map(card);
    let spawns = [set(0, net), set(0, snare), set(1, ced), mon(1, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, THUNDER_KID)]);
    s.run_both([&[Chain(net), ChainOnAttack(snare)], &[Activate(ced), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(prey(&s, 0, THUNDER_KID), 1, "Hunting Net's catch: {}", s.context());
    assert_eq!(prey(&s, 0, WARWOLF), 1, "Capture Snare's catch: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", snare), 1, "{}", s.context());
}

/// Capture Snare needs a card with a Prey Counter: without one (no Hunting Net) the attack goes through and Capture Snare is
/// not offered.
#[test]
fn capture_snare_needs_a_prey_counter() {
    let snare = card("Capture Snare");
    let Some(mut s) = field(&[], &[set(0, snare), mon(1, WARWOLF)]) else { return };
    let err = s.run_both([&[ChainOnAttack(snare)], &[Attack(WARWOLF)]]).expect_err("no Prey Counter");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "szone", snare), 1, "{}", s.context());
}

/// Dark Auction: "Target 1 monster on the field with a Prey Counter; return it to the hand, then, inflict damage to your opponent
/// equal to that monster's ATK." Warwolf (2000) sits in team 0's Spell & Trap Zone with a Prey Counter: it returns to its owner's
/// hand (team 1's) and team 1 takes 2000.
#[test]
fn dark_auction_returns_a_prey_counter_monster_and_inflicts_its_atk() {
    let auction = card("Dark Auction");
    let Some(mut s) = net_catches(WARWOLF, &[hand(0, auction)]) else { return };
    s.run_both([&[Activate(auction)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(in_spell_zone(&s, 0, WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(1, "hand", WARWOLF), 1, "Warwolf returned to its owner's hand: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "grave", auction), 1, "{}", s.context());
}

/// Dark Auction without a Prey Counter anywhere is not offered.
#[test]
fn dark_auction_needs_a_prey_counter() {
    let auction = card("Dark Auction");
    let Some(mut s) = field(&[], &[hand(0, auction), mon(1, WARWOLF)]) else { return };
    let err = s.run(&[Activate(auction)]).expect_err("nothing marked");
    assert!(err.contains("never became possible"), "{err}");
}

/// Decoy Baby: "Target 1 monster in your Spell & Trap Zone with a Prey Counter; place 1 monster your opponent controls with the
/// same Type as that monster face-up in your Spell & Trap Zone, and place 1 Prey Counter on it." Warwolf (Beast-Warrior) is
/// held with a Prey Counter; team 1's second Warwolf follows it and gets one, while its Thunder Kid (Thunder) stays.
#[test]
fn decoy_baby_takes_a_monster_of_the_same_type_as_the_prey() {
    let baby = card("Decoy Baby");
    let Some(mut s) = net_catches(WARWOLF, &[hand(0, baby), mon(1, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    s.run_both([&[Activate(baby)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", WARWOLF), 2, "both Warwolves are held: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "a Thunder monster is not the Prey's Type: {}", s.context());
    let counters: Vec<i64> = s.state()[0]["szone"].as_array().unwrap().iter().filter(|c| c["cid"] == WARWOLF).map(|c| c["counters"][PREY].as_i64().unwrap_or(0)).collect();
    assert_eq!(counters, vec![1, 1], "each has a Prey Counter: {}", s.context());
    assert_eq!(s.count(0, "grave", baby), 1, "{}", s.context());
}

/// Decoy Baby needs a monster in my Spell & Trap Zone with a Prey Counter: not offered with none.
#[test]
fn decoy_baby_needs_a_held_monster() {
    let baby = card("Decoy Baby");
    let Some(mut s) = field(&[], &[hand(0, baby), mon(1, WARWOLF)]) else { return };
    let err = s.run(&[Activate(baby)]).expect_err("nothing held");
    assert!(err.contains("never became possible"), "{err}");
}

/// Substitute Pain: "Once per turn: Inflict 400 damage to your opponent for each card you control with a Prey Counter(s)."
/// One held monster: 400; once per turn, so a second use waits for the next turn; and with no Prey Counter at all it
/// inflicts nothing.
#[test]
fn substitute_pain_inflicts_400_for_each_card_with_a_prey_counter() {
    let pain = card("Substitute Pain");
    let Some(mut s) = net_catches(THUNDER_KID, &[hand(0, pain)]) else { return };
    s.run_both([&[Activate(pain), Activate(pain)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 400, "one marked card: {}", s.context());
    // Once per turn: it is used again only in turn 5.
    s.run_both([&[Activate(pain)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
    clean(&s);
}

/// Substitute Pain with two marked cards (Hunting Net's and Decoy Baby's): 800.
#[test]
fn substitute_pain_counts_every_marked_card() {
    let [pain, baby] = ["Substitute Pain", "Decoy Baby"].map(card);
    let Some(mut s) = net_catches(WARWOLF, &[hand(0, pain), hand(0, baby), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Activate(baby), Activate(pain), Activate(pain)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
}

/// Substitute Pain with no Prey Counter: its effect resolves for 0 damage (the case the per-card fuzz saw 168 times).
#[test]
fn substitute_pain_with_no_prey_counter_inflicts_nothing() {
    let pain = card("Substitute Pain");
    let Some(mut s) = field(&[], &[hand(0, pain)]) else { return };
    s.run(&[Activate(pain), Activate(pain)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    assert_eq!(s.count(0, "szone", pain), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Elegant Light LV4
// ---------------------------------------------------------------------------------------------------------------

/// Elegant Light LV4 (a Trap): "If this card is sent to the Graveyard by a card effect: You can Special Summon 1 Level 4
/// monster from your Deck." Team 1's Mystical Space Typhoon destroys the Set card and team 0 brings Warwolf from its Deck.
#[test]
fn elegant_light_summons_a_level_4_monster_when_a_card_effect_sends_it_to_the_graveyard() {
    let elegant = card("Elegant Light LV4");
    let Some(mut s) = field(&[WARWOLF], &[set(0, elegant), hand(1, MST)]) else { return };
    s.prefer(&[(0, elegant), (0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[Chain(elegant)], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", elegant), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf came from the Deck: {}", s.context());
    assert_eq!(s.position_of(0, WARWOLF), Some(1), "face-up Attack: {}", s.context());
}

/// Elegant Light LV4: "sent to the Graveyard by a card effect": discarded in the End Phase for the hand size (a rule, not an
/// effect) it does nothing: Warwolf stays in the Deck.
#[test]
fn elegant_light_does_nothing_when_discarded_for_the_hand_size() {
    let elegant = card("Elegant Light LV4");
    let Some(mut s) = field(&[WARWOLF], &[hand(0, elegant), hand(0, FILLER)]) else { return };
    s.prefer(&[(0, elegant)]);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", elegant), 1, "it was the discard: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Sartyr: the Curry cards
// ---------------------------------------------------------------------------------------------------------------

/// Curry Pot: "Any monster sent to the Graveyard is banished instead. If "Carrotman", "Potatoman" and "Onionman" that you own
/// are currently banished by this effect (Quick Effect): You can banish all monsters on the field; Special Summon 1 "Curry
/// Fiend Roo" from your hand or Deck." Dark Hole destroys the three (and team 1's Warwolf): all four are banished, not sent
/// to a Graveyard; Thunder Kid comes after it and is banished as the cost, and Curry Fiend Roo is Special Summoned from the
/// hand with 300 ATK for each of the five banished monsters.
#[test]
fn curry_pot_banishes_monsters_and_brings_curry_fiend_roo_when_the_three_vegetables_are_banished() {
    let [pot, roo, potatoman, carrotman, onionman] = ["Curry Pot", "Curry Fiend Roo", "Potatoman", "Carrotman", "Onionman"].map(card);
    let spawns = [hand(0, pot), hand(0, DARK_HOLE), hand(0, THUNDER_KID), hand(0, roo), mon(0, potatoman), mon(0, carrotman), mon(0, onionman), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(pot), Activate(DARK_HOLE), Summon(THUNDER_KID)]).unwrap();
    clean(&s);
    for cid in [potatoman, carrotman, onionman] {
        assert_eq!((s.count(0, "removed", cid), s.count(0, "grave", cid)), (1, 0), "{cid} is banished, not in the Graveyard: {}", s.context());
    }
    assert_eq!((s.count(1, "removed", WARWOLF), s.count(1, "grave", WARWOLF)), (1, 0), "{}", s.context());
    s.run(&[Activate(pot)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", THUNDER_KID), 1, "banished as the cost: {}", s.context());
    assert_eq!(s.count(0, "mzone", roo), 1, "Curry Fiend Roo came from the hand: {}", s.context());
    assert_eq!(s.attack_of(0, roo), Some(5 * 300), "300 for each banished monster: {}", s.context());
}

/// Curry Pot: the Quick Effect needs all three vegetables banished by it: with Onionman missing, it is not offered.
#[test]
fn curry_pot_needs_all_three_vegetables_banished() {
    let [pot, roo, potatoman, carrotman] = ["Curry Pot", "Curry Fiend Roo", "Potatoman", "Carrotman"].map(card);
    let spawns = [hand(0, pot), hand(0, DARK_HOLE), hand(0, THUNDER_KID), hand(0, roo), mon(0, potatoman), mon(0, carrotman)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(pot), Activate(DARK_HOLE), Summon(THUNDER_KID), Activate(pot)]).expect_err("two of three");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "hand", roo) + s.count(0, "grave", roo), 1, "Roo stays where it was: {}", s.context());
    assert_eq!(s.count(0, "mzone", roo), 0, "{}", s.context());
}

/// Curry Fiend Roo: "This card gains 200 ATK for each "Spice" card in your Graveyard and 300 ATK for each banished monster."
/// Three Spice cards in team 0's Graveyard (Spell Spice Caraway, Mix Spice Garamasala, Counter Spice Cumin; the Spice card in
/// team 1's Graveyard does not count) and two banished monsters (one of each side's) give 600 + 600; a Spice Spell played
/// afterwards adds 200.
#[test]
fn curry_fiend_roo_gains_atk_for_spice_cards_and_banished_monsters() {
    let [roo, caraway, garamasala, cumin, pepper] = ["Curry Fiend Roo", "Spell Spice Caraway", "Mix Spice Garamasala", "Counter Spice Cumin", "Red Pepper Spice"].map(card);
    let spawns = [mon(0, roo), grave(0, caraway), grave(0, garamasala), grave(0, cumin), grave(1, pepper), banished(0, WARWOLF), banished(1, THUNDER_KID), hand(0, caraway)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    assert_eq!(s.attack_of(0, roo), Some(600 + 600), "3 Spice cards and 2 banished monsters: {}", s.context());
    s.run(&[Activate(caraway)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", caraway), 2, "{}", s.context());
    assert_eq!(s.attack_of(0, roo), Some(800 + 600), "a fourth Spice card: {}", s.context());
}

/// Potatoman, Carrotman and Onionman: vanilla Plant monsters, Level 3 900/800, Level 3 800/900 and Level 4 1200/1200.
#[test]
fn the_three_vegetables_are_vanilla_plants() {
    let [potatoman, carrotman, onionman] = ["Potatoman", "Carrotman", "Onionman"].map(card);
    let Some(mut s) = field(&[], &[mon(0, potatoman), mon(0, carrotman), mon(0, onionman)]) else { return };
    for (cid, atk, def, level) in [(potatoman, 900, 800, 3), (carrotman, 800, 900, 3), (onionman, 1200, 1200, 4)] {
        let m = s.monster(0, cid).unwrap_or_else(|| panic!("{cid}: {}", s.context()));
        assert_eq!((m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64()), (Some(atk), Some(def), Some(level), Some(0x400)), "{cid}");
    }
    // Without Curry Pot a destroyed one goes to the Graveyard: Dark Hole.
    s.run(&[]).unwrap();
    let Some(mut s) = field(&[], &[hand(0, DARK_HOLE), mon(0, carrotman)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", carrotman), 1, "{}", s.context());
}

/// Spell Spice Caraway: "Inflict 200 damage to your opponent and gain 200 LP."
#[test]
fn spell_spice_caraway_inflicts_200_and_gains_200() {
    let caraway = card("Spell Spice Caraway");
    let Some(mut s) = field(&[], &[hand(0, caraway)]) else { return };
    s.run(&[Activate(caraway)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8200, 7800), "{}", s.context());
    assert_eq!(s.count(0, "grave", caraway), 1, "{}", s.context());
}

/// Red Pepper Spice: "All monsters you control gain 300 ATK, and all monsters your opponent controls lose 300 ATK. These
/// changes last until the End Phase." Warwolf 2300 against Thunder Kid 400; both are back to normal after the turn. It needs
/// monsters on both sides.
#[test]
fn red_pepper_spice_raises_mine_and_lowers_the_opponents_until_the_end_phase() {
    let pepper = card("Red Pepper Spice");
    let Some(mut s) = field(&[], &[hand(0, pepper), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(pepper)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, THUNDER_KID), s.attack_of(1, DARK_BLADE)), (Some(2300), Some(1000), Some(1500)), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, THUNDER_KID), s.attack_of(1, DARK_BLADE)), (Some(2000), Some(700), Some(1800)), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, pepper), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(pepper)]).expect_err("no monster of the opponent's");
    assert!(err.contains("never became possible"), "{err}");
}

/// Mix Spice Garamasala: "Add 3 "Spell Spice" cards from your Deck to your hand." Caraway, Cinnamon and Red Pepper Spice are
/// at the bottom of the Deck; all three are added. With only two in the Deck it can't be activated.
#[test]
fn mix_spice_garamasala_adds_three_spell_spice_cards_from_the_deck() {
    let [mix, caraway, cinnamon, pepper] = ["Mix Spice Garamasala", "Spell Spice Caraway", "Spell Spice Cinnamon", "Red Pepper Spice"].map(card);
    let Some(mut s) = field(&[caraway, cinnamon, pepper], &[hand(0, mix)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, caraway), (0, cinnamon), (0, pepper)]);
    s.prefer_most(true);
    let deck = deck_size(&s, 0);
    s.run(&[Activate(mix)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", caraway), s.count(0, "hand", cinnamon), s.count(0, "hand", pepper)), (1, 1, 1), "{}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 3, "{}", s.context());
    assert_eq!(s.count(0, "grave", mix), 1, "{}", s.context());
    let Some(mut s) = field(&[caraway, cinnamon], &[hand(0, mix)]) else { return };
    let err = s.run(&[Activate(mix)]).expect_err("only two Spell Spice cards");
    assert!(err.contains("never became possible"), "{err}");
}

/// Spell Spice Cinnamon: "Target 1 face-up Attack Position monster you control and 1 face-up Defense Position monster your
/// opponent controls; Change your monster to face-up Defense Position, and change your opponent's monster to face-up Attack
/// Position." Warwolf goes to Defense and team 1's Thunder Kid (in Defense) to Attack.
#[test]
fn spell_spice_cinnamon_swaps_the_positions_of_two_monsters() {
    let cinnamon = card("Spell Spice Cinnamon");
    let Some(mut s) = field(&[], &[hand(0, cinnamon), mon(0, WARWOLF), mon_def(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(cinnamon)]).unwrap();
    clean(&s);
    assert_eq!((s.position_of(0, WARWOLF), s.position_of(1, THUNDER_KID)), (Some(4), Some(1)), "{}", s.context());
    assert_eq!(s.count(0, "grave", cinnamon), 1, "{}", s.context());
    // It needs both targets: my monster in Defense does not do.
    let Some(mut s) = field(&[], &[hand(0, cinnamon), mon_def(0, WARWOLF), mon_def(1, THUNDER_KID)]) else { return };
    let err = s.run(&[Activate(cinnamon)]).expect_err("no Attack Position monster of mine");
    assert!(err.contains("never became possible"), "{err}");
}

/// Spell Spice Cinnamon answered by Spell Transfer (the per-card fuzz, seed 64067): Spell Transfer negates it and runs its
/// effect as its own, which picks the targets again for Spell Transfer's controller; with no Attack Position monster of
/// his own there is none, and ProjectIgnis' script indexed the missing target (engine/overrides/c511001223.lua). Now
/// the negated Cinnamon changes no position, and no error is raised.
#[test]
fn spell_transfer_negating_cinnamon_without_targets_of_its_own_does_nothing() {
    let [cinnamon, transfer] = ["Spell Spice Cinnamon", "Spell Transfer"].map(card);
    let spawns = [set(0, transfer), mon_def(0, THUNDER_KID), hand(1, cinnamon), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(transfer)], &[Activate(cinnamon)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", transfer), 1, "Spell Transfer was activated: {}", s.context());
    assert_eq!(s.count(1, "grave", cinnamon), 1, "{}", s.context());
    assert_eq!((s.position_of(0, THUNDER_KID), s.position_of(1, WARWOLF)), (Some(4), Some(1)), "no position changed: {}", s.context());
}

/// Counter Spice Cumin (a Trap): "When a monster you control is destroyed: Special Summon 1 Level 4 or lower monster from your
/// hand in Attack Position." Team 0's own Dark Hole destroys its Warwolf; Thunder Kid (Level 2) comes from the hand in Attack
/// Position, and Dark Magician (Level 7) in the hand is no choice.
#[test]
fn counter_spice_cumin_summons_a_level_4_or_lower_monster_when_my_monster_is_destroyed() {
    let cumin = card("Counter Spice Cumin");
    let spawns = [set(0, cumin), mon(0, WARWOLF), hand(0, THUNDER_KID), hand(0, DARK_MAGICIAN), hand(0, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(DARK_HOLE), Chain(cumin)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Dark Hole destroyed Warwolf: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid came from the hand: {}", s.context());
    assert_eq!(s.position_of(0, THUNDER_KID), Some(1), "in Attack Position: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", cumin), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Tyranno Hassleberry: Dino Blast, Dino Stomp, New Ultra Evolution, Archeonys
// ---------------------------------------------------------------------------------------------------------------

/// Dino Blast: "Target 1 face-up Dinosaur-Type monster you control. Inflict damage to your opponent equal to that monster's Level
/// x300." Sabersaurus (Level 4) deals 1200 and Ultimate Tyranno (Level 8) 2400. With no Dinosaur it is not offered.
#[test]
fn dino_blast_inflicts_300_for_each_level_of_a_dinosaur() {
    let blast = card("Dino Blast");
    for (dinosaur, damage) in [(SABERSAURUS, 1200), (ULTIMATE_TYRANNO, 2400)] {
        let Some(mut s) = field(&[], &[hand(0, blast), mon(0, SABERSAURUS), mon(0, ULTIMATE_TYRANNO), mon(0, WARWOLF)]) else { return };
        keep_hands(&mut s);
        s.prefer(&[(0, dinosaur)]);
        s.run(&[Activate(blast)]).unwrap();
        clean(&s);
        assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - damage), "{dinosaur}: {}", s.context());
        assert_eq!(s.count(0, "grave", blast), 1, "{}", s.context());
    }
    let Some(mut s) = field(&[], &[hand(0, blast), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(blast)]).expect_err("Warwolf is no Dinosaur");
    assert!(err.contains("never became possible"), "{err}");
}

/// Dino Stomp (a Quick-Play Spell): "When a Dinosaur-Type monster you control declares an attack: Target 1 Spell/Trap your
/// opponent controls; destroy it." Sabersaurus attacks in turn 3 and the Set Dino Stomp destroys team 1's Set Dian Keto.
/// Warwolf, no Dinosaur, does not allow it.
#[test]
fn dino_stomp_destroys_a_spell_or_trap_when_a_dinosaur_attacks() {
    let stomp = card("Dino Stomp");
    let Some(mut s) = field(&[], &[set(0, stomp), mon(0, SABERSAURUS), set(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Attack(SABERSAURUS), ChainOnAttack(stomp)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "team 1's Set Spell was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", stomp), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, stomp), mon(0, WARWOLF), set(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[EndTurn, Attack(WARWOLF), ChainOnAttack(stomp)]).expect_err("Warwolf is no Dinosaur");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "szone", DIAN_KETO), 1, "{}", s.context());
}

/// New Ultra Evolution: "Tribute 1 Winged Beast-Type monster; Special Summon 1 Dinosaur-Type monster from your hand." Harpie
/// Lady is Tributed and Sabersaurus comes from the hand in face-up Position. A Tribute that is no Winged Beast (Warwolf) is
/// not offered.
#[test]
fn new_ultra_evolution_tributes_a_winged_beast_for_a_dinosaur_from_the_hand() {
    let evolution = card("New Ultra Evolution");
    let Some(mut s) = field(&[], &[hand(0, evolution), hand(0, SABERSAURUS), mon(0, HARPIE_LADY), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(evolution)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", HARPIE_LADY), 1, "Harpie Lady was Tributed: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", SABERSAURUS), 1, "Sabersaurus came from the hand: {}", s.context());
    assert_eq!(s.position_of(0, SABERSAURUS), Some(1), "{}", s.context());
    assert_eq!(s.count(0, "grave", evolution), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, evolution), hand(0, SABERSAURUS), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(evolution)]).expect_err("no Winged Beast to Tribute");
    assert!(err.contains("never became possible"), "{err}");
}

/// Archeonys: "This card can attack directly." Team 1 keeps Warwolf in Attack Position and Archeonys (300/1300) attacks in turn
/// 3: the direct attack is offered (the prompt's `can_direct`) and the default answer takes it: team 1 loses 300 and Archeonys
/// lives, where an attack on Warwolf would have destroyed it.
#[test]
fn archeonys_can_attack_directly_past_a_monster() {
    let archeonys = card("Archeonys");
    let Some(mut s) = field(&[], &[mon(0, archeonys), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_to(&[EndTurn, Attack(archeonys)], |p| p["type"] == msg::SELECT_BATTLECMD && p["player"] == 0).unwrap();
    let attack = s.prompt()["attacks"].as_array().and_then(|a| a.iter().find(|x| x["can_direct"] == true)).cloned();
    assert!(attack.is_some(), "a direct attack is offered: {}", s.prompt());
    s.run(&[Attack(archeonys)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 300, "{}", s.context());
    assert_eq!(s.count(0, "mzone", archeonys), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Aster Phoenix: Plasma Roar, Plasma Discharger, Plasma Counter, D - Acceleration
// ---------------------------------------------------------------------------------------------------------------

fn counters_on(s: &Scenario, team: usize, cid: u16, counter: &str) -> i64 {
    let card = s.state()[team]["szone"].as_array().and_then(|z| z.iter().find(|c| c["cid"] == cid));
    card.map_or(-1, |c| c["counters"][counter].as_i64().unwrap_or(0))
}

/// Plasma Roar: "During your opponent's Standby Phase, if you control a "Destiny HERO - Plasma": Place 1 Plasma Counter on this
/// card. While this card has 2 or more Plasma Counters on it: You can send this face-up card to the Graveyard; destroy 1
/// monster on the field, and if you do, inflict damage to your opponent equal to half the ATK of that monster." (A Continuous
/// Trap.) Activated in turn 1 it has one counter after team 1's turn 2 and two after turn 4: in turn 5 it is sent to the Graveyard, Warwolf
/// (2000) is destroyed and team 1 takes 1000.
#[test]
fn plasma_roar_gets_a_counter_each_standby_phase_of_the_opponent_and_destroys_a_monster() {
    let roar = card("Plasma Roar");
    let Some(mut s) = field(&[], &[set(0, roar), mon(0, PLASMA), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(roar)], &[EndTurn]]).unwrap();
    assert_eq!(counters_on(&s, 0, roar, "146"), 1, "one Plasma Counter after team 1's Standby Phase: {}", s.context());
    assert!(!offered(&s, "activates").contains(&roar.into()), "one counter is not enough: {:?}", offered(&s, "activates"));
    s.prefer(&[(0, FILLER), (1, FILLER), (1, WARWOLF)]);
    s.run(&[Activate(roar)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", roar), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", PLASMA), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "half of Warwolf's 2000 ATK: {}", s.context());
}

/// Plasma Roar counts only with a "Destiny HERO - Plasma" under its controller: without one no counter is ever placed
/// and the destruction never comes, in eight turns.
#[test]
fn plasma_roar_places_no_counter_without_plasma() {
    let roar = card("Plasma Roar");
    let Some(mut s) = field(&[], &[set(0, roar), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(roar), Activate(roar)]).expect_err("no counters");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(counters_on(&s, 0, roar, "146"), 0, "{}", s.context());
}

/// Plasma Discharger (a Trap): "Destroy 1 Equip Card you control; skip this turn's Battle Phase." Diamond Dude equipped with
/// Legendary Sword (1700 ATK) would attack directly for 1700; with the Discharger the Sword is destroyed and Diamond Dude has
/// no Battle Phase to attack in, so team 1 keeps its LP.
#[test]
fn plasma_discharger_destroys_an_equip_card_and_skips_the_battle_phase() {
    let discharger = card("Plasma Discharger");
    let spawns = [hand(0, LEGENDARY_SWORD), set(0, discharger), mon(0, DIAMOND_DUDE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(LEGENDARY_SWORD), Activate(discharger)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LEGENDARY_SWORD), 1, "the Equip Card was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", discharger), 1, "{}", s.context());
    // The Battle Phase is skipped: going there lands in the Main Phase 2, and the attack waits for turn 5.
    s.run_to(&[Attack(DIAMOND_DUDE)], |p| p["type"] == msg::SELECT_IDLECMD && p["player"] == 1).unwrap();
    assert_eq!(s.lp(1), 8000, "no attack in turn 3: {}", s.context());
    clean(&s);
    // The control: with the Sword alone Diamond Dude's attack goes through for 1700.
    let Some(mut s) = field(&[], &[hand(0, LEGENDARY_SWORD), mon(0, DIAMOND_DUDE)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(LEGENDARY_SWORD), Attack(DIAMOND_DUDE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1700, "{}", s.context());
}

/// Plasma Discharger needs an Equip Card of mine to destroy: not offered without one.
#[test]
fn plasma_discharger_needs_an_equip_card() {
    let discharger = card("Plasma Discharger");
    let Some(mut s) = field(&[], &[set(0, discharger), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[EndTurn, Activate(discharger)]).expect_err("no Equip Card");
    assert!(err.contains("never became possible"), "{err}");
}

/// Plasma Counter (a Trap): "During the Damage Step, when a "Destiny HERO - Plasma" you control is attacked by an opponent's
/// monster and "D - Force" is face-up on top of your Deck: The attacking monster's ATK is halved, and "Destiny HERO - Plasma"
/// gains 1500 ATK." Its third condition cannot be met in a scenario: no card in this game turns the top of a Deck face-up,
/// and a spawn can't place a card there, so the per-card fuzz never saw it either. What can be checked is that it is not
/// offered with D - Force in the Deck (face-down) when Plasma is attacked.
#[test]
fn plasma_counter_is_not_offered_without_a_face_up_d_force_on_top_of_the_deck() {
    let [counter, d_force] = ["Plasma Counter", "D - Force"].map(card);
    let spawns = [set(0, counter), mon_def(0, PLASMA), mon(1, WARWOLF)];
    let Some(mut s) = field(&[d_force], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(counter)], &[Attack(WARWOLF)]]).expect_err("D - Force is not face-up on top of the Deck");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", counter), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", PLASMA), 0, "Plasma (DEF 600) fell to Warwolf: {}", s.context());
}

/// D - Acceleration (a Quick-Play Spell): "During your Battle Phase: Target 1 "Destiny HERO" monster you control that is
/// equipped with 1 or more Equip card(s); Destroy all Equip cards equipped to that monster, and halve that monster's ATK,
/// also, that monster can attack twice during this turn's Battle Phase." (The script wants the monster to have declared an
/// attack: it is chained to the first one.) Diamond Dude (1400) with Legendary Sword (1700) attacks directly in turn 3 and
/// D - Acceleration destroys the Sword and halves what is left: 700 for the first attack and 700 for the second.
#[test]
fn d_acceleration_destroys_the_equip_cards_halves_the_atk_and_gives_a_second_attack() {
    let acceleration = card("D - Acceleration");
    let spawns = [hand(0, LEGENDARY_SWORD), set(0, acceleration), mon(0, DIAMOND_DUDE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(LEGENDARY_SWORD), EndTurn]).unwrap();
    assert_eq!(s.attack_of(0, DIAMOND_DUDE), Some(1700), "{}", s.context());
    s.run(&[Attack(DIAMOND_DUDE), ChainOnAttack(acceleration), Attack(DIAMOND_DUDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LEGENDARY_SWORD), 1, "the Equip Card was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", acceleration), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, DIAMOND_DUDE), Some(700), "half of 1400: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 700 - 700, "two attacks of 700: {}", s.context());
}

/// D - Acceleration needs a "Destiny HERO" equipped with an Equip card: a Warrior that is no Destiny HERO (Dark Blade),
/// equipped with the Sword, is no target.
#[test]
fn d_acceleration_needs_a_destiny_hero_with_an_equip_card() {
    let acceleration = card("D - Acceleration");
    let spawns = [hand(0, LEGENDARY_SWORD), set(0, acceleration), mon(0, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(LEGENDARY_SWORD), EndTurn, EndTurn, Attack(DARK_BLADE), ChainOnAttack(acceleration)]).expect_err("no Destiny HERO");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Super Guard
// ---------------------------------------------------------------------------------------------------------------

/// Super Guard: "Once per turn, during your Standby Phase, you must pay 500 LP (this is not optional), or destroy this card.
/// Monsters you control cannot be destroyed by battle, and you cannot conduct your Battle Phase." Team 1's Warwolf (2000)
/// attacks Thunder Kid (700) in turn 2: Thunder Kid lives and team 0 takes the 1300; in turn 3's Standby Phase 500 are paid
/// and team 0 has no Battle Phase (the idle prompt offers none).
#[test]
fn super_guard_protects_my_monsters_costs_500_each_standby_phase_and_ends_my_battle_phase() {
    let guard = card("Super Guard");
    let Some(mut s) = field(&[], &[hand(0, guard), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(guard)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 1300 - 500, "500 paid in the Standby Phase: {}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase: {}", s.prompt());
    assert_eq!(s.count(0, "szone", guard), 1, "{}", s.context());
    clean(&s);
}

/// Super Guard with fewer than 500 LP: "you must pay 500 LP, or destroy this card": at 400 LP it is destroyed at the Standby
/// Phase and the LP stay.
#[test]
fn super_guard_is_destroyed_when_the_500_cannot_be_paid() {
    let guard = card("Super Guard");
    let Some(mut s) = duel(7, 400, [FILLER, FILLER], &[], &[hand(0, guard)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(guard)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", guard), 1, "{}", s.context());
    assert_eq!(s.lp(0), 400, "{}", s.context());
}

/// Super Guard at ∞ LP (the 500 are worked out from the LP, infinite_lp.rs' "Infinite ATK and LP"): Mystik Wok Tributes the
/// anime Divine Serpent Geh (∞ ATK) for ∞ LP; the 500 paid at the Standby Phase leave ∞, and Super Guard is not destroyed.
#[test]
fn super_guard_costs_nothing_from_infinite_lp() {
    let (wok, geh) = (5978, 12671); // Mystik Wok, and the anime Divine Serpent Geh: a printed ∞ ATK
    let guard = card("Super Guard");
    let Some(dir) = engine_dir() else { return };
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    let spawns = [hand(0, wok), mon(0, geh), hand(0, guard)];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
    s.prefer(&[(0, geh)]);
    s.run(&[Activate(wok)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(guard)]).unwrap();
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), INFINITE, "half... 500 less than ∞ is ∞: {}", s.context());
    assert_eq!(s.count(0, "szone", guard), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Howard X Miller: Level Pod, Monster Register, Purse with a Hole, Recycle Barrier, Soul Connection, Force of Four
// ---------------------------------------------------------------------------------------------------------------

/// Level Pod: "FLIP: Return all monsters on the field to the Decks. Then, each player must draw cards equal to the combined
/// Levels of their returned monsters, and reveal those cards. If a player doesn't draw 1 of their returned monsters, they
/// must discard all cards they drew." Level Pod (2) and Warwolf (4) against Thunder Kid (2) and Dark Blade (4): every monster
/// is back in a Deck and each player draws 6; a player who drew one of his own returned monsters keeps all 6, the others
/// send them to the Graveyard. Over many seeds both outcomes happen, and each is checked against the hands.
#[test]
fn level_pod_returns_every_monster_and_each_player_draws_their_levels_or_discards_them() {
    let pod = card("Level Pod");
    let (mut kept, mut discarded) = (0, 0);
    for seed in 1..=16u64 {
        let spawns = [mon_set(0, pod), mon(0, WARWOLF), mon(1, THUNDER_KID), mon(1, DARK_BLADE)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        keep_hands(&mut s);
        let before: Vec<(usize, usize, i64)> = (0..2).map(|t| (hand_size(&s, t), pile(&s, t, "grave"), deck_size(&s, t))).collect();
        s.run(&[Reposition(pod)]).unwrap();
        clean(&s);
        for (team, &(hand0, grave0, deck0)) in before.iter().enumerate() {
            assert_eq!(pile(&s, team, "mzone"), 0, "seed {seed}: every monster went back to its Deck: {}", s.context());
            let own = if team == 0 { [pod, WARWOLF] } else { [THUNDER_KID, DARK_BLADE] };
            let drew_own = own.iter().any(|&c| s.count(team, "hand", c) > 0);
            assert_eq!(deck_size(&s, team), deck0 + 2 - 6, "seed {seed}: two cards returned, six drawn: {}", s.context());
            if drew_own {
                kept += 1;
                assert_eq!((hand_size(&s, team), pile(&s, team, "grave")), (hand0 + 6, grave0), "seed {seed}, team {team} keeps all 6: {}", s.context());
            } else {
                discarded += 1;
                assert_eq!((hand_size(&s, team), pile(&s, team, "grave")), (hand0, grave0 + 6), "seed {seed}, team {team} discards all 6: {}", s.context());
            }
        }
    }
    assert!(kept > 0 && discarded > 0, "both outcomes were seen: kept {kept}, discarded {discarded}");
}

/// Monster Register (a Continuous Trap): "Activate only by paying 1000 LP. If a monster(s) is Summoned: Its controller sends
/// cards from the top of their Deck to the Graveyard equal to the Level(s) of the Summoned monster(s)." Team 0 pays 1000 and
/// Normal Summons Warwolf (Level 4): 4 cards of its own Deck go to its Graveyard; in turn 2 team 1's Thunder Kid (Level 2)
/// sends 2 cards of team 1's Deck, and team 0 mills nothing more.
#[test]
fn monster_register_pays_1000_and_sends_each_summoned_monsters_level_from_its_controllers_deck() {
    let register = card("Monster Register");
    let Some(mut s) = field(&[], &[set(0, register), hand(0, WARWOLF), hand(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let graves = (pile(&s, 0, "grave"), pile(&s, 1, "grave"));
    s.run_both([&[Activate(register), Summon(WARWOLF)], &[Summon(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(pile(&s, 0, "grave"), graves.0 + 4, "Warwolf's Level 4 from team 0's Deck: {}", s.context());
    assert_eq!(pile(&s, 1, "grave"), graves.1 + 2, "Thunder Kid's Level 2 from team 1's Deck: {}", s.context());
    assert_eq!(s.count(0, "szone", register), 1, "{}", s.context());
}

/// Purse with a Hole (a Trap): "If your opponent has 5 or more cards in their hand: Send random cards from their hand to the
/// Graveyard until they have 5 cards in their hand." Team 0 Summons Thunder Kid in turn 1 with 8 cards in its hand (the Draw
/// Phase's, Thunder Kid and a Dian Keto): team 1's Purse takes two at random, down to 5. Its "for the rest of this turn,
/// your opponent cannot play cards from their hand" works (ProjectIgnis' EFFECT_FORBIDDEN): this duel chains it in turn 1,
/// so the lock ends with that turn; `anime_fixes.rs` checks it in the turn it is used.
#[test]
fn purse_with_a_hole_sends_random_cards_down_to_five() {
    let purse = card("Purse with a Hole");
    let Some(mut s) = duel(1, 8000, [FILLER, FILLER], &[], &[set(1, purse), hand(0, THUNDER_KID), hand(0, DIAN_KETO)]) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER)]);
    let (hand0, grave0) = (hand_size(&s, 0), pile(&s, 0, "grave"));
    s.run_both([&[Summon(THUNDER_KID)], &[Chain(purse)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", purse), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), 5, "down to 5 cards: {}", s.context());
    // The hand held `hand0` cards before the Draw Phase's card, and one left it by the Summon.
    assert_eq!(pile(&s, 0, "grave"), grave0 + hand0 - 5, "{}", s.context());
}

/// Recycle Barrier: "If you would take battle damage: You take no battle damage this turn. During your Standby Phase, if this
/// card is in your Graveyard: You can discard 1 Trap to the Graveyard; add this card to your hand." Team 1's Warwolf attacks
/// Thunder Kid (1300 damage): none is taken; in turn 3's Standby Phase Mirror Force is discarded and Recycle Barrier returns
/// to the hand.
#[test]
fn recycle_barrier_stops_battle_damage_and_comes_back_for_a_trap() {
    let barrier = card("Recycle Barrier");
    let spawns = [set(0, barrier), hand(0, MIRROR_FORCE), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(barrier)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.count(0, "grave", barrier), 1, "{}", s.context());
    s.run_both([&[Chain(barrier)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", barrier), 1, "Recycle Barrier is back in the hand: {}", s.context());
    assert_eq!(s.count(0, "grave", MIRROR_FORCE), 1, "Mirror Force was discarded: {}", s.context());
}

/// Soul Connection (a Trap): "Add 1 monster from your Deck to your hand." Dark Magician is at the bottom of the Deck.
#[test]
fn soul_connection_adds_a_monster_from_the_deck() {
    let connection = card("Soul Connection");
    let Some(mut s) = field(&[DARK_MAGICIAN], &[set(0, connection)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_MAGICIAN)]);
    let deck = deck_size(&s, 0);
    s.run(&[Activate(connection)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", connection), 1, "{}", s.context());
}

/// Greed Pact (a Trap): "Both players draw 1 card."
#[test]
fn greed_pact_lets_both_players_draw() {
    let pact = card("Greed Pact");
    let Some(mut s) = field(&[], &[set(0, pact)]) else { return };
    let hands = (hand_size(&s, 0), hand_size(&s, 1));
    s.run(&[Activate(pact)]).unwrap();
    clean(&s);
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), (hands.0 + 1, hands.1 + 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", pact), 1, "{}", s.context());
}

/// Force of Four (a Continuous Spell): "When this card is activated, if a player(s) has more than 4 cards in their hand, they
/// must send cards to the Graveyard until there are 4 cards in their hand." Both hands are cut to 4 when it is activated, and
/// again whenever a card is added to a hand: after team 1's Draw Phase and mine, both hold 4.
#[test]
fn force_of_four_cuts_both_hands_to_four_now_and_after_each_draw() {
    let four = card("Force of Four");
    let Some(mut s) = field(&[], &[hand(0, four)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(four)]).unwrap();
    clean(&s);
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), (4, 4), "{}", s.context());
    assert_eq!(s.count(0, "szone", four), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), (4, 4), "after both Draw Phases: {}", s.context());
}

/// Card of Reversal: "If no other cards are in your hand and you control no cards: Draw 1 card for each face-up card your
/// opponent controls." Team 0's Deck is all Dian Keto: the six in its hand are played (14000 LP) and the hand holds Card of
/// Reversal alone; against team 1's three monsters it draws 3. With other cards in the hand it is not offered.
#[test]
fn card_of_reversal_draws_for_each_face_up_card_of_the_opponent_when_i_have_nothing() {
    let reversal = card("Card of Reversal");
    let spawns = [hand(0, reversal), mon(1, WARWOLF), mon(1, THUNDER_KID), mon(1, DARK_BLADE)];
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &spawns) else { return };
    let mut plan = vec![Activate(DIAN_KETO); 6];
    plan.push(Activate(reversal));
    s.run(&plan).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 + 6000, "{}", s.context());
    assert_eq!(hand_size(&s, 0), 3, "three cards drawn: {}", s.context());
    assert_eq!(s.count(0, "hand", DIAN_KETO), 3, "{}", s.context());
    assert_eq!(s.count(0, "grave", reversal), 1, "{}", s.context());
    // Cards left in the hand: not offered.
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &spawns) else { return };
    let mut plan = vec![Activate(DIAN_KETO); 5];
    plan.push(Activate(reversal));
    let err = s.run(&plan).expect_err("a Dian Keto is left in the hand");
    assert!(err.contains("never became possible") || err.contains("the duel is over"), "{err}");
}

/// Card of Variation: "Draw 2 cards. During the End Phase, send 1 card from your hand to the Graveyard or take 3000 damage."
/// Answered yes the hand loses a card to the Graveyard and the LP stay; answered no, 3000 damage is taken.
#[test]
fn card_of_variation_draws_two_and_costs_a_card_or_3000_damage_in_the_end_phase() {
    let variation = card("Card of Variation");
    for yes in [true, false] {
        let Some(mut s) = field(&[], &[hand(0, variation)]) else { return };
        keep_hands(&mut s);
        let (hand0, grave0) = (hand_size(&s, 0), pile(&s, 0, "grave"));
        s.run_to(&[Activate(variation), EndTurn], |p| p["type"] == msg::SELECT_YESNO).unwrap();
        assert_eq!(hand_size(&s, 0), hand0 - 1 + 2, "two cards drawn: {}", s.context());
        s.respond(yo3_server::ocg::Response::YesNo(yes).json()).unwrap();
        s.run_both([&[], &[EndTurn]]).unwrap();
        clean(&s);
        if yes {
            assert_eq!(s.lp(0), 8000, "{}", s.context());
            assert!(pile(&s, 0, "grave") >= grave0 + 2, "Card of Variation and a card from the hand: {}", s.context());
        } else {
            assert_eq!(s.lp(0), 8000 - 3000, "{}", s.context());
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Infernal Blasthound, Lure Phantom, Amber Pitfall
// ---------------------------------------------------------------------------------------------------------------

/// Infernal Blasthound: "If this card inflicts battle damage to your opponent: Destroy 1 random card in your opponent's hand."
/// Blasthound (1000) attacks directly in turn 3: team 1 takes 1000 and one card of its hand is destroyed.
#[test]
fn infernal_blasthound_destroys_a_random_card_in_the_hand_when_it_inflicts_battle_damage() {
    let hound = card("Infernal Blasthound");
    let Some(mut s) = field(&[], &[mon(0, hound)]) else { return };
    keep_hands(&mut s);
    let (hand1, grave1) = (hand_size(&s, 1), pile(&s, 1, "grave"));
    s.run(&[EndTurn, Attack(hound)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    // Team 1 drew in turn 2: one more card, and one destroyed.
    assert_eq!(hand_size(&s, 1), hand1 + 1 - 1, "{}", s.context());
    assert_eq!(pile(&s, 1, "grave"), grave1 + 1, "{}", s.context());
}

/// Infernal Blasthound: "When this card is destroyed: You can Special Summon 1 "Infernal Blasthound" from your hand or Deck."
/// Team 1's Dark Hole destroys it; the second Blasthound comes from the Deck.
#[test]
fn infernal_blasthound_brings_another_when_it_is_destroyed() {
    let hound = card("Infernal Blasthound");
    let Some(mut s) = field(&[hound], &[mon(0, hound), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(hound)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", hound), s.count(0, "mzone", hound)), (1, 1), "{}", s.context());
}

/// Lure Phantom: "When an opponent's monster declares an attack: you can return this card and the attacking monster to the
/// hand." Warwolf attacks in turn 2: it and Lure Phantom go back to their hands, and no damage is dealt.
#[test]
fn lure_phantom_returns_itself_and_the_attacker_to_the_hand() {
    let lure = card("Lure Phantom");
    let Some(mut s) = field(&[], &[mon(0, lure), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(lure)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", lure), s.count(1, "hand", WARWOLF)), (1, 1), "both are back in their owners' hands: {}", s.context());
    assert_eq!((s.count(0, "mzone", lure), s.count(1, "mzone", WARWOLF)), (0, 0), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Amber Pitfall (a Continuous Trap): "When an opponent's monster declares an attack: Target the attacking monster; negate the
/// attack, and if you do, change that target to Defense Position and it cannot change its battle position while this card is
/// face-up on the field." Warwolf's attack is negated, it is in Defense Position and in turn 4 it cannot be repositioned.
#[test]
fn amber_pitfall_negates_an_attack_and_locks_the_attacker_in_defense_position() {
    let amber = card("Amber Pitfall");
    let Some(mut s) = field(&[], &[set(0, amber), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(amber)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "Defense Position: {}", s.context());
    assert_eq!(s.count(0, "szone", amber), 1, "Amber Pitfall stays face-up: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert!(!s.prompt()["pos_changes"].as_array().unwrap().iter().any(|c| c["cid"] == WARWOLF), "{}", s.prompt());
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "{}", s.context());
}
