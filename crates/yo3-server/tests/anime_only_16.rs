//! Anime-only cards, group 16 (rows 5730-5764 of engine/added/cards.tsv): Camula's, Dr. Eisenstein's, Harrington Rosewood's,
//! Jim Crocodile Cook's, Jean-Louis Bonaparte's and Maitre' D's cards. These cards exist only in the anime, so every duel plays
//! with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models
//! checked after every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play Spell
//! answering the opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//!
//! Covered: the Token (Crimson, made by Divine Chalice), the cards the per-card fuzz flagged (Genesis Crisis, which found no
//! Zombie-Type monster to add in its Decks; Red Ghost Moon and Zombie Bed, which need a Zombie and the opponent's Battle Phase or
//! a destroyed Set card; Battle Constant, whose cost needs a monster, a Continuous Spell and a Continuous Trap; Deuce, which needs
//! 1000 LP each; Receive Ace and Unequal Treaty, which need a direct attack; Bacchus Banquet, whose script declares a Special
//! Summon it never makes: it only reflects battle damage), the signature cards (Camula's Genesis Crisis and Infernalvania,
//! Dr. Eisenstein's Brain Dragon with Battle Constant and Draw Paradox, Harrington Rosewood's Deuce and Receive Ace, Jim Crocodile
//! Cook's Fossil Hammer and Uluru, Jean-Louis Bonaparte's Forced March and Toy Cannon, Maitre' D's Magna Mutton with Bacchus
//! Banquet) and a seeded sample of the rest (Divine Chalice, Dowsing Burn, Giant Racket, Half Life, Infernalvania, Sample
//! Fossil, Smash Ace, Uluru the Guardian Spirit); every other card of the batch has its test too. Relativity Field and Deuce
//! have a case at ∞ LP (`relativity_field_takes_no_atk_off_infinite_lp`, `deuce_needs_exactly_1000_lp_each_and_never_at_infinite_lp`).
//! Fossil Hammer has an override (`engine/overrides/c511001591.lua`): upstream took the Level its returning monster must be
//! below from the first of the opponent's monsters, not from the destroyed one. The fuzz's failing duel (Draw Paradox, seed
//! 64395) was Cursed Ring's script of batch 14 (a nil equip target in its field effect), not this batch's card.
//! A Special Summon at the start of the opponent's Battle Phase (Uluru the Guardian Spirit) sends the core back to that player's
//! Main Phase 1, where the turn cannot be ended and the Battle Phase is entered again with the attack forced.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const SKULL_SERVANT: u16 = 4030; // Level 1 Zombie, 300/200, vanilla
const AQUA_MADOOR: u16 = 4219; // Level 4 WATER Spellcaster, 1200/2000, vanilla
const DARK_HOLE: u16 = 4342;
const MST: u16 = 4909; // Mystical Space Typhoon
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const KURIBOH: u16 = 4064;
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK
const INFINITE: i64 = 19_999_998;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }

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

fn deck_size(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap()
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// A monster's (ATK, DEF, Level, race, attribute) as the engine holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

/// A card that is not offered: the plan waits for it to become possible and gives up with this message.
fn never(err: Result<(), String>) -> String {
    let err = err.expect_err("the card must not be offered");
    assert!(err.contains("never became possible"), "{err}");
    err
}

/// Team 0 at ∞ LP (Mystik Wok on the anime Geh: 8000 and ∞ is ∞, as `infinite_lp.rs` does), with `spawns` in place; team 1 at 8000.
fn infinite(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(0, WOK), mon(0, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &field).expect("the duel starts");
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[]);
    Some(s)
}

/// Team 0 ends turn 1; team 1's turn 2 goes to its Battle Phase, and every prompt is passed until `chain` is on offer to team 0
/// (the card is chained then) or team 1's next Main Phase comes. Returns whether the card was chained.
fn chain_in_their_battle_phase(s: &mut Scenario, chain: u16) -> bool {
    s.run_both([&[EndTurn], &[]]).unwrap();
    assert_eq!(s.prompt()["player"], 1, "team 1's turn: {}", s.context());
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
    let mut chained = false;
    for _ in 0..40 {
        let p = s.prompt().clone();
        let at = p["selects"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == chain));
        if p["type"] == msg::SELECT_CHAIN && p["player"] == 0 && at.is_some() && !chained {
            chained = true;
            s.respond(yo3_server::ocg::Response::Chain(at).json()).unwrap();
        } else if p["type"] == msg::SELECT_IDLECMD {
            break;
        } else {
            s.pass().unwrap();
        }
    }
    chained
}

// ---------------------------------------------------------------------------------------------------------------
// Divine Chalice and the Crimson Token
// ---------------------------------------------------------------------------------------------------------------

