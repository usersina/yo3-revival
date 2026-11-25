//! Printed cards from the anime, group 08 (Influence Dragon to Legendary Knight Timaeus): one scenario per effect through the
//! real core, with both seats' client models checked after every step (`duelcheck::scenario`). The printed effect is
//! tested with anime cards off, and the anime version of a card that differs with anime cards on (`start_anime_field`).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// Added cards (engine/added/cards.tsv; an id is the Konami id, or the stand-in where there is none).
const INFLUENCE_DRAGON: u16 = 9800;
const DEMISE_LORD: u16 = 5169; // Invincible Demise Lord
const SLASH_DRAW: u16 = 13814;
const IPIRIA: u16 = 14546;
const IRON_CAGE: u16 = 13810;
const IRON_HANS: u16 = 13804;
const IRON_KNIGHT: u16 = 13805;
const GOLDEN_CASTLE: u16 = 13808; // Golden Castle of Stromberg
const JORMUNGARDR: u16 = 11465;
const JUNK_BERSERKER: u16 = 9508;
const JUNK_SERVANT: u16 = 9472;
const JURAGEDO: u16 = 5636;
const JURASSIC_IMPACT: u16 = 13156;
const KAITOPTERA: u16 = 4827;
const KAMION: u16 = 13160; // Kamion, the Timelord
const LAZION: u16 = 13157; // Lazion, the Timelord
const KURIBABYLON: u16 = 3994;
const KURIBAH: u16 = 3989; // stand-in for Konami id 16397
const KURIBEE: u16 = 3991; // stand-in for 16398
const KURIBEH: u16 = 3993; // stand-in for 16400
const KURIBOO: u16 = 3992; // stand-in for 16399
const LEFT_ARM_OFFERING: u16 = 5637;
const LEGEND_OF_HEART: u16 = 11152;
const KNIGHT_CRITIAS: u16 = 11881; // Legendary Knight Critias
const KNIGHT_HERMOS: u16 = 11887; // Legendary Knight Hermos
const KNIGHT_TIMAEUS: u16 = 11149; // Legendary Knight Timaeus
const EYE_OF_TIMAEUS: u16 = 10326;
const CLAW_OF_HERMOS: u16 = 11892;
const FANG_OF_CRITIAS: u16 = 11885;
// The game's own.
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect, a Beast-Warrior
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;
const SABERSAURUS: u16 = 6796; // a Dinosaur, 1900 ATK
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const JUNK_SYNCHRON: u16 = 7687; // a Level 3 Tuner Warrior, 1300 ATK
const MIRROR_FORCE: u16 = 4887;
const SOUL_RELEASE: u16 = 4857;
const POLYMERIZATION: u16 = 4837;
const OFFERINGS: u16 = 5214; // Offerings to the Doomed
const STOP_DEFENSE: u16 = 4326;
const FLUTE_OF_KURIBOH: u16 = 6636; // The Flute of Summoning Kuriboh
const GATE_GUARDIAN: u16 = 4380; // 3750/3400
const THOR: u16 = 9329; // Thor, Lord of the Aesir
const DIMENSIONAL_PRISON: u16 = 7400;
const MONSTER_REBORN: u16 = 4842;

