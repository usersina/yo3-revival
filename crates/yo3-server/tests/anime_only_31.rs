//! Anime-only cards, group 31 (rows 6244-6269 of engine/added/cards.tsv): the video game's versions of anime cards that
//! ProjectIgnis marks Video Game (Mr. Stein, Prince Ojin, Princess Rose, Sarina, Sartorius, Syrus Truesdale, Tania, Titan,
//! Trueman, Yubel, Yusuke Fujiwara, Zure and the Knight of Dark World). Every duel plays with anime cards on
//! (`Scenario::start_anime_field`); each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`). The texts are the anime ones. Every Trap and every Quick-Play Spell answering the
//! opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! A card that asks for an Attribute (Attribute Bomb, Attribute Mastery) gets the scenario's default answer, the lowest
//! Attribute on offer: EARTH. Gene-Warped Warwolf is the EARTH monster, Feral Imp (DARK) the one that is not.
//! Covered: the flagged cards of the per-card fuzz (Satellite Laser Balsam, The Material Lord, The Sky Lord, Kiteroid (VG),
//! Ritual of the Matador, Volcanic Wall, Attribute Bomb), Zero Sprite (its script note), the Sartorius line (Arcana Force VIII -
//! The Strength, Arcana Force XII - The Hangman (VG), Suit of Sword X, The Material/Spiritual/Sky Lords and the Light Ruler
//! they bring), the Scab Scar Knight (the anime's, with Scab Counters) with Scab Blast's damage and Victim Barrier's counter
//! clause (batch 17's cards), and the seeded sample (Amphibian Angel - Frog-Hael, Attribute Mastery, Scab Scar Knight, The
//! Spiritual Lord, The Unselected One), with Amazoness Arena, Power Zone and Flame Wall besides. Not tested: Dark Arena,
//! Dark Archetype, Dark Psycho Eye, Attribute Chameleon and Attribute Gravity (not flagged, not in the sample).
//! Two overrides came out of it: `engine/overrides/c100000197.lua` (Attribute Bomb: a Set Bomb destroyed has no equip
//! target, `ec is nil`) and `engine/overrides/c511000010.lua` (Ritual of the Matador only knew the anime's Archfiend
//! Matador, a card the game does not have: the game's Matador Archfiend is the one it brings now).
//! Not coverable: the Load of Darkness Token. Infinite Fiend Mirror makes it only while its turn player controls the monster
//! "Load of Darkness" (100000081), which the game does not have and no row of any batch adds, so the Token can never be made.
//! The harness cannot put the Token on the field either (a spawned card is an ordinary card to the engine, not a Token);
//! `infinite_fiend_mirror_makes_no_token_without_a_load_of_darkness` plays the Mirror. Arcana Force XII - The Hangman (VG)'s override is batch 22's.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, EARTH, no effect
const THUNDER_KID: u16 = 4217; // Level 2 WIND Thunder, 700/600 vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, DARK, vanilla
const DARK_HOLE: u16 = 4342;
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const POLYMERIZATION: u16 = 4837;
const INPACHI: u16 = 5876; // Blazing Inpachi: a Pyro (1850/0)
const COMPULSORY_EVACUATION: u16 = 5914; // a Trap: return 1 monster on the field to the hand
const OFFERINGS_TO_THE_DOOMED: u16 = 5214; // destroy 1 face-up monster on the field, skip your next Draw Phase
const GAIA: u16 = 4044; // Gaia The Fierce Knight, EARTH 2300/2100
const TREEBORN_FROG: u16 = 6603; // a "Frog"
const DES_FROG: u16 = 6424; // a "Frog"
const POISON_DRAW_FROG: u16 = 6426; // a "Frog"
const MATADOR: u16 = 5080; // Matador Archfiend
const RITUAL_OF_THE_MATADOR_VG: u16 = 16303; // the anime's Ritual of the Matador (the game's own printed one is 5081)

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

