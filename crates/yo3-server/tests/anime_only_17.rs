//! Anime-only cards, group 17 (rows 5765-5796 of engine/added/cards.tsv): Mr. Stein's, T-Bone's, Zure's, the Knight of Dark
//! World's, Jesse Anderson's, Blair Flannigan's and Damon's cards. These cards exist only in the anime, so every duel plays
//! with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client
//! models checked after every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play
//! Spell answering the opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait
//! for turn 3.
//! Covered: the Homunculus Token (made by Blue Moon, one to each side, gone when destroyed), the cards the per-card fuzz
//! flagged (Scab Scream and the Scar-Knight it brings, Blue Moon and Shadow Moon, Dormant Volcano, Guidance to Ore, Power
//! Bombard, Ring of Fiendish Power, Amber Crystal Circle, GEM Burst, M Force, Prism Wall (both of its answers), Mystic
//! Revolution, Miracle Draw, Amazoness Call, Amazoness Trainer, Pride of Tribe), the signature cards (Explosion Fuse with
//! Power Bombard, Mystic Egg, Baby Dragon, Revolution and Mystic Dragon, Dowsing Point, Victim Barrier, Amazoness Charm)
//! and the seeded sample of the rest (Mystic Baby Dragon, Mystic Egg, Power Annihilator, Power Spirit).
//! Two overrides: Shadow Moon (`engine/overrides/c511000610.lua`: it looked for the anime's Demiurge Ema, passcode 511000609,
//! but the client's Ema plays by the official 26364381, so Shadow Moon could never be activated) and Mystic Revolution
//! (`c511009189.lua`: a Mystic Dragon that was never Summoned could not come from the Graveyard, and its Tribute filter read
//! nil globals). A Spell that destroys itself without a Fiend (Ring of Fiendish Power) is spawned after the monsters, or the
//! duel's first check destroys it before they are there.
//! Not coverable: Scab Blast's damage and Victim Barrier's Scab Counter. Scab Blast is played only while a Scab Counter is on
//! the field, and the only card that places one is the anime's Scab Scarknight (513000140), which the client does not have
//! (the "Scab Scar Knight" it has is the video game's, with no counters), so the fuzz never saw it activated and no
//! scenario can start with a counter. Its other effect (the Graveyard's draw replacement) is covered.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const GAIA_KNIGHT: u16 = 4044; // Gaia The Fierce Knight: Level 7 EARTH Warrior, 2300/2100 vanilla
const FLAME_CEREBRUS: u16 = 4535; // Level 6 FIRE Pyro, 2100/1800 vanilla
const KAIRYU_SHIN: u16 = 4079; // Level 5 WATER Sea Serpent, 1800/1500 vanilla
const ORION: u16 = 4132; // Orion the Battle King: Level 5 LIGHT Warrior, a vanilla needing one Tribute
const BLUE_EYES: u16 = 4007; // Level 8 LIGHT Dragon, 3000/2500 vanilla
const SUMMONED_SKULL: u16 = 4028; // Level 6 DARK Fiend, 2500/1200 (its effect is a lightning, not used here)
const GIANT_SOLDIER: u16 = 4080; // Giant Soldier of Stone: Level 3 EARTH Rock, 1300/2000 vanilla
const DARK_HOLE: u16 = 4342;
const HEAVY_STORM: u16 = 4891;
const POLYMERIZATION: u16 = 4837;
const MONSTER_REBORN: u16 = 4842;
const BRIONAC: u16 = 7548;

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

/// A duel with both Decks given, bottom first (the opening hand is the last five cards, the first draw the card before).
fn decks(mine: &[u16], theirs: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(Scenario::start_anime_field(&dir, 7, 8000, [(mine, &[][..]), (theirs, &[][..])], spawns).unwrap())
}

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

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

// ---------------------------------------------------------------------------------------------------------------
// Shadow Moon, Blue Moon and the Homunculus Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Shadow Moon and Blue Moon's duel: team 0 holds Shadow Moon and four monsters of Level 5 or higher with four Attributes
/// (Dark Magician DARK, Gaia The Fierce Knight EARTH, Flame Cerebrus FIRE, Kairyu-Shin WATER), Blue Moon and Demiurge Ema at the
/// bottom of its Deck; each side has a face-up Power Spirit for Blue Moon to destroy. Shadow Moon is activated in turn 1.
fn moon_duel(extra_spawns: &[Spawn]) -> Option<Scenario> {
    let [shadow, blue, ema, spirit] = ["Shadow Moon", "Blue Moon", "Demiurge Ema", "Power Spirit"].map(card);
    let mut spawns = vec![hand(0, shadow), hand(0, DARK_MAGICIAN), hand(0, GAIA_KNIGHT), hand(0, FLAME_CEREBRUS), hand(0, KAIRYU_SHIN)];
    spawns.extend([spell(0, spirit), spell(1, spirit)]);
    spawns.extend_from_slice(extra_spawns);
    let mut s = field(&[blue, ema], &spawns)?;
    s.prefer(&[(0, DARK_MAGICIAN), (0, GAIA_KNIGHT), (0, FLAME_CEREBRUS), (0, KAIRYU_SHIN), (0, spirit), (1, spirit), (0, ema), (0, blue)]);
    s.prefer_most(true);
    s.run(&[Activate(shadow)]).unwrap();
    Some(s)
}

