//! Anime-only cards, group 20 (rows 5859-5882 of engine/added/cards.tsv): Skull Knight (character)'s, Titan's, Yusuke
//! Fujiwara's, Axel Brodie's, Frost's and Nightshroud's cards (Reverse Effector, Undead Lineage, the Fiends, Clear
//! Sacrifice, Nihilistic Summoning Technique, Dimensional Catapult, Volcanic Cyclone, Mirror of Duality, Attachment Dragon,
//! the swords, Soul Guide, Cursed Waters Level 3, the Goblins, Scrap Fusion, Sealer Formation, Resurrection Tribute, Stage
//! Select, Perfect Counter Code 123, Gamble Angel Bunny, Jewel Sword, Sand Doodlebug and Grim Reaper). These cards exist
//! only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through
//! the real core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime
//! ones. Every Trap and every Quick-Play Spell answering the opponent is Set first, and neither player has a Battle Phase
//! in turn 1, so team 0's attacks wait for turn 3. A coin's result comes from the duel's seed: Gamble Angel Bunny runs
//! over seeds until both calls have come up.
//! Covered: no Token row in this batch; the cards the per-card fuzz flagged (Reverse Effector, Clear Sacrifice,
//! Nihilistic Summoning Technique, Dimensional Catapult, Volcanic Cyclone, Scrap Fusion, Perfect Counter Code 123, Jewel
//! Sword, Sand Doodlebug: each needs a field the random duels never build, a Set monster targeted, a Clear monster and two
//! more in the Graveyard, a banished Level 4 monster, the Dragon in the Graveyard, Pyro monsters in the hand and Spells
//! and Traps to hit, Levels 1, 2 and 3 on the field, a destroyed Equip Card, and Wasteland or two Level 3 monsters); the
//! card with a script note (Soul Guide: its cost can only be paid with a Deck copy left to add, so it never stops on
//! the Lua error the note feared; both options and the no-copy case are tried here, no override); the signature cards
//! (Mirror of Duality, Goblin Dancers, Gamble Angel Bunny, Banderillero Fiend, Half Goblin, Soul Guide) and the seeded
//! 30% of the rest (Attachment Dragon, Picador Fiend, Resurrection Tribute, Stage Select), with the other cards of the
//! batch on the way (Great Sword, Cursed Waters Level 3, Sealer Formation).
//! No override: Resurrection Tribute's flag is cleared at the first Adjust of every turn (an Adjust effect's once per turn),
//! so "this turn" holds; Soul Guide's cost cannot leave nothing to choose.
//! Not coverable: Grim Reaper, a Normal Monster whose text is only flavour; Undead Lineage's "you can send this card to the
//! Graveyard instead" is a yes/no the scenario harness answers by itself.
//! Scrap Fusion takes its materials from the opponent's Graveyard and monsters, and Clear Sacrifice banishes Clear
//! monsters from the Graveyard and the field (the scripts'; the texts name the Graveyard only): no override, listed in the
//! report. Dimensional Catapult's Cannon must have been properly Summoned once (its revive limit), so its test Contact
//! Fusion Summons it and destroys it with Dark Hole first.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 2, 700/600 vanilla
const GRAVE_13: u16 = 4138; // The 13th Grave: Level 3, 1200/900 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const HEAVY_STORM: u16 = 4891; // destroys every Spell and Trap on the field
const HINOTAMA_SOUL: u16 = 4216; // Level 2 Pyro, 600/500 vanilla
const KURIBOH: u16 = 4064; // Level 1
const GAIA_KNIGHT: u16 = 4044; // Level 7 Warrior, 2300/2100 vanilla (two Tributes)
const UMI: u16 = 4340;
const WASTELAND: u16 = 4337;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }
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

