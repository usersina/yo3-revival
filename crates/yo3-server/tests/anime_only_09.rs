//! Anime-only cards, group 9 (rows 5517-5551 of engine/added/cards.tsv): Chazz Princeton's cards (the Ojama Union Beetles, the
//! Ojama Spells and Traps, the Level Monsters' Spells, the White Knights and his Equip and Continuous cards). These cards
//! exist only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through
//! the real core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones.
//! Every Trap and every Quick-Play Spell answering the opponent is Set first, and neither player has a Battle Phase in turn 1,
//! so team 0's attacks wait for turn 3.
//! Covered: the Token (Ojamachine Yellow Token, made by Ojamachine Yellow, shattering when it leaves), the cards the per-card
//! fuzz flagged (Assault Cannon Beetle, Buried Destiny, Dramatic Crossroads, Fusion Buster, Hidden Wish, Hyper Coat, Level Bond,
//! Level Copy, Level Soul, Ojama Delta Thunder!!, Ojama Ride, Ojamarble, Ojamarking, Sealed Gate), the two cards `--find` noted
//! (Fusion Buster, Level Soul: their costs and targets are checked, the effects stop on no Lua error), the signature cards
//! (Assault Cannon Beetle with the three Beetrons, Ojama Delta Thunder!!, Ojama Delta Wear, Ojamachine Yellow, Fusion Buster,
//! the White Knights) and a seeded sample of the rest (Cemetery Change, Different Dimension Hangar, Infernal Gauntlet, Love
//! Letter, Ojamachine Yellow, Quick Rush), with Dress Up, Direct Border, Goblin Negotiator, Desperado Manager, Infernal
//! Transaction, Infernal White, Hidden Wish, Ojama Ride and the Knights on the way.
//! Level Bond has an override (`engine/overrides/c100000254.lua`: the monster it summons cannot trigger either, so Armed Dragon
//! LV3 does not pay its cost in the next Standby Phase for a negated effect).
//! Not coverable: Front Change ("Combat Scissor Beetle" is a card the game never had and no batch adds: docs/anime-left-out.md,
//! so the Extra Deck never holds the monster either of its effects Special Summons, and `front_change_waits_for_a_combat_scissor_beetle`
//! shows it is not offered); Infernal White's "play with your hand revealed" (EFFECT_PUBLIC: the scenario's client model has no check on a
//! revealed hand, so only its damage is tested); the
//! fuzz's Beetron-1 Beetletop failure was Skill Twist (batch 22) against Arcana Force XII - The Hangman (VG) (batch 31):
//! a Lua error in Skill Twist's own target check, not a card of this batch.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const RAIGEKI: u16 = 4343;
const UMI: u16 = 4340; // Field Spell
const MST: u16 = 4909; // Mystical Space Typhoon
const TRUNADE: u16 = 4910; // Giant Trunade
const POLYMERIZATION: u16 = 4837;
const MONSTER_REBORN: u16 = 4842;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
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

/// The monsters on a team's field.
fn monsters(s: &Scenario, team: usize) -> usize {
    s.state()[team]["mzone"].as_array().map_or(0, |z| z.iter().filter(|c| c["cid"].as_u64().is_some_and(|c| c != 0)).count())
}

/// The cards in a team's Spell & Trap Zones.
fn spells(s: &Scenario, team: usize) -> usize {
    s.state()[team]["szone"].as_array().map_or(0, |z| z.iter().filter(|c| c["cid"].as_u64().is_some_and(|c| c != 0)).count())
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// A monster's (ATK, DEF, Level) as the engine holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64())
}

/// A card that is not offered: the plan waits for it to become possible and gives up with this message.
fn never(err: Result<(), String>) -> String {
    let err = err.expect_err("the card must not be offered");
    assert!(err.contains("never became possible"), "{err}");
    err
}

// ---------------------------------------------------------------------------------------------------------------
// The Beetrons and Assault Cannon Beetle
// ---------------------------------------------------------------------------------------------------------------

/// Beetron-1 Beetletop: a Level 4 Machine, 1700/1300, no effect; "Beetron-1 Beetletop" is what the other two Beetrons equip to.
#[test]
fn beetron_1_beetletop_is_a_vanilla_level_4_machine() {
    let b1 = card("Beetron-1 Beetletop");
    let Some(s) = field(&[], &[mon(0, b1)]) else { return };
    assert_eq!(stats(&s, 0, b1), (Some(1700), Some(1300), Some(4)), "{}", s.context());
    assert_eq!(s.monster(0, b1).unwrap()["race"].as_i64(), Some(0x20), "Machine: {}", s.context());
}