/// Shadow Moon: "Send 4 Level 5 or higher monsters with different Attributes from your hand or your side of the field to the
/// Graveyard. Activate 1 "Blue Moon" from your hand, Deck or Graveyard. Then, Special Summon 1 "Demiurge Ema" from your Deck or
/// Graveyard." Blue Moon: "Activated only with "Shadow Moon". When you activate this card: Destroy 1 face-up Spell/Trap on each
/// player's side of the field, then Special Summon 1 "Homunculus Token" (Fairy-Type/LIGHT/Level 2/ATK 0/DEF 800) to each player's
/// side of the field in Defense Position." The four monsters and Shadow Moon are in the Graveyard, each side's Power Spirit is
/// destroyed, Blue Moon stays on the field as the Field Spell it is, each side has one Token (Fairy, LIGHT, Level 2, 0/800,
/// face-up Defense) and Demiurge Ema (Level 8, 3300/2000) came from the Deck.
#[test]
fn shadow_moon_activates_blue_moon_and_brings_demiurge_ema() {
    let [shadow, blue, ema, spirit] = ["Shadow Moon", "Blue Moon", "Demiurge Ema", "Power Spirit"].map(card);
    let token = 12369; // Homunculus Token (Blue Moon's: the card's passcode 511000608)
    let Some(s) = moon_duel(&[]) else { return };
    clean(&s);
    for cid in [DARK_MAGICIAN, GAIA_KNIGHT, FLAME_CEREBRUS, KAIRYU_SHIN, shadow] {
        assert_eq!(s.count(0, "grave", cid), 1, "{cid} was sent to the Graveyard: {}", s.context());
    }
    assert_eq!((s.count(0, "grave", spirit), s.count(1, "grave", spirit)), (1, 1), "each side's Spell was destroyed: {}", s.context());
    assert_eq!(s.count(0, "szone", blue) + s.count(0, "fzone", blue), 1, "Blue Moon is on the field: {}", s.context());
    assert_eq!((s.count(0, "mzone", token), s.count(1, "mzone", token)), (1, 1), "one Token to each side: {}", s.context());
    for team in [0, 1] {
        let t = stats(&s, team, token);
        assert_eq!((t.0, t.1, t.2), (Some(0), Some(800), Some(2)), "Level 2, 0/800: {}", s.context());
        assert_eq!((t.3, t.4), (0x4, 0x10), "Fairy, LIGHT: {}", s.context());
        assert_eq!(s.position_of(team, token), Some(4), "face-up Defense: {}", s.context());
    }
    assert_eq!(s.count(0, "mzone", ema), 1, "Demiurge Ema: {}", s.context());
    assert_eq!(stats(&s, 0, ema).2, Some(8), "{}", s.context());
}