/// A duel whose Decks are these cards, forty of each.
fn field_decks(decks: [u16; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[decks[0]; 40][..], &[][..]), (&[decks[1]; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

/// A run that must stop because the action never became possible.
fn never(result: Result<(), String>) {
    let err = result.expect_err("the action should never become possible");
    assert!(err.contains("never became possible") || err.contains("can't attack"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// The Scab Scar Knight (the anime's, with Scab Counters), Scab Blast and Victim Barrier
// ---------------------------------------------------------------------------------------------------------------

/// Scab Scar Knight: "Monsters your opponent controls must attack this card, if able. This Attack Position card cannot be
/// destroyed by battle. At the end of the Damage Step, if this card battled a monster: Place 1 Scab Counter on that monster.
/// At the end of the Battle Phase: Take control of all monsters your opponent controls with a Scab Counter(s)." Scab Blast:
/// "Inflict 200 damage to your opponent for each Scab Counter on the field." Team 1's Warwolf (2000) has to attack the Knight
/// (0 ATK) in turn 2: I take 2000, the Knight stays, and at the end of that Battle Phase Warwolf is mine. In turn 3 Scab
/// Blast finds one counter, on my new Warwolf: 200 damage.
#[test]
fn scab_scar_knight_counters_a_monster_that_attacks_it_and_takes_it_at_the_end_of_the_battle_phase() {
    let [knight, blast] = ["Scab Scar Knight (Anime)", "Scab Blast"].map(card);
    let Some(mut s) = field(&[], &[mon(0, knight), hand(0, blast), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    // Without a counter on the field Scab Blast cannot be played: turn 1 passes, turn 2 has the attack.
    s.run_both([&[Activate(blast)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", knight), 1, "the Knight in Attack Position is not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "the Knight has 0 ATK: {}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", WARWOLF)), (1, 0), "Warwolf changed sides: {}", s.context());
    assert_eq!(s.count(0, "grave", blast), 1, "Scab Blast was played in turn 3: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 200, "one Scab Counter, 200 damage: {}", s.context());
}

/// Scab Scar Knight: "This Attack Position card cannot be destroyed by battle." In Defense Position (0 DEF) it can: after
/// I switch it in turn 1 and Warwolf attacks it, it is in the Graveyard and nothing is taken.
#[test]
fn scab_scar_knight_in_defense_position_is_destroyed_by_battle() {
    let knight = card("Scab Scar Knight (Anime)");
    let Some(mut s) = field(&[], &[mon(0, knight), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Reposition(knight)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", knight), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "no counter, so Warwolf stays: {}", s.context());
    assert_eq!(s.lp(0), 8000, "a Defense Position monster gives no damage: {}", s.context());
}

/// Scab Scar Knight: "When a card or effect is activated that would increase LP: Negate the effect." My own Dian Keto the Cure
/// Master (gain 1000) with the Knight on my field: 8000 (the control: 9000).
#[test]
fn scab_scar_knight_negates_an_effect_that_increases_life_points() {
    let knight = card("Scab Scar Knight (Anime)");
    let Some(mut s) = field(&[], &[mon(0, knight), hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the recovery was negated: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    assert_eq!(s.lp(0), 9000, "{}", s.context());
}

/// Victim Barrier: "If an opponent's monster declares an attack: Change the attack target to another monster you control.
/// If the new target has a Scab Counter, the attacker gets a Scab Counter at the end of the Damage Step." The Knight makes
/// every attack aim at itself and the core then lets no attack be redirected (`Duel.ChangeAttackTarget` fails while a "must
/// attack this monster" effect is on the field), so the Knight has to be off the field. Team 1's Warwolf attacks it in turn 2
/// (2000 damage to me), gets a counter and is taken at the end of the Battle Phase; its Gaia The Fierce Knight sits in Defense
/// Position, unable to attack. In turn 3 Offerings to the Doomed destroys the Knight. In turn 4 Gaia (2300) is switched to
/// Attack Position and attacks my Feral Imp, Victim Barrier changes the target to Warwolf (with its counter), which Gaia
/// destroys (300 damage to me), and Gaia takes a counter from the clause alone. In turn 5 Scab Blast sees that one counter, on
/// Gaia: 200 damage.
#[test]
fn victim_barrier_puts_a_scab_counter_on_the_attacker_of_a_monster_that_has_one() {
    let [knight, barrier, blast] = ["Scab Scar Knight (Anime)", "Victim Barrier", "Scab Blast"].map(card);
    let spawns = [mon(0, knight), set(0, barrier), hand(0, OFFERINGS_TO_THE_DOOMED), hand(0, blast), mon(0, FILLER), mon(1, WARWOLF), mon_def(1, GAIA)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, knight), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(OFFERINGS_TO_THE_DOOMED), Chain(barrier), Activate(blast)], &[Attack(WARWOLF), EndTurn, Reposition(GAIA), Attack(GAIA)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", knight), 1, "the Knight was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", barrier), 1, "Victim Barrier was activated: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF) + s.count(0, "grave", WARWOLF), 1, "Warwolf took the attack and was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", GAIA), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000 - 300, "2000 from the Knight, 300 from Gaia against Warwolf: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 200, "Scab Blast: one counter, on Gaia: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Satellite Laser Balsam
// ---------------------------------------------------------------------------------------------------------------

/// Satellite Laser Balsam: "3 "Satellite Cannon" Fusion Material. Once per turn, during your End Phase: This card gains 3000
/// ATK. After damage calculation, if this card attacks, the ATK gained by this effect returns to 0. If this card attacks a
/// Defense Position monster, inflict piercing battle damage." Fusion Summoned in turn 1 with Polymerization (0 ATK, 2000 DEF), it
/// has 3000 ATK from the End Phase on; in turn 3 it attacks a Feral Imp in Defense Position (1400 DEF): the Imp is destroyed
/// and 1600 goes through, and its ATK is back to 0.
#[test]
fn satellite_laser_balsam_gains_3000_atk_at_the_end_phase_pierces_and_loses_it_after_attacking() {
    let [balsam, cannon] = ["Satellite Laser Balsam", "Satellite Cannon"].map(card);
    let spawns = [hand(0, POLYMERIZATION), mon(0, cannon), mon(0, cannon), mon(0, cannon), mon_def(1, FILLER)];
    let Some(mut s) = field_extra([&[balsam], &[]], &spawns) else { return };
    s.run(&[Activate(POLYMERIZATION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", balsam), 1, "Fusion Summoned: {}", s.context());
    assert_eq!(s.count(0, "grave", cannon), 3, "the three Cannons were the materials: {}", s.context());
    let b = stats(&s, 0, balsam);
    assert_eq!((b.0, b.1, b.2), (Some(0), Some(2000), Some(8)), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, balsam), Some(3000), "+3000 in my End Phase, not in the opponent's: {}", s.context());
    s.run(&[Attack(balsam)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", FILLER), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - (3000 - 1400), "piercing damage: {}", s.context());
    assert_eq!(s.attack_of(0, balsam), Some(0), "the gained ATK returns to 0 after the attack: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Amphibian Angel - Frog-Hael
// ---------------------------------------------------------------------------------------------------------------

/// Amphibian Angel - Frog-Hael: "You can Special Summon this card (from your hand) by Tributing 2 monsters you control. When
/// you do: You can Special Summon as many "Frog" monsters as possible from your Graveyard." Feral Imp and Dark Blade are the
/// Tributes; the three Frogs of my Graveyard (Treeborn Frog, Des Frog, Poison Draw Frog) come to the field, and the Tributes
/// go to the Graveyard.
#[test]
fn frog_hael_special_summons_every_frog_of_the_graveyard_when_summoned_by_two_tributes() {
    let hael = card("Amphibian Angel - Frog-Hael");
    let spawns = [hand(0, hael), mon(0, FILLER), mon(0, DARK_BLADE), grave(0, TREEBORN_FROG), grave(0, DES_FROG), grave(0, POISON_DRAW_FROG)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, FILLER), (0, DARK_BLADE)]);
    s.prefer_most(true);
    s.run(&[SpSummon(hael)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", hael), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", FILLER), s.count(0, "grave", DARK_BLADE)), (1, 1), "the two Tributes: {}", s.context());
    for frog in [TREEBORN_FROG, DES_FROG, POISON_DRAW_FROG] {
        assert_eq!(s.count(0, "mzone", frog), 1, "Frog {frog} came back: {}", s.context());
    }
    let h = stats(&s, 0, hael);
    assert_eq!((h.0, h.1, h.2), (Some(1400), Some(800), Some(5)), "{}", s.context());
}

/// Amphibian Angel - Frog-Hael: "While you control another card, your opponent cannot target this card for attacks." With a
/// Set Spell beside it Warwolf (turn 2) has no attack at all (no direct attack either, Frog-Hael is a monster); alone, Frog-Hael
/// is attacked and destroyed (2000 against 1400, 600 damage).
#[test]
fn frog_hael_cannot_be_attacked_while_its_controller_has_another_card() {
    let hael = card("Amphibian Angel - Frog-Hael");
    let Some(mut s) = field(&[], &[mon(0, hael), set(0, DARK_HOLE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[], &[Attack(WARWOLF)]]));
    assert_eq!(s.count(0, "mzone", hael), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, hael), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", hael), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 600, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Load of Darkness Token and Infinite Fiend Mirror
// ---------------------------------------------------------------------------------------------------------------

/// Infinite Fiend Mirror and its Load of Darkness Token. The Mirror makes Tokens (Fiend/DARK/Level 10, 3000/1000) at the End
/// Phase only while the turn player controls "Load of Darkness", which the game has no card of (see the header), so the Token
/// cannot be made: the Mirror is activated from the hand (a Field Spell) and nothing happens at the End Phases. (The harness
/// cannot put the Token itself on the field: a spawned card is an ordinary card to the engine, not a Token.)
#[test]
fn infinite_fiend_mirror_makes_no_token_without_a_load_of_darkness() {
    let [mirror, token] = ["Infinite Fiend Mirror", "Load of Darkness Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, mirror)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(mirror), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", mirror) + s.count(0, "fzone", mirror), 1, "the Mirror is on the field: {}", s.context());
    assert_eq!((s.count(0, "mzone", token), s.count(1, "mzone", token)), (0, 0), "no Load of Darkness, no Token: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Arcana Force VIII - The Strength, Arcana Force XII - The Hangman (VG) and Suit of Sword X
// ---------------------------------------------------------------------------------------------------------------

/// Arcana Force VIII - The Strength: "When this card is Summoned: Toss a coin and gain the appropriate effect. Heads: Take
/// control of 1 monster your opponent controls. Tails: Your opponent takes control of 1 monster you control, except this
/// card." Tributing Feral Imp for it, with Dark Blade left beside it and Warwolf on the other side: in some seeds Warwolf comes
/// to me (heads), in others Dark Blade goes to the opponent (tails).
#[test]
fn arcana_force_the_strength_takes_a_monster_on_heads_and_gives_one_on_tails() {
    let strength = card("Arcana Force VIII - The Strength");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=10 {
        let spawns = [hand(0, strength), mon(0, FILLER), mon(0, DARK_BLADE), mon(1, WARWOLF)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        s.prefer(&[(0, FILLER), (0, DARK_BLADE), (1, WARWOLF)]);
        s.run(&[Summon(strength)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "mzone", strength), 1, "seed {seed}: {}", s.context());
        assert_eq!(s.count(0, "grave", FILLER), 1, "seed {seed}: the Tribute: {}", s.context());
        if s.count(0, "mzone", WARWOLF) == 1 {
            assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(1, "mzone", WARWOLF)), (1, 0), "seed {seed}: heads: {}", s.context());
            heads += 1;
        } else {
            assert_eq!((s.count(1, "mzone", DARK_BLADE), s.count(0, "mzone", DARK_BLADE), s.count(1, "mzone", WARWOLF)), (1, 0, 1), "seed {seed}: tails: {}", s.context());
            tails += 1;
        }
    }
    assert!(heads > 0 && tails > 0, "both coin faces in 10 seeds: heads {heads}, tails {tails}");
}

/// Arcana Force XII - The Hangman (VG) (engine/overrides/c100000106.lua, batch 22's): "When this card is Summoned: Toss a coin.
/// Heads: Once per turn, during your End Phase: Target 1 face-up monster you control; destroy that target, and if you do, take
/// damage equal to its ATK on the field. Tails: ... Target 1 face-up monster your opponent controls; destroy that target, and
/// if you do, inflict damage to your opponent equal to its ATK on the field." Summoned in turn 1 for a Feral Imp Tribute, with
/// Dark Blade (1800) and Warwolf (2000) beside it: heads destroys my Dark Blade and I take 1800, tails destroys Warwolf and
/// the opponent takes 2000.
#[test]
fn arcana_force_the_hangman_vg_destroys_a_monster_at_the_end_phase_and_deals_its_atk() {
    let hangman = card("Arcana Force XII - The Hangman (VG)");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=10 {
        let spawns = [hand(0, hangman), mon(0, FILLER), mon(0, DARK_BLADE), mon(1, WARWOLF)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        s.prefer(&[(0, FILLER), (0, DARK_BLADE), (1, WARWOLF)]);
        s.run_both([&[Summon(hangman), EndTurn], &[EndTurn]]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "mzone", hangman), 1, "seed {seed}: {}", s.context());
        if s.count(0, "grave", DARK_BLADE) == 1 {
            assert_eq!((s.lp(0), s.lp(1), s.count(1, "mzone", WARWOLF)), (8000 - 1800, 8000, 1), "seed {seed}: heads: {}", s.context());
            heads += 1;
        } else {
            assert_eq!((s.lp(0), s.lp(1), s.count(1, "grave", WARWOLF)), (8000, 8000 - 2000, 1), "seed {seed}: tails: {}", s.context());
            assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "seed {seed}: {}", s.context());
            tails += 1;
        }
    }
    assert!(heads > 0 && tails > 0, "both coin faces in 10 seeds: heads {heads}, tails {tails}");
}

/// Suit of Sword X: "Toss a coin. Heads: Destroy all monsters your opponent controls. Tails: Destroy all monsters you
/// control." Two monsters on each side: in some seeds the opponent's two go to its Graveyard, in others mine do.
#[test]
fn suit_of_sword_x_destroys_all_monsters_of_one_side_by_the_coin() {
    let sword = card("Suit of Sword X");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=10 {
        let spawns = [hand(0, sword), mon(0, FILLER), mon(0, DARK_BLADE), mon(1, WARWOLF), mon(1, FILLER)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        s.run(&[Activate(sword)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "grave", sword), 1, "seed {seed}: {}", s.context());
        let (mine, theirs) = (s.count(0, "mzone", FILLER) + s.count(0, "mzone", DARK_BLADE), s.count(1, "mzone", WARWOLF) + s.count(1, "mzone", FILLER));
        if theirs == 0 {
            assert_eq!((mine, s.count(1, "grave", WARWOLF)), (2, 1), "seed {seed}: heads: {}", s.context());
            heads += 1;
        } else {
            assert_eq!((mine, theirs, s.count(0, "grave", DARK_BLADE)), (0, 2, 1), "seed {seed}: tails: {}", s.context());
            tails += 1;
        }
    }
    assert!(heads > 0 && tails > 0, "both coin faces in 10 seeds: heads {heads}, tails {tails}");
}


// ---------------------------------------------------------------------------------------------------------------
// The Material Lord, The Spiritual Lord, The Sky Lord and the Light Ruler
// ---------------------------------------------------------------------------------------------------------------

/// The Material Lord: "Activate this card only if you have a Level 4 or lower "Arcana Force" monster in your Graveyard. You can
/// send 1 Level 5 or 6 "Arcana Force" monster from your hand to the Graveyard; add 1 "The Spiritual Lord" from your Deck to your
/// hand." A Continuous Trap Set in turn 1 and activated in turn 3 (the card, and its effect with it), with Arcana Force I - The
/// Magician (Level 4) in my Graveyard: Arcana Force XIV - Temperance (Level 6) is sent from my hand and The Spiritual Lord
/// comes from the bottom of my Deck. Without the Magician in the Graveyard the card cannot be activated at all.
#[test]
fn the_material_lord_sends_a_level_5_or_6_arcana_force_to_add_the_spiritual_lord() {
    let [material, spiritual, magician, temperance] =
        ["The Material Lord", "The Spiritual Lord", "Arcana Force I - The Magician", "Arcana Force XIV - Temperance"].map(card);
    let spawns = [set(0, material), grave(0, magician), hand(0, temperance)];
    let Some(mut s) = field(&[spiritual], &spawns) else { return };
    s.prefer(&[(0, FILLER), (0, temperance)]); // the hand-size discard of turn 1 takes Imps, not the cost
    s.run_both([&[EndTurn, Activate(material)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", material), 1, "face-up on the field: {}", s.context());
    assert_eq!(s.count(0, "grave", temperance), 1, "the cost: {}", s.context());
    assert_eq!(s.count(0, "hand", spiritual), 1, "The Spiritual Lord was added: {}", s.context());
    // No Level 4 or lower Arcana Force in the Graveyard: no activation.
    let Some(mut s) = field(&[spiritual], &[set(0, material), hand(0, temperance)]) else { return };
    never(s.run_both([&[EndTurn, Activate(material)], &[EndTurn]]));
}

/// The Spiritual Lord: "Activate this card only if you have a Level 5 or 6 "Arcana Force" monster in your Graveyard. You can send
/// 1 Level 7 or higher "Arcana Force" monster from your hand to the Graveyard; add 1 "The Sky Lord" from your Deck to your hand."
/// With Temperance (Level 6) in the Graveyard, Arcana Force XVIII - The Moon (Level 7) is the cost and The Sky Lord is added.
#[test]
fn the_spiritual_lord_sends_a_level_7_arcana_force_to_add_the_sky_lord() {
    let [spiritual, sky, temperance, moon] =
        ["The Spiritual Lord", "The Sky Lord", "Arcana Force XIV - Temperance", "Arcana Force XVIII - The Moon"].map(card);
    let spawns = [set(0, spiritual), grave(0, temperance), hand(0, moon)];
    let Some(mut s) = field(&[sky], &spawns) else { return };
    s.prefer(&[(0, FILLER), (0, moon)]);
    s.run_both([&[EndTurn, Activate(spiritual)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", spiritual), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", moon), 1, "the cost: {}", s.context());
    assert_eq!(s.count(0, "hand", sky), 1, "The Sky Lord was added: {}", s.context());
    // A Level 5 or 6 Arcana Force in the hand is no cost for it: the card is activated, its effect has nothing to send.
    let Some(mut s) = field(&[sky], &[set(0, spiritual), grave(0, temperance), hand(0, temperance)]) else { return };
    s.run_both([&[EndTurn, Activate(spiritual)], &[EndTurn]]).unwrap();
    assert_eq!((s.count(0, "szone", spiritual), s.count(0, "hand", sky)), (1, 0), "activated, with nothing to send: {}", s.context());
}

/// The Sky Lord: "Activate this card only if you have a Level 7 or higher "Arcana Force" monster in your Graveyard. You can send
/// this face-up card, 1 "The Material Lord", and 1 "The Spiritual Lord" from your field to the Graveyard; Special Summon 1
/// "Arcana Force EX - The Light Ruler" from your hand, Deck, or Graveyard, ignoring its Summoning conditions." The Moon (Level 7)
/// in my Graveyard, the two Lords face-up and the Light Ruler in my hand: after both activations the three Spells and Traps are in
/// the Graveyard and the Light Ruler is on my field (the fuzz had no Light Ruler to bring). Without a Level 7 or higher in the
/// Graveyard the Spell cannot be activated.
#[test]
fn the_sky_lord_sends_itself_and_both_lords_to_bring_the_light_ruler() {
    let [sky, material, spiritual, moon, ruler] =
        ["The Sky Lord", "The Material Lord", "The Spiritual Lord", "Arcana Force XVIII - The Moon", "Arcana Force EX - The Light Ruler"].map(card);
    let spawns = [hand(0, sky), grave(0, moon), spell(0, material), spell(0, spiritual), hand(0, ruler)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(sky), Activate(sky)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ruler), 1, "the Light Ruler is on my field: {}", s.context());
    assert_eq!((s.count(0, "grave", sky), s.count(0, "grave", material), s.count(0, "grave", spiritual)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "szone", sky) + s.count(0, "szone", material) + s.count(0, "szone", spiritual), 0, "{}", s.context());
    assert_eq!(s.attack_of(0, ruler), Some(4000), "{}", s.context());
    // The Light Ruler from the Deck works too (its bottom card).
    let Some(mut s) = field(&[ruler], &[hand(0, sky), grave(0, moon), spell(0, material), spell(0, spiritual)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(sky), Activate(sky)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ruler), 1, "from the Deck: {}", s.context());
    // No Level 7 or higher Arcana Force in the Graveyard: no activation.
    let Some(mut s) = field(&[], &[hand(0, sky), grave(0, card("Arcana Force XIV - Temperance")), hand(0, ruler)]) else { return };
    never(s.run(&[Activate(sky)]));
}

// ---------------------------------------------------------------------------------------------------------------
// Kiteroid (VG)
// ---------------------------------------------------------------------------------------------------------------

/// Kiteroid (VG): "During damage calculation, if your opponent's monster attacks directly (Quick Effect): You can discard this
/// card; you take no battle damage from that battle. ... (Quick Effect): You can banish this card from your Graveyard; you take no
/// battle damage from that battle." Team 1's Warwolf (2000) attacks directly in turn 2: with Kiteroid in my hand it is discarded
/// and I keep 8000 LP; with Kiteroid in my Graveyard it is banished and I keep 8000; the control takes 2000.
#[test]
fn kiteroid_vg_stops_the_battle_damage_of_a_direct_attack_from_the_hand_or_the_graveyard() {
    let kiteroid = card("Kiteroid (VG)");
    let Some(mut s) = field(&[], &[hand(0, kiteroid), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(kiteroid)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", kiteroid), 1, "discarded: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    let Some(mut s) = field(&[], &[grave(0, kiteroid), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(kiteroid)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", kiteroid), 1, "banished: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    let Some(mut s) = field(&[], &[mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Ritual of the Matador (engine/overrides/c511000010.lua)
// ---------------------------------------------------------------------------------------------------------------

/// Ritual of the Matador (the anime's, engine/overrides/c511000010.lua): "This card is used to Ritual Summon "Archfiend
/// Matador". You must also Tribute monsters from your hand or field whose total Levels equal 6 or more." The game has the card
/// as Matador Archfiend; upstream's Spell named only the anime's own card, so it never had a monster to bring. Warwolf (Level 4)
/// and Thunder Kid (Level 2) from my hand are the Tributes and Matador Archfiend comes.
#[test]
fn ritual_of_the_matador_brings_the_games_matador_archfiend() {
    let Some(mut s) = field(&[], &[hand(0, RITUAL_OF_THE_MATADOR_VG), hand(0, MATADOR), hand(0, WARWOLF), hand(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, WARWOLF), (0, THUNDER_KID)]);
    s.prefer_most(true);
    s.run(&[Activate(RITUAL_OF_THE_MATADOR_VG)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", MATADOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF) + s.count(0, "grave", THUNDER_KID), 2, "both Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", RITUAL_OF_THE_MATADOR_VG), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Volcanic Wall
// ---------------------------------------------------------------------------------------------------------------

/// Volcanic Wall: "Once per turn: You can send the top 3 cards of your Deck to the Graveyard, and if you do, inflict 500 damage
/// to your opponent for each Pyro-Type monster sent to the Graveyard this way. You cannot declare an attack the turn you use
/// this effect." The fuzz's Decks held no Pyro, so it never saw the damage. A Deck of Blazing Inpachi (Pyro): three of them go
/// to the Graveyard and the opponent takes 1500; with a Deck of Feral Imps (not Pyro) three cards go and no damage is done.
/// Used in turn 3 it stops the attack of that turn.
#[test]
fn volcanic_wall_mills_three_cards_and_deals_500_for_each_pyro() {
    let wall = card("Volcanic Wall");
    let Some(mut s) = field_decks([INPACHI, FILLER], &[hand(0, wall)]) else { return };
    let deck_before = s.state()[0]["deck"].as_u64().unwrap();
    s.run(&[Activate(wall), Activate(wall)]).unwrap();
    clean(&s);
    assert_eq!(s.state()[0]["deck"].as_u64().unwrap(), deck_before - 3, "three cards left the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", INPACHI), 3, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1500), "{}", s.context());
    let Some(mut s) = field_decks([FILLER, FILLER], &[hand(0, wall)]) else { return };
    let deck_before = s.state()[0]["deck"].as_u64().unwrap();
    s.run(&[Activate(wall), Activate(wall)]).unwrap();
    clean(&s);
    assert_eq!(s.state()[0]["deck"].as_u64().unwrap(), deck_before - 3, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "no Pyro, no damage: {}", s.context());
    // No attack in the turn it is used.
    let Some(mut s) = field_decks([INPACHI, FILLER], &[spell(0, wall), mon(0, WARWOLF)]) else { return };
    never(s.run(&[EndTurn, Activate(wall), Attack(WARWOLF)]));
    assert_eq!(s.lp(1), 8000 - 1500, "the effect was used: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Attribute Bomb (engine/overrides/c100000197.lua) and Attribute Mastery
// ---------------------------------------------------------------------------------------------------------------

/// Attribute Bomb: "Activate this card by declaring an Attribute. If the equipped monster is destroyed by battle with a monster
/// of that Attribute: Inflict 1000 damage to your opponent." EARTH is declared (the scenario's default answer) and equipped to
/// my Thunder Kid. Team 1's Warwolf (EARTH, 2000) destroys it in turn 2: I take 1300 and the opponent 1000. With Feral Imp (DARK,
/// 1300) as the attacker instead only the battle damage is done (600).
#[test]
fn attribute_bomb_deals_1000_when_the_equipped_monster_dies_to_a_monster_of_the_declared_attribute() {
    let bomb = card("Attribute Bomb");
    let Some(mut s) = field(&[], &[hand(0, bomb), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(bomb)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", THUNDER_KID), s.count(0, "grave", bomb)), (1, 1), "the Kid and the Bomb are in the Graveyard: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1300, 8000 - 1000), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, bomb), mon(0, THUNDER_KID), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(bomb)], &[Attack(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 600, 8000), "a DARK attacker: no damage from the Bomb: {}", s.context());
}

/// Attribute Bomb (engine/overrides/c100000197.lua): a Bomb Set and never equipped has no equip target when it leaves the field.
/// Upstream read `ec:GetReasonCard()` on nil and the core stopped the duel with a Lua error (batch 12's fuzz seed 63091:
/// Allegro Toile destroying it). Team 1's Allegro Toile is chained to its Normal Summon in turn 2 and destroys my Set Bomb:
/// the duel goes on, the Bomb is in the Graveyard and nobody takes damage.
#[test]
fn a_set_attribute_bomb_destroyed_without_an_equip_target_is_not_a_lua_error() {
    let [bomb, toile] = ["Attribute Bomb", "Allegro Toile"].map(card);
    let Some(mut s) = field(&[], &[set(0, bomb), set(1, toile)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Summon(FILLER), Chain(toile)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bomb), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", toile), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
}

/// Attribute Mastery: "Activate this card by declaring an Attribute. Before damage calculation, if the equipped monster battles a
/// monster of the declared Attribute: Destroy it." EARTH is declared and Mastery is equipped to my Thunder Kid (700). In turn 3
/// it attacks Warwolf (EARTH, 2000): Warwolf is destroyed before damage calculation and nobody takes damage. Against a Feral
/// Imp (DARK) the Kid is destroyed by the battle (600 damage to me), the Imp lives.
#[test]
fn attribute_mastery_destroys_a_monster_of_the_declared_attribute_the_equipped_monster_battles() {
    let mastery = card("Attribute Mastery");
    let Some(mut s) = field(&[], &[hand(0, mastery), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(mastery), EndTurn, Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "no damage calculation: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, mastery), mon(0, THUNDER_KID), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(mastery), EndTurn, Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", THUNDER_KID), s.count(1, "mzone", FILLER)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 600, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Zero Sprite
// ---------------------------------------------------------------------------------------------------------------

/// Zero Sprite: "Target 1 face-up monster you control: equip this card to that target. The equipped monster's original ATK
/// becomes 0, but it can attack twice during each Battle Phase." (A Trap in the database, Set in turn 1, activated in turn 3.)
/// Warwolf's ATK is 0 and it attacks twice in the same Battle Phase against an empty field (both attacks are on offer).
#[test]
fn zero_sprite_sets_the_original_atk_to_0_and_allows_two_attacks() {
    let sprite = card("Zero Sprite");
    let Some(mut s) = field(&[], &[set(0, sprite), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(sprite)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(0), "original ATK 0: {}", s.context());
    s.run(&[Attack(WARWOLF), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000, "0 ATK, no damage: {}", s.context());
}

/// Zero Sprite (the script note: its operation reads the target after the cost): the target leaves the field with the Trap on the
/// chain. My own Compulsory Evacuation Device returns Warwolf, the only target, to my hand in response; Zero Sprite
/// resolves with nothing to equip (no Lua error) and the duel goes on.
#[test]
fn zero_sprite_with_its_target_gone_does_nothing_and_is_not_a_lua_error() {
    let sprite = card("Zero Sprite");
    let spawns = [set(0, sprite), mon(0, WARWOLF), set(0, COMPULSORY_EVACUATION)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER), (0, WARWOLF), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(sprite), Chain(COMPULSORY_EVACUATION)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", WARWOLF), 1, "Warwolf went back to my hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Unselected One
// ---------------------------------------------------------------------------------------------------------------

/// The Unselected One: "While your opponent controls 2 or more monsters: Your opponent chooses 1 monster they control, and if
/// they do, destroy 1 other monster they control, then, Special Summon that monster from your opponent's Graveyard in Attack
/// Position, also, you cannot conduct your Battle Phase for the rest of this turn." Team 1 keeps Warwolf (its choice), I
/// destroy Dark Blade and it comes to my field in Attack Position; Feral Imp stays with team 1. In turn 3 the card is played and
/// no Battle Phase follows. Played with only one monster on the other side it is not offered.
#[test]
fn the_unselected_one_destroys_a_monster_the_opponent_did_not_choose_and_takes_it() {
    let unselected = card("The Unselected One");
    let spawns = [hand(0, unselected), mon(0, FILLER), mon(1, WARWOLF), mon(1, DARK_BLADE), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF), (1, DARK_BLADE)]);
    s.run(&[Activate(unselected)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade is mine now: {}", s.context());
    assert_eq!(s.position_of(0, DARK_BLADE), Some(1), "in Attack Position: {}", s.context());
    assert_eq!((s.count(1, "mzone", WARWOLF), s.count(1, "mzone", FILLER), s.count(1, "grave", DARK_BLADE)), (1, 1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", unselected), 1, "{}", s.context());
    // No Battle Phase for the rest of that turn.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, FILLER), (1, WARWOLF), (1, DARK_BLADE)]); // the hand-size discard of turn 1 takes an Imp
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["to_bp"], true, "turn 3 has a Battle Phase: {}", s.context());
    s.run(&[Activate(unselected)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase for the rest of the turn: {}", s.context());
    // One monster is not enough.
    let Some(mut s) = field(&[], &[hand(0, unselected), mon(1, WARWOLF)]) else { return };
    never(s.run(&[Activate(unselected)]));
}

// ---------------------------------------------------------------------------------------------------------------
// Amazoness Arena, Power Zone and Flame Wall
// ---------------------------------------------------------------------------------------------------------------

/// Amazoness Arena: "When this card is activated: Both players gain 600 LP. At the end of the Damage Step, when a player declares
/// an attack: That player can pay 100 LP; inflict 100 damage to their opponent." Activated in turn 1 (8600 each); in turn 3 my
/// Warwolf attacks team 1's Feral Imp (700 battle damage) and I pay 100 for 100 more: 8500 and 7800.
#[test]
fn amazoness_arena_gives_600_to_both_and_lets_the_attacker_pay_100_for_100_damage() {
    let arena = card("Amazoness Arena");
    let Some(mut s) = field(&[], &[hand(0, arena), mon(0, WARWOLF), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(arena)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8600, 8600), "{}", s.context());
    s.run_both([&[EndTurn, Attack(WARWOLF), Chain(arena)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", FILLER), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8600 - 100, 8600 - 700 - 100), "{}", s.context());
}

/// Power Zone: "If a monster is destroyed by battle: Its controller takes damage equal to the destroyed monster's original ATK."
/// Team 1's Feral Imp (1300) attacks my Warwolf (2000) in turn 2: its controller takes 700 of battle damage and 1300 from the Zone.
#[test]
fn power_zone_damages_the_controller_of_a_monster_destroyed_by_battle_for_its_original_atk() {
    let zone = card("Power Zone");
    let Some(mut s) = field(&[], &[hand(0, zone), mon(0, WARWOLF), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(zone)], &[Attack(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", FILLER), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 700 - 1300), "{}", s.context());
}

/// Flame Wall: "If there is a Pyro-Type monster in your Graveyard, you take no damage from your opponent's card effects." With
/// Blazing Inpachi in my Graveyard team 1's Hinotama (500) does nothing to me; without it I take 500.
#[test]
fn flame_wall_stops_effect_damage_while_a_pyro_is_in_my_graveyard() {
    let wall = card("Flame Wall");
    let hinotama = card("Hinotama");
    let Some(mut s) = field(&[], &[spell(0, wall), grave(0, INPACHI), hand(1, hinotama)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(hinotama)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", hinotama), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let Some(mut s) = field(&[], &[spell(0, wall), hand(1, hinotama)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(hinotama)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
}