fn deck_size(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap()
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// The positions of a team's monsters with this cid.
fn positions(s: &Scenario, team: usize, cid: u16) -> Vec<u64> {
    s.state()[team]["mzone"].as_array().unwrap().iter().filter(|c| c["cid"] == cid).map(|c| c["position"].as_u64().unwrap()).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// Reverse Effector
// ---------------------------------------------------------------------------------------------------------------

/// Reverse Effector: "When a face-down Defense Position monster on the field is targeted by a Spell/Trap Card: Flip the
/// face-down Defense Position monster to face-up Attack Position, and negate its activation, and if you do, destroy it, then,
/// send the flipped monster to the Graveyard." Team 0's Set Warwolf is targeted by team 1's Raigeki Break (a Trap, Set
/// first): the Trap is negated and destroyed (it is in team 1's Graveyard, its own destruction not having happened), and
/// Warwolf, flipped, is sent to team 0's Graveyard. The controls: the same Trap on a face-up monster, and a Spell that
/// targets nothing (Dark Hole).
#[test]
fn reverse_effector_flips_a_targeted_set_monster_and_negates_the_spell() {
    let [effector, raigeki] = ["Reverse Effector", "Raigeki Break"].map(card);
    let Some(mut s) = field(&[], &[set(0, effector), mon_set(0, WARWOLF), set(1, raigeki), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (1, FILLER)]);
    s.run_both([&[Chain(effector)], &[Activate(raigeki)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the flipped monster went to the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(1, "grave", raigeki), 1, "the Trap was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", effector), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "Raigeki Break itself was negated: {}", s.context());
    // A face-up monster is no target for it.
    let Some(mut s) = field(&[], &[set(0, effector), mon(0, WARWOLF), set(1, raigeki)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (1, FILLER)]);
    let err = s.run_both([&[Chain(effector)], &[Activate(raigeki)]]).expect_err("a face-up monster");
    assert!(err.contains("never became possible"), "{err}");
    // A Spell with no target.
    let Some(mut s) = field(&[], &[set(0, effector), mon_set(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(effector)], &[Activate(DARK_HOLE)]]).expect_err("nothing is targeted");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Banderillero Fiend and Picador Fiend
// ---------------------------------------------------------------------------------------------------------------

/// Banderillero Fiend: "When this card is Normal Summoned: You can target 1 monster on the field; destroy it. Cannot attack
/// the turn it is Summoned." Summoned in turn 1, it destroys team 1's Warwolf; summoned in turn 3 it cannot attack, though
/// the field is empty then.
#[test]
fn banderillero_fiend_destroys_a_monster_and_cannot_attack_that_turn() {
    let fiend = card("Banderillero Fiend");
    let Some(mut s) = field(&[], &[hand(0, fiend), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Summon(fiend), Chain(fiend)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", fiend), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, fiend), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF), (0, FILLER)]);
    let err = s.run_both([&[EndTurn, Summon(fiend), Chain(fiend), Attack(fiend)], &[EndTurn]]).expect_err("cannot attack");
    assert!(err.contains("can't attack"), "{err}");
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Picador Fiend: "While "Dark Arena" is on the field, this card can attack directly." With Dark Arena face-up and only a Set
/// monster on the other side, its attack in turn 3 is a direct one (1600) and the Set Warwolf stays; without the Field Spell the
/// Battle Phase offers no direct attack.
#[test]
fn picador_fiend_attacks_directly_while_dark_arena_is_on_the_field() {
    let [picador, arena] = ["Picador Fiend", "Dark Arena"].map(card);
    let Some(mut s) = field(&[], &[mon(0, picador), spell(0, arena), mon_set(1, WARWOLF)]) else { return };
    s.run(&[EndTurn]).unwrap();
    s.run_to(&[Attack(picador)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    let attack = s.prompt()["attacks"].as_array().unwrap().iter().find(|a| a["cid"] == picador).cloned().expect("Picador can attack");
    assert_eq!(attack["can_direct"], true, "a direct attack is offered: {attack}");
    s.run(&[Attack(picador)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1600, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    // The control: no Dark Arena.
    let Some(mut s) = field(&[], &[mon(0, picador), mon_set(1, WARWOLF)]) else { return };
    s.run(&[EndTurn]).unwrap();
    s.run_to(&[Attack(picador)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    let attack = s.prompt()["attacks"].as_array().unwrap().iter().find(|a| a["cid"] == picador).cloned().expect("Picador can attack");
    assert_ne!(attack["can_direct"], true, "no direct attack past a monster: {attack}");
}

// ---------------------------------------------------------------------------------------------------------------
// Clear Sacrifice
// ---------------------------------------------------------------------------------------------------------------

/// Clear Sacrifice: "Reveal 1 Level 5 or higher "Clear" monster in your hand, banish "Clear" monsters from your Graveyard equal
/// to the number of monsters needed to Tribute Summon the revealed monster; You can Normal Summon that monster without
/// Tributing this turn." Clear Vice Dragon (Level 8, two Tributes) is revealed and Clear Phantom and Clear Cube are banished;
/// it is Normal Summoned with no Tribute (without the Spell it is not even offered). With a single Clear monster in the
/// Graveyard the Spell is not offered.
#[test]
fn clear_sacrifice_banishes_clear_monsters_for_a_tributeless_summon() {
    let [sacrifice, vice, phantom, cube] = ["Clear Sacrifice", "Clear Vice Dragon", "Clear Phantom", "Clear Cube"].map(card);
    let Some(mut s) = field(&[], &[hand(0, sacrifice), hand(0, vice), grave(0, phantom), grave(0, cube)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, vice), (0, phantom), (0, cube)]);
    s.prefer_most(true);
    s.run(&[Activate(sacrifice)]).unwrap();
    assert!(offered(&s, "summons").contains(&vice.into()), "Vice Dragon is offered without Tributes: {:?}", offered(&s, "summons"));
    s.run(&[Summon(vice)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", vice), 1, "{}", s.context());
    assert_eq!((s.count(0, "removed", phantom), s.count(0, "removed", cube)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", sacrifice), 1, "{}", s.context());
    // The control: without the Spell a Level 8 monster needs two Tributes.
    let Some(mut s) = field(&[], &[hand(0, vice)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&vice.into()), "{:?}", offered(&s, "summons"));
    // One Clear monster in the Graveyard is one too few.
    let Some(mut s) = field(&[], &[hand(0, sacrifice), hand(0, vice), grave(0, phantom)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&sacrifice.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Nihilistic Summoning Technique
// ---------------------------------------------------------------------------------------------------------------

/// Nihilistic Summoning Technique: "Target 1 of your banished Level 4 or lower monsters, Special Summon it in Attack Position,
/// and equip this card to it. The equipped monster's original ATK becomes 0. Also, the equipped monster cannot change its
/// battle position. When this card leaves the field, destroy the equipped monster." Warwolf (2000) comes back from the
/// banished zone with 0 ATK, the card beside it, and cannot change position; team 1's Heavy Storm destroys the card and so
/// the monster.
#[test]
fn nihilistic_summoning_technique_revives_a_banished_monster_with_no_atk() {
    let nst = card("Nihilistic Summoning Technique");
    let Some(mut s) = field(&[], &[set(0, nst), banished(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(nst)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(0, "szone", nst), 1, "the card stays as the monster's equip: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(0), "original ATK 0: {}", s.context());
    assert_eq!(s.position_of(0, WARWOLF), Some(1), "Attack Position: {}", s.context());
    // It cannot change its battle position (in turn 3 the monster could normally).
    let err = s.run(&[EndTurn, Reposition(WARWOLF)]).expect_err("no position change");
    assert!(err.contains("never became possible"), "{err}");
    // When the card leaves the field the monster is destroyed.
    let Some(mut s) = field(&[], &[set(0, nst), banished(0, WARWOLF), hand(1, HEAVY_STORM)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(nst)], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", nst), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the equipped monster was destroyed with it: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
}

/// Nihilistic Summoning Technique: "Level 4 or lower": a banished Dark Magician (Level 7) is no target.
#[test]
fn nihilistic_summoning_technique_leaves_a_high_level_monster_banished() {
    let nst = card("Nihilistic Summoning Technique");
    let Some(mut s) = field(&[], &[set(0, nst), banished(0, DARK_MAGICIAN)]) else { return };
    let err = s.run(&[Activate(nst)]).expect_err("Level 7");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Dimensional Catapult
// ---------------------------------------------------------------------------------------------------------------

/// The Cannon's route to the Graveyard in a real duel, for Dimensional Catapult: VWXYZ-Dragon Catapult Cannon has a revive
/// limit (it comes back from the Graveyard only if it was properly Summoned once, which a card the scenario puts in the
/// Graveyard never was), so team 0 Contact Fusion Summons it (banishing VW-Tiger Catapult and XYZ-Dragon Cannon) and the
/// test destroys it with Dark Hole. The fuzz's random duels never do this, so it never reached the Spell. Returns the
/// duel with the Cannon on the field, at team 0's Main Phase.
fn cannon_on_field(extra_spawns: &[Spawn]) -> Option<Scenario> {
    let [cannon, tiger, dragon] = ["VWXYZ-Dragon Catapult Cannon", "VW-Tiger Catapult", "XYZ-Dragon Cannon"].map(card);
    let mut spawns = vec![mon(0, tiger), mon(0, dragon), hand(0, DARK_HOLE)];
    spawns.extend_from_slice(extra_spawns);
    let mut s = field_extra([&[cannon][..], &[][..]], &spawns)?;
    keep_hands(&mut s);
    s.run(&[SpSummon(cannon)]).unwrap();
    assert!(s.count(0, "mzone", cannon) == 1, "the Cannon was Fusion Summoned: {}", s.context());
    Some(s)
}

/// Dimensional Catapult: "Activate this card by targeting 1 "VWXYZ-Dragon Catapult Cannon" in your Graveyard; Special Summon
/// it, ignoring its Summoning conditions, and equip it with this card, but its effects are negated. When this card is
/// destroyed, destroy the equipped monster. If it attacks a Defense Position monster, inflict piercing battle damage to your
/// opponent." VWXYZ (3000/2800) is destroyed by Dark Hole and comes back from the Graveyard; its banish effect, which banishes
/// a monster of team 1's by itself (the control), does nothing now, and in turn 3 it attacks a Set Warwolf (DEF 100): 2900
/// pierce through.
#[test]
fn dimensional_catapult_revives_the_cannon_negated_and_piercing() {
    let [catapult, cannon] = ["Dimensional Catapult", "VWXYZ-Dragon Catapult Cannon"].map(card);
    // The control: the Cannon alone banishes a monster of team 1's.
    let Some(mut s) = cannon_on_field(&[mon_def(1, THUNDER_KID)]) else { return };
    s.prefer(&[(1, THUNDER_KID)]);
    s.run(&[Activate(cannon)]).unwrap();
    assert_eq!(s.count(1, "removed", THUNDER_KID), 1, "{}", s.context());
    let Some(mut s) = cannon_on_field(&[hand(0, catapult), mon_def(1, THUNDER_KID), hand(1, WARWOLF)]) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    assert_eq!(s.count(0, "grave", cannon), 1, "{}", s.context());
    s.run(&[Activate(catapult)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", cannon), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", catapult), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[SetMonster(WARWOLF), EndTurn]]).unwrap();
    assert_eq!(s.prompt()["player"], 0, "team 0's turn 3: {}", s.context());
    s.prefer(&[(1, WARWOLF)]);
    if offered(&s, "activates").contains(&cannon.into()) {
        s.run(&[Activate(cannon)]).unwrap();
    }
    assert_eq!(s.count(1, "removed", WARWOLF), 0, "its effect is negated: {}", s.context());
    s.run(&[Attack(cannon)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - (3000 - 100), "piercing damage: {}", s.context());
}

/// Dimensional Catapult: "When this card is destroyed, destroy the equipped monster." Team 1's Heavy Storm destroys the Equip
/// Spell and the Cannon with it.
#[test]
fn dimensional_catapult_destroyed_takes_the_cannon_with_it() {
    let [catapult, cannon] = ["Dimensional Catapult", "VWXYZ-Dragon Catapult Cannon"].map(card);
    let Some(mut s) = cannon_on_field(&[hand(0, catapult), hand(1, HEAVY_STORM)]) else { return };
    s.run(&[Activate(DARK_HOLE), Activate(catapult)]).unwrap();
    assert_eq!(s.count(0, "mzone", cannon), 1, "{}", s.context());
    s.run_both([&[], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", catapult), s.count(0, "grave", cannon)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "mzone", cannon), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Volcanic Cyclone
// ---------------------------------------------------------------------------------------------------------------

/// Volcanic Cyclone: "Send any number of Pyro-Type monsters from your hand to the Graveyard to target an equal number of
/// Spells/Traps your opponent controls; destroy them. During the End Phase, draw cards equal to the number of monsters sent to
/// the Graveyard for this card's activation." Two Hinotama Souls are sent and both of team 1's Set Spells destroyed, and two
/// cards are drawn in the End Phase (the Deck is two cards shorter); with one Soul only one Spell is destroyed and one card drawn.
#[test]
fn volcanic_cyclone_destroys_as_many_cards_as_pyros_sent_and_draws_as_many() {
    let cyclone = card("Volcanic Cyclone");
    let spawns = [hand(0, cyclone), hand(0, HINOTAMA_SOUL), hand(0, HINOTAMA_SOUL), set(1, DIAN_KETO), set(1, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, HINOTAMA_SOUL)]);
    s.prefer_most(true);
    let before = deck_size(&s, 0);
    s.run(&[Activate(cyclone), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DIAN_KETO), 2, "both Set Spells were destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", HINOTAMA_SOUL), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", cyclone), 1, "{}", s.context());
    assert_eq!(deck_size(&s, 0), before - 2, "two cards drawn in the End Phase: {}", s.context());
    // One Soul: one Spell, one card.
    let spawns = [hand(0, cyclone), hand(0, HINOTAMA_SOUL), set(1, DIAN_KETO), set(1, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, HINOTAMA_SOUL)]);
    let before = deck_size(&s, 0);
    s.run(&[Activate(cyclone), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.count(1, "szone", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(deck_size(&s, 0), before - 1, "{}", s.context());
}

/// Volcanic Cyclone: "Pyro-Type monsters": with none in the hand it is not offered.
#[test]
fn volcanic_cyclone_needs_a_pyro_monster_in_the_hand() {
    let cyclone = card("Volcanic Cyclone");
    let Some(mut s) = field(&[], &[hand(0, cyclone), set(1, DIAN_KETO)]) else { return };
    let err = s.run(&[Activate(cyclone)]).expect_err("no Pyro");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Mirror of Duality
// ---------------------------------------------------------------------------------------------------------------

/// Mirror of Duality: "When an opponent's monster declares an attack: Pay 1000 LP; destroy all monsters on the field, and if you
/// do, your opponent takes 500 damage for each monster destroyed." Team 1's Warwolf attacks in turn 2: team 0 pays 1000
/// (7000), the three monsters on the field (Thunder Kid, Warwolf, Dark Blade) are destroyed and team 1 takes 1500 (6500); no
/// battle damage.
#[test]
fn mirror_of_duality_destroys_every_monster_and_hurts_for_each() {
    let mirror = card("Mirror of Duality");
    let spawns = [set(0, mirror), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(mirror)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1000, 8000 - 1500), "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", mirror), 1, "{}", s.context());
}

/// Mirror of Duality: "an opponent's monster": my own attack in turn 3 does not allow it.
#[test]
fn mirror_of_duality_does_not_answer_the_controllers_own_attack() {
    let mirror = card("Mirror of Duality");
    let Some(mut s) = field(&[], &[set(0, mirror), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(WARWOLF), ChainOnAttack(mirror)], &[EndTurn]]).expect_err("not on my own attack");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", mirror), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Attachment Dragon
// ---------------------------------------------------------------------------------------------------------------

/// Attachment Dragon: "When this card is Summoned, you can equip this card as an Equip Card to a monster your opponent
/// controls. Then, choose the equipped monster's battle position. After that, the equipped monster cannot change its battle
/// position." Warwolf is equipped and put in Defense Position; the Dragon is in team 0's Spell & Trap Zone, and in turn 2
/// team 1 cannot change Warwolf's position.
#[test]
fn attachment_dragon_equips_to_an_opponents_monster_and_fixes_its_position() {
    let dragon = card("Attachment Dragon");
    let Some(mut s) = field(&[], &[hand(0, dragon), mon(1, WARWOLF)]) else { return };
    s.prefer_position(Some(4));
    s.run(&[Summon(dragon), Chain(dragon)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", dragon), 1, "the Dragon is an Equip Card: {}", s.context());
    assert_eq!(s.count(0, "mzone", dragon), 0, "{}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "the chosen position, Defense: {}", s.context());
    // Team 1 cannot change it back.
    let Some(mut s) = field(&[], &[hand(0, dragon), mon(1, WARWOLF)]) else { return };
    s.prefer_position(Some(4));
    let err = s.run_both([&[Summon(dragon), Chain(dragon)], &[Reposition(WARWOLF)]]).expect_err("fixed");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "{}", s.context());
    // The other choice: Attack Position stays Attack Position.
    let Some(mut s) = field(&[], &[hand(0, dragon), mon_def(1, WARWOLF)]) else { return };
    s.prefer_position(Some(1));
    s.run(&[Summon(dragon), Chain(dragon)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, WARWOLF), Some(1), "Attack, as chosen: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Great Sword and Jewel Sword
// ---------------------------------------------------------------------------------------------------------------

/// Great Sword: "Equip only to a Warrior-Type monster. It gains 300 ATK, and can be treated as 2 Tributes for the Tribute Summon
/// of a Warrior-Type monster." Dark Blade (1800) with it is 2100, and as the only monster it is the two Tributes Gaia the
/// Fierce Knight (Level 7, Warrior) needs; without the sword Gaia is not offered. Thunder Kid (not a Warrior) is no target.
#[test]
fn great_sword_adds_atk_and_counts_as_two_tributes_for_a_warrior() {
    let sword = card("Great Sword");
    let Some(mut s) = field(&[], &[hand(0, sword), mon(0, DARK_BLADE), hand(0, GAIA_KNIGHT)]) else { return };
    keep_hands(&mut s);
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&GAIA_KNIGHT.into()), "one Tribute is not enough: {:?}", offered(&s, "summons"));
    s.run(&[Activate(sword)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(2100), "{}", s.context());
    assert!(offered(&s, "summons").contains(&GAIA_KNIGHT.into()), "two Tributes in one: {:?}", offered(&s, "summons"));
    s.run(&[Summon(GAIA_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GAIA_KNIGHT), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(0, "grave", sword)), (1, 1), "the Tribute and its Sword: {}", s.context());
    // Not a Warrior.
    let Some(mut s) = field(&[], &[hand(0, sword), mon(0, THUNDER_KID)]) else { return };
    let err = s.run(&[Activate(sword)]).expect_err("Thunder is no Warrior");
    assert!(err.contains("never became possible"), "{err}");
}

/// Jewel Sword: "Equip only to a Warrior-Type monster. It gains 300 ATK. When this equipped card is sent from the field to the
/// Graveyard: draw 1 card." Dark Blade is 2100 with it; team 1's Heavy Storm destroys it, and team 0 draws one card.
#[test]
fn jewel_sword_adds_atk_and_draws_a_card_when_it_goes() {
    let jewel = card("Jewel Sword");
    let Some(mut s) = field(&[], &[hand(0, jewel), mon(0, DARK_BLADE), hand(1, HEAVY_STORM)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(jewel)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(2100), "{}", s.context());
    let before = deck_size(&s, 0);
    s.run_both([&[], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", jewel), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "only the Sword was destroyed: {}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(1800), "{}", s.context());
    assert_eq!(deck_size(&s, 0), before - 1, "one card drawn: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Soul Guide
// ---------------------------------------------------------------------------------------------------------------

/// A duel whose Deck team 0's is all Warwolf (every Warwolf the Deck holds is a copy for Soul Guide to add).
fn warwolf_deck(spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [WARWOLF, FILLER], &[], spawns) }

/// Soul Guide: "Tribute 1 monster; Choose either its ATK or DEF and gain that amount of LP, then add 1 monster with the same
/// name as the Tributed monster from your Deck to your hand." Warwolf (2000/100) is Tributed with a Deck of Warwolves:
/// choosing ATK gains 2000 (10000), choosing DEF 100 (8100); either way a Warwolf of the Deck is added to the hand and the
/// Tributed one is in the Graveyard.
#[test]
fn soul_guide_gains_the_atk_or_the_def_and_adds_the_same_monster() {
    let guide = card("Soul Guide");
    for (option, lp) in [(0, 10000), (1, 8100)] {
        let Some(mut s) = warwolf_deck(&[hand(0, guide), mon(0, WARWOLF)]) else { return };
        s.choose_options(&[option]);
        let (before, hand_before) = (deck_size(&s, 0), s.count(0, "hand", WARWOLF));
        s.run(&[Activate(guide)]).unwrap();
        clean(&s);
        assert_eq!(s.lp(0), lp, "option {option}: {}", s.context());
        assert_eq!(s.count(0, "hand", WARWOLF), hand_before + 1, "a Warwolf of the Deck is in the hand: {}", s.context());
        assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
        assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
        assert_eq!(s.count(0, "grave", guide), 1, "{}", s.context());
        assert_eq!(deck_size(&s, 0), before - 1, "{}", s.context());
    }
}

/// Soul Guide, the cost leaving nothing to choose: the Tribute must have a copy in the Deck, so with a Dark Blade (no copy
/// in the Deck) as the only monster it is not offered, and with a Warwolf beside it that one is Tributed (the Dark Blade
/// stays) with no Lua error.
#[test]
fn soul_guide_tributes_only_a_monster_with_a_copy_in_the_deck() {
    let guide = card("Soul Guide");
    let Some(mut s) = warwolf_deck(&[hand(0, guide), mon(0, DARK_BLADE)]) else { return };
    let err = s.run(&[Activate(guide)]).expect_err("no copy of Dark Blade in the Deck");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = warwolf_deck(&[hand(0, guide), mon(0, DARK_BLADE), mon(0, WARWOLF)]) else { return };
    let hand_before = s.count(0, "hand", WARWOLF);
    s.run(&[Activate(guide)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "mzone", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "hand", WARWOLF), hand_before + 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Cursed Waters Level 3 and Half Goblin
// ---------------------------------------------------------------------------------------------------------------

/// Cursed Waters Level 3: "If "Umi" is on the field: You can Normal Summon 1 Level 3 or lower monster this turn, in addition to
/// your Normal Summon/Set." In turn 3 with Umi, after one Thunder Kid (Level 3) is Normal Summoned the second one is offered
/// (a Level 4 Feral Imp never is); without the Trap the second is not.
#[test]
fn cursed_waters_level_3_gives_one_more_normal_summon_of_a_low_level_monster() {
    let waters = card("Cursed Waters Level 3");
    let spawns = [set(0, waters), spell(0, UMI), hand(0, THUNDER_KID), hand(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(waters), Summon(THUNDER_KID)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert!(offered(&s, "summons").contains(&THUNDER_KID.into()), "the extra Normal Summon: {:?}", offered(&s, "summons"));
    assert!(!offered(&s, "summons").contains(&FILLER.into()), "not a Level 4: {:?}", offered(&s, "summons"));
    s.run(&[Summon(THUNDER_KID)]).unwrap();
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 2, "both in the same turn: {}", s.context());
    // The control.
    let Some(mut s) = field(&[], &[spell(0, UMI), hand(0, THUNDER_KID), hand(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Summon(THUNDER_KID)]).unwrap();
    assert!(!offered(&s, "summons").contains(&THUNDER_KID.into()), "one Normal Summon a turn: {:?}", offered(&s, "summons"));
}

/// Cursed Waters Level 3: "If "Umi" is on the field": without it the Trap cannot be activated.
#[test]
fn cursed_waters_level_3_needs_umi() {
    let waters = card("Cursed Waters Level 3");
    let Some(mut s) = field(&[], &[set(0, waters), hand(0, THUNDER_KID)]) else { return };
    let err = s.run(&[EndTurn, Activate(waters)]).expect_err("no Umi");
    assert!(err.contains("never became possible"), "{err}");
}

/// Half Goblin: "You can discard 1 Warrior-Type monster; Special Summon this card from your hand." Dark Blade is discarded and
/// Half Goblin (500/500) comes to the field; with no Warrior in the hand it cannot be Special Summoned.
#[test]
fn half_goblin_special_summons_itself_by_discarding_a_warrior() {
    let goblin = card("Half Goblin");
    let Some(mut s) = field(&[], &[hand(0, goblin), hand(0, DARK_BLADE)]) else { return };
    s.prefer(&[(0, DARK_BLADE)]);
    s.run(&[SpSummon(goblin)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", goblin), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the Warrior was discarded: {}", s.context());
    assert_eq!(s.attack_of(0, goblin), Some(500), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, goblin)]) else { return };
    let err = s.run(&[SpSummon(goblin)]).expect_err("no Warrior to discard");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Scrap Fusion
// ---------------------------------------------------------------------------------------------------------------

/// Scrap Fusion: "Fusion Summon 1 Fusion Monster from either player's Extra Deck to your side of the field, by banishing Fusion
/// Materials listed on it from your opponent's Graveyard." Team 1's Extra Deck holds Gaia the Dragon Champion and team 1's
/// Graveyard Gaia The Fierce Knight and Curse of Dragon: team 0 Fusion Summons it from there, the two materials are banished,
/// and the Fusion Monster is team 0's. The same from team 0's own Extra Deck. Materials in team 0's own Graveyard are no use.
#[test]
fn scrap_fusion_summons_from_either_extra_deck_with_the_opponents_graveyard() {
    let [scrap, champion, curse] = ["Scrap Fusion", "Gaia the Dragon Champion", "Curse of Dragon"].map(card);
    for extra in [[&[][..], &[champion][..]], [&[champion][..], &[][..]]] {
        let spawns = [hand(0, scrap), grave(1, GAIA_KNIGHT), grave(1, curse)];
        let Some(mut s) = field_extra(extra, &spawns) else { return };
        keep_hands(&mut s);
        s.run(&[Activate(scrap)]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "mzone", champion), 1, "the Fusion Monster is mine: {}", s.context());
        assert_eq!((s.count(1, "removed", GAIA_KNIGHT), s.count(1, "removed", curse)), (1, 1), "banished from team 1's Graveyard: {}", s.context());
        assert_eq!(s.count(0, "grave", scrap), 1, "{}", s.context());
    }
    let spawns = [hand(0, scrap), grave(0, GAIA_KNIGHT), grave(0, curse)];
    let Some(mut s) = field_extra([&[champion][..], &[][..]], &spawns) else { return };
    let err = s.run(&[Activate(scrap)]).expect_err("materials of my own Graveyard");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Sealer Formation
// ---------------------------------------------------------------------------------------------------------------

/// Sealer Formation: "During your Draw Phase, instead of conducting your normal draw: You can add 1 "Seal" card from your
/// Graveyard to your hand." In turn 3 Block Attack (a "Seal" card) comes from the Graveyard and the Deck is a card longer than
/// with the normal draw (the control).
#[test]
fn sealer_formation_adds_a_seal_card_instead_of_the_draw() {
    let [sealer, block] = ["Sealer Formation", "Block Attack"].map(card);
    let Some(mut s) = field(&[], &[spell(0, sealer), grave(0, block)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", block), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", block), 0, "{}", s.context());
    let kept = deck_size(&s, 0);
    let Some(mut s) = field(&[], &[grave(0, block)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(deck_size(&s, 0), kept - 1, "the control drew its card: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Goblin Dancers
// ---------------------------------------------------------------------------------------------------------------

/// Goblin Dancers: "When this card is Normal Summoned: You can Special Summon up to 2 "Goblin Dancers" from your hand." Three
/// in the hand: one is Normal Summoned and two come with it, all face-up in Attack Position (2000/1000).
#[test]
fn goblin_dancers_bring_two_more_from_the_hand() {
    let dancers = card("Goblin Dancers");
    let Some(mut s) = field(&[], &[hand(0, dancers), hand(0, dancers), hand(0, dancers)]) else { return };
    s.prefer(&[(0, dancers)]);
    s.prefer_most(true);
    s.run(&[Summon(dancers), Chain(dancers)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dancers), 3, "{}", s.context());
    assert_eq!(positions(&s, 0, dancers), vec![1, 1, 1], "{}", s.context());
    assert_eq!(s.attack_of(0, dancers), Some(2000), "{}", s.context());
    assert_eq!(s.count(0, "hand", dancers), 0, "{}", s.context());
}

/// Goblin Dancers: "If a "Goblin Dancers" attacks, all "Goblin Dancers" you control are changed to Defense Position at the end
/// of the Battle Phase, and its battle position cannot be changed until the end of your next turn." In turn 3 one of two
/// attacks directly (2000 damage) and the turn ends; both are in Defense Position then. In turn 5 (the end of its next turn
/// still to come) they cannot change position, in turn 7 they can.
#[test]
fn goblin_dancers_go_to_defense_after_an_attack_and_stay_there() {
    let dancers = card("Goblin Dancers");
    let Some(mut s) = field(&[], &[mon(0, dancers), mon(0, dancers)]) else { return };
    keep_hands(&mut s);
    assert_eq!(positions(&s, 0, dancers), vec![1, 1], "{}", s.context());
    s.run_both([&[EndTurn, Attack(dancers), EndTurn], &[EndTurn, EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.prompt()["player"], 0, "team 0's turn 5: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
    assert_eq!(positions(&s, 0, dancers), vec![4, 4], "both in Defense after the Battle Phase: {}", s.context());
    assert!(!offered(&s, "pos_changes").contains(&dancers.into()), "locked through turn 5: {:?}", offered(&s, "pos_changes"));
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["player"], 0, "team 0's turn 7: {}", s.context());
    assert!(offered(&s, "pos_changes").contains(&dancers.into()), "free in turn 7: {:?}", offered(&s, "pos_changes"));
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Resurrection Tribute
// ---------------------------------------------------------------------------------------------------------------

/// Resurrection Tribute: "If a monster you controlled was destroyed by battle and sent to the Graveyard this turn: Destroy 1
/// monster your opponent controls, also, Special Summon 1 monster from your Graveyard in face-up Defense Position, ignoring
/// its Summoning conditions." Thunder Kid (700) attacks Warwolf (2000) in turn 3 and is destroyed; in the Main Phase 2 the
/// Spell destroys Dark Blade (chosen) and brings Thunder Kid back in Defense Position. Not offered before the battle.
#[test]
fn resurrection_tribute_follows_a_monster_destroyed_in_battle() {
    let tribute = card("Resurrection Tribute");
    let spawns = [mon(0, THUNDER_KID), hand(0, tribute), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["player"], 0, "team 0's turn 3: {}", s.context());
    assert!(!offered(&s, "activates").contains(&tribute.into()), "nothing destroyed yet: {:?}", offered(&s, "activates"));
    s.prefer(&[(1, DARK_BLADE), (0, THUNDER_KID), (0, FILLER)]);
    s.run(&[Attack(THUNDER_KID), Activate(tribute)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "the chosen monster was destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid is back: {}", s.context());
    assert_eq!(s.position_of(0, THUNDER_KID), Some(4), "in Defense Position: {}", s.context());
    assert_eq!(s.count(0, "grave", tribute), 1, "{}", s.context());
}

/// Resurrection Tribute: "this turn": the monster lost in turn 3 is no reason in turn 5 (the script's flag is cleared at the
/// first Adjust of each turn).
#[test]
fn resurrection_tribute_is_not_offered_the_turns_after_the_battle() {
    let tribute = card("Resurrection Tribute");
    let spawns = [mon(0, THUNDER_KID), hand(0, tribute), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(THUNDER_KID), EndTurn], &[EndTurn, EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.prompt()["player"], 0, "team 0's turn 5: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", tribute), 1, "the Spell is still in the hand: {}", s.context());
    assert!(!offered(&s, "activates").contains(&tribute.into()), "the next turn: {:?}", offered(&s, "activates"));
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["player"], 0, "team 0's turn 7: {}", s.context());
    assert!(!offered(&s, "activates").contains(&tribute.into()), "two turns on: {:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Stage Select
// ---------------------------------------------------------------------------------------------------------------

/// Stage Select: "Shuffle your opponent's Deck. Then, move 1 card from your Deck to the top of your Deck." Dark Magician, the
/// bottom card of team 0's Deck, is moved to the top and is the card drawn in turn 3; without the Spell the draw is a Feral Imp.
#[test]
fn stage_select_puts_a_chosen_card_on_top_of_the_deck() {
    let stage = card("Stage Select");
    let Some(mut s) = field(&[DARK_MAGICIAN], &[hand(0, stage)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run_both([&[Activate(stage), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "Dark Magician was drawn: {}", s.context());
    assert_eq!(s.count(0, "grave", stage), 1, "{}", s.context());
    let Some(mut s) = field(&[DARK_MAGICIAN], &[]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Perfect Counter Code 123
// ---------------------------------------------------------------------------------------------------------------

/// Perfect Counter Code 123: "When your opponent Summons a monster while you control exactly 3 face-up monsters whose Levels
/// are 1, 2, and 3 (and no other monsters): Negate that Summon, and destroy it." Kuriboh, Hinotama Soul and The 13th Grave are on
/// team 0's field; team 1's Normal Summon of Warwolf is negated and Warwolf is destroyed. With a fourth monster, or without the
/// Level 1, it cannot be activated.
#[test]
fn perfect_counter_code_negates_and_destroys_a_summon() {
    let code = card("Perfect Counter Code 123");
    let spawns = [set(0, code), mon(0, KURIBOH), mon(0, HINOTAMA_SOUL), mon(0, GRAVE_13), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(code)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the summoned monster was destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", code), 1, "{}", s.context());
    // A fourth monster.
    let spawns = [set(0, code), mon(0, KURIBOH), mon(0, HINOTAMA_SOUL), mon(0, GRAVE_13), mon(0, DARK_BLADE), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(code)], &[Summon(WARWOLF)]]).expect_err("four monsters");
    assert!(err.contains("never became possible"), "{err}");
    // No Level 1.
    let spawns = [set(0, code), mon(0, HINOTAMA_SOUL), mon(0, GRAVE_13), mon(0, DARK_BLADE), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(code)], &[Summon(WARWOLF)]]).expect_err("Levels 2, 3 and 4");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Gamble Angel Bunny
// ---------------------------------------------------------------------------------------------------------------

/// Gamble Angel Bunny: "Once per turn: You can toss a coin and call it. If you call it right, inflict 1000 damage to your
/// opponent. Otherwise, take 1000 damage." Over twenty seeds the call is sometimes right (the opponent at 7000) and sometimes
/// wrong (me at 7000), never both and never anything else; the effect is then used up for the turn.
#[test]
fn gamble_angel_bunny_damages_one_side_by_the_coin() {
    let bunny = card("Gamble Angel Bunny");
    let (mut right, mut wrong) = (0, 0);
    for seed in 1..=20 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[mon(0, bunny)]) else { return };
        s.run(&[Activate(bunny)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        match (s.lp(0), s.lp(1)) {
            (8000, 7000) => right += 1,
            (7000, 8000) => wrong += 1,
            other => panic!("seed {seed}: 1000 damage to one side, got {other:?}: {}", s.context()),
        }
        assert!(!offered(&s, "activates").contains(&bunny.into()), "once per turn: {:?}", offered(&s, "activates"));
    }
    assert!(right > 0 && wrong > 0, "both calls in 20 seeds: right {right}, wrong {wrong}");
}

// ---------------------------------------------------------------------------------------------------------------
// Sand Doodlebug
// ---------------------------------------------------------------------------------------------------------------

/// Sand Doodlebug: "During each player's End Phase: Destroy 1 Level 3 or lower monster on the field." Team 1's Thunder Kid is
/// destroyed at team 0's End Phase and team 0's own at team 1's, the Doodlebug (Level 4) staying throughout.
#[test]
fn sand_doodlebug_destroys_a_low_level_monster_in_each_end_phase() {
    let doodlebug = card("Sand Doodlebug");
    let Some(mut s) = field(&[], &[mon(0, doodlebug), mon(0, THUNDER_KID), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, THUNDER_KID), (0, THUNDER_KID), (0, FILLER)]);
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(0, "mzone", THUNDER_KID)), (1, 1), "one at my End Phase: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "and mine at theirs: {}", s.context());
    assert_eq!(s.count(0, "mzone", doodlebug), 1, "{}", s.context());
}

/// Sand Doodlebug: "This card can attack directly." In turn 3 it hits past team 1's Warwolf (1200 damage) and the Warwolf stays.
#[test]
fn sand_doodlebug_attacks_directly() {
    let doodlebug = card("Sand Doodlebug");
    let Some(mut s) = field(&[], &[mon(0, doodlebug), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn]).unwrap();
    s.run_to(&[Attack(doodlebug)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    let attack = s.prompt()["attacks"].as_array().unwrap().iter().find(|a| a["cid"] == doodlebug).cloned().expect("it can attack");
    assert_eq!(attack["can_direct"], true, "{attack}");
    s.run(&[Attack(doodlebug)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1200, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Sand Doodlebug: "Once per turn, when an opponent's monster declares an attack while "Wasteland" is face-up on the field: You
/// can negate that attack." Team 1's Warwolf (2000) attacks the Doodlebug (1200) in turn 2: with Wasteland face-up the attack
/// is negated (no damage, the Doodlebug stays); without it the Doodlebug has no answer to offer and is destroyed (800 damage).
#[test]
fn sand_doodlebug_negates_an_attack_while_wasteland_is_face_up() {
    let doodlebug = card("Sand Doodlebug");
    let Some(mut s) = field(&[], &[mon(0, doodlebug), mon(1, WARWOLF), spell(0, WASTELAND)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(doodlebug)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "mzone", doodlebug), 1, "{}", s.context());
    // The control: no Wasteland.
    let Some(mut s) = field(&[], &[mon(0, doodlebug), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", doodlebug), 1, "the attack went through: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
}
