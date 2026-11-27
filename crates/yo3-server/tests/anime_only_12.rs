//! Anime-only cards, group 12 (rows 5585-5618 of engine/added/cards.tsv): Orlando's, Sarina's and Alexis Rhodes' cards. These
//! cards exist only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes
//! through the real core with both seats' client models checked after every step; the texts are the anime ones. Every Trap
//! and Quick-Play Spell answering the opponent is Set first, and neither player has a Battle Phase in turn 1.
//! Covered: the Tokens (Return Token, made by Return Talisman; Icicle Token, by Icicle Sacrifice, each reaching the field and
//! leaving), every card the per-card fuzz flagged (Genghis Ghan, Big Bridge, Cherry Blossom Mountain, Monk Halberd, Buried
//! Soul Talisman, Asogi, Nayuta, Mirror Bind, Mirror Call, Redirection, Sakyo, Ukyo, Angel Wing, White Blizzard: the fuzz's
//! duels simply never reached most of them; Big Bridge's and Cherry Blossom Mountain's flagged resolutions could not be
//! reproduced, both effects do their part here for both seats), the script notes (Big Bridge's and Cherry Blossom Mountain's
//! cost, with and without a Kabuki Stage in the Deck), the signature cards (the Silver Spirits with Mirror Bind and Mirror
//! Call, the Magic Mirror Spirits, Kabuki Stage, Icicle Sacrifice, Cold Sleeper, White Night Fort) and the seeded sample
//! (Angel Blast, The Rough Seas, Pure Pupil, Return Talisman, Yoshitsune), with the rest of the batch.
//! Overrides: Genghis Ghan (`engine/overrides/c511001914.lua`: no monster in the opponent's Graveyard stopped the effect on
//! a Lua error, fuzz seed 62892) and Allegro Toile (`c511000134.lua`: "cannot be negated by the effects of other
//! Spells/Traps" was missing).
//! Not coverable: Allegro Toile's clause is approximated (no negation of that chain link by anything); Icicle Sacrifice's
//! Token is placed in the zone the same effect closes while it stands (the core's place prompt does not know the closed zone
//! yet), which leaves one zone fewer for the rest of the duel, as the text says, once it is Tributed.
use yo3_server::duelcheck::scenario::{Action, Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const MYSTICAL_ELF: u16 = 4008; // Level 4 Spellcaster, 800/2000, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const SOGEN: u16 = 4339; // Field Spell

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: `bottoms[team]` are at the
/// very bottom of that team's Deck (out of the opening hand and the first draws), for a card an effect fetches from it.
fn duel(seed: u64, lp: u32, bottoms: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let deck = |bottom: &[u16]| -> Vec<u16> { bottom.iter().copied().chain(std::iter::repeat_n(FILLER, 40 - bottom.len())).collect() };
    let (mine, theirs) = (deck(bottoms[0]), deck(bottoms[1]));
    let decks = [(&mine[..], &[][..]), (&theirs[..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, team 0's Deck has `bottom` at its bottom.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [bottom, &[]], spawns) }

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

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

const RACE_FAIRY: i64 = 0x4;
const RACE_AQUA: i64 = 0x40;
const ATTR_WATER: i64 = 0x2;
const ATTR_DARK: i64 = 0x20;
const FACEUP_ATTACK: u64 = 1;
const FACEUP_DEFENSE: u64 = 4;

// ---------------------------------------------------------------------------------------------------------------
// Return Talisman and the Return Token
// ---------------------------------------------------------------------------------------------------------------

/// Return Talisman: "During your opponent's End Phase, if they did not Summon a monster this turn, Special Summon 1
/// "Return Token" (Fairy-Type/DARK/Level 2/ATK 1000/DEF 1000) to their side of the field in Defense Position." Team 1
/// summons nothing in turn 2: a Return Token comes to team 1's side, face-up Defense, and Dark Hole in turn 3 destroys it:
/// a Token is gone, not in the Graveyard. Team 1 Normal Summoning a monster gets none.
#[test]
fn return_talisman_gives_a_return_token_to_an_opponent_who_summoned_nothing() {
    let [talisman, token] = ["Return Talisman", "Return Token"].map(card);
    let Some(mut s) = field(&[], &[set(0, talisman), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(talisman)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", token), 1, "a Return Token on team 1's side: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 0, "{}", s.context());
    let t = stats(&s, 1, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(1000), Some(1000), Some(2), RACE_FAIRY, ATTR_DARK), "Fairy, DARK, Level 2, 1000/1000: {}", s.context());
    assert_eq!(s.position_of(1, token), Some(FACEUP_DEFENSE), "face-up Defense: {}", s.context());
    // The Token leaves: Dark Hole destroys it and it does not go to the Graveyard.
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", token), s.count(1, "grave", token)), (0, 0), "a Token is gone: {}", s.context());
    // The control: team 1 Normal Summons in turn 2.
    let Some(mut s) = field(&[], &[set(0, talisman), hand(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(talisman)], &[Summon(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", token), 0, "no Token after a Summon: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Icicle Sacrifice and the Icicle Token
// ---------------------------------------------------------------------------------------------------------------

/// Icicle Sacrifice: "Choose 1 of your unused Monster Zones; Special Summon 1 "Icicle Token" (Aqua-Type/WATER/Level 1/ATK
/// 0/DEF 0). That chosen Monster Zone cannot be used for the entire Duel. The "Icicle Token" can be treated as 2 Tributes
/// for the Tribute Summon of a monster." The Token (0/0, Level 1, Aqua, WATER) is Tributed alone for Dark Magician (Level 7,
/// two Tributes) and is gone, not in the Graveyard.
#[test]
fn icicle_sacrifice_makes_an_icicle_token_worth_two_tributes() {
    let [sacrifice, token] = ["Icicle Sacrifice", "Icicle Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, sacrifice), hand(0, DARK_MAGICIAN)]) else { return };
    s.run(&[Activate(sacrifice)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 1, "{}", s.context());
    let t = stats(&s, 0, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(0), Some(0), Some(1), RACE_AQUA, ATTR_WATER), "Aqua, WATER, Level 1, 0/0: {}", s.context());
    assert_eq!(s.count(0, "grave", sacrifice), 1, "{}", s.context());
    s.run(&[Summon(DARK_MAGICIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "Dark Magician was Tribute Summoned with the Token alone: {}", s.context());
    assert_eq!((s.count(0, "mzone", token), s.count(0, "grave", token)), (0, 0), "the Tributed Token is gone: {}", s.context());
    // The control: a Level 7 monster with no Token and no other monster cannot be Tribute Summoned.
    let Some(mut s) = field(&[], &[hand(0, DARK_MAGICIAN)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "{:?}", offered(&s, "summons"));
}

/// The Monster Zones team 0 can still Summon into, as the zone prompt of a Normal Summon of Feral Imp shows them.
fn free_zones_for_a_summon(s: &mut Scenario) -> usize {
    s.run_to(&[Summon(FILLER)], |p| p["type"] == 18).unwrap();
    let mask = s.prompt()["field_mask"].as_u64().unwrap();
    (0..5).filter(|bit| mask >> bit & 1 == 0).count()
}

/// Icicle Sacrifice: "That chosen Monster Zone cannot be used for the entire Duel." Dark Magician is Tribute Summoned with
/// the Token (gone again) and in turn 3 only three of the five Monster Zones can be Summoned into: the one Dark Magician
/// stands in and the chosen one are out. With Dark Magician alone and no Icicle Sacrifice four can.
#[test]
fn icicle_sacrifice_takes_a_monster_zone_for_the_rest_of_the_duel() {
    let sacrifice = card("Icicle Sacrifice");
    let Some(mut s) = field(&[], &[hand(0, sacrifice), hand(0, DARK_MAGICIAN)]) else { return };
    s.run(&[Activate(sacrifice), Summon(DARK_MAGICIAN), EndTurn]).unwrap();
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(free_zones_for_a_summon(&mut s), 3, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, DARK_MAGICIAN)]) else { return };
    s.run(&[]).unwrap();
    assert_eq!(free_zones_for_a_summon(&mut s), 4, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Genghis Ghan the Emperor Dragon (engine/overrides/c511001914.lua)
// ---------------------------------------------------------------------------------------------------------------

/// Genghis Ghan the Emperor Dragon: "When this card is destroyed by a card effect: Target 1 monster in your opponent's
/// Graveyard; inflict damage to your opponent equal to its ATK." Team 1's Dark Hole destroys it and team 0 targets Gene-Warped
/// Warwolf (2000) in team 1's Graveyard: team 1 takes 2000. Destroyed by battle (Dark Magician attacks it in Defense) it
/// does nothing.
#[test]
fn genghis_ghan_inflicts_the_atk_of_a_monster_in_the_opponents_graveyard_when_destroyed_by_an_effect() {
    let ghan = card("Genghis Ghan the Emperor Dragon");
    let spawns = [mon(0, ghan), hand(1, DARK_HOLE), grave(1, WARWOLF), grave(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ghan), 1, "Dark Hole destroyed it: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2000), "team 1 took Warwolf's ATK: {}", s.context());
    // The other target, Dark Blade (1800).
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1800, "{}", s.context());
    // Destroyed by battle: no damage.
    let spawns = [mon_def(0, ghan), mon(1, DARK_MAGICIAN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ghan), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
}

/// Genghis Ghan: with no monster in the opponent's Graveyard there is nothing to target: the effect is not activated
/// (the script's mandatory target chose from an empty group and stopped on a Lua error, the fuzz's seed 62892).
#[test]
fn genghis_ghan_destroyed_with_no_monster_in_the_opponents_graveyard_does_nothing() {
    let ghan = card("Genghis Ghan the Emperor Dragon");
    let Some(mut s) = field(&[], &[mon(0, ghan), hand(1, DARK_HOLE), grave(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ghan), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "only the opponent's Graveyard counts: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Kabuki Stage - Big Bridge, Cherry Blossom Mountain and The Rough Seas
// ---------------------------------------------------------------------------------------------------------------

/// How many monsters a team has in its Monster Zones.
fn monsters(s: &Scenario, team: usize) -> usize {
    s.state()[team]["mzone"].as_array().map_or(0, |m| m.iter().filter(|c| !c.is_null()).count())
}

/// Kabuki Stage - Big Bridge: "Once per turn, at the end of the Damage Step, if a monster is attacked: Its controller can
/// Special Summon 1 Level 4 or lower monster from their hand in face-up Attack Position." Team 1's Dark Blade attacks team 0's
/// Warwolf and loses; a Level 4 or lower monster (Thunder Kid or a Feral Imp, the first the effect offers) comes from team 0's
/// hand in face-up Attack, and Dark Magician (Level 7) does not (it stays in the hand).
#[test]
fn big_bridge_special_summons_a_low_level_monster_for_the_one_attacked() {
    let bridge = card("Kabuki Stage - Big Bridge");
    let spawns = [hand(0, bridge), mon(0, WARWOLF), hand(0, THUNDER_KID), hand(0, DARK_MAGICIAN), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(bridge), Chain(bridge)], &[EndTurn, Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(monsters(&s, 0), 2, "Warwolf and the monster from the hand: {}", s.context());
    let summoned = if s.count(0, "mzone", FILLER) == 1 { FILLER } else { THUNDER_KID };
    assert_eq!(s.count(0, "mzone", summoned), 1, "{}", s.context());
    assert_eq!(s.position_of(0, summoned), Some(FACEUP_ATTACK), "face-up Attack: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "Level 7 is too high: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
}

/// Kabuki Stage - Big Bridge: the effect is both players': "Its controller" is whoever's monster is attacked. Team 0's Warwolf
/// attacks team 1's Thunder Kid in turn 3 and team 1 chains Big Bridge (on team 0's side of the field): a Level 4 or lower
/// monster comes from team 1's hand in face-up Attack (Dark Magician stays).
#[test]
fn big_bridge_lets_the_other_player_summon_when_its_monster_is_attacked() {
    let bridge = card("Kabuki Stage - Big Bridge");
    let spawns = [hand(0, bridge), mon(0, WARWOLF), mon(1, THUNDER_KID), hand(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(bridge), EndTurn, Attack(WARWOLF)], &[Chain(bridge)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "team 1's Thunder Kid lost the battle: {}", s.context());
    assert_eq!(monsters(&s, 1), 1, "team 1 Special Summoned from its hand: {}", s.context());
    assert_eq!(s.count(1, "mzone", FILLER), 1, "{}", s.context());
    assert_eq!(s.position_of(1, FILLER), Some(FACEUP_ATTACK), "{}", s.context());
    assert_eq!(s.count(1, "hand", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(monsters(&s, 0), 1, "team 0 summoned nothing: {}", s.context());
}

/// Kabuki Stage - Big Bridge: "You can send this face-up card to the Graveyard; activate 1 "Kabuki Stage" card from your Deck."
/// With Kabuki Stage - The Rough Seas at the bottom of the Deck: Big Bridge goes to the Graveyard and The Rough Seas is on the
/// field (activated: its own effect then adds a Level 5 or higher monster, Dark Magician, from the Deck). With no Kabuki
/// Stage in the Deck the effect is not offered (nothing to choose after the cost).
#[test]
fn big_bridge_activates_another_kabuki_stage_from_the_deck() {
    let [bridge, seas] = ["Kabuki Stage - Big Bridge", "Kabuki Stage - The Rough Seas"].map(card);
    let Some(mut s) = field(&[seas, DARK_MAGICIAN], &[hand(0, bridge)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(bridge), Activate(bridge)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bridge), 1, "Big Bridge was the cost: {}", s.context());
    assert_eq!(s.count(0, "szone", seas), 1, "The Rough Seas is on the field: {}", s.context());
    // No Kabuki Stage in the Deck: never offered.
    let Some(mut s) = field(&[], &[hand(0, bridge)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(bridge), Activate(bridge)]).expect_err("nothing to activate");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", bridge), 1, "{}", s.context());
}

/// Kabuki Stage - Cherry Blossom Mountain: "When a monster(s) is Normal or Special Summoned: Change that monster(s) to face-up
/// Attack Position." Monster Reborn revives Dark Blade, chosen in Defense Position, and Cherry Blossom Mountain turns it to
/// Attack (the control without it stays in Defense). The fuzz could not reach it: only a Special Summon in Defense (a
/// revival, Call of the Haunted) sets the effect off.
#[test]
fn cherry_blossom_mountain_turns_a_summoned_monster_to_attack_position() {
    let cherry = card("Kabuki Stage - Cherry Blossom Mountain");
    let Some(mut s) = field(&[], &[hand(0, cherry), hand(0, MONSTER_REBORN), grave(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.prefer_position(Some(FACEUP_DEFENSE));
    s.run(&[Activate(cherry), Activate(MONSTER_REBORN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.position_of(0, DARK_BLADE), Some(FACEUP_ATTACK), "changed to face-up Attack: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, MONSTER_REBORN), grave(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.prefer_position(Some(FACEUP_DEFENSE));
    s.run(&[Activate(MONSTER_REBORN)]).unwrap();
    assert_eq!(s.position_of(0, DARK_BLADE), Some(FACEUP_DEFENSE), "the control: {}", s.context());
    // It is no one's alone: the opponent's revival in Defense Position is changed too.
    let Some(mut s) = field(&[], &[hand(0, cherry), hand(1, MONSTER_REBORN), grave(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.prefer_position(Some(FACEUP_DEFENSE));
    s.run_both([&[Activate(cherry)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, DARK_BLADE), Some(FACEUP_ATTACK), "team 1's too: {}", s.context());
}

/// Kabuki Stage - Cherry Blossom Mountain: "When a monster declares an attack, the attack target is chosen randomly." Team 1's
/// Warwolf declares an attack on team 0's Dark Blade with Thunder Kid beside it and its own Dark Magician: over twelve
/// duels (twelve seeds) the target is not always Dark Blade, and it is always one of those three (the outcome of the battle
/// tells which: Dark Blade loses to Warwolf, Thunder Kid too, and Warwolf loses to Team 1's Dark Magician).
#[test]
fn cherry_blossom_mountain_chooses_the_attack_target_at_random() {
    let cherry = card("Kabuki Stage - Cherry Blossom Mountain");
    let mut outcomes = std::collections::BTreeSet::new();
    for seed in 1..=12 {
        let spawns = [hand(0, cherry), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_MAGICIAN)];
        let Some(mut s) = duel(seed, 8000, [&[], &[]], &spawns) else { return };
        keep_hands(&mut s);
        s.prefer(&[(0, DARK_BLADE), (0, FILLER), (1, FILLER)]);
        s.run_both([&[Activate(cherry), EndTurn], &[Attack(WARWOLF)]]).unwrap();
        clean(&s);
        let hit = [(0, DARK_BLADE), (0, THUNDER_KID), (1, WARWOLF)].into_iter().filter(|&(t, c)| s.count(t, "grave", c) == 1).collect::<Vec<_>>();
        assert_eq!(hit.len(), 1, "exactly one monster lost the battle (seed {seed}): {}", s.context());
        outcomes.insert(hit[0]);
    }
    assert!(outcomes.len() >= 2, "the target was always the same: {outcomes:?}");
}

/// Kabuki Stage - Cherry Blossom Mountain: "You can send this card to the Graveyard; activate 1 "Kabuki Stage" Spell Card from
/// your Deck." With Big Bridge at the bottom of the Deck it is activated from there as the Field Spell (Cherry Blossom Mountain
/// is the cost, in the Graveyard; Big Bridge is on the field). With no Kabuki Stage in the Deck the effect is not offered:
/// the script's cost sends the card to the Graveyard first, so a Deck without a Kabuki Stage has to stop it before (it does:
/// the target check looks at the Deck, no Lua error). Only one Field Spell exists at a time: Team 1's Sogen, activated after
/// Cherry Blossom Mountain, put it in the Graveyard by the rules, so the effect is there to use only in the activating turn.
#[test]
fn cherry_blossom_mountain_activates_a_kabuki_stage_from_the_deck() {
    let [cherry, bridge] = ["Kabuki Stage - Cherry Blossom Mountain", "Kabuki Stage - Big Bridge"].map(card);
    let Some(mut s) = field(&[bridge], &[hand(0, cherry)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(cherry), Activate(cherry)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cherry), 1, "Cherry Blossom Mountain was the cost: {}", s.context());
    assert_eq!(s.count(0, "szone", bridge), 1, "Big Bridge is on team 0's field: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, cherry)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(cherry), Activate(cherry)]).expect_err("no Kabuki Stage in the Deck");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", cherry), 1, "{}", s.context());
    // A second Field Spell replaces the first, whoever's it is: Sogen from team 1 sends Cherry Blossom Mountain away.
    let Some(mut s) = field(&[], &[hand(0, cherry), hand(1, SOGEN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(cherry)], &[Activate(SOGEN)]]).unwrap();
    assert_eq!((s.count(0, "grave", cherry), s.count(1, "szone", SOGEN)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Kabuki Stage - The Rough Seas
// ---------------------------------------------------------------------------------------------------------------

/// Kabuki Stage - The Rough Seas: "During each player's Main Phase 1: The turn player can add 1 Level 5 or higher monster from
/// their Deck to their hand." In turn 1 team 0 takes Dark Magician (the Level 4 Dark Blade above it stays in the Deck) and
/// cannot do it twice; in turn 2 team 1 takes its own Blue-Eyes White Dragon from team 1's Deck.
#[test]
fn the_rough_seas_lets_each_player_add_a_high_level_monster_in_its_own_main_phase() {
    let [seas, blue_eyes] = ["Kabuki Stage - The Rough Seas", "Blue-Eyes White Dragon"].map(card);
    let Some(mut s) = duel(7, 8000, [&[DARK_BLADE, DARK_MAGICIAN], &[blue_eyes]], &[hand(0, seas)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(seas), Activate(seas)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", DARK_MAGICIAN), s.count(0, "hand", DARK_BLADE)), (1, 0), "{}", s.context());
    assert!(!offered(&s, "activates").contains(&seas.into()), "once per turn: {:?}", offered(&s, "activates"));
    s.run_both([&[EndTurn], &[Activate(seas)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", blue_eyes), 1, "team 1 added its own: {}", s.context());
    assert_eq!(s.count(0, "hand", blue_eyes), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Battle Claw Fox and New Year Drum
// ---------------------------------------------------------------------------------------------------------------

/// Battle Claw Fox: "If this card battles an opponent's monster: destroy that monster during the End Phase." In turn 3 the Fox
/// (800) attacks team 1's Mystical Elf in Defense Position (2000): neither is destroyed (the Fox's controller takes the 1200
/// difference), and the Elf is destroyed in the End Phase.
#[test]
fn battle_claw_fox_destroys_the_monster_it_battled_in_the_end_phase() {
    let fox = card("Battle Claw Fox");
    let Some(mut s) = field(&[], &[mon(0, fox), mon_def(1, MYSTICAL_ELF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(fox)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", fox), s.count(1, "mzone", MYSTICAL_ELF)), (1, 1), "the battle destroyed neither: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1200, "{}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MYSTICAL_ELF), 1, "the Elf was destroyed in the End Phase: {}", s.context());
    assert_eq!(s.count(0, "mzone", fox), 1, "{}", s.context());
}

/// New Year Drum: "When the equipped monster is targeted for an attack: Special Summon 1 "Battle Claw Fox" from your Deck and
/// switch the attack target to that monster." Team 1's Warwolf attacks team 0's Dark Blade, equipped with the Drum: the Fox
/// (800/1200, face-up Attack) comes from the Deck and takes the attack: it is destroyed and team 0 takes the 1200
/// difference, where Dark Blade (1800) would have lost 200 more. Dark Blade and the Drum stay.
#[test]
fn new_year_drum_brings_a_battle_claw_fox_to_take_the_attack() {
    let [drum, fox] = ["New Year Drum", "Battle Claw Fox"].map(card);
    let Some(mut s) = field(&[fox], &[hand(0, drum), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(drum), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", fox), 1, "the Fox took the attack and lost: {}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(0, "szone", drum)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1200, "2000 against the Fox's 800: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dramatic Pose
// ---------------------------------------------------------------------------------------------------------------

/// Dramatic Pose: "Target 1 face-up monster your opponent controls. Until the End Phase, it loses 500 ATK and all face-up monsters
/// you control gain 500 ATK." Warwolf (2000) falls to 1500, my Dark Blade (1800) and Thunder Kid (700) rise to 2300 and 1200;
/// it all ends with the turn.
#[test]
fn dramatic_pose_takes_500_atk_from_a_target_and_gives_it_to_my_monsters() {
    let pose = card("Dramatic Pose");
    let spawns = [hand(0, pose), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(pose)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(1, WARWOLF), s.attack_of(1, DARK_MAGICIAN)), (Some(1500), Some(2500)), "only the target loses: {}", s.context());
    assert_eq!((s.attack_of(0, DARK_BLADE), s.attack_of(0, THUNDER_KID)), (Some(2300), Some(1200)), "{}", s.context());
    assert_eq!(s.count(0, "grave", pose), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.attack_of(1, WARWOLF), s.attack_of(0, DARK_BLADE), s.attack_of(0, THUNDER_KID)), (Some(2000), Some(1800), Some(700)), "until the End Phase: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Monk Halberd
// ---------------------------------------------------------------------------------------------------------------

/// Monk Halberd: "Equip only to a Warrior-Type monster. It gains 500 ATK until the End Phase. If the equipped monster would be
/// destroyed by battle, destroy this card instead. When this card is destroyed by this effect: inflict 500 damage to your
/// opponent." Dark Blade (Warrior) goes 1800 to 2300 until the End Phase (Warwolf, a Beast-Warrior, cannot be equipped). In
/// turn 2 Team 1's Dark Magician (2500) attacks Dark Blade (back at 1800): the Halberd is destroyed instead, Dark Blade stays,
/// team 1 takes 500 and team 0 takes the 700 battle damage.
#[test]
fn monk_halberd_dies_in_place_of_a_warrior_and_burns_the_opponent() {
    let halberd = card("Monk Halberd");
    let spawns = [hand(0, halberd), mon(0, DARK_BLADE), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run(&[Activate(halberd)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", halberd), 1, "{}", s.context());
    assert_eq!((s.attack_of(0, DARK_BLADE), s.attack_of(0, WARWOLF)), (Some(2300), Some(2000)), "{}", s.context());
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade was not destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", halberd), 1, "the Halberd was destroyed instead: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "2500 against 1800: {}", s.context());
}

/// Monk Halberd: "Equip only to a Warrior-Type monster": with no Warrior (Warwolf is a Beast-Warrior) it cannot be activated.
#[test]
fn monk_halberd_equips_only_a_warrior() {
    let halberd = card("Monk Halberd");
    let Some(mut s) = field(&[], &[hand(0, halberd), mon(0, WARWOLF), mon(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, FILLER)]);
    let err = s.run(&[Activate(halberd)]).expect_err("no Warrior");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "hand", halberd), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Shizuka the Heavenly Dancer, Thousand Strings and Yoshitsune the Goblin of Beauty
// ---------------------------------------------------------------------------------------------------------------

/// Shizuka the Heavenly Dancer: "When this card is targeted for an attack by an opponent's monster: Gain 600 LP." Team 1's
/// Thunder Kid (700) attacks her (300 ATK): team 0 gains 600 and takes the 400 battle damage, 8200 in all, and she is
/// destroyed.
#[test]
fn shizuka_gains_600_life_points_when_attacked() {
    let shizuka = card("Shizuka the Heavenly Dancer");
    let Some(mut s) = field(&[], &[mon(0, shizuka), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 + 600 - 400, "{}", s.context());
    assert_eq!(s.count(0, "grave", shizuka), 1, "{}", s.context());
}

/// Thousand Strings: "When an opponent's monster declares an attack: Negate the attack and the ATK of all face-up monsters on
/// the field becomes 0." Team 1's Warwolf attacks: no damage, and Dark Blade, Warwolf and Dark Magician all have 0 ATK
/// (which lasts). It is not offered at my own attack.
#[test]
fn thousand_strings_negates_the_attack_and_zeroes_every_monster() {
    let strings = card("Thousand Strings");
    let spawns = [set(0, strings), mon(0, DARK_BLADE), mon(1, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(strings)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, DARK_BLADE), s.attack_of(1, WARWOLF), s.attack_of(1, DARK_MAGICIAN)), (Some(0), Some(0), Some(0)), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", strings), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.attack_of(0, DARK_BLADE), s.attack_of(1, WARWOLF)), (Some(0), Some(0)), "it lasts: {}", s.context());
    let Some(mut s) = field(&[], &[set(0, strings), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(DARK_BLADE), ChainOnAttack(strings)], &[EndTurn]]).expect_err("my own attack");
    assert!(err.contains("never became possible"), "{err}");
}

/// Yoshitsune the Goblin of Beauty: "When this card is targeted for an attack by an opponent's monster with 1000 or more original
/// ATK: Change this card to Defense Position. Once per turn, this face-up Defense Position card cannot be destroyed by battle."
/// Warwolf (2000) attacks Yoshitsune (800/800): he turns to Defense and is not destroyed; Dark Blade's attack in the same turn
/// destroys him. Thunder Kid (700, under 1000) leaves him in Attack Position, and loses the battle.
#[test]
fn yoshitsune_turns_to_defense_and_survives_one_battle_a_turn() {
    let yoshitsune = card("Yoshitsune the Goblin of Beauty");
    let Some(mut s) = field(&[], &[mon(0, yoshitsune), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, yoshitsune), Some(FACEUP_DEFENSE), "{}", s.context());
    assert_eq!((s.count(0, "mzone", yoshitsune), s.lp(0)), (1, 8000), "not destroyed, no damage: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, yoshitsune), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", yoshitsune), 1, "once per turn: the second attack destroyed him: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, yoshitsune), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, yoshitsune), Some(FACEUP_ATTACK), "{}", s.context());
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.lp(1)), (1, 8000 - 100), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Buried Soul Talisman
// ---------------------------------------------------------------------------------------------------------------

/// Buried Soul Talisman: "During the turn 2 or more monsters you controlled were destroyed by battle and sent to the Graveyard:
/// Destroy the monster(s) that destroyed those monsters, then Special Summon 1 Level 5 or higher monster from your hand."
/// Team 1's Warwolf and Dark Blade each destroy one of team 0's Thunder Kid and Mystical Elf; the Talisman, chained at the
/// first chance it can be, destroys both attackers and Dark Magician (Level 7) comes from the hand. After only one
/// destroyed monster it is not offered.
#[test]
fn buried_soul_talisman_destroys_the_destroyers_and_summons_from_the_hand() {
    let talisman = card("Buried Soul Talisman");
    let spawns = [set(0, talisman), mon(0, THUNDER_KID), mon(0, MYSTICAL_ELF), hand(0, DARK_MAGICIAN), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(talisman)], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", THUNDER_KID), s.count(0, "grave", MYSTICAL_ELF)), (1, 1), "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "the destroyers were destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", talisman), 1, "{}", s.context());
    // One destroyed monster is not enough.
    let spawns = [set(0, talisman), mon(0, THUNDER_KID), hand(0, DARK_MAGICIAN), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(talisman)], &[Attack(WARWOLF)]]).expect_err("one monster only");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Magic Mirror Spirits, the Silver Spirits and the Mirror cards
// ---------------------------------------------------------------------------------------------------------------

/// Magic Mirror Spirit Asogi and Nayuta: "When this card is Normal Summoned, you can target 1 "Magic Mirror Spirit Nayuta"
/// (Asogi, for Nayuta) in your Graveyard, and Special Summon it." Asogi (Level 6, Tributing Thunder Kid) brings Nayuta
/// (2600/1400) from team 0's Graveyard, and Nayuta brings Asogi (1500/2500). A partner in the opponent's Graveyard, or none,
/// leaves the effect unused.
#[test]
fn the_magic_mirror_spirits_bring_each_other_from_the_graveyard() {
    let [asogi, nayuta] = ["Magic Mirror Spirit Asogi", "Magic Mirror Spirit Nayuta"].map(card);
    for (summoned, partner, atk, def) in [(asogi, nayuta, 2600, 1400), (nayuta, asogi, 1500, 2500)] {
        let Some(mut s) = field(&[], &[hand(0, summoned), mon(0, THUNDER_KID), grave(0, partner)]) else { return };
        keep_hands(&mut s);
        s.run(&[Summon(summoned), Chain(summoned)]).unwrap();
        clean(&s);
        assert_eq!((s.count(0, "mzone", summoned), s.count(0, "mzone", partner)), (1, 1), "{}", s.context());
        let p = stats(&s, 0, partner);
        assert_eq!((p.0, p.1, p.2), (Some(atk), Some(def), Some(6)), "{}", s.context());
        assert_eq!(s.position_of(0, partner), Some(FACEUP_ATTACK), "{}", s.context());
        assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the Tribute: {}", s.context());
    }
    // The partner is in the opponent's Graveyard: no effect.
    let Some(mut s) = field(&[], &[hand(0, asogi), mon(0, THUNDER_KID), grave(1, nayuta)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Summon(asogi), Chain(asogi)]).expect_err("not my Graveyard");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "grave", nayuta), 1, "{}", s.context());
}

/// Silver Spirit - Ukyo and Sakyo: "When this card is Summoned, you can Special Summon 1 "Silver Spirit - Sakyo" (Ukyo, for
/// Sakyo) from your Graveyard." Ukyo (Level 3) brings Sakyo (Level 4, 600/800) and the other way round (Ukyo, 800/600).
#[test]
fn the_silver_spirits_bring_each_other_from_the_graveyard() {
    let [ukyo, sakyo] = ["Silver Spirit Ukyo", "Silver Spirit Sakyo"].map(card);
    for (summoned, partner, atk, def, level) in [(ukyo, sakyo, 600, 800, 4), (sakyo, ukyo, 800, 600, 3)] {
        let Some(mut s) = field(&[], &[hand(0, summoned), grave(0, partner)]) else { return };
        keep_hands(&mut s);
        s.run(&[Summon(summoned), Chain(summoned)]).unwrap();
        clean(&s);
        assert_eq!((s.count(0, "mzone", summoned), s.count(0, "mzone", partner)), (1, 1), "{}", s.context());
        let p = stats(&s, 0, partner);
        assert_eq!((p.0, p.1, p.2), (Some(atk), Some(def), Some(level)), "{}", s.context());
        assert_eq!(s.count(0, "grave", partner), 0, "{}", s.context());
    }
}

/// Mirror Bind: "When an opponent's monster declares an attack if you control a "Silver Spirit - Ukyo" and "Silver Spirit -
/// Sakyo": Destroy all monsters your opponent controls with original ATK equal to or less than the ATK of the attacking monster."
/// Team 1's Warwolf (2000) attacks: it and Thunder Kid (700) are destroyed, Dark Magician (2500) stays, and I take no damage.
/// With only Ukyo on my side it cannot be chained.
#[test]
fn mirror_bind_destroys_the_monsters_that_are_no_stronger_than_the_attacker() {
    let [bind, ukyo, sakyo] = ["Mirror Bind", "Silver Spirit Ukyo", "Silver Spirit Sakyo"].map(card);
    let theirs = [mon(1, WARWOLF), mon(1, THUNDER_KID), mon(1, DARK_MAGICIAN)];
    let spawns = [vec![set(0, bind), mon(0, ukyo), mon(0, sakyo)], theirs.to_vec()].concat();
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(bind)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", THUNDER_KID)), (1, 1), "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "2500 is above the attacker's 2000: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let spawns = [vec![set(0, bind), mon(0, ukyo)], theirs.to_vec()].concat();
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(bind)], &[EndTurn, Attack(WARWOLF)]]).expect_err("no Sakyo");
    assert!(err.contains("never became possible"), "{err}");
}

/// Mirror Call: "If you control a "Silver Spirit - Ukyo" and "Silver Spirit - Sakyo": Target 1 Trap in your Graveyard; add it to
/// your hand." Trap Hole comes back (a Spell in the Graveyard is no target); with only Ukyo it is not offered.
#[test]
fn mirror_call_returns_a_trap_from_the_graveyard() {
    let [call, ukyo, sakyo, trap_hole] = ["Mirror Call", "Silver Spirit Ukyo", "Silver Spirit Sakyo", "Trap Hole"].map(card);
    let spawns = [hand(0, call), mon(0, ukyo), mon(0, sakyo), grave(0, trap_hole), grave(0, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DIAN_KETO), (0, trap_hole)]);
    s.run(&[Activate(call)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", trap_hole), s.count(0, "grave", trap_hole)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "only a Trap can be chosen: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, call), mon(0, ukyo), grave(0, trap_hole)]) else { return };
    s.prefer(&[(0, FILLER)]);
    let err = s.run(&[Activate(call)]).expect_err("no Sakyo");
    assert!(err.contains("never became possible"), "{err}");
}

/// Mirror Route: "Target 1 monster you control with 1000 or less ATK. It can attack directly this turn." In turn 3 Thunder Kid
/// (700) attacks directly past team 1's Dark Blade: 700 damage.
#[test]
fn mirror_route_lets_a_weak_monster_attack_directly() {
    let route = card("Mirror Route");
    let Some(mut s) = field(&[], &[hand(0, route), mon(0, THUNDER_KID), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(route), Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
}

const OLD_VINDICTIVE_MAGICIAN: u16 = 5651; // Flip: destroy 1 monster on the field (it targets)
const MYSTICAL_SPACE_TYPHOON: u16 = 4909;
const MAGIC_JAMMER: u16 = 4862; // Counter Trap: discard 1 card; negate a Spell Card's activation

/// Silver Spirit Redirection: "Negate the activation of an opponent's Effect Monster that targets 1 monster you control, and banish
/// that Effect Monster." Team 1 flips Old Vindictive Magician (its flip effect targets my Warwolf): the effect is negated,
/// Warwolf stays and Old Vindictive Magician is banished.
#[test]
fn silver_spirit_redirection_negates_a_targeting_monster_effect_and_banishes_it() {
    let redirection = card("Silver Spirit Redirection");
    let spawns = [set(0, redirection), mon(0, WARWOLF), mon_set(1, OLD_VINDICTIVE_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(redirection)], &[Reposition(OLD_VINDICTIVE_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "the effect was negated: {}", s.context());
    assert_eq!(s.count(1, "removed", OLD_VINDICTIVE_MAGICIAN), 1, "banished: {}", s.context());
    assert_eq!(s.count(0, "grave", redirection), 1, "{}", s.context());
    // The control: without it the flip effect destroys Warwolf.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon_set(1, OLD_VINDICTIVE_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[EndTurn], &[Reposition(OLD_VINDICTIVE_MAGICIAN)]]).unwrap();
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// Pure Pupil: "Activate only when your opponent activates an Effect Monster's effect while you control a monster with 1000 or
/// less original ATK: Negate that effect, and destroy that monster." Thunder Kid (700) makes it possible when team 1 flips Old
/// Vindictive Magician: Warwolf is spared and the Magician is destroyed. With only Warwolf (2000) on my side it cannot be chained.
#[test]
fn pure_pupil_negates_a_monster_effect_and_destroys_the_monster() {
    let pupil = card("Pure Pupil");
    let spawns = [set(0, pupil), mon(0, THUNDER_KID), mon(0, WARWOLF), mon_set(1, OLD_VINDICTIVE_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(pupil)], &[Reposition(OLD_VINDICTIVE_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "mzone", THUNDER_KID)), (1, 1), "the effect was negated: {}", s.context());
    assert_eq!(s.count(1, "grave", OLD_VINDICTIVE_MAGICIAN), 1, "destroyed: {}", s.context());
    let spawns = [set(0, pupil), mon(0, WARWOLF), mon_set(1, OLD_VINDICTIVE_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    let err = s.run_both([&[Chain(pupil)], &[Reposition(OLD_VINDICTIVE_MAGICIAN)]]).expect_err("no monster of 1000 ATK or less");
    assert!(err.contains("never became possible"), "{err}");
}

/// Soul Mirror: "Send 1 card you control to the Graveyard to target 1 monster in your Graveyard. Special Summon that monster."
/// Thunder Kid is the cost and Dark Magician comes back from the Graveyard.
#[test]
fn soul_mirror_sends_a_card_to_bring_back_a_monster() {
    let mirror = card("Soul Mirror");
    let Some(mut s) = field(&[], &[set(0, mirror), mon(0, THUNDER_KID), grave(0, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run(&[Activate(mirror)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", THUNDER_KID), s.count(0, "mzone", THUNDER_KID)), (1, 0), "the cost: {}", s.context());
    assert_eq!(s.count(0, "grave", mirror), 1, "{}", s.context());
}

/// Allegro Toile: "When you Normal Summon a monster: Target 1 Spell/Trap your opponent controls; destroy it. This card cannot be
/// negated by the effects of other Spells/Traps." Set in turn 1, chained to my Thunder Kid's Normal Summon in turn 3 it
/// destroys team 1's Dian Keto (Set). Team 1's Magic Jammer is chained and cannot negate it (the script lacked the clause:
/// engine/overrides/c511000134.lua): Toile resolves and Dian Keto is destroyed.
#[test]
fn allegro_toile_destroys_a_spell_or_trap_when_i_normal_summon_and_cannot_be_negated_by_one() {
    let toile = card("Allegro Toile");
    let Some(mut s) = field(&[], &[set(0, toile), hand(0, THUNDER_KID), set(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Summon(THUNDER_KID), Chain(toile)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", toile), 1, "{}", s.context());
    // Magic Jammer (negate a Spell Card's activation, discarding a card) is there for team 1: it is chained and has nothing to negate.
    let spawns = [set(0, toile), hand(0, THUNDER_KID), set(1, DIAN_KETO), set(1, MAGIC_JAMMER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DIAN_KETO), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Summon(THUNDER_KID), Chain(toile)], &[Chain(MAGIC_JAMMER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MAGIC_JAMMER), 1, "Magic Jammer was chained: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "and negated nothing: Toile resolved: {}", s.context());
    assert_eq!(s.count(0, "grave", toile), 1, "{}", s.context());
    // The control: Magic Jammer does negate a Spell without the clause (team 0's Dian Keto).
    let Some(mut s) = field(&[], &[hand(0, DIAN_KETO), set(1, MAGIC_JAMMER)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, FILLER), (0, FILLER)]);
    s.run_both([&[Activate(DIAN_KETO)], &[Chain(MAGIC_JAMMER)]]).unwrap();
    assert_eq!(s.lp(0), 8000, "Dian Keto was negated: {}", s.context());
}

/// Angel Blast: "When your opponent activates a Spell/Trap Card or Effect Monster's effect that would destroy a monster(s) on the
/// field: Negate its activation, and if you do, destroy it." Team 1's Dark Hole is negated and destroyed: Warwolf and Thunder
/// Kid stay. Monster Reborn does not destroy: it cannot be chained. Old Vindictive Magician's flip effect (a monster effect
/// that destroys a monster) is negated and the Magician destroyed.
#[test]
fn angel_blast_negates_a_destroying_spell_or_effect_and_destroys_its_source() {
    let blast = card("Angel Blast");
    let spawns = [set(0, blast), mon(0, WARWOLF), hand(1, DARK_HOLE), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(blast)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", THUNDER_KID)), (1, 1), "Dark Hole was negated: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "and destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", blast), 1, "{}", s.context());
    // A monster effect.
    let spawns = [set(0, blast), mon(0, WARWOLF), mon_set(1, OLD_VINDICTIVE_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(blast)], &[Reposition(OLD_VINDICTIVE_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", OLD_VINDICTIVE_MAGICIAN), 1, "{}", s.context());
    // Monster Reborn destroys nothing.
    let spawns = [set(0, blast), hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(blast)], &[Activate(MONSTER_REBORN)]]).expect_err("nothing destroyed");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Angel Wing: "If the equipped monster inflicts battle damage to your opponent: Inflict 300 damage to your opponent. If this
/// face-up card is sent to the Graveyard: Draw 1 card." Warwolf with Angel Wing destroys team 1's Thunder Kid: 1300 battle
/// damage and 300 more (6400 left). The second effect: when Dark Hole destroys the equipped monster the Wing goes with it and
/// team 0 draws a card (as many cards in hand as with the Wing never played).
#[test]
fn angel_wing_adds_300_damage_and_draws_when_it_goes_to_the_graveyard() {
    let wing = card("Angel Wing");
    let Some(mut s) = field(&[], &[hand(0, wing), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Activate(wing), EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1300 - 300, "{}", s.context());
    assert_eq!(s.count(0, "szone", wing), 1, "{}", s.context());
    // The draw: the Wing is in the Graveyard and team 0 holds one card more than before Dark Hole (no discard, no turn draw yet).
    let spawns = [hand(0, wing), mon(0, WARWOLF), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(wing)]).unwrap();
    let before = hand_size(&s, 0);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", wing), 1, "{}", s.context());
    assert_eq!(s.prompt()["player"], 1, "still team 1's turn 2: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "the Wing drew a card: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Cold Sleeper, Illusion Ice Sculpture, White Blizzard, White Night Fort and Snow Fairy
// ---------------------------------------------------------------------------------------------------------------

/// Cold Sleeper: "When a monster you control is destroyed by battle and sent to the Graveyard: You can choose 1 of your unused
/// Monster Zones; Special Summon the destroyed monster at the end of the Battle Phase, but it cannot change its battle
/// position, also, the chosen Monster Zone cannot be used for the rest of this Duel." Team 1's Warwolf destroys Dark Blade:
/// Dark Blade is back before Main Phase 2; in turn 3 it can't change position (Cold Sleeper can) and, with four monsters
/// and the lost zone, no monster can be Summoned (without the effect the fifth zone is free).
#[test]
fn cold_sleeper_brings_back_the_destroyed_monster_and_takes_a_zone() {
    let sleeper = card("Cold Sleeper");
    let board = [mon(0, sleeper), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(0, MYSTICAL_ELF)];
    let spawns = [board.to_vec(), vec![mon(1, WARWOLF)]].concat();
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[Chain(sleeper)], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "still in the Battle Phase: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade came back at the end of the Battle Phase: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 200, "{}", s.context());
    assert!(!offered(&s, "pos_changes").contains(&DARK_BLADE.into()), "{:?}", offered(&s, "pos_changes"));
    assert!(offered(&s, "pos_changes").contains(&sleeper.into()), "{:?}", offered(&s, "pos_changes"));
    assert!(!offered(&s, "summons").contains(&FILLER.into()), "the fifth zone is gone: {:?}", offered(&s, "summons"));
    // The control: with no Cold Sleeper effect the same board has a free zone.
    let Some(mut s) = field(&[], &[board[1], board[2], board[3], mon(0, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "summons").contains(&FILLER.into()), "{:?}", offered(&s, "summons"));
}

/// Illusion Ice Sculpture: "When this card is Normal or Flip Summoned: Target 1 monster you control; this card's ATK/DEF become
/// equal to that monster's original ATK/DEF. Other monsters you control cannot be targeted for an attack." Summoned with Dark
/// Magician and Dark Blade on the field and Dark Magician chosen it is 2500/2100; team 1's Warwolf (2000) can only attack it
/// (even though Dark Blade, 1800, is preferred as the target) and loses 500.
#[test]
fn illusion_ice_sculpture_copies_a_monster_and_shields_the_others() {
    let sculpture = card("Illusion Ice Sculpture");
    let spawns = [hand(0, sculpture), mon(0, DARK_MAGICIAN), mon(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run(&[Summon(sculpture)]).unwrap();
    clean(&s);
    let t = stats(&s, 0, sculpture);
    assert_eq!((t.0, t.1, t.2), (Some(2500), Some(2100), Some(1)), "{}", s.context());
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf hit the Sculpture: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 500), "{}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(0, "mzone", DARK_MAGICIAN)), (1, 1), "{}", s.context());
}

/// White Blizzard: "If an opponent's monster is destroyed by battle: Inflict 600 damage to your opponent. When this card is
/// destroyed: Target 1 card you control; destroy it." Warwolf destroys Thunder Kid: 1300 battle damage and 600 more (6100).
/// In turn 2 Team 1's Mystical Space Typhoon destroys the Blizzard and Warwolf, the card I choose, is destroyed with it.
#[test]
fn white_blizzard_burns_for_a_battle_kill_and_takes_a_card_with_it() {
    let blizzard = card("White Blizzard");
    let Some(mut s) = field(&[], &[hand(0, blizzard), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(blizzard), EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1300 - 600, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, blizzard), mon(0, WARWOLF), hand(1, MYSTICAL_SPACE_TYPHOON)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, blizzard), (0, WARWOLF)]);
    s.run_both([&[Activate(blizzard), EndTurn], &[Activate(MYSTICAL_SPACE_TYPHOON)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", blizzard), s.count(0, "grave", WARWOLF)), (1, 1), "{}", s.context());
}

/// Whether the other team's Set Just Desserts gets a chance to be chained when `summoner` Normal Summons Thunder Kid in its own
/// turn (turn 2 for team 1, turn 3 for team 0), with White Night Fort on team 0's side or without it.
fn chain_chance_at_a_summon(with_fort: bool, summoner: usize) -> bool {
    let [fort, desserts] = ["White Night Fort", "Just Desserts"].map(card);
    let mut spawns = vec![set(1 - summoner as u32, desserts), hand(summoner as u32, THUNDER_KID)];
    if with_fort {
        spawns.push(hand(0, fort));
    }
    let Some(mut s) = field(&[], &spawns) else { return true };
    keep_hands(&mut s);
    let turn_one: &[Action] = if with_fort { &[Activate(fort), EndTurn] } else { &[EndTurn] };
    // Up to the summoner's idle prompt: team 1's in turn 2, team 0's in turn 3.
    s.run_both([turn_one, if summoner == 0 { &[EndTurn] } else { &[] }]).unwrap();
    let i = s.prompt()["summons"].as_array().unwrap().iter().position(|c| c["cid"] == THUNDER_KID).unwrap();
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::SUMMON, "index": i})).unwrap();
    for _ in 0..6 {
        let p = s.prompt();
        if p["type"] == msg::SELECT_CHAIN && p["player"] == 1 - summoner && offered(&s, "selects").contains(&desserts.into()) {
            return true;
        }
        if p["type"] == msg::SELECT_IDLECMD {
            return false;
        }
        s.pass().unwrap();
    }
    panic!("no end: {}", s.context());
}

/// White Night Fort: "Neither player can activate Trap Cards during their opponent's turn." With the Fort out, team 0's Set Just
/// Desserts is not offered to be chained when team 1 Summons in turn 2 (it is without the Fort), and team 1's is not offered
/// at my Summon in turn 3. In my own turn 3 I can still activate mine (500 for each of team 1's monsters).
#[test]
fn white_night_fort_stops_traps_in_the_opponents_turn() {
    let [fort, desserts] = ["White Night Fort", "Just Desserts"].map(card);
    for summoner in [1, 0] {
        assert!(chain_chance_at_a_summon(false, summoner), "the control: the other team's Trap at team {summoner}'s Summon");
        assert!(!chain_chance_at_a_summon(true, summoner), "the Fort: the other team's Trap at team {summoner}'s Summon");
    }
    let spawns = [hand(0, fort), set(0, desserts), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(fort), EndTurn, Activate(desserts)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500, "my own turn: {}", s.context());
}

/// Snow Fairy: "Your opponent cannot activate Spell Cards from their hand, nor activate Spell Cards during the turn they are Set."
/// Team 1's Dian Keto cannot be activated from the hand while she stands (it can without her), but a Dian Keto Set in turn 2
/// can be activated in turn 4 (9000).
#[test]
fn snow_fairy_stops_the_opponent_activating_spells_from_the_hand() {
    let fairy = card("Snow Fairy");
    let Some(mut s) = field(&[], &[mon(0, fairy), hand(1, DIAN_KETO)]) else { return };
    s.prefer(&[(1, FILLER)]);
    let err = s.run_both([&[], &[Activate(DIAN_KETO)]]).expect_err("not from the hand");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, fairy), hand(1, DIAN_KETO)]) else { return };
    s.prefer(&[(1, FILLER)]);
    s.run_both([&[], &[SetSpell(DIAN_KETO), Activate(DIAN_KETO)]]).unwrap();
    assert_eq!(s.lp(1), 9000, "Set and activated two turns later: {}", s.context());
    let Some(mut s) = field(&[], &[hand(1, DIAN_KETO)]) else { return };
    s.prefer(&[(1, FILLER)]);
    s.run_both([&[], &[Activate(DIAN_KETO)]]).unwrap();
    assert_eq!(s.lp(1), 9000, "the control: {}", s.context());
}