/// Beetron-2 Beetleturbo: "Once per turn, you can either: Target 1 "Beetron-1 Beetletop" you control; equip this card to that
/// target, OR: Unequip this card and Special Summon it in face-up Attack Position. While equipped by this effect, the equipped
/// monster gains 400 ATK/DEF. (A monster can only be equipped with 1 Union monster at a time.)" Beetletop (1700/1300) gains 400
/// of each (2100/1700) and the Beetron leaves the Monster Zones; Beetron-3 cannot also equip it; next turn the Beetron is
/// unequipped into Attack Position (1500/1800) and Beetletop is back to 1700/1300.
#[test]
fn beetron_2_equips_to_beetletop_for_400_and_comes_out_again() {
    let [b1, b2, b3] = ["Beetron-1 Beetletop", "Beetron-2 Beetleturbo", "Beetron-3 Spider Base"].map(card);
    let Some(mut s) = field(&[], &[mon(0, b1), mon(0, b2), mon(0, b3)]) else { return };
    assert_eq!(stats(&s, 0, b2), (Some(1500), Some(1800), Some(4)), "{}", s.context());
    s.prefer(&[(0, b1)]);
    s.run(&[Activate(b2)]).unwrap();
    clean(&s);
    assert_eq!(stats(&s, 0, b1), (Some(2100), Some(1700), Some(4)), "+400/+400: {}", s.context());
    assert_eq!((s.count(0, "mzone", b2), s.count(0, "szone", b2)), (0, 1), "equipped, not on the field: {}", s.context());
    // Only 1 Union monster at a time: Beetron-3 has no Beetletop left to equip to.
    assert!(!offered(&s, "activates").contains(&b3.into()), "{:?}", offered(&s, "activates"));
    // Once per turn.
    assert!(!offered(&s, "activates").contains(&b2.into()), "{:?}", offered(&s, "activates"));
    s.run_both([&[EndTurn, Activate(b2)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", b2), 1, "unequipped: {}", s.context());
    assert_eq!(s.position_of(0, b2), Some(1), "face-up Attack Position: {}", s.context());
    assert_eq!(stats(&s, 0, b1), (Some(1700), Some(1300), Some(4)), "{}", s.context());
}

/// Beetron-3 Spider Base: as Beetron-2 with 600 ATK/DEF (1000/2000): Beetletop becomes 2300/1900. If the equipped Beetletop would
/// be destroyed the Beetron is destroyed instead: against Dark Hole Beetletop survives and the Union goes to the
/// Graveyard in its place.
#[test]
fn beetron_3_gives_600_and_dies_in_place_of_beetletop() {
    let [b1, b3] = ["Beetron-1 Beetletop", "Beetron-3 Spider Base"].map(card);
    let Some(mut s) = field(&[], &[mon(0, b1), mon(0, b3), hand(1, DARK_HOLE)]) else { return };
    s.prefer(&[(0, b1)]);
    s.run(&[Activate(b3)]).unwrap();
    clean(&s);
    assert_eq!(stats(&s, 0, b1), (Some(1700 + 600), Some(1300 + 600), Some(4)), "{}", s.context());
    // Dark Hole's destruction of Beetletop is applied to the Union in its place.
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", b1), 1, "Beetletop stayed: {}", s.context());
    assert_eq!(s.count(0, "grave", b3), 1, "the Union was destroyed in its place: {}", s.context());
    assert_eq!(stats(&s, 0, b1), (Some(1700), Some(1300), Some(4)), "{}", s.context());
}

/// Assault Cannon Beetle: "Beetron-1 Beetletop" + "Beetron-2 Beetleturbo" + "Beetron-3 Spider Base"; "Must first be Special Summoned
/// (from your Extra Deck) by returning the above cards you control to the Deck. You can Tribute 1 monster; inflict 800 damage to
/// your opponent." The three leave the field for the Deck (not the Graveyard) and the Fusion Monster (2400/2800, Level 8)
/// takes their place, Warwolf stays; Tributing Warwolf inflicts 800, and again with the Beetle itself as the Tribute.
#[test]
fn assault_cannon_beetle_is_summoned_from_the_three_beetrons_and_inflicts_800() {
    let [acb, b1, b2, b3] = ["Assault Cannon Beetle", "Beetron-1 Beetletop", "Beetron-2 Beetleturbo", "Beetron-3 Spider Base"].map(card);
    let Some(mut s) = field_extra([&[acb], &[]], &[mon(0, b1), mon(0, b2), mon(0, b3), mon(0, WARWOLF)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.prefer_most(true); // the Tribute is chosen from a list with a way to finish
    s.run(&[SpSummon(acb)]).unwrap();
    clean(&s);
    assert_eq!(stats(&s, 0, acb), (Some(2400), Some(2800), Some(8)), "{}", s.context());
    for b in [b1, b2, b3] {
        assert_eq!((s.count(0, "mzone", b), s.count(0, "grave", b), s.count(0, "removed", b)), (0, 0, 0), "{b} went back to the Deck: {}", s.context());
    }
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    s.run(&[Activate(acb)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was Tributed: {}", s.context());
    assert_eq!(s.count(0, "mzone", acb), 1, "{}", s.context());
    // With nothing else to Tribute it Tributes itself: 800 more.
    s.run(&[Activate(acb)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1600, "{}", s.context());
    assert_eq!(s.count(0, "grave", acb), 1, "{}", s.context());
}

/// Assault Cannon Beetle: "Must first be Special Summoned (from your Extra Deck)": one of the Beetrons missing, it is not offered, and
/// from the Graveyard Monster Reborn cannot bring it back.
#[test]
fn assault_cannon_beetle_cannot_be_summoned_any_other_way() {
    let [acb, b1, b2, b3] = ["Assault Cannon Beetle", "Beetron-1 Beetletop", "Beetron-2 Beetleturbo", "Beetron-3 Spider Base"].map(card);
    let Some(mut s) = field_extra([&[acb], &[]], &[mon(0, b1), mon(0, b2)]) else { return };
    never(s.run(&[SpSummon(acb)]));
    clean(&s);
    let Some(mut s) = field(&[], &[grave(0, acb), hand(0, MONSTER_REBORN), mon(0, b3)]) else { return };
    never(s.run(&[Activate(MONSTER_REBORN)]));
    clean(&s);
    assert_eq!(s.count(0, "grave", acb), 1, "{}", s.context());
}

/// Front Change: "Return 1 "Assault Cannon Beetle" you control to the Extra Deck, and if you do, Special Summon 1 "Combat Scissor
/// Beetle" from your Extra Deck" (or the other way round). "Combat Scissor Beetle" is not in the game (docs/anime-left-out.md): with
/// Assault Cannon Beetle on the field and in the Extra Deck, the Spell has no monster to bring and is not offered.
#[test]
fn front_change_waits_for_a_combat_scissor_beetle() {
    let [front, acb] = ["Front Change", "Assault Cannon Beetle"].map(card);
    let Some(mut s) = field_extra([&[acb], &[]], &[hand(0, front), mon(0, acb)]) else { return };
    never(s.run(&[Activate(front)]));
    clean(&s);
    assert_eq!(s.count(0, "mzone", acb), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Buried Destiny, Cemetery Change, Desperado Manager
// ---------------------------------------------------------------------------------------------------------------

/// Buried Destiny: "If a Spell your opponent already used in the Duel is not in their Graveyard: Target 1 card with the same
/// name in your Graveyard; return it to the hand." Team 1 activates Umi (a Field Spell stays on the field) and team 0 takes its
/// own Umi from the Graveyard; where the opponent's Spell lies in their Graveyard (Dian Keto, resolved) or none was used, it is
/// not offered.
#[test]
fn buried_destiny_returns_a_card_named_like_an_opponents_spell_that_left_their_graveyard() {
    let destiny = card("Buried Destiny");
    let Some(mut s) = field(&[], &[hand(0, destiny), grave(0, UMI), hand(1, UMI)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(destiny)], &[Activate(UMI)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", UMI), 1, "my Umi came back: {}", s.context());
    assert_eq!(s.count(0, "grave", UMI), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", destiny), 1, "{}", s.context());
    // The opponent's Spell is in their Graveyard: nothing to return.
    let Some(mut s) = field(&[], &[hand(0, destiny), grave(0, DIAN_KETO), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[EndTurn, Activate(destiny)], &[Activate(DIAN_KETO)]]));
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    // The opponent used no Spell.
    let Some(mut s) = field(&[], &[hand(0, destiny), grave(0, UMI)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[EndTurn, Activate(destiny)], &[EndTurn]]));
}

/// Cemetery Change: "Pay 1000 LP; destroy all monsters on the field, then switch Graveyards with your opponent." 7000 LP; Warwolf
/// and Thunder Kid in team 0's Graveyard, Dark Blade in team 1's, a monster on each side destroyed: team 0 ends with team 1's
/// Graveyard (Dark Blade and its destroyed Dark Magician), team 1 with team 0's (Warwolf, Thunder Kid and the destroyed Feral Imp);
/// the Spell itself lands in team 0's Graveyard after resolving.
#[test]
fn cemetery_change_pays_1000_destroys_every_monster_and_switches_graveyards() {
    let change = card("Cemetery Change");
    let spawns = [hand(0, change), mon(0, FILLER), grave(0, WARWOLF), grave(0, THUNDER_KID), grave(1, DARK_BLADE), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(change)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "1000 LP paid: {}", s.context());
    assert_eq!((monsters(&s, 0), monsters(&s, 1)), (0, 0), "every monster was destroyed: {}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", THUNDER_KID), s.count(1, "grave", FILLER)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(0, "grave", DARK_MAGICIAN)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", change), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (0, 0), "{}", s.context());
    // With no monster and no card in either Graveyard it is not offered.
    let Some(mut s) = field(&[], &[hand(0, change)]) else { return };
    never(s.run(&[Activate(change)]));
}

/// Desperado Manager: "If you have 1 or more cards in your hand other than this card: Draw 2 cards, then return 3 cards from your
/// hand to the top of your Deck." Hand cards Warwolf, Thunder Kid and Dark Blade go back: the hand is 2 smaller than before (the
/// Spell, and 3 back for 2 drawn), the Deck 1 larger, and the next draw is one of them.
#[test]
fn desperado_manager_draws_2_and_returns_3_to_the_top() {
    let manager = card("Desperado Manager");
    let spawns = [hand(0, manager), hand(0, WARWOLF), hand(0, THUNDER_KID), hand(0, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, THUNDER_KID), (0, DARK_BLADE)]);
    let (h, d) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run(&[Activate(manager)]).unwrap();
    clean(&s);
    assert_eq!((hand_size(&s, 0), deck_size(&s, 0)), (h - 2, d + 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", manager), 1, "{}", s.context());
    let back = |s: &Scenario| s.count(0, "hand", WARWOLF) + s.count(0, "hand", THUNDER_KID) + s.count(0, "hand", DARK_BLADE);
    assert_eq!(back(&s), 0, "the three went to the Deck: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(back(&s), 1, "the draw took the top card of the three: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Different Dimension Hangar
// ---------------------------------------------------------------------------------------------------------------

/// Different Dimension Hangar: "Activate only by banishing up to 3 Level 4 or lower Union monsters from your Deck. If you Normal or
/// Special Summon a monster written in the card text of the Union monster(s) banished this way: You can Special Summon them, but
/// they cannot attack or be Tributed." Beetron-2 and Beetron-3 are banished from the Deck; Normal Summoning Beetron-1 brings them
/// both to the field, where they cannot attack (Beetron-1 can).
#[test]
fn different_dimension_hangar_banishes_unions_and_brings_them_back_with_beetletop() {
    let [hangar, b1, b2, b3] = ["Different Dimension Hangar", "Beetron-1 Beetletop", "Beetron-2 Beetleturbo", "Beetron-3 Spider Base"].map(card);
    let spawns = [hand(0, hangar), hand(0, b1)];
    let Some(mut s) = field(&[b2, b3], &spawns) else { return };
    s.prefer(&[(0, b2), (0, b3)]);
    s.prefer_most(true);
    s.run(&[Activate(hangar)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", b2), s.count(0, "removed", b3)), (1, 1), "banished from the Deck: {}", s.context());
    s.run(&[Summon(b1), Chain(hangar)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", b2), s.count(0, "mzone", b3)), (1, 1), "both came back: {}", s.context());
    assert_eq!((s.count(0, "removed", b2), s.count(0, "removed", b3)), (0, 0), "{}", s.context());
    let err = s.run_both([&[EndTurn, Attack(b2)], &[EndTurn]]).expect_err("the Unions cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{b1}")) && !err.contains(&format!("\"cid\":{b2}")), "{err}");
    clean(&s);
}

/// Different Dimension Hangar without a Level 4 or lower Union monster in the Deck is not offered.
#[test]
fn different_dimension_hangar_needs_a_union_monster_in_the_deck() {
    let hangar = card("Different Dimension Hangar");
    let Some(mut s) = field(&[], &[hand(0, hangar)]) else { return };
    never(s.run(&[Activate(hangar)]));
}

// ---------------------------------------------------------------------------------------------------------------
// Direct Border, Dramatic Crossroads
// ---------------------------------------------------------------------------------------------------------------

/// Direct Border: "Neither player can declare a direct attack with monsters with 1000 or more ATK. If you take battle damage by a
/// direct attack from a monster with less than 1000 ATK: Draw 1 card." Team 1 controls it against an empty field: team 0's Warwolf
/// (2000) cannot attack directly, Thunder Kid (700) can: team 1 takes 700 and draws 1 card.
#[test]
fn direct_border_stops_strong_direct_attacks_and_draws_for_weak_ones() {
    let border = card("Direct Border");
    let spawns = [spell(1, border), mon(0, WARWOLF), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).expect_err("2000 ATK cannot attack directly");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{THUNDER_KID}")) && !err.contains(&format!("\"cid\":{WARWOLF}")), "{err}");
    clean(&s);
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let before = hand_size(&s, 1);
    s.run(&[Attack(THUNDER_KID)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
    assert_eq!(hand_size(&s, 1), before + 1, "team 1 drew 1: {}", s.context());
    // The control: without the Spell Warwolf attacks directly.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
}

/// Dramatic Crossroads: "When you take battle damage: Your opponent chooses and applies 1 of these effects. Discard 1 random card, or
/// the opponent looks at the hand, chooses 1 card in it and adds it to their hand" (in the script the player who dealt the damage
/// chooses between discarding a random card of their own hand and letting the damaged player take a card of it). Team 1 takes
/// Thunder Kid's 700 and answers; team 0 first discards 1 random card, then (second duel) lets team 1 take Dark Magician.
#[test]
fn dramatic_crossroads_makes_the_attacker_discard_or_give_a_card() {
    let crossroads = card("Dramatic Crossroads");
    let spawns = [set(1, crossroads), mon(0, THUNDER_KID), hand(0, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.choose_options(&[0]);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let (mine, graves) = (hand_size(&s, 0), s.state()[0]["grave"].as_array().unwrap().len());
    s.run_both([&[Attack(THUNDER_KID)], &[Chain(crossroads)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
    assert_eq!(s.count(1, "grave", crossroads), 1, "{}", s.context());
    assert_eq!((hand_size(&s, 0), s.state()[0]["grave"].as_array().unwrap().len()), (mine - 1, graves + 1), "team 0 discarded 1 random card: {}", s.context());
    // The other choice: the opponent takes the card they pick from the attacker's hand.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.choose_options(&[1]);
    keep_hands(&mut s); // the End Phase's hand-size discards are Feral Imps
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    s.prefer(&[(0, DARK_MAGICIAN)]);
    let (mine, theirs, graves) = (hand_size(&s, 0), hand_size(&s, 1), s.state()[0]["grave"].as_array().unwrap().len());
    s.run_both([&[Attack(THUNDER_KID)], &[Chain(crossroads)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", DARK_MAGICIAN), 1, "team 1 took Dark Magician: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 0, "{}", s.context());
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), (mine - 1, theirs + 1), "{}", s.context());
    assert_eq!(s.state()[0]["grave"].as_array().unwrap().len(), graves, "nothing was discarded: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dress Up
// ---------------------------------------------------------------------------------------------------------------

/// Dress Up: "Equip only to a monster you control. The equipped monster gains 300 ATK. Once per turn: you can target 1 monster your
/// opponent controls; change its Battle Position. The equipped monster can only target your opponent's Attack Position monster
/// with the lowest ATK for an attack." Warwolf (2000 to 2300) with Thunder Kid (700) and Dark Blade (1800) across: it can only
/// attack Thunder Kid, so 1600 damage, Dark Blade stays untouched.
#[test]
fn dress_up_gives_300_and_only_the_weakest_attacker_can_be_targeted() {
    let dress = card("Dress Up");
    let spawns = [hand(0, dress), mon(0, WARWOLF), mon(1, THUNDER_KID), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(dress)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2300), "{}", s.context());
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "mzone", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - (2300 - 700), "{}", s.context());
}

/// Dress Up: the once-per-turn position change: team 1's only monster, Dark Blade, goes to Defense Position (it can no longer be
/// targeted for an attack, and a monster of the opponent keeps Warwolf from a direct attack), and the effect is used once a turn.
#[test]
fn dress_up_changes_an_opponents_monsters_battle_position() {
    let dress = card("Dress Up");
    let spawns = [hand(0, dress), mon(0, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(dress), Activate(dress)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, DARK_BLADE), Some(4), "face-up Defense Position: {}", s.context());
    assert!(!offered(&s, "activates").contains(&dress.into()), "once per turn: {:?}", offered(&s, "activates"));
    let err = s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).expect_err("nothing the equipped monster may attack");
    assert!(err.contains("can't attack"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Fusion Buster
// ---------------------------------------------------------------------------------------------------------------

/// Fusion Buster: "The equipped monster gains 300 ATK. When your opponent activates a "Polymerization": You can send this face-up
/// card to the Graveyard; negate its activation, and if you do, destroy it." Team 1's Polymerization (Gaia the Fierce Knight and
/// Curse of Dragon for Gaia the Dragon Champion) is negated and destroyed, the Buster goes to the Graveyard as the cost, and the
/// Fusion Monster stays in the Extra Deck. Without the answer the Fusion Summon happens.
#[test]
fn fusion_buster_gives_300_and_negates_a_polymerization() {
    let [buster, champion, knight, dragon] = ["Fusion Buster", "Gaia the Dragon Champion", "Gaia The Fierce Knight", "Curse of Dragon"].map(card);
    let spawns = [hand(0, buster), mon(0, WARWOLF), hand(1, POLYMERIZATION), hand(1, knight), hand(1, dragon)];
    let Some(mut s) = field_extra([&[], &[champion]], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(buster)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2300), "+300: {}", s.context());
    s.run_both([&[Chain(buster)], &[Activate(POLYMERIZATION)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", buster), 1, "the Buster was sent as the cost: {}", s.context());
    assert_eq!(s.count(1, "grave", POLYMERIZATION), 1, "Polymerization was destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", champion), 0, "no Fusion Summon: {}", s.context());
    assert_eq!((s.count(1, "hand", knight), s.count(1, "hand", dragon)), (1, 1), "the materials stayed in hand: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "the equip is gone: {}", s.context());
    // The control: the Buster does not answer.
    let Some(mut s) = field_extra([&[], &[champion]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(buster)], &[Activate(POLYMERIZATION)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", champion), 1, "the Fusion Summon happened: {}", s.context());
    assert_eq!(s.count(0, "grave", buster), 0, "{}", s.context());
}

/// Fusion Buster: "Once per turn: You can target 1 Fusion Monster your opponent controls; return it to the Extra Deck, then, if all the
/// Fusion Material that were used to Fusion Summon it are in your opponent's Graveyard, Special Summon them to your opponent's
/// side of the field." Gaia the Dragon Champion, Fusion Summoned by team 1 from Gaia the Fierce Knight and Curse of Dragon, goes
/// back to its Extra Deck and both materials come to team 1's field.
#[test]
fn fusion_buster_returns_a_fusion_monster_and_brings_back_its_materials() {
    let [buster, champion, knight, dragon] = ["Fusion Buster", "Gaia the Dragon Champion", "Gaia The Fierce Knight", "Curse of Dragon"].map(card);
    let spawns = [hand(0, buster), mon(0, WARWOLF), hand(1, POLYMERIZATION), hand(1, knight), hand(1, dragon)];
    let Some(mut s) = field_extra([&[], &[champion]], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, champion), (1, knight), (1, dragon), (0, FILLER), (1, FILLER)]);
    s.run_both([&[Activate(buster), EndTurn, Activate(buster)], &[Activate(POLYMERIZATION)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", champion), 0, "the Fusion Monster went back to the Extra Deck: {}", s.context());
    assert_eq!((s.count(1, "mzone", knight), s.count(1, "mzone", dragon)), (1, 1), "its materials came back on team 1's side: {}", s.context());
    assert_eq!((s.count(1, "grave", knight), s.count(1, "grave", dragon)), (0, 0), "{}", s.context());
    assert_eq!(s.count(1, "grave", champion), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Goblin Negotiator, Hidden Wish
// ---------------------------------------------------------------------------------------------------------------

/// Goblin Negotiator: "Once per turn, if a monster you control battles a monster your opponent controls for the first time this turn:
/// It cannot be destroyed by battle and you take no battle damage from that battle, then, your opponent draws 1 card." Team 1's
/// Thunder Kid (700) is attacked by Warwolf (2000): it is not destroyed, team 1 takes no damage and team 0 draws 1 card; the
/// effect works once: Dark Blade's attack on team 1's Feral Imp kills it and hurts.
#[test]
fn goblin_negotiator_protects_the_first_battle_of_the_turn_and_gives_the_opponent_a_card() {
    let negotiator = card("Goblin Negotiator");
    let spawns = [spell(1, negotiator), mon(1, THUNDER_KID), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let before = hand_size(&s, 0);
    s.run(&[Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no battle damage: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "the opponent drew 1: {}", s.context());
    // Once per turn: a second battle is a normal one.
    let spawns = [spell(1, negotiator), mon(1, THUNDER_KID), mon(1, FILLER), mon(0, WARWOLF), mon(0, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let before = hand_size(&s, 0);
    s.run(&[Attack(WARWOLF), Attack(DARK_BLADE)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), before + 1, "once: {}", s.context());
    assert_eq!(monsters(&s, 1), 1, "the second battle destroyed a monster: {}", s.context());
    assert!(s.lp(1) < 8000, "and hurt: {}", s.context());
}

/// Hidden Wish: "When this face-down card is returned from the field to the hand: Inflict 1000 damage to your opponent and gain
/// 1000 LP." Set, it is returned by Giant Trunade: 9000 LP against 7000. Face-up (a copy on the field) the return does nothing.
#[test]
fn hidden_wish_pays_when_a_set_copy_is_returned_to_the_hand() {
    let wish = card("Hidden Wish");
    let Some(mut s) = field(&[], &[set(0, wish), hand(0, TRUNADE)]) else { return };
    s.run(&[Activate(TRUNADE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", wish), 1, "returned to the hand: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (9000, 7000), "{}", s.context());
    let Some(mut s) = field(&[], &[spell(0, wish), hand(0, TRUNADE)]) else { return };
    s.run(&[Activate(TRUNADE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", wish), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "face-up: nothing happens: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Hyper Coat
// ---------------------------------------------------------------------------------------------------------------

/// Hyper Coat: "Target 1 Machine-Type monster you control with "V", "W", "X", "Y", and/or "Z" in its name; equip this card to it. It
/// gains 500 ATK and is unaffected by your opponent's card effects. If it leaves the field, destroy this card." V-Tiger Jet (1600)
/// gains 500; the opponent's Raigeki destroys Warwolf and leaves V-Tiger Jet. Warwolf (not a Machine) is no target.
#[test]
fn hyper_coat_gives_500_atk_and_immunity_to_the_opponents_effects() {
    let [coat, jet] = ["Hyper Coat", "V-Tiger Jet"].map(card);
    let spawns = [set(0, coat), mon(0, jet), mon(0, WARWOLF), hand(1, RAIGEKI)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, jet), (1, FILLER)]);
    s.run(&[EndTurn, Activate(coat)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, jet), Some(2100), "+500: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "{}", s.context());
    s.run_both([&[], &[Activate(RAIGEKI)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", jet), s.count(0, "grave", WARWOLF)), (1, 1), "only Warwolf was destroyed: {}", s.context());
    // Warwolf is not a Machine: no target.
    let Some(mut s) = field(&[], &[set(0, coat), mon(0, WARWOLF)]) else { return };
    never(s.run(&[EndTurn, Activate(coat)]));
}

// ---------------------------------------------------------------------------------------------------------------
// Infernal Gauntlet, Infernal Transaction, Infernal White
// ---------------------------------------------------------------------------------------------------------------

/// Infernal Gauntlet: "During your Battle Phase, you can Tribute 1 monster to have the equipped monster gain an additional attack in
/// addition to its normal attack." Warwolf equipped, with Thunder Kid as the Tribute, attacks two of team 1's three Feral Imps in
/// one Battle Phase: 2 x (2000 - 1300); without the Gauntlet it attacks once.
#[test]
fn infernal_gauntlet_tributes_a_monster_for_an_additional_attack() {
    let gauntlet = card("Infernal Gauntlet");
    let spawns = [hand(0, gauntlet), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, FILLER), mon(1, FILLER), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(gauntlet)]).unwrap();
    clean(&s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.prefer_most(true);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    s.run_to(&[Attack(WARWOLF)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    let list = s.prompt()["chains"].as_array().cloned().unwrap_or_default();
    let i = list.iter().position(|c| c["cid"] == gauntlet).unwrap_or_else(|| panic!("the Gauntlet is offered in the Battle Phase: {}", s.prompt()));
    s.respond(serde_json::json!({"type": 1, "action": battle::CHAIN, "index": i})).unwrap();
    s.run(&[Attack(WARWOLF), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was Tributed: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 2, "two attacks, two Feral Imps: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * 700, "{}", s.context());
    // Without the Gauntlet one attack only.
    let spawns = [mon(0, WARWOLF), mon(1, FILLER), mon(1, FILLER), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[EndTurn, Attack(WARWOLF), Attack(WARWOLF)], &[EndTurn]]).expect_err("a second attack");
    assert!(err.contains("can't attack"), "{err}");
}

/// Infernal Transaction: "Target 1 monster with 2000 or more ATK in your opponent's Graveyard; Special Summon it to their side of the
/// field, then, add 1 Spell from your Graveyard to your hand." Warwolf (2000) in team 1's Graveyard comes to team 1's field and
/// Dian Keto returns from team 0's Graveyard to its hand. Thunder Kid (700) is no target, and with no Spell of its own in the
/// Graveyard the card is not offered.
#[test]
fn infernal_transaction_revives_a_strong_monster_for_the_opponent_and_takes_a_spell_back() {
    let transaction = card("Infernal Transaction");
    let Some(mut s) = field(&[], &[hand(0, transaction), grave(1, WARWOLF), grave(1, THUNDER_KID), grave(0, DIAN_KETO)]) else { return };
    s.prefer(&[(1, WARWOLF), (0, DIAN_KETO)]);
    s.run(&[Activate(transaction)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", WARWOLF), s.count(1, "grave", WARWOLF)), (1, 0), "Warwolf is on team 1's field: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", DIAN_KETO), s.count(0, "grave", DIAN_KETO)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", transaction), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, transaction), grave(1, THUNDER_KID), grave(0, DIAN_KETO)]) else { return };
    never(s.run(&[Activate(transaction)]));
    let Some(mut s) = field(&[], &[hand(0, transaction), grave(1, WARWOLF)]) else { return };
    never(s.run(&[Activate(transaction)]));
}

/// Infernal White: "Play with your hand revealed. Once per turn, during your opponent's Standby Phase: Inflict 600 damage to your
/// opponent." Team 1 takes 600 at each of its Standby Phases (7400, then 6800) and team 0 takes nothing.
#[test]
fn infernal_white_inflicts_600_in_the_opponents_standby_phase() {
    let white = card("Infernal White");
    let Some(mut s) = field(&[], &[spell(0, white)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7400), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000, 6800), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Level Bond, Level Copy, Level Soul
// ---------------------------------------------------------------------------------------------------------------

/// Level Bond, a Trap: "Banish 2 "LV" monsters with the same name from your Graveyard; Your opponent draws 2 cards, also, after that,
/// Special Summon 1 "LV" monster from your Deck with the same name as the banished monsters, ignoring their Summoning conditions,
/// but its effects are negated, and it cannot attack this turn." Two Armed Dragon LV3 are banished, team 1 draws 2, the third LV3
/// comes from the Deck; its effect is negated (at the next Standby Phase it does not become Armed Dragon LV5) and it cannot attack
/// this turn. The cost needs two copies of one name: with LV3 and LV5 in the Graveyard it is not offered. (The negation is the
/// override's: `engine/overrides/c100000254.lua`, upstream left the monster's trigger activatable.)
#[test]
fn level_bond_banishes_two_copies_for_a_negated_third_from_the_deck() {
    let [bond, lv3, lv5] = ["Level Bond", "Armed Dragon LV3", "Armed Dragon LV5"].map(card);
    let spawns = [set(0, bond), grave(0, lv3), grave(0, lv3)];
    let Some(mut s) = field(&[lv3, lv5], &spawns) else { return };
    let theirs = hand_size(&s, 1);
    s.run(&[EndTurn, Activate(bond)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", lv3), 2, "the two copies were banished: {}", s.context());
    assert_eq!(s.count(0, "grave", lv3), 0, "{}", s.context());
    assert_eq!(hand_size(&s, 1), theirs + 2 + 1, "the opponent drew 2 (and 1 for its turn): {}", s.context());
    assert_eq!(s.count(0, "mzone", lv3), 1, "the third came from the Deck: {}", s.context());
    // Its effect is negated: at team 0's next Standby Phase it stays Level 3, with LV5 in the Deck waiting.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", lv3), s.count(0, "mzone", lv5)), (1, 0), "negated: {}", s.context());
    // Not two copies of one name.
    let Some(mut s) = field(&[lv3, lv5], &[set(0, bond), grave(0, lv3), grave(0, lv5)]) else { return };
    never(s.run(&[EndTurn, Activate(bond)]));
}

/// Level Bond: the Special Summoned monster "cannot attack this turn" (the turn it was Special Summoned, 3 here: it is Special
/// Summoned during team 0's own turn and does not attack, where another monster does).
#[test]
fn level_bond_summon_cannot_attack_that_turn() {
    let [bond, lv3] = ["Level Bond", "Armed Dragon LV3"].map(card);
    let spawns = [set(0, bond), grave(0, lv3), grave(0, lv3), mon(0, WARWOLF)];
    let Some(mut s) = field(&[lv3], &spawns) else { return };
    let err = s.run_both([&[EndTurn, Activate(bond), Attack(lv3)], &[EndTurn]]).expect_err("cannot attack this turn");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{WARWOLF}")) && !err.contains(&format!("\"cid\":{lv3}")), "{err}");
    clean(&s);
}

/// Level Copy: "Target 1 Level 5 or lower "LV" monster on the field; Special Summon 1 "LV" monster from your Deck with the same name
/// as that targeted monster." Team 0's Armed Dragon LV3 is copied from the Deck (two on the field), and so is team 1's: team 0
/// Special Summons its own Armed Dragon LV3 for it. Armed Dragon LV7 (Level 7) is no target.
#[test]
fn level_copy_summons_a_copy_of_a_level_monster_from_the_deck() {
    let [copy, lv3, lv7] = ["Level Copy", "Armed Dragon LV3", "Armed Dragon LV7"].map(card);
    let Some(mut s) = field(&[lv3], &[hand(0, copy), mon(0, lv3)]) else { return };
    s.run(&[Activate(copy)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", lv3), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", copy), 1, "{}", s.context());
    // The opponent's Level Monster as the target.
    let Some(mut s) = field(&[lv3], &[hand(0, copy), mon(1, lv3)]) else { return };
    s.run(&[Activate(copy)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", lv3), s.count(1, "mzone", lv3)), (1, 1), "one for me, theirs stays: {}", s.context());
    // Level 7 and "no copy in the Deck" are no good.
    let Some(mut s) = field(&[lv7], &[hand(0, copy), mon(0, lv7)]) else { return };
    never(s.run(&[Activate(copy)]));
    let Some(mut s) = field(&[], &[hand(0, copy), mon(0, lv3)]) else { return };
    never(s.run(&[Activate(copy)]));
}

/// Level Soul, a Trap: "Tribute 1 monster; banish 1 "LV" monster in your Graveyard, and if you do, Special Summon 1 monster from your
/// hand or Deck that is written in the card text of the banished monster, ignoring its Summoning conditions." Warwolf is Tributed,
/// Armed Dragon LV3 in the Graveyard banished and Armed Dragon LV5 comes from the Deck (a hand copy works the same).
#[test]
fn level_soul_tributes_a_monster_to_summon_the_next_level_from_the_deck() {
    let [soul, lv3, lv5] = ["Level Soul", "Armed Dragon LV3", "Armed Dragon LV5"].map(card);
    let spawns = [set(0, soul), mon(0, WARWOLF), grave(0, lv3)];
    let Some(mut s) = field(&[lv5], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, lv3)]);
    s.prefer_most(true);
    s.run(&[EndTurn, Activate(soul)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was Tributed: {}", s.context());
    assert_eq!((s.count(0, "removed", lv3), s.count(0, "grave", lv3)), (1, 0), "LV3 was banished: {}", s.context());
    assert_eq!(s.count(0, "mzone", lv5), 1, "LV5 was Special Summoned: {}", s.context());
    // The next level may come from the hand.
    let [lv5b, lv7] = ["Armed Dragon LV5", "Armed Dragon LV7"].map(card);
    let spawns = [set(0, soul), mon(0, WARWOLF), grave(0, lv5b), hand(0, lv7)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, lv5b), (0, FILLER)]); // the End Phase's hand-size discards are Feral Imps
    s.prefer_most(true);
    s.run(&[EndTurn, Activate(soul)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", lv5b), s.count(0, "mzone", lv7), s.count(0, "grave", WARWOLF)), (1, 1, 1), "{}", s.context());
    // "in your Graveyard": a Level Monster on the field is not banished for it.
    let Some(mut s) = field(&[lv7], &[set(0, soul), mon(0, WARWOLF), mon(0, lv5b)]) else { return };
    never(s.run(&[EndTurn, Activate(soul)]));
    clean(&s);
    assert_eq!(s.count(0, "mzone", lv5b), 1, "{}", s.context());
}

/// Level Soul: its cost needs a Level Monster in the Graveyard to banish and the next level to bring: with Horus LV4 as the only
/// monster (it cannot be both the Tribute and the banished card), or with no LV5 in the hand or Deck, it is not offered
/// (the cost never leaves nothing to choose: no Lua error stops the effect).
#[test]
fn level_soul_needs_a_tribute_a_level_monster_and_the_next_level() {
    let [soul, lv3, horus4, horus6] = ["Level Soul", "Armed Dragon LV3", "Horus the Black Flame Dragon LV4", "Horus the Black Flame Dragon LV6"].map(card);
    let Some(mut s) = field(&[horus6], &[set(0, soul), mon(0, horus4)]) else { return };
    never(s.run(&[EndTurn, Activate(soul)]));
    clean(&s);
    let Some(mut s) = field(&[], &[set(0, soul), mon(0, WARWOLF), grave(0, lv3)]) else { return };
    never(s.run(&[EndTurn, Activate(soul)]));
    clean(&s);
    assert_eq!(s.count(0, "grave", lv3), 1, "nothing was paid: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Love Letter
// ---------------------------------------------------------------------------------------------------------------

/// Love Letter: "If you control a monster and a face-down Spell/Trap: Your opponent chooses and applies 1 of these effects: Take
/// control of 1 monster your opponent controls; or take control of 1 Set Spell/Trap your opponent controls." The chooser is the
/// opponent, taking one of the activator's cards: with Warwolf and a Set Dian Keto, team 1 takes Warwolf, or (second duel) the Set
/// card.
#[test]
fn love_letter_lets_the_opponent_take_a_monster_or_a_set_card() {
    let letter = card("Love Letter");
    let spawns = [hand(0, letter), mon(0, WARWOLF), set(0, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.choose_options(&[0]);
    s.run(&[Activate(letter)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", WARWOLF)), (0, 1), "team 1 took Warwolf: {}", s.context());
    assert_eq!(s.count(0, "szone", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", letter), 1, "{}", s.context());
    let Some(mut s) = field(&[], &spawns) else { return };
    s.choose_options(&[1]);
    s.run(&[Activate(letter)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", DIAN_KETO), s.count(1, "szone", DIAN_KETO)), (0, 1), "team 1 took the Set card: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    // Without a monster, or without a face-down Spell/Trap, it is not offered.
    let Some(mut s) = field(&[], &[hand(0, letter), set(0, DIAN_KETO)]) else { return };
    never(s.run(&[Activate(letter)]));
    let Some(mut s) = field(&[], &[hand(0, letter), mon(0, WARWOLF)]) else { return };
    never(s.run(&[Activate(letter)]));
}

// ---------------------------------------------------------------------------------------------------------------
// The Ojama cards
// ---------------------------------------------------------------------------------------------------------------

const OJAMA_GREEN: u16 = 5761;
const OJAMA_YELLOW: u16 = 5811;
const OJAMA_BLACK: u16 = 5812;
const OJAMA_HURRICANE: u16 = 5839; // Ojama Delta Hurricane!!

/// Ojama Delta Thunder!!: "If you control an "Ojama Green", "Ojama Yellow" and "Ojama Black": Your opponent takes 500 damage for each
/// card in their hand and side of the field, then, you can send 1 "Ojama Delta Hurricane!!" from your hand or Deck to the Graveyard,
/// and if you do, destroy all cards your opponent controls." Team 1 has its hand and a monster and a Set card: 500 for each (hand +
/// 2); Hurricane goes from the hand to the Graveyard and both field cards are destroyed. Missing one of the three Ojamas it is
/// not offered.
#[test]
fn ojama_delta_thunder_burns_for_every_card_and_destroys_the_opponents_field() {
    let thunder = card("Ojama Delta Thunder!!");
    let spawns = [hand(0, thunder), mon(0, OJAMA_GREEN), mon(0, OJAMA_YELLOW), mon(0, OJAMA_BLACK), hand(0, OJAMA_HURRICANE), mon(1, WARWOLF), set(1, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let cards = hand_size(&s, 1) + 2;
    s.run(&[Activate(thunder)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500 * cards as i64, "500 for each of {cards} cards: {}", s.context());
    assert_eq!(s.count(0, "grave", OJAMA_HURRICANE), 1, "Hurricane was sent from the hand: {}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DIAN_KETO)), (1, 1), "the opponent's field was destroyed: {}", s.context());
    assert_eq!((monsters(&s, 1), spells(&s, 1)), (0, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", thunder), 1, "{}", s.context());
    // The Ojamas stay.
    assert_eq!(monsters(&s, 0), 3, "{}", s.context());
    // Without the Hurricane the damage is dealt and nothing is destroyed (the Deck is all Feral Imps).
    let spawns = [hand(0, thunder), mon(0, OJAMA_GREEN), mon(0, OJAMA_YELLOW), mon(0, OJAMA_BLACK), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let cards = hand_size(&s, 1) + 1;
    s.run(&[Activate(thunder)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500 * cards as i64, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "nothing was destroyed: {}", s.context());
    // Missing the Black.
    let Some(mut s) = field(&[], &[hand(0, thunder), mon(0, OJAMA_GREEN), mon(0, OJAMA_YELLOW), hand(0, OJAMA_HURRICANE)]) else { return };
    never(s.run(&[Activate(thunder)]));
}

/// Ojama Delta Wear, a Trap: "Special Summon "Ojama Yellow", "Ojama Green", and "Ojama Black" from your Graveyard that were destroyed
/// by battle this turn." Team 0's Green and Black attack into Dark Blade in turn 3 and are destroyed; Yellow lies in the Graveyard
/// from the start, not destroyed this turn. The two come back and Yellow stays.
#[test]
fn ojama_delta_wear_brings_back_the_ojamas_destroyed_by_battle_this_turn() {
    let wear = card("Ojama Delta Wear");
    let spawns = [set(0, wear), mon(0, OJAMA_GREEN), mon(0, OJAMA_BLACK), grave(0, OJAMA_YELLOW), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, OJAMA_GREEN), (0, OJAMA_BLACK), (0, OJAMA_YELLOW), (1, DARK_BLADE)]);
    s.prefer_most(true);
    s.run_both([&[EndTurn, Attack(OJAMA_GREEN), Attack(OJAMA_BLACK), Activate(wear)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", OJAMA_GREEN), s.count(0, "mzone", OJAMA_BLACK)), (1, 1), "both came back: {}", s.context());
    assert_eq!((s.count(0, "grave", OJAMA_YELLOW), s.count(0, "mzone", OJAMA_YELLOW)), (1, 0), "Yellow was not destroyed this turn: {}", s.context());
    assert_eq!(s.count(0, "grave", wear), 1, "{}", s.context());
}

/// Ojama Delta Wear: with no Ojama destroyed by battle this turn it is not offered.
#[test]
fn ojama_delta_wear_needs_an_ojama_destroyed_by_battle() {
    let wear = card("Ojama Delta Wear");
    let Some(mut s) = field(&[], &[set(0, wear), grave(0, OJAMA_GREEN)]) else { return };
    never(s.run(&[EndTurn, Activate(wear)]));
}

/// Ojama Ride: "Discard 1 each of "Ojama Yellow", "Ojama Black" and "Ojama Green" to the Graveyard: Special Summon up to 3 Level 4 or
/// lower Machine-Type Union monsters from your Deck in face-up Defense Position, but they cannot change their battle positions."
/// The three Ojamas are discarded and W-Wing Catapult, Y-Dragon Head and Z-Metal Tank come from the Deck in Defense Position and
/// stay there. Without all three Ojamas in the hand it is not offered.
#[test]
fn ojama_ride_discards_the_three_ojamas_for_three_union_monsters() {
    let ride = card("Ojama Ride");
    let [x, y, z] = ["W-Wing Catapult", "Y-Dragon Head", "Z-Metal Tank"].map(card);
    let spawns = [hand(0, ride), hand(0, OJAMA_YELLOW), hand(0, OJAMA_BLACK), hand(0, OJAMA_GREEN)];
    let Some(mut s) = field(&[x, y, z], &spawns) else { return };
    s.prefer(&[(0, OJAMA_YELLOW), (0, OJAMA_BLACK), (0, OJAMA_GREEN), (0, x), (0, y), (0, z)]);
    s.prefer_most(true);
    s.run(&[Activate(ride)]).unwrap();
    clean(&s);
    for o in [OJAMA_YELLOW, OJAMA_BLACK, OJAMA_GREEN] {
        assert_eq!(s.count(0, "grave", o), 1, "{o} was discarded: {}", s.context());
    }
    for u in [x, y, z] {
        assert_eq!((s.count(0, "mzone", u), s.position_of(0, u)), (1, Some(4)), "{u} in face-up Defense Position: {}", s.context());
    }
    // They cannot change their battle positions.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert!(offered(&s, "pos_changes").is_empty(), "{:?}", offered(&s, "pos_changes"));
    // Two of the three Ojamas are not enough.
    let Some(mut s) = field(&[x], &[hand(0, ride), hand(0, OJAMA_YELLOW), hand(0, OJAMA_BLACK)]) else { return };
    never(s.run(&[Activate(ride)]));
}

/// Ojamarble, a Trap: "When a face-up "Ojama" monster you control is destroyed and sent to the Graveyard: Return that card to the Deck,
/// and draw 2 cards, then, discard 1 card." Team 1's Warwolf destroys Ojama Green in battle: Green goes back to the Deck (not the
/// Graveyard), 2 cards are drawn and 1 discarded (net +1 card in hand, one more in the Graveyard besides the Trap).
#[test]
fn ojamarble_returns_a_destroyed_ojama_and_draws_2_discards_1() {
    let marble = card("Ojamarble");
    let Some(mut s) = field(&[], &[set(0, marble), mon(0, OJAMA_GREEN), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let (h, d) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run_both([&[Chain(marble)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", OJAMA_GREEN), 0, "Ojama Green went back to the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", marble), 1, "{}", s.context());
    assert_eq!(monsters(&s, 0), 0, "{}", s.context());
    // Back +1, drawn 2 (and the turn's own draw at team 0's next turn is not here yet).
    assert_eq!(deck_size(&s, 0), d + 1 - 2, "{}", s.context());
    assert_eq!(hand_size(&s, 0), h + 2 - 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "a Feral Imp was discarded: {}", s.context());
}

/// Ojamarking, a Trap: "When this Set card on the field would be sent to the Graveyard, banish it instead. When a monster you control is
/// destroyed by battle and sent to the Graveyard: You can banish this card from your Graveyard, then target 1 "Ojama" monster in
/// your Graveyard; Special Summon it." Set, it is destroyed by Mystical Space Typhoon and banished; from the Graveyard it answers
/// the battle destruction of Warwolf and brings Ojama Yellow back.
#[test]
fn ojamarking_banishes_itself_when_set_and_revives_an_ojama_from_the_graveyard() {
    let marking = card("Ojamarking");
    let Some(mut s) = field(&[], &[set(0, marking), hand(1, MST)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", marking), s.count(0, "grave", marking)), (1, 0), "banished instead: {}", s.context());
    let Some(mut s) = field(&[], &[grave(0, marking), grave(0, OJAMA_YELLOW), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(marking)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", marking), 1, "it banished itself as the cost: {}", s.context());
    assert_eq!((s.count(0, "mzone", OJAMA_YELLOW), s.count(0, "grave", OJAMA_YELLOW)), (1, 0), "Ojama Yellow is back: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Ojamachine Yellow and its Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Ojamachine Yellow (Level 3 LIGHT Machine, 0/1000): "When this card is Summoned: Special Summon as many "Ojamachine Yellow Tokens"
/// as possible, with the same Level, Type, Attribute, ATK, DEF and effects as this card. You take no Battle Damage from battles
/// involving this card. When this card is destroyed, inflict 300 damage to your opponent." Normal Summoned with 5 free zones it
/// makes 4 Tokens (0/1000 Level 3 LIGHT Machine); Dark Hole destroys all five: 300 for each, 1500 in all, and the Tokens vanish
/// instead of going to the Graveyard.
#[test]
fn ojamachine_yellow_makes_tokens_that_hurt_when_destroyed() {
    let [yellow, token] = ["Ojamachine Yellow", "Ojamachine Yellow Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, yellow), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Summon(yellow)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 4, "a Token for every free zone: {}", s.context());
    assert_eq!(stats(&s, 0, token), (Some(0), Some(1000), Some(3)), "{}", s.context());
    assert_eq!(s.monster(0, token).unwrap()["race"].as_i64(), Some(0x20), "Machine: {}", s.context());
    assert_eq!(s.monster(0, token).unwrap()["attribute"].as_i64(), Some(16), "LIGHT: {}", s.context());
    assert_eq!(stats(&s, 0, yellow), (Some(0), Some(1000), Some(3)), "{}", s.context());
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 5 * 300, "300 for Ojamachine Yellow and each Token: {}", s.context());
    assert_eq!((s.count(0, "mzone", token), s.count(0, "grave", token), s.count(0, "grave", yellow)), (0, 0, 1), "{}", s.context());
}

/// Ojamachine Yellow: "You take no Battle Damage from battles involving this card", the Tokens too: team 1's Warwolf (2000)
/// attacks the Attack Position monsters (0 ATK each): 0 damage, and the monster it destroys inflicts 300.
#[test]
fn ojamachine_yellow_and_its_tokens_protect_their_controller_from_battle_damage() {
    let yellow = card("Ojamachine Yellow");
    let Some(mut s) = field(&[], &[hand(0, yellow), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Summon(yellow)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "the destroyed monster hurt its destroyer: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Quick Rush, Sealed Gate and the White Knights
// ---------------------------------------------------------------------------------------------------------------

/// Quick Rush: "Apply 1 of these effects. Draw 1 card. Level 4 or lower monsters can attack during the first turn." First option:
/// Quick Rush goes to the Graveyard and 1 card is drawn: the hand is as large as before (the Spell out, 1 card in).
#[test]
fn quick_rush_draws_1_card() {
    let rush = card("Quick Rush");
    let Some(mut s) = field(&[], &[hand(0, rush)]) else { return };
    s.choose_options(&[0]);
    let h = hand_size(&s, 0);
    s.run(&[Activate(rush)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), h, "the Spell left the hand and 1 card came: {}", s.context());
    assert_eq!(s.count(0, "grave", rush), 1, "{}", s.context());
}

/// Quick Rush, the second option: Level 4 or lower monsters can attack during the first turn (neither player has a Battle Phase in
/// turn 1 otherwise): Warwolf (Level 4) attacks Feral Imp in turn 1 and hurts for 700, Dark Magician (Level 7) cannot attack.
#[test]
fn quick_rush_lets_level_4_or_lower_monsters_attack_in_the_first_turn() {
    let rush = card("Quick Rush");
    let spawns = [hand(0, rush), mon(0, WARWOLF), mon(0, DARK_MAGICIAN), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.choose_options(&[1]);
    s.run(&[Activate(rush), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", FILLER), 1, "Warwolf attacked in turn 1: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
    // With it a Level 7 monster does not attack either; without it there is no Battle Phase in turn 1 at all.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.choose_options(&[1]);
    s.run(&[Activate(rush)]).unwrap();
    s.run_to(&[Attack(WARWOLF)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    let attackers: Vec<_> = s.prompt()["attacks"].as_array().unwrap().iter().map(|a| a["cid"].clone()).collect();
    assert!(attackers.contains(&WARWOLF.into()) && !attackers.contains(&DARK_MAGICIAN.into()), "{attackers:?}");
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(1, FILLER)]) else { return };
    s.run(&[]).unwrap();
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase in turn 1 without it: {}", s.prompt());
}

/// Sealed Gate, a Trap: "When this card is destroyed and sent to the Graveyard, you can banish 1 "White Knight Swordsman", "White Knight
/// Lancer" and "White Knight Gardna" from your Graveyard to Special Summon 1 "White Knight Lord" from your hand or Deck." Set, it
/// is destroyed by Mystical Space Typhoon; the three Knights in the Graveyard are banished and White Knight Lord comes from the
/// Deck. With one Knight missing nothing is banished.
#[test]
fn sealed_gate_banishes_the_three_knights_for_white_knight_lord() {
    let [gate, swordsman, lancer, gardna, lord] = ["Sealed Gate", "White Knight Swordsman", "White Knight Lancer", "White Knight Gardna", "White Knight Lord"].map(card);
    let spawns = [set(0, gate), grave(0, swordsman), grave(0, lancer), grave(0, gardna), hand(1, MST)];
    let Some(mut s) = field(&[lord], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, swordsman), (0, lancer), (0, gardna), (1, FILLER)]);
    s.prefer_most(true);
    s.run_both([&[Chain(gate)], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", lord), 1, "White Knight Lord came from the Deck: {}", s.context());
    for k in [swordsman, lancer, gardna] {
        assert_eq!((s.count(0, "removed", k), s.count(0, "grave", k)), (1, 0), "{k} was banished: {}", s.context());
    }
    assert_eq!(s.count(0, "grave", gate), 1, "{}", s.context());
    // One Knight short.
    let spawns = [set(0, gate), grave(0, swordsman), grave(0, lancer), hand(1, MST)];
    let Some(mut s) = field(&[lord], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(gate)], &[Activate(MST)]]).expect_err("no answer without the third Knight");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", lord), 0, "{}", s.context());
    assert_eq!(s.count(0, "removed", swordsman), 0, "nothing was banished: {}", s.context());
}

const SWORDSMAN: &str = "White Knight Swordsman";

/// White Knight Swordsman: "While this card is in your Graveyard, all "White Knight" monsters you control gain 300 ATK." White Knight
/// Lancer (1500) is 1800 with it in the Graveyard, 1500 without; White Knight Gardna on the field gains too, a
/// monster that is no White Knight (Warwolf) and the opponent's Knights do not.
#[test]
fn white_knight_swordsman_in_the_graveyard_gives_300_to_every_white_knight() {
    let [swordsman, lancer, gardna] = [SWORDSMAN, "White Knight Lancer", "White Knight Gardna"].map(card);
    let Some(s) = field(&[], &[grave(0, swordsman), mon(0, lancer), mon(0, gardna), mon(0, WARWOLF), mon(1, lancer)]) else { return };
    assert_eq!((s.attack_of(0, lancer), s.attack_of(0, gardna), s.attack_of(0, WARWOLF)), (Some(1800), Some(1100), Some(2000)), "{}", s.context());
    assert_eq!(s.attack_of(1, lancer), Some(1500), "the opponent's Knights gain nothing: {}", s.context());
    let Some(s) = field(&[], &[mon(0, lancer), mon(0, swordsman)]) else { return };
    assert_eq!((s.attack_of(0, lancer), s.attack_of(0, swordsman)), (Some(1500), Some(1200)), "on the field it gives nothing: {}", s.context());
}

/// White Knight Lancer: "If this card attacks a Defense Position monster, inflict piercing battle damage." Lancer (1500) attacks Thunder
/// Kid (DEF 600) in Defense Position: 900 damage; with Swordsman in the Graveyard 1800 and 1200.
#[test]
fn white_knight_lancer_inflicts_piercing_damage() {
    let [lancer, swordsman] = ["White Knight Lancer", SWORDSMAN].map(card);
    let Some(mut s) = field(&[], &[mon(0, lancer), mon_def(1, THUNDER_KID)]) else { return };
    s.run_both([&[EndTurn, Attack(lancer)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 900, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, lancer), mon_def(1, THUNDER_KID), grave(0, swordsman)]) else { return };
    s.run_both([&[EndTurn, Attack(lancer)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(1), 8000 - 1200, "{}", s.context());
    // The control: Warwolf has no piercing.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon_def(1, THUNDER_KID)]) else { return };
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// White Knight Gardna: "If you would take battle damage from a battle involving a "White Knight" monster you control: You can banish
/// this card from your Graveyard; you take no battle damage from that battle, also that "White Knight" monster cannot be destroyed
/// by that battle." Team 1's Warwolf (2000) attacks White Knight Swordsman (1200): with Gardna banished from the Graveyard no damage
/// and the Swordsman lives; without it 800 damage and the Swordsman is destroyed.
#[test]
fn white_knight_gardna_banishes_itself_to_save_a_white_knight_and_its_controller() {
    let [gardna, swordsman] = ["White Knight Gardna", SWORDSMAN].map(card);
    let Some(mut s) = field(&[], &[grave(0, gardna), mon(0, swordsman), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Chain(gardna)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", gardna), 1, "Gardna was banished: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", swordsman), 1, "not destroyed by that battle: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, swordsman), mon(1, WARWOLF)]) else { return };
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
    assert_eq!(s.count(0, "grave", swordsman), 1, "{}", s.context());
    // A monster that is no White Knight is not protected.
    let Some(mut s) = field(&[], &[grave(0, gardna), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!((s.lp(0), s.count(0, "removed", gardna)), (8000 - 200, 0), "{}", s.context());
}

/// Cemetery Change moves each Graveyard's cards through no place (batch 7's `limbo` in `translate/events.rs`). The per-card fuzz's
/// seed 62225 (three Cemetery Changes, copies of one card in both Graveyards, a card of one side lying in the other's) left the
/// client's view of one new Graveyard in the wrong order: a card leaving limbo was taken from the index it had left, which the
/// cards that arrived first had moved, so a copy with the same id was taken in its place; it is now found by the mirror's own
/// card. This scenario is the simple form (Warwolf and Thunder Kid in both Graveyards, in opposite orders); it passed before the
/// fix too, the seed itself is the regression test: `YO3_FUZZ_ROWS=5522-5522 YO3_FUZZ_DUELS=8 task check:fuzz`.
#[test]
fn cemetery_change_swaps_graveyards_that_hold_copies_of_the_same_cards() {
    let change = card("Cemetery Change");
    let spawns = [hand(0, change), grave(0, WARWOLF), grave(0, THUNDER_KID), grave(1, THUNDER_KID), grave(1, WARWOLF), grave(1, change)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(change)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", THUNDER_KID), s.count(0, "grave", change)), (1, 1, 2), "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", THUNDER_KID), s.count(1, "grave", change)), (1, 1, 0), "{}", s.context());
}