const DRAGON: u64 = 0x2000;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn sp(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

const FILLER: [u16; 40] = [KURIBOH; 40];

/// A duel at `lp` with `spawns` in place, standing at team 0's first idle prompt (its opening hand and first draw done);
/// the decks are Kuriboh but for `deck` (team 0's cards asked for, at the bottom) and `extra`. The decks are shuffled by the
/// seed: the first seed that leaves none of `deck` in team 0's hand is the one played.
fn field(anime: bool, lp: u32, deck: &[u16], extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let main: Vec<u16> = FILLER.iter().copied().take(40 - deck.len().min(40)).chain(deck.iter().copied()).collect();
    let decks: [(&[u16], &[u16]); 2] = [(&main, extra), (&FILLER, &[])];
    for seed in 7..60 {
        let mut s = if anime {
            Scenario::start_anime_field(&dir, seed, lp, decks, spawns)
        } else {
            Scenario::start_field(&dir, seed, lp, decks, spawns)
        }
        .unwrap();
        s.run(&[]).unwrap();
        if deck.iter().all(|&c| s.count(0, "hand", c) == 0) {
            return Some(s);
        }
    }
    panic!("no seed keeps {deck:?} in the Deck");
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// Whether the pending idle prompt offers this card in that list (`summons`, `special_summons`, `activates`...).
fn offers(s: &Scenario, list: &str, cid: u16) -> bool {
    s.prompt()[list].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
}

fn race_of(s: &Scenario, team: usize, cid: u16) -> Option<u64> {
    s.monster(team, cid)?["race"].as_u64()
}

// ---------------------------------------------------------------------------------------------------------------------
// Influence Dragon

/// Influence Dragon: "Once per turn: You can target 1 face-up monster you control; it becomes Dragon-Type until the End
/// Phase." Gene-Warped Warwolf is a Beast-Warrior until then.
#[test]
fn influence_dragon_makes_a_monster_a_dragon() {
    let spawns = [sp(0, Monster, INFLUENCE_DRAGON), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    assert_ne!(race_of(&s, 0, WARWOLF), Some(DRAGON), "{}", s.context());
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(INFLUENCE_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(race_of(&s, 0, WARWOLF), Some(DRAGON), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Invincible Demise Lord

/// Invincible Demise Lord: "During the End Phase, if a monster(s) was destroyed by battle this turn: You can Special
/// Summon this card from your hand or GY". A Warwolf destroys a Kuriboh; the Lord comes from the hand, with its own 1300 ATK.
#[test]
fn demise_lord_comes_back_after_a_battle_destruction() {
    let spawns = [sp(0, Hand, DEMISE_LORD), sp(0, Monster, WARWOLF), sp(1, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Attack(WARWOLF), Chain(DEMISE_LORD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DEMISE_LORD), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, DEMISE_LORD), Some(1300), "{}", s.context());
}

/// Invincible Demise Lord: "...if "Invincible Demise Lord" was destroyed by battle this turn, this card's original ATK
/// becomes 3000, also it cannot be destroyed by card effects." The Lord loses a battle to a Warwolf, then returns from the
/// GY at 3000 ATK (printed text; the End Phase offers it to its player).
#[test]
fn demise_lord_destroyed_by_battle_returns_with_3000_atk() {
    let spawns = [sp(0, Monster, DEMISE_LORD), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Attack(DEMISE_LORD), Chain(DEMISE_LORD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DEMISE_LORD), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, DEMISE_LORD), Some(3000), "{}", s.context());
}

/// ...and "cannot be destroyed by card effects": Dark Hole, afterwards, leaves it.
#[test]
fn demise_lord_returned_cannot_be_destroyed_by_effects() {
    let spawns = [sp(0, Monster, DEMISE_LORD), sp(1, Monster, WARWOLF), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, KURIBOH)]); // the hand limit's discard
    s.run_both([&[Attack(DEMISE_LORD), Chain(DEMISE_LORD)], &[EndTurn, Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DEMISE_LORD), 1, "Dark Hole destroyed it: {}", s.context());
}

/// Invincible Demise Lord: "During your Main Phase: You can reveal cards in your Deck, including "Slash Draw", equal to
/// the number of cards your opponent controls +1 and place those revealed cards on top of the Deck". The opponent
/// controls nothing: Slash Draw alone goes on top, and is the next card drawn.
#[test]
fn demise_lord_puts_slash_draw_on_top_of_the_deck() {
    let spawns = [sp(0, Monster, DEMISE_LORD)];
    let Some(mut s) = field(false, 8000, &[SLASH_DRAW], &[], &spawns) else { return };
    s.run(&[Activate(DEMISE_LORD), EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", SLASH_DRAW), 1, "{}", s.context());
}

/// The anime Invincible Demise Lord: "If this card is destroyed by battle and sent to the Graveyard: You can Special
/// Summon it during the End Phase... its ATK becomes 3000, and it cannot be destroyed by card effects."
#[test]
fn demise_lord_anime_returns_at_the_end_phase_with_3000_atk() {
    let spawns = [sp(0, Monster, DEMISE_LORD), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run(&[Attack(DEMISE_LORD), Chain(DEMISE_LORD), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DEMISE_LORD), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, DEMISE_LORD), Some(3000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Ipiria

/// Ipiria: "If this card is Summoned: Draw 1 card." The deck is all Kuriboh: the hand holds one more.
#[test]
fn ipiria_draws_when_summoned() {
    let spawns = [sp(0, Hand, IPIRIA)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    let before = s.count(0, "hand", KURIBOH);
    s.run(&[Summon(IPIRIA)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", IPIRIA), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", KURIBOH), before + 1, "{}", s.context());
}

/// The anime Ipiria: "If this card is Summoned: Draw 1 card."
#[test]
fn ipiria_anime_draws_when_summoned() {
    let spawns = [sp(0, Hand, IPIRIA)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    let before = s.count(0, "hand", KURIBOH);
    s.run(&[Summon(IPIRIA)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", IPIRIA), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", KURIBOH), before + 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Iron Cage

/// Iron Cage: "When this card is activated: Send 1 monster you control to the GY... Once per turn, during your Standby
/// Phase: You can target the monster that was sent to the GY by this card's effect; destroy this card, and if you do,
/// Special Summon that monster to your field."
#[test]
fn iron_cage_sends_a_monster_and_brings_it_back() {
    let spawns = [sp(0, SpellSet, IRON_CAGE), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(IRON_CAGE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "mzone", WARWOLF)), (1, 0), "{}", s.context());
    s.run(&[Chain(IRON_CAGE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "grave", IRON_CAGE)), (1, 1), "{}", s.context());
}

/// Iron Cage: "...or if "Golden Castle of Stromberg" is in a Field Zone, you can send 1 monster your opponent controls
/// to the GY, instead." Then the Standby Phase Special Summons that monster "to your field": the opponent's Warwolf
/// ends up on the Cage's controller's side.
#[test]
fn iron_cage_with_the_castle_sends_the_opponents_monster() {
    let spawns = [sp(0, SpellSet, IRON_CAGE), sp(0, Hand, GOLDEN_CASTLE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(GOLDEN_CASTLE), Activate(IRON_CAGE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    s.run(&[Chain(IRON_CAGE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// The anime Iron Cage: "Target 1 monster on the field; send it to the Graveyard. During your next Standby Phase: Special
/// Summon that monster to its owner's side of the field, and destroy this card." The opponent's Warwolf goes back to
/// the opponent.
#[test]
fn iron_cage_anime_returns_the_monster_to_its_owner() {
    let spawns = [sp(0, SpellSet, IRON_CAGE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(IRON_CAGE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    s.run(&[EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", WARWOLF), s.count(0, "grave", IRON_CAGE)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Iron Hans and Iron Knight

/// Iron Hans: "If this card is Summoned: You can Special Summon 1 "Iron Knight" from your Deck, also if "Golden Castle of
/// Stromberg" is not in a Field Zone when this effect resolves, you cannot Special Summon monsters from the Extra Deck for
/// the rest of this turn." Iron Knight "Loses 1000 ATK while you control "Iron Hans"". A Level 7 Synchro Monster (Junk
/// Berserker, from Junk Synchron and Iron Hans) is then not on offer.
#[test]
fn iron_hans_brings_the_knight_and_locks_the_extra_deck() {
    let spawns = [sp(0, Hand, IRON_HANS), sp(0, Monster, JUNK_SYNCHRON)];
    let Some(mut s) = field(false, 8000, &[IRON_KNIGHT], &[JUNK_BERSERKER], &spawns) else { return };
    s.run(&[Summon(IRON_HANS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", IRON_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, IRON_KNIGHT), Some(700), "{}", s.context());
    assert_eq!(s.attack_of(0, IRON_HANS), Some(1200), "{}", s.context());
    assert!(!offers(&s, "special_summons", JUNK_BERSERKER), "the Extra Deck is locked: {}", s.context());
}

/// The same with "Golden Castle of Stromberg" in a Field Zone: no lock (Junk Berserker is on offer), and Iron Hans
/// "Gains 1000 ATK for each "Iron Knight" you control, while "Golden Castle of Stromberg" is in a Field Zone".
#[test]
fn iron_hans_with_the_castle_has_no_lock_and_gains_attack() {
    let spawns = [sp(0, Hand, IRON_HANS), sp(0, Monster, JUNK_SYNCHRON), sp(0, Hand, GOLDEN_CASTLE)];
    let Some(mut s) = field(false, 8000, &[IRON_KNIGHT], &[JUNK_BERSERKER], &spawns) else { return };
    s.run(&[Activate(GOLDEN_CASTLE), Summon(IRON_HANS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", IRON_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, IRON_HANS), Some(2200), "{}", s.context());
    assert!(offers(&s, "special_summons", JUNK_BERSERKER), "{}", s.context());
}

/// Iron Hans, the anime version: "When this card is Summoned: you can Special Summon 1 "Iron Knight" from your Deck."
#[test]
fn iron_hans_anime_brings_the_knight() {
    let spawns = [sp(0, Hand, IRON_HANS)];
    let Some(mut s) = field(true, 8000, &[IRON_KNIGHT], &[], &spawns) else { return };
    s.run(&[Summon(IRON_HANS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", IRON_KNIGHT), 1, "{}", s.context());
}

/// Iron Knight: "If this card on the field is destroyed by battle...: You can add 1 "Iron Hans" from your Deck to your
/// hand". A Warwolf destroys the Knight (1700 ATK) and Iron Hans is added.
#[test]
fn iron_knight_destroyed_adds_iron_hans() {
    let spawns = [sp(0, Monster, IRON_KNIGHT), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[IRON_HANS], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", IRON_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", IRON_HANS), 1, "{}", s.context());
}

/// Iron Knight: "...or, if "Golden Castle of Stromberg" is in a Field Zone, you can add 1 Warrior monster from your Deck to
/// your hand, instead." Junk Synchron (a Warrior) is chosen over Iron Hans (sent to the GY by Dark Hole: the Castle's own attack effect would otherwise
/// get in the way of a battle).
#[test]
fn iron_knight_with_the_castle_adds_a_warrior() {
    let spawns = [sp(0, Monster, IRON_KNIGHT), sp(0, Hand, GOLDEN_CASTLE), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, &[IRON_HANS, JUNK_SYNCHRON], &[], &spawns) else { return };
    s.prefer(&[(0, JUNK_SYNCHRON)]);
    s.run_both([&[Activate(GOLDEN_CASTLE), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", IRON_KNIGHT), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", JUNK_SYNCHRON), s.count(0, "hand", IRON_HANS)), (1, 0), "{}", s.context());
}

/// Iron Knight: "If this card on the field is ... sent to the GY by a card effect" (Dark Hole).
#[test]
fn iron_knight_destroyed_by_an_effect_adds_iron_hans() {
    let spawns = [sp(0, Monster, IRON_KNIGHT), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, &[IRON_HANS], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", IRON_HANS), 1, "{}", s.context());
}

/// The anime Iron Knight: "While there is a face-up "Iron Hans" on the field, this card loses 1000 ATK."
#[test]
fn iron_knight_anime_loses_1000_attack_beside_iron_hans() {
    let spawns = [sp(0, Monster, IRON_KNIGHT)];
    let Some(s) = field(true, 8000, &[], &[], &spawns) else { return };
    assert_eq!(s.attack_of(0, IRON_KNIGHT), Some(1700), "{}", s.context());
    drop(s);
    let spawns = [sp(0, Monster, IRON_KNIGHT), sp(1, Monster, IRON_HANS)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, IRON_KNIGHT), Some(700), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Jormungardr the Nordic Serpent

/// Jormungardr: "Must first be Special Summoned (from your hand) to your opponent's field in Defense Position, while an
/// "Aesir" monster is on the field." Thor is on the field.
#[test]
fn jormungardr_goes_to_the_opponents_field_in_defense() {
    let spawns = [sp(0, Hand, JORMUNGARDR), sp(0, Monster, THOR)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[SpSummon(JORMUNGARDR)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", JORMUNGARDR), s.count(0, "mzone", JORMUNGARDR)), (1, 0), "{}", s.context());
    assert_eq!(s.position_of(1, JORMUNGARDR), Some(4), "{}", s.context());
}

/// ...and it cannot be Special Summoned without an Aesir monster, nor Normal Summoned.
#[test]
fn jormungardr_needs_an_aesir_monster() {
    let spawns = [sp(0, Hand, JORMUNGARDR)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!offers(&s, "special_summons", JORMUNGARDR) && !offers(&s, "summons", JORMUNGARDR), "{}", s.context());
    assert!(!offers(&s, "monster_sets", JORMUNGARDR), "{}", s.context());
}

/// Jormungardr: "If no "Aesir" monster is on the field, destroy this card." Offerings to the Doomed destroys Thor.
#[test]
fn jormungardr_is_destroyed_without_an_aesir_monster() {
    let spawns = [sp(0, Hand, JORMUNGARDR), sp(0, Monster, THOR), sp(0, Hand, OFFERINGS)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, THOR)]);
    s.run(&[SpSummon(JORMUNGARDR), Activate(OFFERINGS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THOR), 1, "{}", s.context());
    assert_eq!((s.count(1, "mzone", JORMUNGARDR), s.count(0, "grave", JORMUNGARDR)), (0, 1), "{}", s.context());
}

/// Jormungardr: "Once while face-up on the field, if this face-up Defense Position card you control is changed to face-up
/// Attack Position: Take 3000 damage." Stop Defense turns it to Attack Position on its controller's side.
#[test]
fn jormungardr_deals_3000_when_turned_to_attack() {
    let spawns = [sp(0, Hand, JORMUNGARDR), sp(0, Monster, THOR), sp(0, Hand, STOP_DEFENSE)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[SpSummon(JORMUNGARDR), Activate(STOP_DEFENSE)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, JORMUNGARDR), Some(1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 5000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Junk Berserker and Junk Servant

/// Junk Berserker: "Junk Synchron" + 1 or more non-Tuner monsters. Junk Synchron (Level 3) and Junk Servant (Level 4).
#[test]
fn junk_berserker_is_synchro_summoned() {
    let spawns = [sp(0, Monster, JUNK_SYNCHRON), sp(0, Monster, JUNK_SERVANT)];
    let Some(mut s) = field(false, 8000, &[], &[JUNK_BERSERKER], &spawns) else { return };
    s.run(&[SpSummon(JUNK_BERSERKER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_BERSERKER), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", JUNK_SYNCHRON) + s.count(0, "grave", JUNK_SERVANT), 2, "{}", s.context());
}

/// Junk Berserker: "You can banish 1 "Junk" monster from your Graveyard, then target 1 face-up monster your opponent
/// controls; that target loses ATK equal to the banished monster's ATK." Junk Synchron (1300) from the GY: Warwolf 700.
#[test]
fn junk_berserker_banishes_a_junk_to_weaken_a_monster() {
    let spawns = [sp(0, Monster, JUNK_BERSERKER), sp(0, Grave, JUNK_SYNCHRON), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, JUNK_SYNCHRON), (1, WARWOLF)]);
    s.run(&[Activate(JUNK_BERSERKER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", JUNK_SYNCHRON), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(700), "{}", s.context());
}

/// Junk Berserker: "At the start of the Damage Step, if this card attacks a Defense Position monster: Destroy that
/// monster." Gate Guardian (3400 DEF) is beyond Berserker's 2700 ATK, and is destroyed anyway, with no damage to either.
#[test]
fn junk_berserker_destroys_a_defense_position_monster_it_attacks() {
    let spawns = [sp(0, Monster, JUNK_BERSERKER), sp(1, MonsterDefense, GATE_GUARDIAN)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Attack(JUNK_BERSERKER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", GATE_GUARDIAN), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1), s.count(0, "mzone", JUNK_BERSERKER)), (8000, 8000, 1), "{}", s.context());
}

/// Junk Servant: "If you control a "Junk" monster, you can Special Summon this card (from your hand)."
#[test]
fn junk_servant_special_summons_beside_a_junk_monster() {
    let spawns = [sp(0, Hand, JUNK_SERVANT), sp(0, Monster, JUNK_SYNCHRON)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[SpSummon(JUNK_SERVANT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_SERVANT), 1, "{}", s.context());
}

/// ...and without one it cannot.
#[test]
fn junk_servant_needs_a_junk_monster() {
    let spawns = [sp(0, Hand, JUNK_SERVANT), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!offers(&s, "special_summons", JUNK_SERVANT), "{}", s.context());
    assert!(offers(&s, "summons", JUNK_SERVANT), "it can still be Normal Summoned: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Juragedo

/// Juragedo: "During the Battle Step (Quick Effect): You can Special Summon this card from your hand, and if you do, gain
/// 1000 LP." The opponent's Warwolf attacks; Juragedo comes from the hand in answer.
#[test]
fn juragedo_special_summons_in_the_battle_step_and_gains_lp() {
    let spawns = [sp(0, Hand, JURAGEDO), sp(0, Monster, WARWOLF), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the hand limit's discard
    s.run_both([&[EndTurn, Chain(JURAGEDO)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JURAGEDO), 1, "{}", s.context());
    assert_eq!(s.lp(0), 9000, "{}", s.context());
}

/// Juragedo: "(Quick Effect): You can Tribute this card, then target 1 face-up monster you control; it gains 1000 ATK until
/// the end of the next turn."
#[test]
fn juragedo_tributes_itself_to_give_1000_attack() {
    let spawns = [sp(0, Monster, JURAGEDO), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(JURAGEDO)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", JURAGEDO), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(3000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Jurassic Impact

/// Team 0 gains 1000 LP with Dian Keto the Cure Master (9000), so team 1 (8000) has the lower LP; team 1 then has two
/// Sabersaurus (Dinosaurs) and Jurassic Impact set, and team 0 a Warwolf.
fn jurassic_field(anime: bool) -> Option<Scenario> {
    let spawns = [sp(0, Hand, DIAN_KETO), sp(0, Monster, WARWOLF), sp(1, SpellSet, JURASSIC_IMPACT), sp(1, Monster, SABERSAURUS),
        sp(1, Monster, SABERSAURUS)];
    field(anime, 8000, &[], &[], &spawns)
}

/// Jurassic Impact: "If you control 2 or more Dinosaur monsters and your LP is lower than your opponent's: Destroy as many
/// monsters on the field as possible, and if you do, take 1000 damage for each monster destroyed, then inflict damage to
/// your opponent, equal to the damage you took." 3 monsters: 3000 to each player.
#[test]
fn jurassic_impact_destroys_everything_and_returns_the_damage() {
    let Some(mut s) = jurassic_field(false) else { return };
    s.run_both([&[Activate(DIAN_KETO), EndTurn], &[Activate(JURASSIC_IMPACT)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", SABERSAURUS), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (6000, 5000), "{}", s.context());
}

/// ...and it needs a lower LP: at equal LP (no Dian Keto) it cannot be activated.
#[test]
fn jurassic_impact_needs_the_lower_life_points() {
    let Some(mut s) = jurassic_field(false) else { return };
    s.run_both([&[EndTurn], &[]]).unwrap();
    assert!(!offers(&s, "activates", JURASSIC_IMPACT), "{}", s.context());
}

/// The anime Jurassic Impact: "If your LP is lower than your opponent's: Destroy all monsters on the field then each player
/// takes 1000 damage for each monster they controlled that was destroyed". Team 1 lost 2 monsters, team 0 one.
#[test]
fn jurassic_impact_anime_damages_each_player_for_their_own_monsters() {
    let Some(mut s) = jurassic_field(true) else { return };
    s.run_both([&[Activate(DIAN_KETO), EndTurn], &[Activate(JURASSIC_IMPACT)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", SABERSAURUS), 2, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 6000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Kaitoptera

/// Kaitoptera: "During your Main Phase: You can add 1 "Polymerization" from your Deck to your hand."
#[test]
fn kaitoptera_adds_polymerization() {
    let spawns = [sp(0, Monster, KAITOPTERA)];
    let Some(mut s) = field(false, 8000, &[POLYMERIZATION], &[], &spawns) else { return };
    s.run(&[Activate(KAITOPTERA)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", POLYMERIZATION), 1, "{}", s.context());
}

/// Kaitoptera: "If this card is banished: You can Special Summon it, then you can add 1 "Polymerization" from your GY to
/// your hand." Soul Release banishes it from the GY; Polymerization waits in the GY.
#[test]
fn kaitoptera_banished_comes_back_and_adds_polymerization() {
    let spawns = [sp(0, Hand, SOUL_RELEASE), sp(0, Grave, KAITOPTERA), sp(0, Grave, POLYMERIZATION)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, KAITOPTERA)]);
    s.run(&[Activate(SOUL_RELEASE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KAITOPTERA), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", POLYMERIZATION), s.count(0, "grave", POLYMERIZATION)), (1, 0), "{}", s.context());
}

/// Kaitoptera: "If your opponent controls 2 or more face-up monsters, except WIND monsters, those monsters cannot target
/// this card for attacks." With one Warwolf the opponent attacks it; with two, they cannot.
#[test]
fn kaitoptera_cannot_be_attacked_by_two_non_wind_monsters() {
    let one = [sp(0, Monster, KAITOPTERA), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &one) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KAITOPTERA), 1, "one monster attacks it: {}", s.context());
    drop(s);
    let two = [sp(0, Monster, KAITOPTERA), sp(1, Monster, WARWOLF), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &two) else { return };
    let attacked = s.run_both([&[EndTurn], &[Attack(WARWOLF)]]);
    assert!(attacked.is_err(), "two non-WIND monsters could attack it: {}", s.context());
    assert_eq!(s.count(0, "mzone", KAITOPTERA), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// The Timelords

/// Kamion, the Timelord: "If you control no monsters, you can Normal Summon this card without Tributing." Level 10.
#[test]
fn kamion_is_normal_summoned_without_tributes() {
    let spawns = [sp(0, Hand, KAMION)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Summon(KAMION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KAMION), 1, "{}", s.context());
}

/// ...only when you control no monsters.
#[test]
fn kamion_needs_an_empty_field() {
    let spawns = [sp(0, Hand, KAMION), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!offers(&s, "summons", KAMION), "{}", s.context());
}

/// Kamion: "Cannot be destroyed by battle or card effects. You take no battle damage from attacks involving this card. At
/// the end of the Battle Phase, if this card battled: Shuffle 1 card your opponent controls into the Deck, and if you do,
/// inflict 500 damage to your opponent." Team 1's Kamion (0 ATK) attacks the Warwolf: nothing is destroyed or lost
/// in the battle, then the Warwolf is shuffled away and team 0 takes 500.
#[test]
fn kamion_battles_then_shuffles_a_card_and_burns_500() {
    let spawns = [sp(0, Monster, WARWOLF), sp(1, Hand, KAMION)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Summon(KAMION), Attack(KAMION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", KAMION), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "grave", WARWOLF)), (0, 0), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (7500, 8000), "{}", s.context());
}

/// Kamion: "Cannot be destroyed by ... card effects": the opponent's Dark Hole leaves it.
#[test]
fn kamion_survives_dark_hole() {
    let spawns = [sp(0, Hand, DARK_HOLE), sp(1, Hand, KAMION)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, KURIBOH)]); // the hand limit's discard
    s.run_both([&[EndTurn, Activate(DARK_HOLE)], &[Summon(KAMION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", KAMION), 1, "{}", s.context());
}

/// Kamion: "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn kamion_shuffles_itself_at_its_standby_phase() {
    let spawns = [sp(0, Hand, KAMION)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Summon(KAMION), EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", KAMION), s.count(0, "hand", KAMION), s.count(0, "grave", KAMION)), (0, 0, 0), "{}", s.context());
}

/// The anime Kamion: "At the end of the Damage Step, if this card battled: Shuffle as many monsters your opponent controls
/// as possible into the Deck, then inflict 500 damage to your opponent for each card returned." Two Warwolves: 1000. (Revived by Monster Reborn: the anime card has no Normal Summon without Tribute.)
#[test]
fn kamion_anime_shuffles_every_monster() {
    let spawns = [sp(0, Monster, WARWOLF), sp(0, Monster, WARWOLF), sp(1, Grave, KAMION), sp(1, Hand, MONSTER_REBORN)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, KAMION)]);
    s.run_both([&[EndTurn], &[Activate(MONSTER_REBORN), Attack(KAMION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (7000, 8000), "{}", s.context());
}

/// Lazion, the Timelord: "At the end of the Battle Phase, if this card battled: Shuffle all cards from your opponent's GY
/// into the Deck."
#[test]
fn lazion_shuffles_the_opponents_graveyard() {
    let spawns = [sp(0, Monster, WARWOLF), sp(0, Grave, KURIBOH), sp(0, Grave, DARK_HOLE), sp(1, Hand, LAZION)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Summon(LAZION), Attack(LAZION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", LAZION), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH) + s.count(0, "grave", DARK_HOLE), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Lazion: "Once per turn, if your opponent draws a card: Inflict 1000 damage to your opponent." Team 0's Lazion; team 1
/// draws for its turn.
#[test]
fn lazion_burns_1000_when_the_opponent_draws() {
    let spawns = [sp(0, Hand, LAZION)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[Summon(LAZION), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7000), "{}", s.context());
}

/// Lazion: "Cannot be destroyed by battle or card effects. You take no battle damage from attacks involving this card."
/// Team 1 attacks Lazion with a Warwolf: Lazion stays and team 0 loses nothing.
#[test]
fn lazion_takes_no_battle_damage_and_is_not_destroyed() {
    let spawns = [sp(0, Hand, LAZION), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[Summon(LAZION), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", LAZION), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// The anime Lazion: "At the end of the Damage Step, if this card battled: Shuffle all monsters your opponent controls and
/// cards in their Graveyard into the Deck."
#[test]
fn lazion_anime_shuffles_monsters_and_the_graveyard() {
    let spawns = [sp(0, Monster, WARWOLF), sp(0, Grave, KURIBOH), sp(1, Grave, LAZION), sp(1, Hand, MONSTER_REBORN)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, LAZION)]);
    s.run_both([&[EndTurn], &[Activate(MONSTER_REBORN), Attack(LAZION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "grave", KURIBOH)), (0, 0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// The Kuriboh brothers

/// Kuribabylon: "Gains 300 ATK/DEF for each "Kuriboh" monster you control and in your GY." Kuribabylon, with the four
/// brothers on the field (always treated as "Kuriboh") and Kuriboh in the GY: 1500 + 300 x 5.
#[test]
fn kuribabylon_gains_300_for_each_kuriboh() {
    let spawns = [sp(0, Monster, KURIBABYLON), sp(0, Monster, KURIBAH), sp(0, Monster, KURIBEE), sp(0, Monster, KURIBOO),
        sp(0, Monster, KURIBEH), sp(0, Grave, KURIBOH)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    assert_eq!(s.attack_of(0, KURIBABYLON), Some(1500 + 300 * 5), "{}", s.context());
    s.run(&[]).unwrap();
    clean(&s);
}

/// Kuribabylon: "If you have more monsters in your GY than your opponent: You can Special Summon this card from your hand."
#[test]
fn kuribabylon_special_summons_with_more_monsters_in_the_graveyard() {
    let spawns = [sp(0, Hand, KURIBABYLON), sp(0, Grave, KURIBOH)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(KURIBABYLON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBABYLON), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, KURIBABYLON), Some(1800), "{}", s.context());
}

/// ...and not without it: with equal monsters in both graveyards it is not on offer.
#[test]
fn kuribabylon_needs_more_monsters_in_the_graveyard() {
    let spawns = [sp(0, Hand, KURIBABYLON)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!offers(&s, "activates", KURIBABYLON), "{}", s.context());
}

/// Kuribabylon: "During your Main or Battle Phase: You can return this card to the hand, and if you do, Special Summon 5
/// monsters (1 "Kuribah", 1 "Kuribee", 1 "Kuriboo", 1 "Kuribeh", and 1 "Kuriboh") from your hand and/or GY in Attack
/// Position." Two from the hand, three from the GY.
#[test]
fn kuribabylon_returns_to_the_hand_and_summons_the_five() {
    let spawns = [sp(0, Monster, KURIBABYLON), sp(0, Hand, KURIBAH), sp(0, Hand, KURIBEE), sp(0, Grave, KURIBOO),
        sp(0, Grave, KURIBEH), sp(0, Grave, KURIBOH)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(KURIBABYLON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", KURIBABYLON), 1, "{}", s.context());
    for cid in [KURIBAH, KURIBEE, KURIBOO, KURIBEH, KURIBOH] {
        assert_eq!((s.count(0, "mzone", cid), s.position_of(0, cid)), (1, Some(1)), "{cid}: {}", s.context());
    }
}

/// Kuribah: "When this card, or another "Kuriboh" monster, you control is destroyed by battle: You can Special Summon 1
/// monster with 300 ATK/200 DEF from your Deck, except "Kuribah"." A Warwolf destroys Kuribah; Kuribee (300/200) comes
/// from the Deck.
#[test]
fn kuribah_destroyed_in_battle_summons_a_300_200_monster() {
    let spawns = [sp(0, Monster, KURIBAH), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[KURIBEE], &[], &spawns) else { return };
    s.prefer(&[(0, KURIBEE)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KURIBAH), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", KURIBEE), 1, "{}", s.context());
}

/// ...and for another "Kuriboh" monster: Kuriboh is destroyed, Kuribah's effect summons from the Deck.
#[test]
fn kuribah_answers_another_kuriboh_destroyed_in_battle() {
    let spawns = [sp(0, Monster, KURIBAH), sp(0, MonsterDefense, KURIBOH), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[KURIBEE], &[], &spawns) else { return };
    s.prefer(&[(0, KURIBEE), (1, WARWOLF)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBEE), 1, "{}", s.context());
}

/// Kuribah: "You can Tribute this card you control and 1 each of "Kuribee", "Kuriboo", "Kuribeh" and "Kuriboh" from your
/// hand and/or field; Special Summon 1 "Kuribabylon" from your hand, Deck, or GY." Kuribabylon comes from the Deck with
/// the five brothers in the GY (one Kuriboh from the hand or the field).
#[test]
fn kuribah_tributes_the_brothers_for_kuribabylon() {
    let spawns = [sp(0, Monster, KURIBAH), sp(0, Monster, KURIBEE), sp(0, Hand, KURIBOO), sp(0, Hand, KURIBEH), sp(0, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, &[KURIBABYLON], &[], &spawns) else { return };
    s.run(&[Activate(KURIBAH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBABYLON), 1, "{}", s.context());
    for cid in [KURIBAH, KURIBEE, KURIBOO, KURIBEH, KURIBOH] {
        assert_eq!(s.count(0, "grave", cid), 1, "{cid}: {}", s.context());
    }
}

/// Kuribee: "When this card, or another "Kuriboh" monster, you control is destroyed by battle: You can add 1 Spell/Trap
/// from your Deck to your hand that lists the card "Kuriboh" in its text." The Flute of Summoning Kuriboh.
#[test]
fn kuribee_destroyed_in_battle_adds_a_kuriboh_spell() {
    let spawns = [sp(0, Monster, KURIBEE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[FLUTE_OF_KURIBOH], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KURIBEE), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", FLUTE_OF_KURIBOH), 1, "{}", s.context());
}

/// Kuribee: "Once per turn, when an opponent's monster declares an attack while you control another "Kuriboh" monster: You
/// can change the ATK of all other monsters you control to 0 until the end of this turn, and if you do, negate the attack."
#[test]
fn kuribee_negates_an_attack_and_zeroes_the_others() {
    let spawns = [sp(0, Monster, KURIBEE), sp(0, Monster, KURIBOH), sp(0, Monster, WARWOLF), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(KURIBEE)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, KURIBOH), s.attack_of(0, KURIBEE)), (Some(0), Some(0), Some(300)), "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF) + s.count(0, "mzone", KURIBOH), 2, "{}", s.context());
}

/// Kuribeh: "(Quick Effect): You can discard this card, then target 1 "Kuriboh" monster you control; it gains 1500 ATK."
#[test]
fn kuribeh_discards_to_give_a_kuriboh_1500_attack() {
    let spawns = [sp(0, Hand, KURIBEH), sp(0, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(KURIBEH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KURIBEH), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, KURIBOH), Some(1800), "{}", s.context());
}

/// Kuriboo: "When an opponent's monster declares an attack: You can discard this card; add 1 "Kuriboh" monster from your
/// Deck to your hand, except "Kuriboo"." The Deck holds Kuribee and Kuriboh: Kuribee is taken.
#[test]
fn kuriboo_discards_on_an_attack_to_add_a_kuriboh() {
    let spawns = [sp(0, Hand, KURIBOO), sp(0, Monster, WARWOLF), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[KURIBEE], &[], &spawns) else { return };
    s.prefer(&[(0, KURIBEE), (0, KURIBOH)]);
    s.run_both([&[EndTurn, Chain(KURIBOO)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KURIBOO), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", KURIBEE), 1, "{}", s.context());
}

/// Kuriboo: "Once per turn (Quick Effect): You can discard 1 Trap, then target 1 face-up monster your opponent controls; it
/// loses 1500 ATK until the end of this turn."
#[test]
fn kuriboo_discards_a_trap_to_weaken_a_monster() {
    let spawns = [sp(0, Monster, KURIBOO), sp(0, Hand, MIRROR_FORCE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(KURIBOO)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", MIRROR_FORCE), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(500), "{}", s.context());
}

/// The anime Kuribah: "You can banish 1 "Kuribeh", "Kuribee", "Kuriboh", "Kuriboo" and this card you control; Special
/// Summon 1 "Kuribabylon" from your hand, Deck, or Graveyard."
#[test]
fn kuribah_anime_banishes_the_brothers_for_kuribabylon() {
    let spawns = [sp(0, Monster, KURIBAH), sp(0, Monster, KURIBEE), sp(0, Monster, KURIBOO), sp(0, Monster, KURIBEH),
        sp(0, Monster, KURIBOH)];
    let Some(mut s) = field(true, 8000, &[KURIBABYLON], &[], &spawns) else { return };
    s.run(&[Activate(KURIBAH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBABYLON), 1, "{}", s.context());
    for cid in [KURIBAH, KURIBEE, KURIBOO, KURIBEH, KURIBOH] {
        assert_eq!(s.count(0, "removed", cid), 1, "{cid}: {}", s.context());
    }
}

/// The anime Kuriboo: "You can discard 1 Trap, then target 1 monster your opponent controls; It loses 1500 ATK until the
/// End Phase."
#[test]
fn kuriboo_anime_discards_a_trap_to_weaken_a_monster() {
    let spawns = [sp(0, Monster, KURIBOO), sp(0, Hand, MIRROR_FORCE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(KURIBOO)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, WARWOLF), Some(500), "{}", s.context());
}

/// The anime Kuribee: "When a "Kuribah", "Kuribeh", "Kuribee", "Kuriboh" or "Kuriboo" you control is targeted for an
/// attack: You can negate that attack."
#[test]
fn kuribee_anime_negates_an_attack_on_a_brother() {
    let spawns = [sp(0, Monster, KURIBEE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(KURIBEE)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBEE), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Left Arm Offering

/// Left Arm Offering: "If you have 2 or more other cards in your hand: Banish your entire hand; add 1 Spell from your Deck to
/// your hand. You cannot Set Spells/Traps during the turn you activate this card." The 2 others are banished, and Dark Hole
/// comes from the Deck and cannot be Set this turn (it can still be activated: a monster is on the field).
#[test]
fn left_arm_offering_banishes_the_hand_and_adds_a_spell() {
    let spawns = [sp(0, Hand, LEFT_ARM_OFFERING), sp(0, Hand, WARWOLF), sp(0, Hand, KURIBOH), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[DARK_HOLE], &[], &spawns) else { return };
    s.run(&[Activate(LEFT_ARM_OFFERING)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", DARK_HOLE), hand(&s, 0)), (1, 1), "the whole hand went: {}", s.context());
    assert!(!offers(&s, "spell_sets", DARK_HOLE) && offers(&s, "activates", DARK_HOLE), "{}", s.context());
}

/// ...and it is on offer with a full hand.
#[test]
fn left_arm_offering_needs_two_other_cards() {
    // Every opening hand is 5 cards and a draw: the "2 or more other cards" can't be made to fail, so this checks only
    // that it is on offer with a full hand.
    let spawns = [sp(0, Hand, LEFT_ARM_OFFERING)];
    let Some(s) = field(false, 8000, &[DARK_HOLE], &[], &spawns) else { return };
    assert!(offers(&s, "activates", LEFT_ARM_OFFERING), "{}", s.context());
}

/// The anime Left Arm Offering (a Trap there): "Send all cards from your hand to the Graveyard; add 1 Spell from your Deck to
/// your hand." Set, then activated a turn later; the hand goes to the Graveyard, not banished.
#[test]
fn left_arm_offering_anime_sends_the_hand_to_the_graveyard() {
    let spawns = [sp(0, Hand, LEFT_ARM_OFFERING), sp(0, Hand, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[DARK_HOLE], &[], &spawns) else { return };
    s.run(&[SetSpell(LEFT_ARM_OFFERING), Activate(LEFT_ARM_OFFERING)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", DARK_HOLE), hand(&s, 0)), (1, 1), "the whole hand went: {}", s.context());
    assert_eq!(s.state()[0]["removed"].as_array().map_or(0, Vec::len), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Legend of Heart and the Legendary Knights

/// A Warrior (Junk Synchron) to Tribute, the three Legendary Dragon Spells and the three Knights in the hand.
fn legend_field(anime: bool, knights: &[u16], dragons: &[u16], extra: &[Spawn]) -> Option<Scenario> {
    let mut spawns = vec![sp(0, Hand, LEGEND_OF_HEART), sp(0, Monster, JUNK_SYNCHRON)];
    spawns.extend(knights.iter().map(|&c| sp(0, Hand, c)));
    spawns.extend(dragons.iter().map(|&c| sp(0, Hand, c)));
    spawns.extend_from_slice(extra);
    field(anime, 8000, &[], &[], &spawns)
}

/// Legend of Heart: "Pay 2000 LP and Tribute 1 Warrior monster; banish up to 3 "Legendary Dragon" Spells with different
/// names from your hand and/or GY, and if you do, Special Summon that many "Legendary Knight" monsters with different names
/// from your hand, Deck, and/or GY." One Knight for each Spell banished.
#[test]
fn legend_of_heart_summons_as_many_knights_as_spells_banished() {
    let Some(mut s) = legend_field(false, &[KNIGHT_TIMAEUS, KNIGHT_HERMOS, KNIGHT_CRITIAS], &[EYE_OF_TIMAEUS, CLAW_OF_HERMOS], &[])
    else {
        return;
    };
    s.prefer(&[(0, EYE_OF_TIMAEUS), (0, CLAW_OF_HERMOS)]);
    s.run(&[Activate(LEGEND_OF_HEART)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6000, "{}", s.context());
    assert_eq!(s.count(0, "grave", JUNK_SYNCHRON), 1, "{}", s.context());
    let banished = s.count(0, "removed", EYE_OF_TIMAEUS) + s.count(0, "removed", CLAW_OF_HERMOS);
    let knights = [KNIGHT_TIMAEUS, KNIGHT_HERMOS, KNIGHT_CRITIAS].iter().map(|&k| s.count(0, "mzone", k)).sum::<usize>();
    assert!(banished >= 1 && knights == banished, "{banished} Spells banished, {knights} Knights: {}", s.context());
}

/// Legend of Heart with all three Spells banishes them, all three Knights come, and each Knight's "When this card is Special
/// Summoned: You can target 1 face-up Spell/Trap on the field; banish that target": the opponent's three face-up Spells
/// go.
#[test]
fn the_three_knights_each_banish_a_spell_or_trap() {
    let others = [sp(1, Spell, DARK_HOLE), sp(1, Spell, MONSTER_REBORN), sp(1, Spell, SOUL_RELEASE)];
    let Some(mut s) = legend_field(
        false,
        &[KNIGHT_TIMAEUS, KNIGHT_HERMOS, KNIGHT_CRITIAS],
        &[EYE_OF_TIMAEUS, CLAW_OF_HERMOS, FANG_OF_CRITIAS],
        &others,
    ) else {
        return;
    };
    s.prefer(&[(0, EYE_OF_TIMAEUS), (0, CLAW_OF_HERMOS), (0, FANG_OF_CRITIAS)]);
    s.run(&[Activate(LEGEND_OF_HEART)]).unwrap();
    clean(&s);
    let knights = [KNIGHT_TIMAEUS, KNIGHT_HERMOS, KNIGHT_CRITIAS].iter().map(|&k| s.count(0, "mzone", k)).sum::<usize>();
    let banished: usize = [DARK_HOLE, MONSTER_REBORN, SOUL_RELEASE].iter().map(|&c| s.count(1, "removed", c)).sum();
    assert!(knights >= 1 && banished == knights, "{knights} Knights banished {banished} Spells: {}", s.context());
}

/// Legend of Heart needs a Warrior to Tribute: with only a Warwolf (a Beast-Warrior) it is on offer, with a non-Warrior not.
#[test]
fn legend_of_heart_needs_a_warrior_to_tribute() {
    let spawns = [sp(0, Hand, LEGEND_OF_HEART), sp(0, Monster, KURIBOH), sp(0, Hand, EYE_OF_TIMAEUS), sp(0, Hand, KNIGHT_TIMAEUS)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!offers(&s, "activates", LEGEND_OF_HEART), "{}", s.context());
}

/// The anime Legend of Heart: "Pay 1000 LP, Tribute 1 Warrior-Type monster, and banish 1 "The Eye of Timaeus", 1 "The Claw of
/// Hermos", and 1 "The Fang of Critias"...; Special Summon 1 of each Knight". 1000 LP, all three of each.
#[test]
fn legend_of_heart_anime_pays_1000_for_the_three_knights() {
    let Some(mut s) = legend_field(
        true,
        &[KNIGHT_TIMAEUS, KNIGHT_HERMOS, KNIGHT_CRITIAS],
        &[EYE_OF_TIMAEUS, CLAW_OF_HERMOS, FANG_OF_CRITIAS],
        &[],
    ) else {
        return;
    };
    s.run(&[Activate(LEGEND_OF_HEART)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "{}", s.context());
    for k in [KNIGHT_TIMAEUS, KNIGHT_HERMOS, KNIGHT_CRITIAS] {
        assert_eq!(s.count(0, "mzone", k), 1, "{k}: {}", s.context());
    }
}

/// Legendary Knight Timaeus: "When this card is targeted for an attack: You can target 1 Spell in your GY; Set that target."
/// Dark Hole in the GY is Set; Timaeus (2800) wins the battle.
#[test]
fn timaeus_sets_a_spell_when_attacked() {
    let Some(mut s) = legend_field(false, &[KNIGHT_TIMAEUS], &[EYE_OF_TIMAEUS], &[sp(0, Grave, DARK_HOLE), sp(1, Monster, WARWOLF)])
    else {
        return;
    };
    s.prefer(&[(0, DARK_HOLE)]);
    s.run_both([&[Activate(LEGEND_OF_HEART), EndTurn, Chain(KNIGHT_TIMAEUS)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", DARK_HOLE), s.count(0, "grave", DARK_HOLE)), (1, 0), "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Legendary Knight Critias: "When this card is targeted for an attack: You can target 1 Trap in your GY; Set that target.
/// It can be activated this turn."
#[test]
fn critias_sets_a_trap_when_attacked() {
    let Some(mut s) = legend_field(false, &[KNIGHT_CRITIAS], &[FANG_OF_CRITIAS], &[sp(0, Grave, DIMENSIONAL_PRISON), sp(1, Monster, WARWOLF)])
    else {
        return;
    };
    s.run_both([&[Activate(LEGEND_OF_HEART), EndTurn, Chain(KNIGHT_CRITIAS)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", DIMENSIONAL_PRISON), s.count(0, "grave", DIMENSIONAL_PRISON)), (1, 0), "{}", s.context());
}