/// Divine Chalice: "If you control a face-up WATER monster: Special Summon 1 "Crimson Token" (Aqua-Type/WATER/Level 2/ATK 1000/
/// DEF 1000)." With Aqua Madoor (WATER) the Token comes in face-up Attack Position with those stats; Dark Hole then destroys it
/// and it is gone, not in the Graveyard. Without a WATER monster the Spell is not offered.
#[test]
fn divine_chalice_makes_a_crimson_token_that_is_gone_when_it_leaves() {
    let [chalice, token] = ["Divine Chalice", "Crimson Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, chalice), mon(0, AQUA_MADOOR), hand(0, DARK_HOLE)]) else { return };
    s.run(&[Activate(chalice)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 1, "one Token: {}", s.context());
    assert_eq!(s.count(0, "grave", chalice), 1, "{}", s.context());
    let t = stats(&s, 0, token);
    assert_eq!((t.0, t.1, t.2), (Some(1000), Some(1000), Some(2)), "1000/1000, Level 2: {}", s.context());
    assert_eq!((t.3, t.4), (0x40, 2), "Aqua-Type, WATER: {}", s.context());
    assert_eq!(s.position_of(0, token), Some(1), "face-up Attack: {}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", token), 0, "a Token is gone: {}", s.context());
    assert_eq!(s.count(0, "grave", AQUA_MADOOR), 1, "{}", s.context());
    // The control: no WATER monster, no Token.
    let Some(mut s) = field(&[], &[hand(0, chalice), mon(0, WARWOLF)]) else { return };
    never(s.run(&[Activate(chalice)]));
}

// ---------------------------------------------------------------------------------------------------------------
// Camula: Genesis Crisis, Infernalvania, Red Ghost Moon, Vampire Bat, Zombie Bed, Zombie Werewolf
// ---------------------------------------------------------------------------------------------------------------

/// Genesis Crisis: "If you do not control "Vampire Genesis", destroy this card and all Zombie-Type monsters you control. Once
/// per turn: You can add 1 Zombie-Type monster from your Deck to your hand." With Vampire Genesis on the field it stays, and
/// its once-a-turn effect takes the Skull Servant at the Deck's bottom into the hand (then it is not offered again that turn).
/// The per-card fuzz activated it 16 times and never saw the search: its Decks held no Zombie-Type monster.
#[test]
fn genesis_crisis_adds_a_zombie_from_the_deck_once_a_turn() {
    let [crisis, genesis] = ["Genesis Crisis", "Vampire Genesis"].map(card);
    let Some(mut s) = field(&[SKULL_SERVANT], &[hand(0, crisis), mon(0, genesis)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, SKULL_SERVANT), (0, FILLER)]);
    s.run(&[Activate(crisis)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", crisis), s.count(0, "mzone", genesis)), (1, 1), "both stay: {}", s.context());
    let (hand_before, deck_before) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run(&[Activate(crisis)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", SKULL_SERVANT), 1, "the Zombie is in the hand: {}", s.context());
    assert_eq!((hand_size(&s, 0), deck_size(&s, 0)), (hand_before + 1, deck_before - 1), "{}", s.context());
    assert!(!offered(&s, "activates").contains(&crisis.into()), "once per turn: {:?}", offered(&s, "activates"));
}

/// Genesis Crisis without "Vampire Genesis": it destroys itself and every Zombie-Type monster its controller has (Skull
/// Servant), and no other monster (Warwolf stays).
#[test]
fn genesis_crisis_destroys_itself_and_the_zombies_without_vampire_genesis() {
    let crisis = card("Genesis Crisis");
    let Some(mut s) = field(&[], &[hand(0, crisis), mon(0, SKULL_SERVANT), mon(0, WARWOLF), mon(1, SKULL_SERVANT)]) else { return };
    s.run(&[Activate(crisis)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", crisis), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", SKULL_SERVANT), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", SKULL_SERVANT)), (1, 1), "only its own Zombies: {}", s.context());
}

/// Infernalvania: "You can send 1 Zombie-Type monster from your hand to the Graveyard to destroy all monsters on the field. You
/// cannot Normal Summon or Set during the turn this effect is used." Skull Servant is the cost: both players' monsters are
/// destroyed, the Field Spell stays, and no Normal Summon or Set is on offer that turn.
#[test]
fn infernalvania_sends_a_zombie_to_destroy_every_monster_and_stops_normal_summons() {
    let vania = card("Infernalvania");
    let spawns = [hand(0, vania), hand(0, SKULL_SERVANT), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, SKULL_SERVANT), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(vania), Activate(vania)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", vania), 1, "the Field Spell stays: {}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(1, "grave", THUNDER_KID), s.count(0, "grave", SKULL_SERVANT)), (1, 1, 1), "{}", s.context());
    assert!(offered(&s, "summons").is_empty() && offered(&s, "monster_sets").is_empty(), "no Normal Summon or Set: {}", s.context());
    // The control: without the effect the Imps in the hand can be Summoned.
    let Some(mut s) = field(&[], &[hand(0, vania)]) else { return };
    s.run(&[Activate(vania)]).unwrap();
    assert!(offered(&s, "summons").contains(&FILLER.into()), "{:?}", offered(&s, "summons"));
}

/// Infernalvania needs a Zombie-Type monster in the hand to send: with only Imps it cannot be used; and after a Normal Summon
/// this turn it cannot be used either ("cannot Normal Summon or Set during the turn this effect is used": the cost refuses).
#[test]
fn infernalvania_needs_a_zombie_in_the_hand_and_no_normal_summon_yet() {
    let vania = card("Infernalvania");
    let Some(mut s) = field(&[], &[hand(0, vania), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(vania)]).unwrap();
    assert!(!offered(&s, "activates").contains(&vania.into()), "no Zombie in the hand: {:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[hand(0, vania), hand(0, SKULL_SERVANT), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(vania), Summon(FILLER)]).unwrap();
    clean(&s);
    assert!(!offered(&s, "activates").contains(&vania.into()), "already Normal Summoned: {:?}", offered(&s, "activates"));
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
}

/// Red Ghost Moon: "During your opponent's Battle Phase: Discard 1 Zombie-Type monster; Gain LP equal to the ATK of 1 Attack
/// Position monster your opponent controls. Then, end the Battle Phase." Chained as team 1 enters its Battle Phase with a
/// Warwolf: Skull Servant is discarded, team 0 gains 2000 (10000) and the Warwolf's attack never comes: team 1 is back in its
/// Main Phase. The fuzz could not reach it: it needs the opponent's Battle Phase and a Zombie to discard.
#[test]
fn red_ghost_moon_gains_the_attackers_atk_and_ends_the_battle_phase() {
    let moon = card("Red Ghost Moon");
    let Some(mut s) = field(&[], &[set(0, moon), hand(0, SKULL_SERVANT), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    assert!(chain_in_their_battle_phase(&mut s, moon), "Red Ghost Moon was on offer: {}", s.context());
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (10000, 8000), "{}", s.context());
    assert_eq!((s.count(0, "grave", moon), s.count(0, "grave", SKULL_SERVANT)), (1, 1), "{}", s.context());
    assert_eq!((s.prompt()["type"].clone(), s.prompt()["player"].clone()), (msg::SELECT_IDLECMD.into(), 1.into()), "back in the Main Phase: {}", s.context());
}

/// Red Ghost Moon: "Discard 1 Zombie-Type monster": without one in the hand it is not offered in the Battle Phase.
#[test]
fn red_ghost_moon_needs_a_zombie_to_discard() {
    let moon = card("Red Ghost Moon");
    let Some(mut s) = field(&[], &[set(0, moon), mon(1, WARWOLF)]) else { return };
    assert!(!chain_in_their_battle_phase(&mut s, moon), "no Zombie, no activation: {}", s.context());
    assert_eq!(s.count(0, "szone", moon), 1, "{}", s.context());
}

/// Vampire Bat: "All Zombie-Type monsters you control gain 200 ATK. If this card on the field would be destroyed, you can send
/// 1 "Vampire Bat" from your Deck to the Graveyard instead." The Bat (a Zombie itself) is 800 + 200, Skull Servant 300 + 200
/// and Warwolf stays 2000; Dark Hole destroys the Skull Servant but the Bat stays, a second Bat from the Deck in the Graveyard
/// in its place.
#[test]
fn vampire_bat_gives_zombies_200_and_sends_a_copy_in_its_place() {
    let bat = card("Vampire Bat");
    let Some(mut s) = field(&[bat], &[mon(0, bat), mon(0, SKULL_SERVANT), mon(0, WARWOLF), hand(0, DARK_HOLE)]) else { return };
    assert_eq!((s.attack_of(0, bat), s.attack_of(0, SKULL_SERVANT), s.attack_of(0, WARWOLF)), (Some(1000), Some(500), Some(2000)), "{}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", bat), 1, "the Bat was not destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", bat), 1, "the Bat from the Deck went instead: {}", s.context());
    assert_eq!((s.count(0, "grave", SKULL_SERVANT), s.count(0, "grave", WARWOLF)), (1, 1), "{}", s.context());
    // Without a second Bat in the Deck it is destroyed.
    let Some(mut s) = field(&[], &[mon(0, bat), hand(0, DARK_HOLE)]) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", bat), s.count(0, "grave", bat)), (0, 1), "{}", s.context());
}

/// Zombie Bed: "When this Set card is destroyed and sent to the Graveyard: Special Summon 1 Zombie-Type monster from your
/// Graveyard." Team 1's Mystical Space Typhoon destroys the Set Bed and Skull Servant comes back from team 0's Graveyard.
/// The fuzz could not reach it: the trigger needs the Bed Set and destroyed.
#[test]
fn zombie_bed_summons_a_zombie_when_destroyed_while_set() {
    let bed = card("Zombie Bed");
    let Some(mut s) = field(&[], &[set(0, bed), grave(0, SKULL_SERVANT), hand(1, MST)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, bed), (0, SKULL_SERVANT), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bed), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", SKULL_SERVANT), s.count(0, "grave", SKULL_SERVANT)), (1, 0), "{}", s.context());
    // A Bed in the Graveyard some other way (with no Zombie there too) does nothing; without a Zombie nothing is summoned.
    let Some(mut s) = field(&[], &[set(0, bed), hand(1, MST)]) else { return };
    s.prefer(&[(0, bed), (1, FILLER)]);
    s.run_both([&[EndTurn], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bed), 1, "{}", s.context());
    assert_eq!(s.state()[0]["mzone"].as_array().map_or(0, |z| z.iter().filter(|c| c["cid"].as_u64().is_some_and(|c| c != 0)).count()), 0, "{}", s.context());
}

/// Zombie Werewolf: "When this card is destroyed by battle, Special Summon 1 card with the same name from your Deck, and it
/// gains 500 ATK." Team 1's Warwolf (2000) destroys the Werewolf (1200): the copy from the Deck comes up with 1700 ATK, and
/// team 0 took the 800 difference.
#[test]
fn zombie_werewolf_brings_a_copy_with_500_more_atk_when_destroyed_by_battle() {
    let werewolf = card("Zombie Werewolf");
    let Some(mut s) = field(&[werewolf], &[mon(0, werewolf), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", werewolf), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", werewolf), s.attack_of(0, werewolf)), (1, Some(1700)), "the copy: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
    // With no copy in the Deck nothing comes.
    let Some(mut s) = field(&[], &[mon(0, werewolf), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", werewolf), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dr. Eisenstein: Battle Constant, Brain Dragon, Draw Paradox, Laplace, Relativity Field, Schrödinger's Cat
// ---------------------------------------------------------------------------------------------------------------

/// Battle Constant: "Banish 1 monster, Continuous Spell, and Continuous Trap you control; Special Summon 1 "Brain Dragon" from
/// your hand or Deck." Skull Servant, Schrödinger's Cat (a Continuous Spell) and Draw Paradox (a Continuous Trap) are banished
/// and the Brain Dragon at the Deck's bottom is Special Summoned. The fuzz could not reach it: the cost needs the three kinds
/// of card at once. Without a Continuous Trap it is not offered.
#[test]
fn battle_constant_banishes_a_monster_a_continuous_spell_and_a_continuous_trap_for_brain_dragon() {
    let [constant, brain, cat, paradox] = ["Battle Constant", "Brain Dragon", "Schrödinger's Cat", "Draw Paradox"].map(card);
    let spawns = [hand(0, constant), mon(0, SKULL_SERVANT), spell(0, cat), spell(0, paradox)];
    let Some(mut s) = field(&[brain], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, brain), (0, SKULL_SERVANT), (0, cat), (0, paradox), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(constant)]).unwrap();
    clean(&s);
    for banished in [SKULL_SERVANT, cat, paradox] {
        assert_eq!(s.count(0, "removed", banished), 1, "{banished} was banished: {}", s.context());
    }
    assert_eq!(s.count(0, "mzone", brain), 1, "Brain Dragon: {}", s.context());
    assert_eq!(s.count(0, "grave", constant), 1, "{}", s.context());
    // The control: without a Continuous Trap the cost cannot be paid.
    let Some(mut s) = field(&[brain], &[hand(0, constant), mon(0, SKULL_SERVANT), spell(0, cat)]) else { return };
    never(s.run(&[Activate(constant)]));
    // Brain Dragon in the hand is summoned just the same.
    let Some(mut s) = field(&[], &[hand(0, constant), hand(0, brain), mon(0, SKULL_SERVANT), spell(0, cat), spell(0, paradox)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(constant)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", brain), s.count(0, "hand", brain)), (1, 0), "{}", s.context());
}

/// Brain Dragon: "During your Draw Phase, you can draw 2 more cards in addition to your normal draw. Then, place 2 cards from
/// your hand on the top of your Deck in any order." Team 0's Deck, from the top, is Feral Imps and then Dark Blade, Thunder Kid and Warwolf in
/// that order: at its turn-1 Draw Phase the normal draw is Dark Blade and the effect draws the other two; Dark Blade and Thunder
/// Kid are the 2 put back, so the hand ends holding the Warwolf a plain draw would never have reached, and the Deck is where
/// a single draw leaves it.
#[test]
fn brain_dragon_draws_two_more_and_puts_two_back_on_the_deck() {
    let brain = card("Brain Dragon");
    let Some(dir) = engine_dir() else { return };
    // The first card of a Deck's list is its bottom: the top is the last.
    let mut mine = vec![FILLER; 31];
    mine.extend([WARWOLF, THUNDER_KID, DARK_BLADE]);
    mine.extend(vec![FILLER; 6]); // the opening hand and turn 1's draw
    let decks = [(&mine[..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &[mon(0, brain)]).unwrap();
    s.prefer(&[(0, DARK_BLADE), (0, THUNDER_KID), (0, FILLER), (1, FILLER)]);
    s.run(&[Chain(brain)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", WARWOLF), s.count(0, "hand", DARK_BLADE), s.count(0, "hand", THUNDER_KID)), (1, 0, 0), "{}", s.context());
    assert_eq!((hand_size(&s, 0), deck_size(&s, 0)), (6, 40 - 6), "5 + 1 drawn (3 drawn, 2 put back): {}", s.context());
    assert!(s.prompts.iter().any(|&(team, kind)| team == 0 && kind == msg::SELECT_CARD), "team 0 picked the 2 cards to put back: {:?}", s.prompts);
}

/// Draw Paradox: "During each player's Draw Phase, their opponent draws a card instead of the turn player." With it on team 0's
/// side, team 1's Draw Phase of turn 2 is team 0's draw (team 0 +1, team 1 +0), and team 0's own of turn 3 is team 1's (team 1
/// +1, team 0 +0). (The fuzz run's failing duel for this card, seed 64395, was Cursed Ring's script of batch 14 on a nil
/// equip target, not this card.)
#[test]
fn draw_paradox_gives_the_draw_to_the_other_player() {
    let paradox = card("Draw Paradox");
    for (with, after_turn_2, after_turn_3) in [(true, (-1, 0), (-1, -1)), (false, (0, -1), (-1, -1))] {
        let spawns = if with { vec![spell(0, paradox)] } else { vec![] };
        let Some(mut s) = field(&[], &spawns) else { return };
        keep_hands(&mut s);
        let [d0, d1] = [deck_size(&s, 0), deck_size(&s, 1)];
        // Team 1's turn 2, after its Draw Phase.
        s.run(&[EndTurn]).unwrap();
        clean(&s);
        assert_eq!((deck_size(&s, 0) - d0, deck_size(&s, 1) - d1), after_turn_2, "paradox {with}: after turn 2: {}", s.context());
        // Team 0's turn 3.
        s.run_both([&[], &[EndTurn]]).unwrap();
        clean(&s);
        assert_eq!((deck_size(&s, 0) - d0, deck_size(&s, 1) - d1), after_turn_3, "paradox {with}: after turn 3: {}", s.context());
    }
}

/// Laplace the Fiend Mathematician: "If this card leaves the field: Inflict 300 damage to your opponent for each card on the
/// field." Team 1's Warwolf destroys it in battle with a Set card on team 0's side and a Set card on team 1's: three cards are
/// left on the field (the Warwolf and the two Set cards), so team 1 takes 900 besides the 1000 of the battle damage its Warwolf's
/// ATK caused team 0.
#[test]
fn laplace_inflicts_300_for_each_card_left_on_the_field() {
    let laplace = card("Laplace the Fiend Mathematician");
    let spawns = [mon(0, laplace), set(0, DIAN_KETO), mon(1, WARWOLF), set(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", laplace), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000, "the battle damage: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 3 * 300, "three cards on the field: {}", s.context());
    // Leaving by an effect counts the same: Dark Hole destroys Laplace and the Warwolf, two Set cards are left.
    let spawns = [mon(0, laplace), set(0, DIAN_KETO), mon(1, WARWOLF), hand(0, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 300, "one card is left: the Set Dian Keto: {}", s.context());
}

/// Relativity Field: "Each time a player loses LP, all monsters they control lose an equal amount of ATK." Team 1's Dark Blade
/// (1800) destroys Thunder Kid (700): team 0 loses 1100 LP and its other monster, Warwolf, goes from 2000 to 900 ATK, while team
/// 1's Dark Blade keeps its 1800. In a second duel team 0's Warwolf (2000) destroys team 1's Thunder Kid: team 1 loses 1300 LP
/// and its Dark Blade goes from 1800 to 500, Warwolf keeping its 2000.
#[test]
fn relativity_field_takes_the_lost_lp_off_the_losers_monsters_atk() {
    let field_spell = card("Relativity Field");
    let spawns = [hand(0, field_spell), mon(0, THUNDER_KID), mon(0, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(field_spell)]).unwrap();
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "no LP lost yet: {}", s.context());
    s.run_both([&[EndTurn], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1100, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(1, DARK_BLADE)), (Some(2000 - 1100), Some(1800)), "{}", s.context());
    let spawns = [hand(0, field_spell), mon(0, WARWOLF), mon(1, THUNDER_KID), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(field_spell)]).unwrap();
    s.prefer(&[(1, THUNDER_KID)]);
    s.run(&[EndTurn, Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1300, "{}", s.context());
    assert_eq!((s.attack_of(1, DARK_BLADE), s.attack_of(0, WARWOLF)), (Some(1800 - 1300), Some(2000)), "{}", s.context());
}

/// Relativity Field at ∞ LP ("works an amount out from LP": the ATK lost is the LP it had minus the LP it has): team 0 at ∞ LP
/// (the Field Spell's own record of the LP rises to ∞ with it) loses 1100 to the Dark Blade's attack on Thunder Kid, and ∞ is
/// still ∞, so nothing is lost: Warwolf keeps its 2000.
#[test]
fn relativity_field_takes_no_atk_off_infinite_lp() {
    let field_spell = card("Relativity Field");
    let spawns = [hand(0, field_spell), mon(0, THUNDER_KID), mon(0, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = infinite(&spawns) else { return };
    s.run(&[Activate(field_spell)]).unwrap();
    s.run_both([&[EndTurn], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the attack went through: {}", s.context());
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "no ATK lost: {}", s.context());
}

/// Schrödinger's Cat is not asked in a Draw Phase: with it in play, the draws of turn 2 and turn 3 pass without its question.
#[test]
fn schrodingers_cat_is_not_offered_for_a_draw_phase_draw() {
    let cat = card("Schrödinger's Cat");
    let Some(mut s) = field(&[], &[spell(0, cat)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(!s.prompts.iter().any(|&(team, kind)| team == 0 && kind == msg::SELECT_EFFECTYN), "{:?}", s.prompts);
}

/// Schrödinger's Cat: "When you draw a card(s) except during your Draw Phase or by the effect of "Schrödinger's Cat", you can
/// reveal the drawn card(s), then return them into your Deck. Then, draw cards equal to the number of returned cards." Pot of
/// Greed's two cards are offered to the Cat; using it shows them to the opponent, shuffles them into the Deck and draws two
/// again, so the hand and the Deck are as Pot of Greed alone leaves them, and the Cat is not offered for its own draw.
#[test]
fn schrodingers_cat_returns_the_cards_drawn_by_an_effect_and_draws_again() {
    let [cat, pot] = ["Schrödinger's Cat", "Pot of Greed"].map(card);
    let Some(mut s) = field(&[], &[spell(0, cat), hand(0, pot)]) else { return };
    keep_hands(&mut s);
    let (hand_before, deck_before, draws_before) = (hand_size(&s, 0), deck_size(&s, 0), s.prompts.len());
    // Its single optional trigger is asked on its own: "use this effect?" naming the Cat.
    let asked_cat = |p: &serde_json::Value| p["type"] == msg::SELECT_EFFECTYN && p["player"] == 0 && p["cid"] == cat;
    s.run_to(&[Activate(pot)], asked_cat).unwrap();
    assert!(asked_cat(s.prompt()), "the Cat is offered after the draw: {}", s.context());
    s.respond(yo3_server::ocg::Response::yes(yo3_server::ocg::resp::EFFECTYN, true).json()).unwrap();
    let mut seen: Vec<u64> = s.messages().iter().filter_map(|m| m["type"].as_u64()).collect();
    for _ in 0..6 {
        if s.prompt()["type"] == msg::SELECT_IDLECMD {
            break;
        }
        s.pass().unwrap();
        seen.extend(s.messages().iter().filter_map(|m| m["type"].as_u64()));
    }
    clean(&s);
    assert!(seen.contains(&90), "the Cat's own draw: {seen:?}");
    assert_eq!((hand_size(&s, 0), deck_size(&s, 0)), (hand_before - 1 + 2, deck_before - 2), "{}", s.context());
    assert_eq!(s.count(0, "grave", pot), 1, "{}", s.context());
    assert!(s.prompts.len() > draws_before, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&cat.into()), "{}", s.context());
}

/// Deuce: "Activate only if both you and your opponent each have 1000 Life Points. Only 1 monster can attack during each Battle
/// Phase. The player that inflicts damage to their opponent twice wins the Duel. If a player takes damage from their opponent,
/// the number of times that player is considered to have inflicted damage becomes 0. (While this card is face-up on the field,
/// neither player can lose the Duel if their Life Points become 0.)" At 1000 LP each: in turn 3 Warwolf's direct attack takes
/// team 1 to 0 LP without losing, Thunder Kid cannot attack in the same Battle Phase, and in turn 5 the second damage wins the
/// Duel for team 0. The fuzz could not reach it: it needs 1000 LP on both sides.
#[test]
fn deuce_is_won_by_inflicting_damage_twice_and_allows_one_attacker_a_turn() {
    let deuce = card("Deuce");
    let Some(mut s) = duel(7, 1000, [FILLER, FILLER], &[], &[hand(0, deuce), mon(0, WARWOLF), mon(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(deuce), EndTurn, Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (1000, 0), "team 1 is at 0 LP and the duel goes on: {}", s.context());
    assert_eq!(s.prompt()["type"], msg::SELECT_BATTLECMD, "{}", s.context());
    assert!(offered(&s, "attacks").is_empty(), "only 1 monster can attack: {:?}", offered(&s, "attacks"));
    let err = s.run(&[EndTurn, Attack(WARWOLF)]).expect_err("the duel ends");
    assert!(err.contains("the duel is over"), "{err}");
    let won = s.messages().iter().find(|m| m["type"] == msg::WIN).cloned();
    assert_eq!(won.map(|m| m["player"].clone()), Some(0.into()), "team 0 won: {}", s.context());
}

/// Deuce: "If a player takes damage from their opponent, the number of times that player is considered to have inflicted damage
/// becomes 0." Team 0's Warwolf attacks directly in turn 3 (a first damage); in turn 4 team 1's new Dark Blade destroys Thunder Kid
/// (a damage to team 0: its count starts again); in turn 5 Warwolf destroys the Dark Blade, a second damage that would have won
/// without the other, and the duel goes on; in turn 7 the direct attack is the second since and team 0 wins.
#[test]
fn deuce_counts_start_again_when_the_other_player_inflicts_damage() {
    let deuce = card("Deuce");
    let spawns = [hand(0, deuce), mon(0, WARWOLF), mon(0, THUNDER_KID), hand(1, DARK_BLADE)];
    let Some(mut s) = duel(7, 1000, [FILLER, FILLER], &[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (1, FILLER), (0, FILLER)]);
    s.run_both([&[Activate(deuce), EndTurn, Attack(WARWOLF), EndTurn], &[EndTurn, Summon(DARK_BLADE), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was the Dark Blade's target: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (0, 0), "both at 0 LP, neither lost: {}", s.context());
    s.run_both([&[Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert!(s.messages().iter().all(|m| m["type"] != msg::WIN), "team 0's count started again: {}", s.context());
    let err = s.run(&[EndTurn, Attack(WARWOLF)]).expect_err("the duel ends");
    assert!(err.contains("the duel is over"), "{err}");
    let won = s.messages().iter().find(|m| m["type"] == msg::WIN).cloned();
    assert_eq!(won.map(|m| m["player"].clone()), Some(0.into()), "team 0 won in turn 7: {}", s.context());
}

/// Deuce: "Activate only if both you and your opponent each have 1000 Life Points": with 8000 each it is not offered, and with
/// team 0 at ∞ LP (`infinite_lp.rs`'s way: the script compares each player's LP with 1000, so ∞ never matches) neither.
#[test]
fn deuce_needs_exactly_1000_lp_each_and_never_at_infinite_lp() {
    let deuce = card("Deuce");
    let Some(mut s) = field(&[], &[hand(0, deuce)]) else { return };
    never(s.run(&[Activate(deuce)]));
    let Some(mut s) = infinite(&[hand(0, deuce)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&deuce.into()), "{:?}", offered(&s, "activates"));
    // The control: at 1000 each it is.
    let Some(mut s) = duel(7, 1000, [FILLER, FILLER], &[], &[hand(0, deuce)]) else { return };
    s.run(&[Activate(deuce)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", deuce), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Harrington Rosewood: Giant Racket, Receive Ace, Service Ace, Smash Ace, The Big Server
// ---------------------------------------------------------------------------------------------------------------

/// The Big Server: "This card can attack directly. When this card inflicts battle damage to your opponent: You can add 1
/// "Service Ace" from your Deck to your hand, and if you do, your opponent draws 1 card." It attacks directly in turn 3 although
/// Warwolf stands opposite (300 damage), takes the Service Ace from the Deck's bottom and team 1 draws a card.
#[test]
fn the_big_server_attacks_directly_and_fetches_service_ace_while_the_opponent_draws() {
    let [server, ace] = ["The Big Server", "Service Ace"].map(card);
    let Some(mut s) = field(&[ace], &[mon(0, server), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let hand_before = hand_size(&s, 1);
    s.run(&[Attack(server), Chain(server)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 300, "a direct attack, with a Warwolf on the other side: {}", s.context());
    assert_eq!(s.count(0, "hand", ace), 1, "Service Ace was fetched: {}", s.context());
    assert_eq!(hand_size(&s, 1), hand_before + 1, "team 1 drew 1: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Giant Racket: "Equip only to "The Big Server". Once per turn, the equipped monster cannot be destroyed by battle, and its
/// controller takes no battle damage from that battle." Team 1's Warwolf attacks the equipped Server first: it survives and team
/// 0 loses nothing; the Dark Blade's attack in the same Battle Phase destroys it and takes 1500 from team 0. With a Warwolf
/// alone to equip, the Racket cannot be activated.
#[test]
fn giant_racket_protects_the_big_server_once_a_turn() {
    let [server, racket] = ["The Big Server", "Giant Racket"].map(card);
    let Some(mut s) = field(&[], &[hand(0, racket), mon(0, server), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(racket)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", racket), 1, "equipped: {}", s.context());
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", server), s.lp(0)), (1, 8000), "the first battle: no destruction, no damage: {}", s.context());
    s.run_both([&[], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", server), s.count(0, "grave", server), s.lp(0)), (0, 1, 8000 - 1500), "the second battle: {}", s.context());
    // The equip limit.
    let Some(mut s) = field(&[], &[hand(0, racket), mon(0, WARWOLF)]) else { return };
    never(s.run(&[Activate(racket)]));
}

/// Receive Ace: "Activate only when an opponent's monster declares a direct attack. Negate the attack, and inflict 1500 damage to
/// your opponent. Then, send the top 3 cards of your Deck to the Graveyard." Team 1's Warwolf attacks directly in turn 2: the
/// attack is negated, team 1 takes 1500, and team 0's Deck loses its top 3 cards to the Graveyard. The fuzz could not reach it:
/// it needs a direct attack against it.
#[test]
fn receive_ace_negates_a_direct_attack_and_inflicts_1500() {
    let ace = card("Receive Ace");
    let Some(mut s) = field(&[], &[set(0, ace), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let deck_before = deck_size(&s, 0);
    s.run_both([&[EndTurn, ChainOnAttack(ace)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1500), "{}", s.context());
    assert_eq!(s.count(0, "grave", ace), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 3, "3 cards from the Deck: {}", s.context());
    // The Deck lost the 3 and team 0's draw (turn 1's) is in the baseline.
    assert_eq!(deck_size(&s, 0), deck_before - 3, "{}", s.context());
    // It is not for an attack on a monster: with Thunder Kid in the way the attack is a battle and the Trap is not offered.
    let Some(mut s) = field(&[], &[set(0, ace), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[EndTurn, ChainOnAttack(ace)], &[Attack(WARWOLF)]]));
    assert_eq!(s.count(0, "szone", ace), 1, "{}", s.context());
}

/// Service Ace: "Target 1 card in your hand; your opponent must guess whether it is a Spell, Trap or monster, then, reveal it to
/// your opponent. If they guessed wrong, inflict 1500 damage to your opponent. Banish the targeted card after this card
/// resolves." The Dark Blade is the target: a wrong guess (Spell) costs team 1 1500 and a right one (Monster) nothing; the card is
/// banished either way.
#[test]
fn service_ace_banishes_a_hand_card_and_punishes_a_wrong_guess() {
    let ace = card("Service Ace");
    for (guess, lp) in [(1, 8000 - 1500), (0, 8000), (2, 8000 - 1500)] {
        let Some(mut s) = field(&[], &[hand(0, ace), hand(0, DARK_BLADE)]) else { return };
        s.prefer(&[(0, DARK_BLADE)]);
        s.choose_options(&[guess]);
        s.run(&[Activate(ace)]).unwrap();
        clean(&s);
        assert_eq!(s.lp(1), lp, "guess {guess}: {}", s.context());
        assert_eq!(s.count(0, "removed", DARK_BLADE), 1, "banished: {}", s.context());
        assert_eq!(s.count(0, "grave", ace), 1, "{}", s.context());
    }
    // A Spell in the hand: guessing Spell is right.
    let Some(mut s) = field(&[], &[hand(0, ace), hand(0, DIAN_KETO)]) else { return };
    s.prefer(&[(0, DIAN_KETO)]);
    s.choose_options(&[1]);
    s.run(&[Activate(ace)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(1), s.count(0, "removed", DIAN_KETO)), (8000, 1), "{}", s.context());
}

/// Smash Ace: "Excavate the top card of your Deck. If it is a monster, inflict 1000 damage to your opponent, then, send it to
/// the Graveyard." A Deck of Feral Imps: 1000 damage and an Imp in the Graveyard; a Deck of Dian Keto: no damage, the card goes
/// to the Graveyard just the same.
#[test]
fn smash_ace_inflicts_1000_for_an_excavated_monster() {
    let ace = card("Smash Ace");
    let Some(mut s) = field(&[], &[hand(0, ace)]) else { return };
    let deck_before = deck_size(&s, 0);
    s.run(&[Activate(ace)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    assert_eq!((s.count(0, "grave", FILLER), deck_size(&s, 0)), (1, deck_before - 1), "{}", s.context());
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &[hand(0, ace)]) else { return };
    s.run(&[Activate(ace)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(1), s.count(0, "grave", DIAN_KETO)), (8000, 1), "a Spell: no damage: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Jim Crocodile Cook: Dowsing Burn, Fossil Hammer, Half Life, Sample Fossil, Uluru and Uluru's Guardian
// ---------------------------------------------------------------------------------------------------------------

/// Dowsing Burn: "Banish 1 monster in your opponent's Graveyard; destroy 1 monster on the field with the same Type and a Level
/// less than or equal to the banished monster's." Pyramid Turtle (Zombie, Level 4) is banished from team 1's Graveyard and
/// Skull Servant (Zombie, Level 1) destroyed; the Dark Blade (not a Zombie) and Vampire Genesis (Level 8, above 4) stay.
#[test]
fn dowsing_burn_banishes_a_monster_and_destroys_one_of_its_type_and_lower_level() {
    let [burn, genesis, turtle] = ["Dowsing Burn", "Vampire Genesis", "Pyramid Turtle"].map(card);
    let spawns = [hand(0, burn), grave(1, turtle), grave(1, DARK_MAGICIAN), mon(1, SKULL_SERVANT), mon(1, DARK_BLADE), mon(1, genesis)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, turtle), (1, SKULL_SERVANT)]);
    s.run(&[Activate(burn)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "removed", turtle), s.count(1, "grave", turtle)), (1, 0), "banished: {}", s.context());
    assert_eq!((s.count(1, "grave", SKULL_SERVANT), s.count(1, "mzone", SKULL_SERVANT)), (1, 0), "destroyed: {}", s.context());
    assert_eq!((s.count(1, "mzone", DARK_BLADE), s.count(1, "mzone", genesis)), (1, 1), "{}", s.context());
    // A banished monster of a type nothing on the field shares: the Spell is not offered.
    let Some(mut s) = field(&[], &[hand(0, burn), grave(1, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    never(s.run(&[Activate(burn)]));
}

/// Fossil Hammer: "Destroy the monster(s) with the highest Level your opponent controls. If you do, your opponent chooses 1
/// monster from their Graveyard with a lower Level than the destroyed monster(s)' and Special Summons it in Attack Position."
/// Team 1 controls Thunder Kid (Level 3) and Dark Magician (Level 7), Warwolf (Level 4) is in its Graveyard: the Dark Magician is
/// destroyed and Warwolf comes back in Attack Position (Thunder Kid is not the highest, and the Level to beat is the destroyed
/// monster's: the Thunder Kid's Level 3 would not have allowed the Warwolf).
#[test]
fn fossil_hammer_destroys_the_highest_level_monster_and_lets_a_lower_one_come_back() {
    let hammer = card("Fossil Hammer");
    let spawns = [hand(0, hammer), mon(1, THUNDER_KID), mon(1, DARK_MAGICIAN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(hammer)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!((s.count(1, "mzone", THUNDER_KID), s.count(1, "mzone", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(1), "face-up Attack Position: {}", s.context());
    // With the monster order the other way round the same happens.
    let spawns = [hand(0, hammer), mon(1, DARK_MAGICIAN), mon(1, THUNDER_KID), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(hammer)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", DARK_MAGICIAN), s.count(1, "mzone", WARWOLF)), (1, 1), "{}", s.context());
}

/// Fossil Hammer needs no monster in the Graveyard to be activated (upstream's script asked for one): the Dark Magician is
/// destroyed and nothing comes back.
#[test]
fn fossil_hammer_destroys_even_when_nothing_can_come_back() {
    let hammer = card("Fossil Hammer");
    let Some(mut s) = field(&[], &[hand(0, hammer), mon(1, DARK_MAGICIAN), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(hammer)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", DARK_MAGICIAN), s.count(1, "mzone", THUNDER_KID)), (1, 1), "{}", s.context());
    // No monster at all: not offered.
    let Some(mut s) = field(&[], &[hand(0, hammer)]) else { return };
    never(s.run(&[Activate(hammer)]));
}

/// Fossil Hammer: with several monsters of the highest Level, all are destroyed (two Level 4 monsters), and the monster that
/// comes back has a Level below 4 (Thunder Kid, not the Level 4 Warwolf in the Graveyard).
#[test]
fn fossil_hammer_destroys_every_monster_of_the_highest_level() {
    let hammer = card("Fossil Hammer");
    let spawns = [hand(0, hammer), mon(1, WARWOLF), mon(1, DARK_BLADE), grave(1, THUNDER_KID), grave(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, FILLER), (1, THUNDER_KID)]);
    s.run(&[Activate(hammer)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "both Level 4: {}", s.context());
    assert_eq!((s.count(1, "mzone", THUNDER_KID), s.count(1, "mzone", FILLER)), (1, 0), "only a lower Level comes back: {}", s.context());
}

/// Half Life: "Target 1 monster on the field; once during this turn, if it would be destroyed by battle, you can halve its
/// original ATK instead." Set in turn 1 and chained to team 1's attack in turn 2: Dark Magician (2500) attacks Warwolf (2000),
/// which would be destroyed, and instead its ATK is halved to 1000 and it stays; team 0 still takes the 500 difference.
#[test]
fn half_life_halves_the_original_atk_instead_of_a_destruction_by_battle() {
    let life = card("Half Life");
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), set(0, life), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, ChainOnAttack(life)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", life), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "not destroyed: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "its original ATK halved: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
    // Without it the Warwolf is destroyed.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    // "Once during this turn": the second attacker of the same Battle Phase (Dark Blade, 1800, on the halved 1000) destroys it.
    let spawns = [mon(0, WARWOLF), set(0, life), mon(1, DARK_MAGICIAN), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, ChainOnAttack(life)], &[Attack(DARK_MAGICIAN), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.lp(0)), (1, 8000 - 500 - 800), "the second battle destroyed it: {}", s.context());
}

/// Sample Fossil: "When this card is Normal Summoned: target 1 Level 4 or lower Normal Monster in your opponent's Graveyard;
/// Special Summon it to your side of the field. It cannot attack or be Tributed. This card's ATK is equal to that monster's
/// original ATK." The Dark Blade (1800) comes from team 1's Graveyard to team 0's field and the Fossil's ATK becomes 1800; in turn
/// 3 only the Fossil can attack, and with only these two monsters Dark Magician has no Tributes.
#[test]
fn sample_fossil_takes_a_normal_monster_from_the_opponents_graveyard() {
    let fossil = card("Sample Fossil");
    let spawns = [hand(0, fossil), hand(0, DARK_MAGICIAN), grave(1, DARK_BLADE), grave(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.run(&[Summon(fossil)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(1, "grave", DARK_BLADE)), (1, 0), "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "an effect monster is not a Normal Monster: {}", s.context());
    assert_eq!(s.attack_of(0, fossil), Some(1800), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "no Tributes: {:?}", offered(&s, "summons"));
    s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD && p["to_bp"] == true).unwrap();
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
    while s.prompt()["type"] != msg::SELECT_BATTLECMD {
        s.pass().unwrap();
    }
    assert_eq!(offered(&s, "attacks"), [fossil], "only the Fossil can attack: {}", s.context());
}

/// Uluru's Guardian: "Cannot be Normal Summoned unless you control no monsters. When this card is Normal Summoned: Change this
/// card to Defense Position. When this card is changed from face-down Defense Position to face-up Defense Position, destroy
/// it." Summoned with an empty field it is in face-up Defense (0/2300, Level 4, Rock); with a monster of its own it is not
/// offered; Set and flipped it is destroyed.
#[test]
fn ulurus_guardian_is_summoned_alone_in_defense_and_destroyed_when_flipped() {
    let guardian = card("Uluru's Guardian");
    let Some(mut s) = field(&[], &[hand(0, guardian)]) else { return };
    s.run(&[Summon(guardian)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, guardian), Some(4), "face-up Defense: {}", s.context());
    let g = stats(&s, 0, guardian);
    assert_eq!((g.0, g.1, g.2, g.3), (Some(0), Some(2300), Some(4), 0x100), "0/2300, Level 4, Rock: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, guardian), mon(0, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&guardian.into()), "{:?}", offered(&s, "summons"));
    assert!(offered(&s, "monster_sets").contains(&guardian.into()), "it can still be Set: {:?}", offered(&s, "monster_sets"));
    // Set, then turned face-up by the opponent's attack (Warwolf, 2000, on DEF 2300): destroyed by its own effect, before any damage.
    let Some(mut s) = field(&[], &[hand(0, guardian), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[SetMonster(guardian)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", guardian), s.count(0, "grave", guardian)), (0, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "gone before the damage: {}", s.context());
    // Flip Summoned it turns face-up in Attack Position, which is not the text: it stays.
    let Some(mut s) = field(&[], &[hand(0, guardian)]) else { return };
    keep_hands(&mut s);
    s.run(&[SetMonster(guardian), EndTurn, EndTurn, Reposition(guardian)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", guardian), s.position_of(0, guardian)), (1, Some(1)), "{}", s.context());
}

/// Uluru, the Guardian Spirit: "Activate only if an "Uluru's Guardian" is in your Graveyard: Special Summon this card in Defense
/// Position as a Normal Monster (Rock-Type/EARTH/Level 4/ATK 0/DEF 2500). (This card is also still treated as a Trap Card.) All
/// Attack Position monsters must attack this card during this turn's Battle Phase, if able. During the End Phase, destroy this
/// card." Set, with Uluru's Guardian in the Graveyard, it is chained as team 1's Battle Phase starts: Warwolf (2000) has to attack
/// it and the 500 difference is team 1's, and at the End Phase the Spirit is destroyed. Without the Guardian in the Graveyard it
/// is not offered.
#[test]
fn uluru_the_guardian_spirit_becomes_a_wall_that_the_attackers_must_hit() {
    let [spirit, guardian] = ["Uluru the Guardian Spirit", "Uluru's Guardian"].map(card);
    let Some(mut s) = field(&[], &[set(0, spirit), grave(0, guardian), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    assert!(chain_in_their_battle_phase(&mut s, spirit), "the Spirit was on offer: {}", s.context());
    clean(&s);
    assert_eq!(s.count(0, "mzone", spirit), 1, "Special Summoned: {}", s.context());
    let u = stats(&s, 0, spirit);
    assert_eq!((u.0, u.1, u.2, u.3, u.4), (Some(0), Some(2500), Some(4), 0x100, 1), "0/2500, Level 4, Rock, EARTH: {}", s.context());
    assert_eq!(s.position_of(0, spirit), Some(4), "face-up Defense: {}", s.context());
    // The core goes back to the Main Phase after a Special Summon at the start of the Battle Phase; the turn cannot be ended
    // (the Warwolf must attack), so the Battle Phase is entered again and the attack is the Spirit's.
    assert_eq!(s.prompt()["to_ep"], false, "the turn cannot end: {}", s.context());
    s.run_both([&[], &[EndTurn, EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500, "the Warwolf had to attack it: {}", s.context());
    assert_eq!((s.lp(0), s.count(1, "mzone", WARWOLF)), (8000, 1), "{}", s.context());
    assert_eq!((s.count(0, "mzone", spirit), s.count(0, "grave", spirit)), (0, 1), "destroyed in the End Phase: {}", s.context());
    // The condition: without the Guardian in the Graveyard it is not offered.
    let Some(mut s) = field(&[], &[set(0, spirit), mon(1, WARWOLF)]) else { return };
    assert!(!chain_in_their_battle_phase(&mut s, spirit), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Jean-Louis Bonaparte: Fife and Drum Corps, Forced March, Gulliver Chain, Toy Cannon, Unequal Treaty
// ---------------------------------------------------------------------------------------------------------------

/// Fife and Drum Corps: "Double the ATK of all monsters you control with 1000 or less ATK until the End Phase." Thunder Kid (700)
/// and Skull Servant (300) become 1400 and 600, Warwolf (2000) stays, the opponent's weak monster is not touched, and in the next
/// turn they are back.
#[test]
fn fife_and_drum_corps_doubles_the_weak_monsters_until_the_end_phase() {
    let fife = card("Fife and Drum Corps");
    let spawns = [hand(0, fife), mon(0, THUNDER_KID), mon(0, SKULL_SERVANT), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(fife)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, THUNDER_KID), s.attack_of(0, SKULL_SERVANT), s.attack_of(0, WARWOLF)), (Some(1400), Some(600), Some(2000)), "{}", s.context());
    assert_eq!(s.attack_of(1, THUNDER_KID), Some(700), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.attack_of(0, THUNDER_KID), s.attack_of(0, SKULL_SERVANT)), (Some(700), Some(300)), "until the End Phase: {}", s.context());
    // Nothing of 1000 or less: not offered.
    let Some(mut s) = field(&[], &[hand(0, fife), mon(0, WARWOLF)]) else { return };
    never(s.run(&[Activate(fife)]));
}

/// Forced March: "Halve the ATK of all monsters you currently control, then, those monster(s) can attack directly this turn."
/// In turn 3 Warwolf (2000) and Dark Blade (1800) go to 1000 and 900; Warwolf attacks directly past team 1's Dark Magician and
/// takes 1000, the Dark Blade can too (900 more), and the halving stays after the turn.
#[test]
fn forced_march_halves_the_atk_and_lets_the_monsters_attack_directly() {
    let march = card("Forced March");
    let spawns = [hand(0, march), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(march), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, DARK_BLADE)), (Some(1000), Some(900)), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "a direct attack although the Dark Magician stands there: {}", s.context());
    s.run(&[Attack(DARK_BLADE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000 - 900, "{}", s.context());
    // The halving stays: team 1's turn 4 has passed and the ATK is still halved.
    s.run(&[EndTurn]).unwrap();
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, DARK_BLADE)), (Some(1000), Some(900)), "{}", s.context());
}

/// Gulliver Chain: "When a monster(s) position is changed: Target 1 of those monster(s); change that monster to face-up Defense
/// Position. When that monster is changed to Attack Position, destroy both that target and this card." Team 1 changes its
/// Defense Position Warwolf to Attack; the Chain is chained and puts it back in Defense; the next time team 1 changes it to
/// Attack, both are destroyed.
#[test]
fn gulliver_chain_holds_a_monster_in_defense_and_destroys_both_when_it_attacks_again() {
    let chain = card("Gulliver Chain");
    let Some(mut s) = field(&[], &[set(0, chain), mon_def(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(chain)], &[Reposition(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", chain), 1, "it stays on the field: {}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "put back in face-up Defense: {}", s.context());
    s.run_both([&[], &[EndTurn, Reposition(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(0, "grave", chain)), (1, 1), "both destroyed: {}", s.context());
}

/// Toy Cannon: "Target 1 face-up Attack Position monster your opponent controls; Change it to Defense Position, and inflict 400
/// damage to your opponent." Chained as team 1's Battle Phase begins: the Warwolf is in Defense and team 1 takes 400.
#[test]
fn toy_cannon_turns_an_attacker_to_defense_and_inflicts_400() {
    let cannon = card("Toy Cannon");
    let Some(mut s) = field(&[], &[set(0, cannon), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    assert!(chain_in_their_battle_phase(&mut s, cannon), "Toy Cannon was on offer: {}", s.context());
    clean(&s);
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "face-up Defense: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 400), "{}", s.context());
    // A monster already in Defense is no target.
    let Some(mut s) = field(&[], &[set(0, cannon), mon_def(1, WARWOLF)]) else { return };
    assert!(!chain_in_their_battle_phase(&mut s, cannon), "{}", s.context());
}

/// Unequal Treaty: "Activate only when a monster you control inflicts battle damage to your opponent by a direct attack. Each
/// time your opponent draws a card(s): Gain 100 LP and inflict 100 damage to your opponent." Chained after Warwolf's direct attack
/// in turn 3 (team 1 at 6000): team 1's draw in turn 4 gives team 0 100 and costs team 1 100; team 0's own draw in turn 5 does
/// nothing. The fuzz could not reach it: it needs battle damage from a direct attack.
#[test]
fn unequal_treaty_gains_100_and_inflicts_100_for_each_card_the_opponent_draws() {
    let treaty = card("Unequal Treaty");
    let Some(mut s) = field(&[], &[set(0, treaty), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF), Chain(treaty), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", treaty), 1, "it stays: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 + 100, 8000 - 2000 - 100), "team 1's draw of turn 4: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8100 + 100, 5900 - 100), "turn 6's draw too, none of turn 5's own: {}", s.context());
    // An attack on a monster is not a direct attack: the Trap is not offered.
    let Some(mut s) = field(&[], &[set(0, treaty), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[EndTurn, Attack(WARWOLF), Chain(treaty)], &[EndTurn]]));
}

// ---------------------------------------------------------------------------------------------------------------
// Maitre' D: Bacchus Banquet, Bacchus the Dizzy Deity, Big Vintage Magna Mutton, Foaming Beauty
// ---------------------------------------------------------------------------------------------------------------

/// Bacchus the Dizzy Deity ("During each of your Standby Phases, this card gains 300 ATK") and Big Vintage Magna Mutton ("...
/// gains 1000 ATK"): each gains in its controller's Standby Phases only: team 0's pair in turns 1 and 3, team 1's Bacchus in turn 2 and not in team 0's.
#[test]
fn bacchus_and_the_magna_mutton_gain_atk_in_their_controllers_standby_phases() {
    let [bacchus, mutton] = ["Bacchus the Dizzy Deity", "Big Vintage Magna Mutton"].map(card);
    let Some(mut s) = field(&[], &[mon(0, bacchus), mon(0, mutton), mon(1, bacchus)]) else { return };
    assert_eq!((s.attack_of(0, bacchus), s.attack_of(0, mutton)), (Some(1600), Some(2500)), "{}", s.context());
    // Turn 1's Standby Phase was team 0's, turn 2's is team 1's.
    s.run(&[EndTurn]).unwrap();
    assert_eq!((s.attack_of(0, bacchus), s.attack_of(0, mutton), s.attack_of(1, bacchus)), (Some(1900), Some(3500), Some(1900)), "{}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, bacchus), s.attack_of(0, mutton), s.attack_of(1, bacchus)), (Some(2200), Some(4500), Some(1900)), "turn 3: team 0's again, not team 1's: {}", s.context());
}

/// Bacchus Banquet: "Activate only if you control a "Big Vintage Magna Mutton". If an opponent's Level 7 or lower monster declares
/// an attack, your opponent takes any battle damage you would have taken from that attack. If you do not control a "Big Vintage
/// Magna Mutton", destroy this card." Buster Blader (Level 7, 2600) destroys Thunder Kid (700) with team 0's
/// Mutton in play: team 1 takes the 1900, team 0 none. Blue-Eyes White Dragon (Level 8, 3000) is not reflected: team 0 takes 2300. Without a Mutton it is not offered.
/// (The fuzz saw it activated and never saw the Special Summon its script declares: the Spell only reflects battle damage.)
#[test]
fn bacchus_banquet_reflects_the_battle_damage_of_a_level_7_or_lower_attacker() {
    let [banquet, mutton, blader, dragon] = ["Bacchus Banquet", "Big Vintage Magna Mutton", "Buster Blader", "Blue-Eyes White Dragon"].map(card);
    let spawns = [hand(0, banquet), mon(0, mutton), mon(0, THUNDER_KID), mon(1, blader)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(banquet)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", banquet), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[Attack(blader)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was the target: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1900), "the 1900 is team 1's: {}", s.context());
    assert_eq!((s.count(0, "szone", banquet), s.count(0, "mzone", mutton)), (1, 1), "{}", s.context());
    // A Level 8 attacker is not reflected.
    let spawns = [hand(0, banquet), mon(0, mutton), mon(0, THUNDER_KID), mon(1, dragon)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(banquet)]).unwrap();
    s.run_both([&[EndTurn], &[Attack(dragon)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 2300, 8000), "{}", s.context());
    // Without the Mutton it cannot be activated.
    let Some(mut s) = field(&[], &[hand(0, banquet), mon(0, WARWOLF)]) else { return };
    never(s.run(&[Activate(banquet)]));
}

/// Foaming Beauty: "FLIP: Target 1 face-up monster. It loses 1000 ATK." Set in turn 1 and flipped in turn 3 (Flip Summoned),
/// targeting the opponent's Dark Magician: 2500 becomes 1500 and the Beauty (1200) is untouched.
#[test]
fn foaming_beauty_takes_1000_atk_off_a_monster_when_flipped() {
    let beauty = card("Foaming Beauty");
    let Some(mut s) = field(&[], &[hand(0, beauty), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DARK_MAGICIAN), (0, FILLER)]);
    s.run(&[SetMonster(beauty), EndTurn, EndTurn, Reposition(beauty)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, DARK_MAGICIAN), Some(1500), "{}", s.context());
    assert_eq!(s.attack_of(0, beauty), Some(1200), "{}", s.context());
}