/// A Homunculus Token is gone when it is destroyed, not sent to the Graveyard (Dark Hole in turn 2 destroys both Tokens and
/// Demiurge Ema), and "cannot be Tributed for a Tribute Summon": team 1's only monster is its Token, so Orion (Level 5, one
/// Tribute) is not offered to it, where it is with a Feral Imp in the Token's place.
#[test]
fn homunculus_tokens_cannot_be_tributed_and_leave_when_destroyed() {
    let token = 12369;
    let Some(mut s) = moon_duel(&[hand(1, ORION)]) else { return };
    let err = s.run_both([&[], &[Summon(ORION)]]).expect_err("the Token is no Tribute");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    // The control: a Feral Imp is a Tribute.
    let Some(mut s) = field(&[], &[mon(1, FILLER), hand(1, ORION)]) else { return };
    s.run_both([&[], &[Summon(ORION)]]).unwrap();
    assert_eq!(s.count(1, "mzone", ORION), 1, "{}", s.context());
    // Dark Hole.
    let Some(mut s) = moon_duel(&[hand(1, DARK_HOLE)]) else { return };
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", token), s.count(1, "mzone", token)), (0, 0), "both Tokens were destroyed: {}", s.context());
    assert_eq!((s.count(0, "grave", token), s.count(1, "grave", token)), (0, 0), "a Token is not sent to the Graveyard: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Scab Scream, Scar-Knight, Scab Blast and Victim Barrier
// ---------------------------------------------------------------------------------------------------------------

/// Scab Scream: "When a "Scab Scar-Knight" you control is targeted for an attack by a monster with 2000 or more ATK: Destroy
/// the attacking monster, then, send the attacked "Scab Scar-Knight" to the Graveyard, and if you do, Special Summon 1
/// "Scar-Knight" from your hand or Deck." Team 1's Warwolf (2000) attacks the Scab Scar Knight (the only monster, and all must
/// attack it): Warwolf is destroyed, the Knight goes to the Graveyard and the Scar-Knight (Level 1, 0/0) comes from the Deck.
/// Against Feral Imp (1300) the Trap is not played.
#[test]
fn scab_scream_destroys_the_attacker_and_brings_the_scar_knight() {
    let [scream, scab, scar] = ["Scab Scream", "Scab Scar Knight (Anime)", "Scar-Knight"].map(card);
    let Some(mut s) = field(&[scar], &[set(0, scream), mon(0, scab), mon(1, WARWOLF)]) else { return };
    s.run_both([&[ChainOnAttack(scream)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the attacker was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", scab), 1, "the Scab Scar Knight went to the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "mzone", scar), 1, "the Scar-Knight came: {}", s.context());
    assert_eq!(s.count(0, "grave", scream), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    let t = stats(&s, 0, scar);
    assert_eq!((t.0, t.1, t.2), (Some(0), Some(0), Some(1)), "Level 1, 0/0: {}", s.context());
    // A 1300 ATK attacker is under the limit.
    let Some(mut s) = field(&[scar], &[set(0, scream), mon(0, scab), mon(1, FILLER)]) else { return };
    let err = s.run_both([&[ChainOnAttack(scream)], &[Attack(FILLER)]]).expect_err("1300 is under 2000");
    assert!(err.contains("never became possible") || err.contains("can't"), "{err}");
    assert_eq!(s.count(0, "mzone", scab), 1, "{}", s.context());
}

/// Scar-Knight: "During your End Phase: You can Tribute this card; destroy all monsters on the field, and if you do, inflict
/// 500 damage to both players for each monster destroyed." With the Scar-Knight, a Feral Imp of mine and Warwolf and Dark
/// Blade of the opponent: the Scar-Knight is Tributed, the other three are destroyed and each player takes 1500.
#[test]
fn scar_knight_destroys_every_monster_and_hurts_both_players() {
    let scar = card("Scar-Knight");
    let Some(mut s) = field(&[], &[mon(0, scar), mon(0, FILLER), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.run(&[Chain(scar)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", scar), 1, "the Scar-Knight was Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1500, 8000 - 1500), "500 for each of three monsters: {}", s.context());
    // Alone, it has nothing to destroy: no effect offered, the Scar-Knight stays.
    let Some(mut s) = field(&[], &[mon(0, scar)]) else { return };
    let err = s.run(&[Chain(scar)]).expect_err("nothing to destroy");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", scar), 1, "{}", s.context());
}

/// Scab Blast: "During your Draw Phase, if this card is in your Graveyard, instead of conducting your normal draw: You can add
/// this card from your Graveyard to your hand." In my first Draw Phase it comes to my hand and the Deck keeps the card the draw
/// would have taken: the hand holds the five cards of the opening hand and Scab Blast, the Deck still its 35.
#[test]
fn scab_blast_in_the_graveyard_replaces_the_draw() {
    let blast = card("Scab Blast");
    let Some(mut s) = field(&[], &[grave(0, blast)]) else { return };
    s.run_both([&[Chain(blast)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", blast), 1, "added to the hand instead of the draw: {}", s.context());
    assert_eq!(s.count(0, "grave", blast), 0, "{}", s.context());
    assert_eq!(s.state()[0]["hand"].as_array().unwrap().len(), 6, "5 cards and Scab Blast, no card drawn: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_i64(), Some(35), "{}", s.context());
}

/// Victim Barrier: "If an opponent's monster declares an attack: Change the attack target to another monster you control." Team 1's
/// Warwolf attacks my Thunder Kid and the Trap sends the attack to Dark Blade (1800 against Warwolf's 2000: Dark Blade is destroyed,
/// 200 damage to me), where the same attack on Thunder Kid would have cost 1300.
#[test]
fn victim_barrier_changes_the_attack_target() {
    let barrier = card("Victim Barrier");
    let Some(mut s) = field(&[], &[set(0, barrier), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, THUNDER_KID), (0, DARK_BLADE)]);
    s.run_both([&[ChainOnAttack(barrier)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", barrier), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the new target was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 200, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dormant Volcano, Dowsing Point and Guidance to Ore
// ---------------------------------------------------------------------------------------------------------------

/// Dormant Volcano: "When this card is destroyed by a card effect and sent to the Graveyard, inflict 300 damage to your opponent.
/// Then, during your next Standby Phase, both players can add 1 FIRE monster from their Decks to their hands." Team 1's Heavy
/// Storm destroys it in turn 2: 300 damage to team 1. In team 0's Standby Phase of turn 3 each player adds a Flame Cerebrus
/// from its Deck.
#[test]
fn dormant_volcano_hurts_and_gives_both_players_a_fire_monster() {
    let volcano = card("Dormant Volcano");
    let mut mine = vec![FLAME_CEREBRUS];
    mine.extend([FILLER; 39]);
    let theirs = mine.clone();
    let Some(mut s) = decks(&mine, &theirs, &[set(0, volcano), hand(1, HEAVY_STORM)]) else { return };
    s.run_both([&[], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", volcano), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "300 damage to the opponent: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let hands = (s.count(0, "hand", FLAME_CEREBRUS), s.count(1, "hand", FLAME_CEREBRUS));
    assert_eq!(hands, (0, 0), "not yet: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", FLAME_CEREBRUS), 1, "my FIRE monster: {}", s.context());
    assert_eq!(s.count(1, "hand", FLAME_CEREBRUS), 1, "the opponent's FIRE monster: {}", s.context());
}

/// Dowsing Point: "When this card is destroyed by a card effect and sent to the Graveyard: Add 1 monster from your Deck to your
/// hand." Heavy Storm destroys it; Dark Magician at the bottom of the Deck is the monster chosen.
#[test]
fn dowsing_point_adds_a_monster_when_destroyed() {
    let dowsing = card("Dowsing Point");
    let Some(mut s) = field(&[DARK_MAGICIAN], &[set(0, dowsing), hand(1, HEAVY_STORM)]) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run_both([&[], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", dowsing), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "{}", s.context());
}

/// Guidance to Ore: "When a Rock-Type monster you control is destroyed, you can Special Summon 1 monster from your Deck with the
/// same name as the destroyed monster." Team 1's Dark Hole destroys my Giant Soldier of Stone: the second one comes from the Deck
/// (in face-up Attack Position, one in the Graveyard). With no copy in the Deck nothing comes.
#[test]
fn guidance_to_ore_summons_another_copy_of_the_destroyed_rock() {
    let guidance = card("Guidance to Ore");
    let spawns = [spell(0, guidance), mon(0, GIANT_SOLDIER), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[GIANT_SOLDIER], &spawns) else { return };
    s.prefer(&[(0, GIANT_SOLDIER)]);
    s.run_both([&[Chain(guidance)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GIANT_SOLDIER), 1, "the first was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", GIANT_SOLDIER), 1, "the second came from the Deck: {}", s.context());
    // No copy in the Deck.
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[Chain(guidance)], &[Activate(DARK_HOLE)]]).expect_err("no copy to summon");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!((s.count(0, "mzone", GIANT_SOLDIER), s.count(0, "grave", GIANT_SOLDIER)), (0, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Power Annihilator, Power Bombard, Explosion Fuse, Power Spirit and Ring of Fiendish Power
// ---------------------------------------------------------------------------------------------------------------

/// Power Annihilator: "If this card battles a monster with higher ATK than this card, it gains 1000 ATK during the Damage Step.
/// If this card battles a monster with less ATK than this card, it loses 1000 ATK during the Damage Step." Attacking Warwolf
/// (2000) with 1800 ATK it becomes 2800 and destroys it (800 damage); attacking Feral Imp (1300) it falls to 800 and is
/// destroyed itself (500 damage to me).
#[test]
fn power_annihilator_gains_against_a_stronger_monster_and_loses_against_a_weaker_one() {
    let annihilator = card("Power Annihilator");
    let Some(mut s) = field(&[], &[mon(0, annihilator), mon(1, WARWOLF)]) else { return };
    s.run(&[Attack(annihilator)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", annihilator), 1, "{}", s.context());
    assert_eq!((s.lp(1), s.lp(0)), (8000 - 800, 8000), "2800 against 2000: {}", s.context());
    assert_eq!(s.attack_of(0, annihilator), Some(1800), "the bonus ended with the Damage Step: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, annihilator), mon(1, FILLER)]) else { return };
    s.run(&[Attack(annihilator)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", annihilator), 1, "800 against 1300: {}", s.context());
    assert_eq!(s.count(1, "mzone", FILLER), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 500, 8000), "{}", s.context());
}

/// Power Bombard: "If the ATK of a "Power Annihilator" you control changes: destroy this card and inflict 1800 damage to your
/// opponent." Power Annihilator attacks Warwolf and gains 1000 ATK in the Damage Step: Power Bombard is destroyed, the
/// opponent takes 1800 and the 800 of the battle (Warwolf destroyed).
#[test]
fn power_bombard_destroys_itself_when_a_power_annihilator_changes_attack() {
    let [annihilator, bombard] = ["Power Annihilator", "Power Bombard"].map(card);
    let Some(mut s) = field(&[], &[mon(0, annihilator), mon(0, bombard), mon(1, WARWOLF)]) else { return };
    s.run(&[Attack(annihilator)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bombard), 1, "Power Bombard was destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 800 - 1800, "1800 and the battle damage: {}", s.context());
    // Without a Power Annihilator the Bombard stays whatever its controller's other monsters do.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(0, bombard), mon(1, FILLER)]) else { return };
    s.run(&[Attack(WARWOLF)]).unwrap();
    assert_eq!(s.count(0, "mzone", bombard), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
}

/// Explosion Fuse: "Activate only when an opponent's monster is destroyed by battle: Special Summon 1 "Power Bombard" from your
/// hand or Deck in Attack Position and equip it with this card. [...] During the Standby Phase of your next turn: destroy that
/// "Power Bombard" and inflict 1000 damage to your opponent." Warwolf destroys Feral Imp in turn 3 (700 damage): the Fuse
/// brings Power Bombard from the Deck; in the Standby Phase of turn 5 Power Bombard is destroyed and the opponent takes 1000.
#[test]
fn explosion_fuse_brings_power_bombard_and_blows_it_up_a_turn_later() {
    let [fuse, bombard] = ["Explosion Fuse", "Power Bombard"].map(card);
    let Some(mut s) = field(&[bombard], &[set(0, fuse), mon(0, WARWOLF), mon(1, FILLER)]) else { return };
    s.run(&[Attack(WARWOLF), Chain(fuse)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", FILLER), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", bombard), 1, "Power Bombard came: {}", s.context());
    assert_eq!(s.position_of(0, bombard), Some(1), "in Attack Position: {}", s.context());
    assert_eq!(s.count(0, "szone", fuse), 1, "the Fuse is equipped: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bombard), 1, "Power Bombard was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 700 - 1000, "{}", s.context());
}

/// Power Spirit: "Pay 1000 LP to activate this card. Face-up Attack Position monsters you control cannot be destroyed by battle,
/// except by battle with an opponent's monster with 1000 or more ATK than their ATK on the field. (Damage calculation is applied
/// normally.)" Activated for 1000 LP (7000), Dark Blade (1800) survives Warwolf's 2000 (200 damage to me) but not Blue-Eyes' 3000.
#[test]
fn power_spirit_protects_against_a_close_attacker_only() {
    let spirit = card("Power Spirit");
    for (attacker, survives) in [(WARWOLF, true), (BLUE_EYES, false)] {
        let spawns = [hand(0, spirit), mon(0, DARK_BLADE), mon(1, attacker)];
        let Some(mut s) = field(&[], &spawns) else { return };
        s.run_both([&[Activate(spirit)], &[Attack(attacker)]]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "szone", spirit), 1, "Power Spirit stays: {}", s.context());
        assert_eq!(s.count(0, "mzone", DARK_BLADE), survives as usize, "attacked by {attacker}: {}", s.context());
        let damage = if survives { 200 } else { 1200 };
        assert_eq!(s.lp(0), 8000 - 1000 - damage, "1000 paid and the battle damage: {}", s.context());
    }
}

/// Ring of Fiendish Power: "Your opponent can only attack the face-up Fiend-Type monster you control with the highest original
/// ATK. If a Fiend-Type monster you control destroys an opponent's monster by battle: inflict damage to your opponent equal to
/// the destroyed monster's DEF. If you control no face-up Fiend-Type monsters, destroy this card." Team 0 has Summoned Skull
/// (2500, Fiend), Feral Imp (Fiend) and Dark Blade (no Fiend): in turn 2 team 1's Warwolf can only attack Summoned Skull. In
/// turn 3 Summoned Skull destroys Dark Blade (1800/1500 in Attack Position) with 700 battle damage and 1500 more (its DEF).
#[test]
fn ring_of_fiendish_power_picks_the_attack_target_and_burns_for_the_defense() {
    let ring = card("Ring of Fiendish Power");
    let spawns = [mon(0, SUMMONED_SKULL), mon(0, FILLER), mon(0, DARK_BLADE), mon(1, DARK_BLADE), spell(0, ring)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SUMMONED_SKULL), 1, "{}", s.context());
    // Dark Blade 1800 against the Skull's 2500: it is destroyed (the Skull is the only target the Ring allows), and the Ring's
    // damage comes with the battle's.
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "Dark Blade attacked the Skull and lost: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 700 - 1500, "700 of the battle and 1500, the destroyed Dark Blade's DEF, from the Ring: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    // Turn 3: the Skull destroys a monster by battle: its DEF is damage as well.
    let spawns = [mon(0, SUMMONED_SKULL), mon(1, DARK_BLADE), spell(0, ring)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Attack(SUMMONED_SKULL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 700 - 1500, "700 of the battle and the 1500 DEF of the destroyed monster: {}", s.context());
    // Without a face-up Fiend the Ring destroys itself.
    let Some(mut s) = field(&[], &[spell(0, ring), mon(0, DARK_BLADE)]) else { return };
    s.run(&[]).unwrap();
    assert_eq!(s.count(0, "grave", ring), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Prism Wall
// ---------------------------------------------------------------------------------------------------------------

/// Prism Wall: "When your opponent targets exactly 1 monster you control (and no other cards) for an attack, or with a monster
/// effect: Target another monster you control that would be an appropriate target; that attack/monster now targets the new
/// target, then, both players take damage equal to the new target's ATK." Warwolf attacks Thunder Kid; the new target is Dark
/// Blade (1800): both players take 1800, then Dark Blade is destroyed by Warwolf (200 damage to me).
#[test]
fn prism_wall_changes_an_attack_and_hurts_both_players() {
    let prism = card("Prism Wall");
    let Some(mut s) = field(&[], &[set(0, prism), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, THUNDER_KID), (0, DARK_BLADE)]);
    s.run_both([&[ChainOnAttack(prism)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", prism), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the new target was attacked: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1800 - 200, 8000 - 1800), "{}", s.context());
}

/// Prism Wall's second answer, to a monster effect that targets exactly 1 monster of mine: the opponent's Brionac returns the
/// Warwolf it targets; the Wall moves it to Dark Blade (1800 ATK, both players take 1800) and Dark Blade is returned.
#[test]
fn prism_wall_changes_the_target_of_a_monster_effect() {
    let prism = card("Prism Wall");
    let spawns = [set(0, prism), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(1, BRIONAC)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, DARK_BLADE), (1, FILLER)]);
    s.run_both([&[Chain(prism)], &[Activate(BRIONAC)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", prism), 1, "Prism Wall answered: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf is no longer the target: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_BLADE), 1, "Dark Blade was returned: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1800, 8000 - 1800), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Amber Crystal Circle, GEM Burst and M Force
// ---------------------------------------------------------------------------------------------------------------

/// Amber Crystal Circle: "When a "Crystal Beast Amber Mammoth" you control is targeted for an attack: It gains ATK equal to the
/// combined ATK of all other "Crystal Beast" monsters you control, until the End Phase." Topaz Tiger (1600) and Amethyst Cat (1200)
/// stand beside the Mammoth (1700): it becomes 4500, destroys the attacking Warwolf and the opponent takes 2500.
#[test]
fn amber_crystal_circle_adds_the_other_crystal_beasts_attack() {
    let [circle, mammoth, tiger, cat] = ["Amber Crystal Circle", "Crystal Beast Amber Mammoth", "Crystal Beast Topaz Tiger", "Crystal Beast Amethyst Cat"].map(card);
    let spawns = [set(0, circle), mon(0, mammoth), mon(0, tiger), mon(0, cat), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, mammoth)]);
    s.run_both([&[ChainOnAttack(circle)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", circle), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", mammoth), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2500, "4500 against 2000: the attacker's controller takes the difference: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// GEM Burst: "Banish 1 "G Force", "E Force", and "M Force" from your Graveyard; Place as many "Crystal Beast" monsters from your
/// hand as possible, face-up, in your Spell & Trap Zone as Continuous Spell Cards. Then, inflict 500 damage to your opponent for
/// each "Crystal Beast" card in your Spell & Trap Zone." Three Forces in the Graveyard are banished and three Crystal Beasts of the
/// hand (Amethyst Cat, Ruby Carbuncle, Topaz Tiger) go to the Spell & Trap Zone: 1500 damage.
#[test]
fn gem_burst_places_the_crystal_beasts_and_burns_for_each() {
    let [burst, g, e, m, cat, ruby, tiger] =
        ["GEM Burst", "G Force", "E Force", "M Force", "Crystal Beast Amethyst Cat", "Crystal Beast Ruby Carbuncle", "Crystal Beast Topaz Tiger"].map(card);
    let spawns = [hand(0, burst), grave(0, g), grave(0, e), grave(0, m), hand(0, cat), hand(0, ruby), hand(0, tiger)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(burst)]).unwrap();
    clean(&s);
    for force in [g, e, m] {
        assert_eq!(s.count(0, "removed", force), 1, "{force} was banished as the cost: {}", s.context());
    }
    for beast in [cat, ruby, tiger] {
        assert_eq!(s.count(0, "szone", beast), 1, "{beast} is in the Spell & Trap Zone: {}", s.context());
        assert_eq!(s.count(0, "hand", beast), 0, "{}", s.context());
    }
    assert_eq!(s.lp(1), 8000 - 1500, "500 for each of three: {}", s.context());
    // Without all three Forces it is not offered.
    let Some(mut s) = field(&[], &[hand(0, burst), grave(0, g), grave(0, e), hand(0, cat)]) else { return };
    let err = s.run(&[Activate(burst)]).expect_err("M Force is missing");
    assert!(err.contains("never became possible"), "{err}");
}

/// M Force: "Target 1 "Crystal Beast" monster you control; until the End Phase, it gains 500 ATK, also, if it attacks a Defense
/// Position monster, inflict piercing battle damage." Topaz Tiger (1600, 2000 when it attacks a monster) attacks Warwolf (2000/100)
/// in Defense Position: with M Force it has 2500 ATK, destroys it and the opponent takes 2400 (ATK 1600 + 500 + 400 - DEF 100
/// = 2400); the same attack without the Spell inflicts no damage.
#[test]
fn m_force_adds_attack_and_pierces() {
    let [force, tiger] = ["M Force", "Crystal Beast Topaz Tiger"].map(card);
    let Some(mut s) = field(&[], &[hand(0, force), mon(0, tiger), mon_def(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(force), Attack(tiger)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2400, "{}", s.context());
    assert_eq!(s.attack_of(0, tiger), Some(2100), "until the End Phase: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, tiger), mon_def(1, WARWOLF)]) else { return };
    s.run(&[Attack(tiger)]).unwrap();
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "no piercing damage: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Mystic Egg, Mystic Baby Dragon, Mystic Revolution and Mystic Dragon
// ---------------------------------------------------------------------------------------------------------------

/// Mystic Egg: "During the End Phase of a turn this card was destroyed by battle, you can randomly Special Summon 1 "Mystic Baby
/// Dragon", "Mystic Baby Knight", or "Mystic Baby Magician" from your hand or Deck." Warwolf destroys the Egg in turn 2; in that
/// End Phase the Mystic Baby Dragon (Level 4 Dragon, 1200/1600; the only one of the three the game has) comes from the Deck. An
/// Egg that is not destroyed by battle (Dark Hole) brings nothing.
#[test]
fn mystic_egg_brings_a_baby_dragon_when_destroyed_by_battle() {
    let [egg, baby] = ["Mystic Egg", "Mystic Baby Dragon"].map(card);
    let Some(mut s) = field(&[baby], &[mon(0, egg), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Chain(egg)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", egg), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", baby), 1, "the Baby Dragon came: {}", s.context());
    let t = stats(&s, 0, baby);
    assert_eq!((t.0, t.1, t.2, t.3), (Some(1200), Some(1600), Some(4), 0x2000), "Level 4 Dragon, 1200/1600: {}", s.context());
    // Dark Hole is not battle.
    let Some(mut s) = field(&[baby], &[mon(0, egg), hand(1, DARK_HOLE)]) else { return };
    let err = s.run_both([&[Chain(egg)], &[Activate(DARK_HOLE)]]).expect_err("not destroyed by battle");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", baby), 0, "{}", s.context());
}

/// Mystic Baby Dragon: "Cannot be Normal Summoned/Set. This card cannot be Special Summoned except with "Mystic Egg"." It is not
/// offered as a Normal Summon or Set from the hand, and Monster Reborn does not take it from the Graveyard (the Spell is not
/// offered with it as the only monster there), where the Spell takes Warwolf in its place.
#[test]
fn mystic_baby_dragon_cannot_be_summoned_in_other_ways() {
    let baby = card("Mystic Baby Dragon");
    let Some(mut s) = field(&[], &[hand(0, baby), hand(0, MONSTER_REBORN), grave(0, baby)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&baby.into()), "no Normal Summon: {:?}", offered(&s, "summons"));
    assert!(!offered(&s, "monster_sets").contains(&baby.into()), "no Set: {:?}", offered(&s, "monster_sets"));
    assert!(!offered(&s, "activates").contains(&MONSTER_REBORN.into()), "no Monster Reborn target: {:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[hand(0, MONSTER_REBORN), grave(0, baby), grave(0, WARWOLF)]) else { return };
    s.prefer(&[(0, baby), (0, WARWOLF)]);
    s.run(&[Activate(MONSTER_REBORN)]).unwrap();
    assert_eq!((s.count(0, "mzone", baby), s.count(0, "mzone", WARWOLF)), (0, 1), "{}", s.context());
}

/// Mystic Revolution: "Tribute 1 face-up "Mystic Baby Dragon", "Mystic Baby Knight", or "Mystic Baby Magician" you control. Special
/// Summon, from your hand, Deck, or Graveyard, 1 monster written in the card text of the selected card, ignoring the Summoning
/// conditions." The Baby Dragon is Tributed and the Mystic Dragon (Level 8 Dragon, 3600/1600) comes from the Deck; from the
/// Graveyard, too. With no Baby on the field it is not offered. "Mystic Dragon: Cannot be Normal Summoned/Set. Can only be
/// Special Summoned with "Mystic Revolution"": Monster Reborn cannot take it from the Graveyard.
#[test]
fn mystic_revolution_tributes_the_baby_dragon_for_the_mystic_dragon() {
    let [revolution, baby, dragon] = ["Mystic Revolution", "Mystic Baby Dragon", "Mystic Dragon"].map(card);
    let Some(mut s) = field(&[dragon], &[hand(0, revolution), mon(0, baby)]) else { return };
    s.run(&[Activate(revolution)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", baby), 1, "the Baby Dragon was Tributed: {}", s.context());
    assert_eq!(s.count(0, "mzone", dragon), 1, "the Mystic Dragon came from the Deck: {}", s.context());
    let t = stats(&s, 0, dragon);
    assert_eq!((t.0, t.1, t.2, t.3), (Some(3600), Some(1600), Some(8), 0x2000), "Level 8 Dragon, 3600/1600: {}", s.context());
    assert_eq!(s.count(0, "grave", revolution), 1, "{}", s.context());
    // From the Graveyard.
    let Some(mut s) = field(&[], &[hand(0, revolution), mon(0, baby), grave(0, dragon)]) else { return };
    s.run(&[Activate(revolution)]).unwrap();
    assert_eq!((s.count(0, "mzone", dragon), s.count(0, "grave", dragon)), (1, 0), "{}", s.context());
    // No Baby Dragon.
    let Some(mut s) = field(&[dragon], &[hand(0, revolution), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(revolution)]).expect_err("no Baby on the field");
    assert!(err.contains("never became possible"), "{err}");
    // The Mystic Dragon is no target for Monster Reborn.
    let Some(mut s) = field(&[], &[hand(0, MONSTER_REBORN), grave(0, dragon)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&MONSTER_REBORN.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Miracle Draw
// ---------------------------------------------------------------------------------------------------------------

/// Miracle Draw: "During each of your Draw Phases, before you conduct your normal draw, you can declare a card name. Then, reveal
/// the card you draw. If you guessed correctly, inflict 1000 damage to your opponent. If not, take 1000 damage." My Deck is
/// Dark Blades (and a Warwolf at the bottom, so that it can be named), so the turn 1 draw is Dark Blade: declaring it hurts the
/// opponent, declaring Warwolf hurts me.
#[test]
fn miracle_draw_punishes_a_wrong_guess_and_rewards_a_right_one() {
    let miracle = card("Miracle Draw");
    let mut mine = vec![WARWOLF];
    mine.extend([DARK_BLADE; 39]);
    for (guess, lp) in [(DARK_BLADE, (8000, 7000)), (WARWOLF, (7000, 8000))] {
        let Some(mut s) = decks(&mine, &[FILLER; 40], &[spell(0, miracle)]) else { return };
        s.declare_card(Some(guess));
        s.run_both([&[Chain(miracle)], &[]]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "hand", DARK_BLADE), 6, "the draw was Dark Blade: {}", s.context());
        assert_eq!((s.lp(0), s.lp(1)), lp, "guessed {guess}: {}", s.context());
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Amazoness Call, Amazoness Trainer, Amazoness Charm and Pride of Tribe
// ---------------------------------------------------------------------------------------------------------------

/// Amazoness Call (Anime): "If you control an "Amazoness" monster: Target 1 Fusion Monster on the field; return it to the Extra
/// Deck, then, if all of the Fusion Material that were used to Fusion Summon that monster are in the Graveyard(s), Special Summon
/// them to their owners' sides of the field." Polymerization makes Gaia the Dragon Champion from Gaia The Fierce Knight and Curse
/// of Dragon; with an Amazoness Fighter on the field the Call returns the Champion to the Extra Deck and both materials come back.
#[test]
fn amazoness_call_returns_a_fusion_monster_and_brings_its_materials_back() {
    let [call, fighter, champion, knight, dragon] =
        ["Amazoness Call (Anime)", "Amazoness Fighter", "Gaia the Dragon Champion", "Gaia The Fierce Knight", "Curse of Dragon"].map(card);
    let spawns = [hand(0, POLYMERIZATION), hand(0, knight), hand(0, dragon), hand(0, call), mon(0, fighter)];
    let Some(mut s) = field_extra([&[champion], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(POLYMERIZATION)]).unwrap();
    assert_eq!(s.count(0, "mzone", champion), 1, "{}", s.context());
    s.run(&[Activate(call)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", champion), 0, "the Fusion Monster went back: {}", s.context());
    assert_eq!((s.count(0, "mzone", knight), s.count(0, "mzone", dragon)), (1, 1), "its materials came back: {}", s.context());
    // Without an Amazoness it is not offered.
    let spawns = [hand(0, POLYMERIZATION), hand(0, knight), hand(0, dragon), hand(0, call)];
    let Some(mut s) = field_extra([&[champion], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(POLYMERIZATION)]).unwrap();
    let err = s.run(&[Activate(call)]).expect_err("no Amazoness");
    assert!(err.contains("never became possible"), "{err}");
}

/// Amazoness Trainer: "If an "Amazoness Tiger" you control attacks an opponent's monster, and that opponent's monster is still on
/// the field at the end of the Damage Step: That "Amazoness Tiger" gains 400 ATK, and can attack that monster again in a row, and
/// if it does, if its ATK is higher than that monster's DEF, inflict piercing battle damage to your opponent." The Tiger (1100 +
/// 400 for itself = 1500) attacks Dark Blade in Defense Position (DEF 1500): nothing is destroyed; the Trainer gives 400 ATK
/// (1900) and the second attack destroys it with 400 damage.
#[test]
fn amazoness_trainer_lets_the_tiger_attack_again_with_piercing_damage() {
    let [trainer, tiger] = ["Amazoness Trainer", "Amazoness Tiger"].map(card);
    let Some(mut s) = field(&[], &[mon(0, tiger), set(0, trainer), mon_def(1, DARK_BLADE)]) else { return };
    s.run(&[Attack(tiger), Chain(trainer), Attack(tiger)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", trainer), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "the second attack destroyed it: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 400, "piercing damage: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Amazoness Charm: "Change all Defense Position monsters your opponent controls to Attack Position, then those monsters gain 200
/// ATK." Warwolf and Dark Blade in Defense Position become Attack Position with 2200 and 2000 ATK; a monster already in Attack
/// Position (Thunder Kid) gains nothing.
#[test]
fn amazoness_charm_turns_the_defenders_and_strengthens_them() {
    let charm = card("Amazoness Charm");
    let spawns = [hand(0, charm), mon_def(1, WARWOLF), mon_def(1, DARK_BLADE), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(charm)]).unwrap();
    clean(&s);
    for (cid, atk) in [(WARWOLF, 2200), (DARK_BLADE, 2000), (THUNDER_KID, 700)] {
        assert_eq!(s.position_of(1, cid), Some(1), "{cid} is in Attack Position: {}", s.context());
        assert_eq!(s.attack_of(1, cid), Some(atk), "{cid}: {}", s.context());
    }
}

/// Pride of Tribe: "When an "Amazoness" monster you control is destroyed: Special Summon 1 "Amazoness" monster from your Deck with
/// the same name as the destroyed monster." Dark Hole destroys my Amazoness Fighter: the second one comes from the Deck. A monster
/// of another name (Warwolf) destroyed with it brings nothing of its own.
#[test]
fn pride_of_tribe_summons_another_copy_of_the_destroyed_amazoness() {
    let [pride, fighter] = ["Pride of Tribe", "Amazoness Fighter"].map(card);
    let Some(mut s) = field(&[fighter], &[set(0, pride), mon(0, fighter), mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    s.prefer(&[(0, fighter)]);
    s.run_both([&[Chain(pride)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", pride), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", fighter), 1, "the first Fighter was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", fighter), 1, "the second came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
}


