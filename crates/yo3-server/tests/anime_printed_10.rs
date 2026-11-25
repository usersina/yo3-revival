//! Scenario tests for the printed anime cards of group 10 (Meklord Astro Dragon Asterisk to Orichalcos Sword of Sealing, rows
//! 4848-5205 of `engine/added/cards.tsv`): each effect through the real core with both seats' client models checked
//! after every step (`duelcheck::scenario`). The printed text is played with anime cards off; the anime version
//! (`start_anime_field`) is tested where it does something else.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The group's cards (client ids: the Konami id, or the stand-in the added row has).
const ASTERISK: u16 = 9486; // Meklord Astro Dragon Asterisk
const MEKANIKLE: u16 = 9553; // Meklord Astro Mekanikle
const METAION: u16 = 9698; // Metaion, the Timelord
const MICHION: u16 = 13815; // Michion, the Timelord
const MIMICAT: u16 = 4800;
const MIRACLE_CONTACT: u16 = 10213;
const MIRACLE_RUPTURE: u16 = 15372;
const MIRACLE_STONE: u16 = 14557;
const MIRROR_FORCE_DRAGON: u16 = 11884;
const MIRROR_RESONATOR: u16 = 12064;
const MISCHIEF: u16 = 13998; // Mischief of the Time Goddess
const MONO_SYNCHRON: u16 = 10309;
const IMPACT_RETURN: u16 = 4784; // Morphtronic Impact Return
const LANTRON: u16 = 9481; // Morphtronic Lantron
const STAPLEN: u16 = 9482; // Morphtronic Staplen
const MOUND: u16 = 11170; // Mound of the Bound Creator
const MULTIPLE_DESTRUCTION: u16 = 5481;
const MYSTIC_PIPER: u16 = 9476;
const NECRO_DEFENDER: u16 = 9475;
const SHIELD_BEARER: u16 = 4665; // Noble Knight's Shield-Bearer
const SPEARHOLDER: u16 = 4672; // Noble Knight's Spearholder
const OCEAN: u16 = 5055; // Ocean of Regeneration
const ORGOTH: u16 = 13800; // Orgoth the Relentless
const SWORD: u16 = 5105; // Orichalcos Sword of Sealing
// The game's own cards they play with.
const ARMY_OF_GRANEL: u16 = 9485; // Meklord Army of Granel: Machine, Level 4, 1600 ATK
const ARMY_OF_WISEL: u16 = 9483; // 1800 ATK
const ARMY_OF_SKIEL: u16 = 9484; // 1200 ATK
const TOON_WORLD: u16 = 4798;
const TOON_MASKED_SORCERER: u16 = 5475; // a Toon monster
const NEOS: u16 = 6653; // Elemental HERO Neos
const GRAND_MOLE: u16 = 6770; // Neo-Spacian Grand Mole
const GRAND_NEOS: u16 = 6990; // Elemental HERO Grand Neos: "Neos" + "Neo-Spacian Grand Mole"
const FOSSIL_FUSION: u16 = 15369;
const GIANT_SOLDIER_OF_STONE: u16 = 4080; // a Level 3 Rock
const DARK_MAGICIAN: u16 = 4041; // Spellcaster, 2500 ATK
const FORTUNE_FAIRY_HIKARI: u16 = 14550;
const FORTUNE_FAIRY_EN: u16 = 14551;
const MIRROR_FORCE: u16 = 4887;
const FANG_OF_CRITIAS: u16 = 11885;
const HEDGEHOG: u16 = 7701; // Quillbolt Hedgehog: Level 2, 800 ATK
const JUNK_SYNCHRON: u16 = 7687; // Tuner, Level 3
const JUNK_WARRIOR: u16 = 7696; // Synchro, Level 5
const STARDUST: u16 = 7734; // Stardust Dragon: a Synchro Monster, 2500 ATK
const FORMULA_SYNCHRON: u16 = 9118; // Level 2 Synchro: 1 Tuner + 1 non-Tuner
const URUQUIZAS: u16 = 7550; // Flamvell Uruquizas: Level 6 Synchro, 1 Tuner + 1+ non-Tuner
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800 ATK, no effect
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const BLUE_EYES: u16 = 4007; // LIGHT, 3000 ATK
const OBELISK: u16 = 4998; // DIVINE, Level 10, 4000 ATK
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;
const MST: u16 = 4909; // Mystical Space Typhoon
const OOKAZI: u16 = 4352; // 800 damage
const CELFON: u16 = 7874; // Morphtronic Celfon
const PSYCHIC_KAPPA: u16 = 4298; // WATER Aqua, 400 ATK
const WHITE_TIGER: u16 = 5144; // The All-Seeing White Tiger: a Level 3 WIND Beast
const BLACK_PENDANT: u16 = 4317; // an Equip Spell
const DRITTE: u16 = 13992; // Valkyrie Dritte

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn field(anime: bool, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// A duel whose both decks are 40 Kuriboh (Level 1 monsters).
fn duel(anime: bool, spawns: &[Spawn]) -> Option<Scenario> {
    field(anime, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], spawns)
}

/// [`duel`] with these cards in the first player's Extra Deck.
fn duel_x(anime: bool, extras: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    field(anime, [(&[KURIBOH; 40], extras[0]), (&[KURIBOH; 40], extras[1])], spawns)
}

/// A Deck of 20 copies of `card` and 20 Kuriboh: some stay in the Deck whatever the opening hand drew.
fn stocked(card: u16) -> Vec<u16> {
    [card; 20].into_iter().chain([KURIBOH; 20]).collect()
}

fn sp(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn count_all(s: &Scenario, team: usize, place: &str) -> usize {
    s.state()[team][place].as_array().map_or(0, |cards| cards.iter().filter(|c| !c.is_null()).count())
}

fn mzone(s: &Scenario, team: usize) -> usize {
    s.state()[team]["mzone"].as_array().map_or(0, |cards| cards.iter().filter(|c| !c.is_null()).count())
}

fn meklords(team: u32, place: yo3_server::engine::SpawnPlace) -> [Spawn; 3] {
    [sp(team, place, ARMY_OF_GRANEL), sp(team, place, ARMY_OF_WISEL), sp(team, place, ARMY_OF_SKIEL)]
}

// ---- Meklord Astro Dragon Asterisk

/// Meklord Astro Dragon Asterisk: "Must first be Special Summoned (from your hand) by controlling 3 or more "Meklord"
/// monsters. When this card is Special Summoned: You can target any number of "Meklord" monsters you control, except this
/// card; send those targets to the GY, and if you do, this card's ATK becomes the combined original ATK of the monsters
/// sent to the GY by this effect." (The harness picks one target, the minimum; Wisel's 1800.)
#[test]
fn asterisk_sends_the_meklords_and_takes_their_attack() {
    let mut spawns = meklords(0, Monster).to_vec();
    spawns.push(sp(0, Hand, ASTERISK));
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(0, ARMY_OF_WISEL)]);
    s.run(&[SpSummon(ASTERISK), Chain(ASTERISK)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ASTERISK), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", ARMY_OF_WISEL), 1, "sent to the GY: {}", s.context());
    assert_eq!(s.count(0, "mzone", ARMY_OF_GRANEL) + s.count(0, "mzone", ARMY_OF_SKIEL), 2, "the others stay: {}", s.context());
    assert_eq!(s.attack_of(0, ASTERISK), Some(1800), "the original ATK of the monster sent: {}", s.context());
}

/// Asterisk: "target any number of "Meklord" monsters you control, except this card; send those targets to the GY, and
/// if you do, this card's ATK becomes the combined original ATK of the monsters sent". All three Meklords are chosen
/// (`prefer_most`): 1600 + 1800 + 1200 = 4600 ATK, none left on the field.
#[test]
fn asterisk_sends_all_three_meklords_and_takes_their_combined_attack() {
    let mut spawns = meklords(0, Monster).to_vec();
    spawns.push(sp(0, Hand, ASTERISK));
    spawns.push(sp(0, Monster, ARMY_OF_GRANEL));
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(0, ARMY_OF_GRANEL), (0, ARMY_OF_WISEL), (0, ARMY_OF_SKIEL)]);
    s.prefer_most(true);
    s.run(&[SpSummon(ASTERISK), Chain(ASTERISK)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ASTERISK), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", ARMY_OF_WISEL), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", ARMY_OF_SKIEL), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", ARMY_OF_GRANEL), 2, "both Granels: {}", s.context());
    assert_eq!(s.attack_of(0, ASTERISK), Some(1600 * 2 + 1800 + 1200), "{}", s.context());
}

/// Asterisk's Special Summon needs 3 "Meklord" monsters: with two it is not offered, whatever turn it is.
#[test]
fn asterisk_needs_three_meklords() {
    let spawns = [sp(0, Monster, ARMY_OF_GRANEL), sp(0, Monster, ARMY_OF_WISEL), sp(0, Hand, ASTERISK)];
    let Some(mut s) = duel(false, &spawns) else { return };
    assert!(s.run(&[SpSummon(ASTERISK)]).is_err(), "two Meklords are not enough: {}", s.context());
    assert_eq!(s.count(0, "mzone", ASTERISK), 0, "{}", s.context());
}

/// Asterisk: "Each time a Synchro Monster(s) is Special Summoned: Inflict 1000 damage to the Summoning player." Junk Warrior
/// Synchro Summoned by its owner costs that player 1000.
#[test]
fn asterisk_hurts_the_player_who_synchro_summons() {
    let spawns = [sp(0, Monster, ASTERISK), sp(0, Monster, HEDGEHOG), sp(0, Hand, JUNK_SYNCHRON)];
    let Some(mut s) = duel_x(false, [&[JUNK_WARRIOR], &[]], &spawns) else { return };
    // The Hedgehog is on the field already: Normal Summon the Tuner, then Synchro Summon Junk Warrior.
    s.run(&[Summon(JUNK_SYNCHRON), SpSummon(JUNK_WARRIOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "{}", s.context());
    assert_eq!(s.lp(0), 7000, "1000 damage to the Summoning player: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// The anime Asterisk: "If you control 3 or more face-up "Meklord" monsters, you can Special Summon this card from your hand.
/// This card's ATK is equal to the combined ATK of all other face-up Attack Position Machine-Type monsters you control."
#[test]
fn anime_asterisk_has_the_attack_of_the_other_machines() {
    let mut spawns = meklords(0, Monster).to_vec();
    spawns.push(sp(0, Hand, ASTERISK));
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[SpSummon(ASTERISK)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ASTERISK), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", ARMY_OF_GRANEL), 0, "nothing is sent in the anime version: {}", s.context());
    assert_eq!(s.attack_of(0, ASTERISK), Some(1600 + 1800 + 1200), "{}", s.context());
}

// ---- Meklord Astro Mekanikle

/// Meklord Astro Mekanikle: "Must be Special Summoned (from your hand) by sending 3 "Meklord" monsters from your hand to the
/// GY. Once per turn: You can target 1 Synchro Monster your opponent controls; equip that target to this card. This card
/// gains ATK equal to the combined ATK of the monsters equipped to it by this effect."
#[test]
fn mekanikle_summons_by_sending_three_and_equips_a_synchro() {
    let mut spawns = meklords(0, Hand).to_vec();
    spawns.extend([sp(0, Hand, MEKANIKLE), sp(1, Monster, STARDUST)]);
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[SpSummon(MEKANIKLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", MEKANIKLE), 1, "{}", s.context());
    for meklord in [ARMY_OF_GRANEL, ARMY_OF_WISEL, ARMY_OF_SKIEL] {
        assert_eq!(s.count(0, "grave", meklord), 1, "sent from the hand to pay: {}", s.context());
    }
    assert_eq!(s.attack_of(0, MEKANIKLE), Some(4000), "{}", s.context());
    s.run(&[Activate(MEKANIKLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", STARDUST), 0, "equipped to Mekanikle: {}", s.context());
    assert_eq!(s.count(0, "szone", STARDUST), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, MEKANIKLE), Some(4000 + 2500), "{}", s.context());
}

/// Mekanikle: "Once per turn, during your Standby Phase: You can send 1 of these equipped monsters you control to the GY;
/// inflict damage to your opponent equal to that monster's original ATK."
#[test]
fn mekanikle_burns_with_an_equipped_synchro_in_the_standby_phase() {
    let mut spawns = meklords(0, Hand).to_vec();
    spawns.extend([sp(0, Hand, MEKANIKLE), sp(1, Monster, STARDUST)]);
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[SpSummon(MEKANIKLE), Activate(MEKANIKLE), EndTurn, Chain(MEKANIKLE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", STARDUST), 1, "the equipped Synchro is sent to the GY (its owner's): {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2500, "its original ATK: {}", s.context());
}

/// The anime Mekanikle: "During your End Phase: you can send 1 Synchro Monster equipped to this card to the Graveyard;
/// inflict damage to your opponent equal to its ATK." (The anime card has no Summoning procedure: it starts on the field.)
#[test]
fn anime_mekanikle_burns_in_the_end_phase() {
    let spawns = [sp(0, Monster, MEKANIKLE), sp(1, Monster, STARDUST)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(MEKANIKLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", STARDUST), 1, "equipped: {}", s.context());
    assert_eq!(s.attack_of(0, MEKANIKLE), Some(4000 + 2500), "{}", s.context());
    s.run_both([&[EndTurn, Chain(MEKANIKLE)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", STARDUST), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2500, "{}", s.context());
}

// ---- Metaion and Michion, the Timelords

/// Metaion, the Timelord: "If you control no monsters, you can Normal Summon this card without Tributing. Cannot be destroyed
/// by battle or card effects."
#[test]
fn metaion_is_summoned_without_tribute_and_survives_dark_hole() {
    let spawns = [sp(0, Hand, METAION), sp(0, Hand, DARK_HOLE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Summon(METAION), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", METAION), 1, "a Level 10 with no Tribute, and not destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "Dark Hole destroyed the other monster: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Metaion: "You take no battle damage from attacks involving this Attack Position card. At the end of the Battle Phase, if
/// this card battled: Return as many monsters on the field as possible to the hand (other than this card), and if you do,
/// inflict 300 damage to your opponent for each card returned." (The second player attacks: the first can't on turn 1.)
#[test]
fn metaion_returns_the_monsters_and_burns_for_each() {
    let spawns = [sp(1, Hand, METAION), sp(0, Monster, WARWOLF), sp(0, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Summon(METAION), Attack(METAION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", METAION), 1, "not destroyed by the battle: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no battle damage to its controller: {}", s.context());
    assert_eq!(mzone(&s, 0), 0, "both monsters went back to the hand: {}", s.context());
    assert_eq!(s.count(0, "hand", WARWOLF) + s.count(0, "hand", BLUE_EYES), 2, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2 * 300, "300 for each card returned: {}", s.context());
}

/// Metaion: "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn metaion_shuffles_itself_into_the_deck_in_the_standby_phase() {
    let spawns = [sp(0, Hand, METAION)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Summon(METAION)]).unwrap();
    assert_eq!(s.count(0, "mzone", METAION), 1, "{}", s.context());
    s.run_both([&[EndTurn, Chain(METAION)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", METAION), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", METAION), 0, "shuffled into the Deck, not sent: {}", s.context());
}

/// The anime Metaion: "If damage calculation is performed involving this card, at the end of the Battle Phase: Return as many
/// monsters your opponent controls as possible to the hand, and if a card(s) is returned by this effect, inflict 300 damage
/// for each." (The anime card has no Tribute-free Summon: two Tributes.)
#[test]
fn anime_metaion_returns_only_the_opponents_monsters() {
    let spawns = [sp(1, Hand, METAION), sp(1, Monster, WARWOLF), sp(1, Monster, DARK_BLADE), sp(0, Monster, BLUE_EYES)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run_both([&[], &[Summon(METAION), Attack(METAION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 300, "{}", s.context());
    assert_eq!(s.count(1, "mzone", METAION), 1, "{}", s.context());
}

/// Michion, the Timelord: "Cannot be destroyed by battle or card effects. You take no battle damage from attacks involving
/// this card. At the end of the Battle Phase, if this card battled: Halve your opponent's LP."
#[test]
fn michion_halves_the_opponents_life_points() {
    let spawns = [sp(1, Hand, MICHION), sp(0, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Summon(MICHION), Attack(MICHION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", MICHION), 1, "not destroyed by the 3000 ATK monster: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.lp(0), 4000, "halved: {}", s.context());
}

/// Michion: "Cannot be destroyed by ... card effects": Dark Hole leaves it.
#[test]
fn michion_survives_dark_hole() {
    let spawns = [sp(0, Hand, MICHION), sp(0, Hand, DARK_HOLE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Summon(MICHION), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", MICHION), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// The anime Michion: "At the end of the Damage Step, if this card battled: Inflict damage to your opponent equal to half
/// their LP." (Two Tributes for the Summon.)
#[test]
fn anime_michion_deals_half_their_life_points_as_damage() {
    let spawns = [sp(1, Hand, MICHION), sp(1, Monster, WARWOLF), sp(1, Monster, DARK_BLADE), sp(0, Monster, BLUE_EYES)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run_both([&[], &[Summon(MICHION), Attack(MICHION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "{}", s.context());
    assert_eq!(s.count(1, "mzone", MICHION), 1, "{}", s.context());
}

// ---- Mimicat

/// Mimicat: "If you control "Toon World" and a Toon monster: Target 1 card in your opponent's GY; if it is a monster, Special
/// Summon it to your field."
#[test]
fn mimicat_summons_a_monster_from_their_graveyard() {
    let spawns = [sp(0, Hand, MIMICAT), sp(0, Spell, TOON_WORLD), sp(0, Monster, TOON_MASKED_SORCERER), sp(1, Grave, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(MIMICAT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "their Warwolf is on my field: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 0, "{}", s.context());
}

/// Mimicat: "... or if it is a Spell/Trap, Set it to your field."
#[test]
fn mimicat_sets_a_spell_from_their_graveyard() {
    let spawns = [sp(0, Hand, MIMICAT), sp(0, Spell, TOON_WORLD), sp(0, Monster, TOON_MASKED_SORCERER), sp(1, Grave, DARK_HOLE)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(MIMICAT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", DARK_HOLE), 1, "Set to my field: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 0, "{}", s.context());
}

/// Mimicat needs "Toon World" and a Toon monster: with no Toon monster it can't be activated.
#[test]
fn mimicat_needs_a_toon_monster() {
    let spawns = [sp(0, Hand, MIMICAT), sp(0, Spell, TOON_WORLD), sp(0, Monster, WARWOLF), sp(1, Grave, DARK_HOLE)];
    let Some(mut s) = duel(false, &spawns) else { return };
    assert!(s.run(&[Activate(MIMICAT)]).is_err(), "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
}

/// The anime Mimicat: "Banish the top 5 cards from your Deck; add 1 card from your opponent's Graveyard to your hand."
#[test]
fn anime_mimicat_banishes_five_and_takes_a_card() {
    let spawns = [sp(0, Hand, MIMICAT), sp(1, Grave, WARWOLF)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(MIMICAT)]).unwrap();
    clean(&s);
    assert_eq!(s.state()[0]["removed"].as_array().map_or(0, Vec::len), 5, "the top 5 of the Deck: {}", s.context());
    assert_eq!(s.count(0, "hand", WARWOLF), 1, "added to my hand: {}", s.context());
}

// ---- Miracle Contact

/// Miracle Contact: "Shuffle into the Deck, from your hand, field, or GY, the Fusion Materials that are listed on an
/// "Elemental HERO" Fusion Monster that lists "Elemental HERO Neos" as Fusion Material, then Special Summon that Fusion
/// Monster from your Extra Deck, ignoring its Summoning conditions." Neos from the hand and the Mole from the GY make
/// Grand Neos.
#[test]
fn miracle_contact_makes_grand_neos_from_the_hand_and_the_graveyard() {
    let spawns = [sp(0, Hand, MIRACLE_CONTACT), sp(0, Hand, NEOS), sp(0, Grave, GRAND_MOLE)];
    let Some(mut s) = duel_x(false, [&[GRAND_NEOS], &[]], &spawns) else { return };
    s.run(&[Activate(MIRACLE_CONTACT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GRAND_NEOS), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", NEOS), 0, "the materials went back to the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", GRAND_MOLE), 0, "{}", s.context());
}

/// The anime Miracle Contact: "Shuffle into the Deck, from your field or your GY, the Fusion Materials ..." (not from the
/// hand), with Neos on the field and the Mole in the GY.
#[test]
fn anime_miracle_contact_uses_the_field_and_the_graveyard() {
    let spawns = [sp(0, Hand, MIRACLE_CONTACT), sp(0, Monster, NEOS), sp(0, Grave, GRAND_MOLE)];
    let Some(mut s) = duel_x(true, [&[GRAND_NEOS], &[]], &spawns) else { return };
    s.run(&[Activate(MIRACLE_CONTACT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GRAND_NEOS), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", NEOS), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", GRAND_MOLE), 0, "{}", s.context());
}

/// The anime version can't use materials in the hand: Neos and the Mole both in the hand leave the card unusable.
#[test]
fn anime_miracle_contact_cannot_use_the_hand() {
    let spawns = [sp(0, Hand, MIRACLE_CONTACT), sp(0, Hand, NEOS), sp(0, Hand, GRAND_MOLE)];
    let Some(mut s) = duel_x(true, [&[GRAND_NEOS], &[]], &spawns) else { return };
    assert!(s.run(&[Activate(MIRACLE_CONTACT)]).is_err(), "{}", s.context());
    assert_eq!(s.count(0, "mzone", GRAND_NEOS), 0, "{}", s.context());
}

// ---- Miracle Rupture

/// Miracle Rupture: "Send 1 Level 4 or lower Rock monster from your Deck to the GY, then, if "Fossil Fusion" is in your GY,
/// draw 1 card."
#[test]
fn miracle_rupture_sends_a_rock_and_draws_with_fossil_fusion() {
    let deck = stocked(GIANT_SOLDIER_OF_STONE);
    let spawns = [sp(0, Hand, MIRACLE_RUPTURE), sp(0, Grave, FOSSIL_FUSION)];
    let Some(mut s) = field(false, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(MIRACLE_RUPTURE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GIANT_SOLDIER_OF_STONE), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before, "Rupture left the hand, one card was drawn: {}", s.context());
}

/// Miracle Rupture: without "Fossil Fusion" in the GY nothing is drawn.
#[test]
fn miracle_rupture_does_not_draw_without_fossil_fusion() {
    let deck = stocked(GIANT_SOLDIER_OF_STONE);
    let spawns = [sp(0, Hand, MIRACLE_RUPTURE)];
    let Some(mut s) = field(false, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(MIRACLE_RUPTURE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GIANT_SOLDIER_OF_STONE), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before - 1, "no draw: {}", s.context());
}

/// The anime Miracle Rupture: "Send 1 Rock-Type monster from your Deck to the Graveyard, then draw 1 card." Always draws.
#[test]
fn anime_miracle_rupture_always_draws() {
    let deck = stocked(GIANT_SOLDIER_OF_STONE);
    let spawns = [sp(0, Hand, MIRACLE_RUPTURE)];
    let Some(mut s) = field(true, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(MIRACLE_RUPTURE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GIANT_SOLDIER_OF_STONE), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before, "{}", s.context());
}

// ---- Miracle Stone

/// Miracle Stone: "All Spellcaster monsters you control gain 500 ATK/DEF for each "Fortune Fairy" monster you control with
/// different names." Dark Magician with two different Fortune Fairies: +1000; a second Hikari adds nothing.
#[test]
fn miracle_stone_counts_fortune_fairies_by_name() {
    let spawns = [sp(0, Hand, MIRACLE_STONE), sp(0, Monster, DARK_MAGICIAN), sp(0, Monster, FORTUNE_FAIRY_HIKARI)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(MIRACLE_STONE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_MAGICIAN), Some(2500 + 500), "one Fortune Fairy: {}", s.context());
    let spawns = [sp(0, Hand, MIRACLE_STONE), sp(0, Monster, DARK_MAGICIAN), sp(0, Monster, FORTUNE_FAIRY_HIKARI),
        sp(0, Monster, FORTUNE_FAIRY_EN), sp(0, Monster, FORTUNE_FAIRY_HIKARI)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(MIRACLE_STONE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_MAGICIAN), Some(2500 + 1000), "two different names, three Fairies: {}", s.context());
}

/// Miracle Stone: "Once per turn, when an attack is declared involving your "Fortune Fairy" monster: You can draw 1 card."
/// Measured against the same attack without the Stone (the second player attacks: the first can't on turn 1).
#[test]
fn miracle_stone_draws_when_a_fortune_fairy_attacks() {
    let mut hands = vec![];
    for with_stone in [false, true] {
        let mut spawns = vec![sp(1, Monster, FORTUNE_FAIRY_HIKARI)];
        let plan: &[yo3_server::duelcheck::scenario::Action] = if with_stone {
            spawns.push(sp(1, Spell, MIRACLE_STONE));
            &[Attack(FORTUNE_FAIRY_HIKARI), Chain(MIRACLE_STONE)]
        } else {
            &[Attack(FORTUNE_FAIRY_HIKARI)]
        };
        let Some(mut s) = duel(false, &spawns) else { return };
        s.run_both([&[], plan]).unwrap();
        clean(&s);
        hands.push(hand(&s, 1));
    }
    assert_eq!(hands[1], hands[0] + 1, "one more card with the Stone: {hands:?}");
}

// ---- Mirror Force Dragon

/// Mirror Force Dragon: "Must be Special Summoned with "The Fang of Critias", using "Mirror Force"." The Fang sends the
/// Mirror Force from the hand and brings the Dragon from the Extra Deck.
#[test]
fn mirror_force_dragon_comes_with_the_fang_of_critias() {
    let spawns = [sp(0, Hand, FANG_OF_CRITIAS), sp(0, Hand, MIRROR_FORCE)];
    let Some(mut s) = duel_x(false, [&[MIRROR_FORCE_DRAGON], &[]], &spawns) else { return };
    s.run(&[Activate(FANG_OF_CRITIAS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", MIRROR_FORCE_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", MIRROR_FORCE), 1, "{}", s.context());
}

/// Mirror Force Dragon: "When a monster(s) you control is targeted for an attack ... (Quick Effect): You can destroy all
/// cards your opponent controls."
#[test]
fn mirror_force_dragon_destroys_everything_when_attacked() {
    let spawns = [sp(0, Monster, MIRROR_FORCE_DRAGON), sp(1, Monster, WARWOLF), sp(1, Monster, BLUE_EYES), sp(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[ChainOnAttack(MIRROR_FORCE_DRAGON)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(mzone(&s, 1), 0, "every monster destroyed: {}", s.context());
    assert_eq!(s.count(1, "szone", DARK_HOLE), 0, "and the Set card: {}", s.context());
    assert_eq!(s.count(0, "mzone", MIRROR_FORCE_DRAGON), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// The anime Mirror Force Dragon: "At the start of the Damage Step, if this card battles an opponent's monster whose ATK is
/// equal to or greater than this card's ATK: ... destroy all monsters your opponent controls." (A forced trigger.)
#[test]
fn anime_mirror_force_dragon_destroys_the_monsters_when_it_meets_a_stronger_one() {
    let spawns = [sp(0, Monster, MIRROR_FORCE_DRAGON), sp(1, Monster, BLUE_EYES), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.prefer(&[(0, MIRROR_FORCE_DRAGON)]);
    s.run_both([&[], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(mzone(&s, 1), 0, "every monster of theirs destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", MIRROR_FORCE_DRAGON), 1, "{}", s.context());
}

// ---- Mirror Resonator

/// Mirror Resonator: "If your opponent controls a face-up monster(s) that was Special Summoned from the Extra Deck and you
/// do not, while this card is in your hand or Graveyard: You can Special Summon this card, but banish it when it leaves the
/// field." (From the hand, or the GY once the hand limit discarded it.) Junk Warrior destroys it in battle.
#[test]
fn mirror_resonator_answers_an_extra_deck_monster_and_is_banished() {
    let spawns = [sp(0, Hand, MIRROR_RESONATOR), sp(1, Hand, JUNK_SYNCHRON), sp(1, Monster, HEDGEHOG)];
    let Some(mut s) = duel_x(false, [&[], &[JUNK_WARRIOR]], &spawns) else { return };
    s.run_both([&[Activate(MIRROR_RESONATOR)], &[Summon(JUNK_SYNCHRON), SpSummon(JUNK_WARRIOR), EndTurn, Attack(JUNK_WARRIOR)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", MIRROR_RESONATOR), 1, "banished when it left the field: {}", s.context());
    assert_eq!(s.count(0, "grave", MIRROR_RESONATOR), 0, "{}", s.context());
}

/// Mirror Resonator can't be Special Summoned while the opponent has no monster from the Extra Deck.
#[test]
fn mirror_resonator_needs_an_extra_deck_monster_of_theirs() {
    let spawns = [sp(0, Hand, MIRROR_RESONATOR), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    assert!(s.run(&[Activate(MIRROR_RESONATOR)]).is_err(), "{}", s.context());
    assert_eq!(s.count(0, "mzone", MIRROR_RESONATOR), 0, "{}", s.context());
}

// ---- Mischief of the Time Goddess

/// Mischief of the Time Goddess: "Cannot be activated as Chain Link 2 or higher. At the end of your Battle Phase, if all
/// monsters you control are "Valkyrie" monsters (min. 1): Send this card to the GY, also, immediately after this effect
/// resolves, skip to the start of the Battle Phase of your next turn." The first player's turn passes (two turns begin in
/// one step, no card is drawn) and the second player is at the Battle Phase prompt again.
#[test]
fn mischief_goes_to_the_graveyard_and_skips_the_opponents_turn() {
    let spawns = [sp(1, Hand, MISCHIEF), sp(1, Monster, DRITTE)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Attack(DRITTE), Chain(MISCHIEF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MISCHIEF), 1, "{}", s.context());
    let turns = s.messages().iter().filter(|m| m["type"] == 40).count();
    let draws = s.messages().iter().filter(|m| m["type"] == 90).count();
    assert_eq!((turns, draws), (2, 0), "the other player's turn began and ended with no draw: {}", s.context());
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(10), Some(1)), "{}", s.context());
}

// ---- Mono Synchron

/// Mono Synchron: "When using this card as Synchro Material, the other Synchro Material(s) must be Level 4 or lower Warrior
/// or Machine-Type monsters, and they are treated as Level 1." Mono (1) + Dark Blade (4, treated as 1) make the Level 2
/// Formula Synchron.
#[test]
fn mono_synchron_treats_a_warrior_as_level_one() {
    let spawns = [sp(0, Hand, MONO_SYNCHRON), sp(0, Monster, DARK_BLADE)];
    let Some(mut s) = duel_x(false, [&[FORMULA_SYNCHRON], &[]], &spawns) else { return };
    s.run(&[Summon(MONO_SYNCHRON), SpSummon(FORMULA_SYNCHRON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", FORMULA_SYNCHRON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", MONO_SYNCHRON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
}

/// Mono Synchron: the other Material must be a Warrior or Machine: a Beast-Warrior is no Material for it (and its Level 4
/// would make 5, no Synchro of the Extra Deck).
#[test]
fn mono_synchron_refuses_other_types() {
    let spawns = [sp(0, Hand, MONO_SYNCHRON), sp(0, Monster, WARWOLF)];
    let Some(mut s) = duel_x(false, [&[FORMULA_SYNCHRON], &[]], &spawns) else { return };
    assert!(s.run(&[Summon(MONO_SYNCHRON), SpSummon(FORMULA_SYNCHRON)]).is_err(), "{}", s.context());
    assert_eq!(s.count(0, "mzone", FORMULA_SYNCHRON), 0, "{}", s.context());
}

// ---- Morphtronic Impact Return

/// Morphtronic Impact Return: "Target up to 2 Spells/Traps your opponent controls; shuffle 1 "Morphtronic" monster from your
/// hand into the Deck, and if you do, shuffle the targeted card(s) into the Deck." (The harness targets one card.)
#[test]
fn impact_return_shuffles_their_spells_and_my_morphtronic() {
    let spawns = [sp(0, SpellSet, IMPACT_RETURN), sp(0, Hand, CELFON), sp(1, SpellSet, DARK_HOLE), sp(1, SpellSet, MST)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(1, DARK_HOLE)]);
    s.run(&[Activate(IMPACT_RETURN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", DARK_HOLE), 0, "the target was shuffled into the Deck: {}", s.context());
    assert_eq!(s.count(1, "szone", MST), 1, "the other card was not targeted: {}", s.context());
    assert_eq!(s.count(0, "hand", CELFON), 0, "{}", s.context());
    assert_eq!(s.count(1, "hand", DARK_HOLE), 0, "into the Deck, not the hand: {}", s.context());
}

/// Impact Return: "Target up to 2 Spells/Traps your opponent controls; shuffle 1 "Morphtronic" monster ... and if you do,
/// shuffle the targeted card(s) into the Deck." Both Set cards are chosen (`prefer_most`).
#[test]
fn impact_return_takes_two_spells_and_traps() {
    let spawns = [sp(0, SpellSet, IMPACT_RETURN), sp(0, Hand, CELFON), sp(1, SpellSet, DARK_HOLE), sp(1, SpellSet, MST)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(1, DARK_HOLE), (1, MST)]);
    s.prefer_most(true);
    s.run(&[Activate(IMPACT_RETURN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", DARK_HOLE) + s.count(1, "szone", MST), 0, "both shuffled into the Deck: {}", s.context());
    assert_eq!(s.count(1, "hand", DARK_HOLE) + s.count(1, "hand", MST), 0, "{}", s.context());
    assert_eq!(s.count(0, "hand", CELFON), 0, "{}", s.context());
}

/// Morphtronic Impact Return: "You can banish this card from your GY, then target 1 of your "Morphtronic" monsters that is
/// banished or in your GY; Special Summon it in Defense Position."
#[test]
fn impact_return_revives_a_morphtronic_from_the_graveyard() {
    let spawns = [sp(0, Grave, IMPACT_RETURN), sp(0, Grave, CELFON)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(IMPACT_RETURN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", CELFON), 1, "{}", s.context());
    assert_eq!(s.position_of(0, CELFON), Some(4), "in Defense Position: {}", s.context());
    assert_eq!(s.count(0, "removed", IMPACT_RETURN), 1, "banished as the cost: {}", s.context());
}

/// The anime Impact Return: "return the targeted card(s) to the hand, and if you do, shuffle 1 "Morphtronic" monster from your
/// hand into the Deck."
#[test]
fn anime_impact_return_bounces_their_spells_to_the_hand() {
    let spawns = [sp(0, SpellSet, IMPACT_RETURN), sp(0, Hand, CELFON), sp(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(IMPACT_RETURN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", DARK_HOLE), 1, "back to their hand: {}", s.context());
    assert_eq!(s.count(0, "hand", CELFON), 0, "{}", s.context());
}

// ---- Morphtronic Lantron

/// Morphtronic Lantron: "While in Attack Position: Any effect damage you would take from an opponent's card effect is
/// inflicted to your opponent instead."
#[test]
fn lantron_reflects_effect_damage() {
    let spawns = [sp(0, Monster, LANTRON), sp(1, Hand, OOKAZI)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 800, "Ookazi's 800 came back to its user: {}", s.context());
}

/// Lantron in Defense Position reflects nothing: Ookazi hurts its target.
#[test]
fn lantron_in_defense_position_does_not_reflect() {
    let spawns = [sp(0, MonsterDefense, LANTRON), sp(1, Hand, OOKAZI)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Lantron: "While in Defense Position: When this card is destroyed by battle and sent to the Graveyard, you take no Battle
/// Damage this turn." A second attacker then hits an empty field for nothing.
#[test]
fn lantron_in_defense_stops_the_rest_of_the_battle_damage() {
    let spawns = [sp(0, MonsterDefense, LANTRON), sp(1, Monster, WARWOLF), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Attack(WARWOLF), Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LANTRON), 1, "destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage this turn: {}", s.context());
}

// ---- Morphtronic Staplen

/// Morphtronic Staplen: "While in Attack Position: Your opponent cannot select another monster as an attack target. If this
/// card is destroyed by battle, the monster that destroyed this card loses 300 ATK."
#[test]
fn staplen_is_the_only_target_and_weakens_its_destroyer() {
    let spawns = [sp(0, Monster, WARWOLF), sp(0, Monster, STAPLEN), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", STAPLEN), 1, "Staplen took the attack: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "the other monster could not be selected: {}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(2000 - 300), "{}", s.context());
}

/// Staplen: "While in Defense Position: This card cannot be destroyed by battle. If this card is attacked, after damage
/// calculation, select 1 face-up Attack Position monster your opponent controls. Change it to Defense Position and this card
/// to Attack Position."
#[test]
fn staplen_in_defense_survives_and_swaps_positions() {
    let spawns = [sp(0, MonsterDefense, STAPLEN), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", STAPLEN), 1, "not destroyed by the 3000 ATK monster: {}", s.context());
    assert_eq!(s.position_of(0, STAPLEN), Some(1), "now in Attack Position: {}", s.context());
    assert_eq!(s.position_of(1, BLUE_EYES), Some(4), "now in Defense Position: {}", s.context());
}

// ---- Mound of the Bound Creator

/// Mound of the Bound Creator: "Level 10 or higher monsters on the field cannot be targeted or destroyed by card effects."
#[test]
fn mound_protects_level_ten_monsters_from_dark_hole() {
    let spawns = [sp(0, Hand, MOUND), sp(0, Hand, DARK_HOLE), sp(0, Monster, OBELISK), sp(0, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(MOUND), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", OBELISK), 1, "Level 10: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "Level 4: {}", s.context());
}

/// Mound: "If a Level 10 or higher monster destroys a monster by battle and sends it to the GY: The player who controlled the
/// destroyed monster takes 1000 damage." (The second player attacks: the first can't on turn 1.)
#[test]
fn mound_burns_for_a_level_ten_battle_kill() {
    let spawns = [sp(1, Hand, MOUND), sp(1, Monster, OBELISK), sp(0, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[], &[Activate(MOUND), Attack(OBELISK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000 - 1000, "battle damage 4000-2000, then 1000: {}", s.context());
}

/// Mound: "When this card on the field is destroyed by card effect and sent to the GY: You can add 1 DIVINE monster from your
/// Deck to your hand."
#[test]
fn mound_fetches_a_divine_monster_when_destroyed() {
    let deck = stocked(OBELISK);
    let spawns = [sp(0, Hand, MOUND), sp(0, Hand, MST)];
    let Some(mut s) = field(false, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(MOUND)]).unwrap();
    let before = s.count(0, "hand", OBELISK);
    s.prefer(&[(0, MOUND)]);
    s.run(&[Activate(MST), Chain(MOUND)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", MOUND), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", OBELISK), before + 1, "{}", s.context());
}

/// The anime Mound: "If a Level 10 or higher monster destroys a monster by battle: The controller of the destroyed monster
/// takes 400 damage."
#[test]
fn anime_mound_burns_for_four_hundred() {
    let spawns = [sp(1, Hand, MOUND), sp(1, Monster, OBELISK), sp(0, Monster, WARWOLF)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run_both([&[], &[Activate(MOUND), Attack(OBELISK)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000 - 400, "{}", s.context());
}

// ---- Multiple Destruction

/// Multiple Destruction (a Trap): "If both players have 3 or more cards in their hand: Each player places their entire hand
/// on the bottom of the Deck in any order, also you lose LP equal to the total number of cards returned to the Deck by this
/// effect x 300 (min. 1), then each player draws 5 cards."
#[test]
fn multiple_destruction_returns_both_hands_and_costs_300_a_card() {
    let Some(mut s) = duel(false, &[sp(0, SpellSet, MULTIPLE_DESTRUCTION)]) else { return };
    // Set at the start: it is activated in the first player's second turn.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let (mine, theirs) = (hand(&s, 0), hand(&s, 1));
    s.run(&[Activate(MULTIPLE_DESTRUCTION)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 300 * (mine + theirs) as i64, "{mine} and {theirs} cards returned: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    assert_eq!((hand(&s, 0), hand(&s, 1)), (5, 5), "each draws 5: {}", s.context());
    assert_eq!(s.count(0, "grave", MULTIPLE_DESTRUCTION), 1, "{}", s.context());
}

/// The anime Multiple Destruction: "Each player discards their entire hand, also you lose LP equal to the number of cards you
/// discarded by this effect x 100 (min. 1), then each player draws 5 cards."
#[test]
fn anime_multiple_destruction_discards_and_costs_100_a_card() {
    let Some(mut s) = duel(true, &[sp(0, SpellSet, MULTIPLE_DESTRUCTION)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let (mine, theirs) = (hand(&s, 0), hand(&s, 1));
    let graves = (count_all(&s, 0, "grave"), count_all(&s, 1, "grave"));
    s.run(&[Activate(MULTIPLE_DESTRUCTION)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 100 * mine as i64, "only the cards I discarded: {}", s.context());
    assert_eq!(count_all(&s, 1, "grave"), graves.1 + theirs, "their hand went to the GY: {}", s.context());
    assert_eq!(count_all(&s, 0, "grave"), graves.0 + mine + 1, "mine and the Trap: {}", s.context());
    assert_eq!((hand(&s, 0), hand(&s, 1)), (5, 5), "{}", s.context());
}

// ---- Mystic Piper

/// Mystic Piper: "You can Tribute this card to draw and reveal 1 card. If that card is a Level 1 Monster Card, draw 1 more
/// card." With a Deck of Level 1 monsters: two cards.
#[test]
fn mystic_piper_draws_two_on_a_level_one_monster() {
    let Some(mut s) = duel(false, &[sp(0, Monster, MYSTIC_PIPER)]) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(MYSTIC_PIPER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", MYSTIC_PIPER), 1, "Tributed: {}", s.context());
    assert_eq!(hand(&s, 0), before + 2, "{}", s.context());
}

/// Mystic Piper: a Level 4 monster drawn gives no second card.
#[test]
fn mystic_piper_draws_one_otherwise() {
    let Some(mut s) = field(false, [(&[WARWOLF; 40], &[]), (&[KURIBOH; 40], &[])], &[sp(0, Monster, MYSTIC_PIPER)]) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(MYSTIC_PIPER)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before + 1, "{}", s.context());
}

/// The anime Mystic Piper: "If that card is a Monster Card, draw 1 more card."
#[test]
fn anime_mystic_piper_draws_two_on_any_monster() {
    let Some(mut s) = field(true, [(&[WARWOLF; 40], &[]), (&[KURIBOH; 40], &[])], &[sp(0, Monster, MYSTIC_PIPER)]) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(MYSTIC_PIPER)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before + 2, "{}", s.context());
}

// ---- Necro Defender

/// Necro Defender: "During your Main Phase, you can remove from play this card from your Graveyard to select 1 monster you
/// control. Until your opponent's next End Phase, that monster cannot be destroyed by battle, and you take no Battle Damage
/// from battles involving it."
#[test]
fn necro_defender_protects_a_monster_in_battle() {
    let spawns = [sp(0, Grave, NECRO_DEFENDER), sp(0, Monster, WARWOLF), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[Activate(NECRO_DEFENDER)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", NECRO_DEFENDER), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "not destroyed by the 3000 ATK monster: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
}

// ---- Noble Knight's Shield-Bearer

/// Noble Knight's Shield-Bearer: "If this card is Normal or Special Summoned: You can banish 1 LIGHT monster from your GY;
/// draw 1 card."
#[test]
fn shield_bearer_banishes_a_light_monster_to_draw() {
    let spawns = [sp(0, Hand, SHIELD_BEARER), sp(0, Grave, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[Summon(SHIELD_BEARER), Chain(SHIELD_BEARER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before, "the Shield-Bearer left the hand, one card drawn: {}", s.context());
}

/// Shield-Bearer: "You can banish this card from your hand or field; add 1 Level 6 or lower WIND Beast monster from your
/// Deck to your hand."
#[test]
fn shield_bearer_banishes_itself_to_search_a_wind_beast() {
    let deck = stocked(WHITE_TIGER);
    let Some(mut s) = field(false, [(&deck, &[]), (&[KURIBOH; 40], &[])], &[sp(0, Hand, SHIELD_BEARER)]) else { return };
    let before = s.count(0, "hand", WHITE_TIGER);
    s.run(&[Activate(SHIELD_BEARER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", SHIELD_BEARER), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", WHITE_TIGER), before + 1, "{}", s.context());
}

/// Shield-Bearer: "You can only use 1 "Noble Knight's Shield-Bearer" effect per turn, and only once that turn." After the
/// draw, the search is no longer offered.
#[test]
fn shield_bearer_effects_share_one_use_a_turn() {
    let deck = stocked(WHITE_TIGER);
    let spawns = [sp(0, Hand, SHIELD_BEARER), sp(0, Grave, BLUE_EYES)];
    let Some(mut s) = field(false, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Summon(SHIELD_BEARER), Chain(SHIELD_BEARER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", BLUE_EYES), 1, "the first effect was used: {}", s.context());
    let offered = s.prompt()["activates"].as_array().is_some_and(|a| a.iter().any(|c| c["cid"] == SHIELD_BEARER));
    assert!(!offered, "the second effect is used up this turn: {}", s.prompt());
}

// ---- Noble Knight's Spearholder

/// Noble Knight's Spearholder: "If this card is Normal or Special Summoned: You can target 1 Level 8 or lower monster you
/// control; its name becomes "Fleur Synchron", also it is treated as a Tuner, until the End Phase." The Warwolf, a Tuner for
/// the turn, and the Spearholder (2) make the Level 6 Flamvell Uruquizas.
#[test]
fn spearholder_makes_a_monster_a_tuner() {
    let spawns = [sp(0, Hand, SPEARHOLDER), sp(0, Monster, WARWOLF)];
    let Some(mut s) = duel_x(false, [&[URUQUIZAS], &[]], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Summon(SPEARHOLDER), Chain(SPEARHOLDER), SpSummon(URUQUIZAS)]).unwrap();
    assert_eq!(s.count(0, "mzone", URUQUIZAS), 1, "{}", s.context());
    clean(&s);
}

/// Spearholder: "You can Tribute this card; add 1 Equip Spell from your Deck to your hand."
#[test]
fn spearholder_tributes_itself_to_fetch_an_equip_spell() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([BLACK_PENDANT]).collect();
    let Some(mut s) = field(false, [(&deck, &[]), (&[KURIBOH; 40], &[])], &[sp(0, Monster, SPEARHOLDER)]) else { return };
    s.run(&[Activate(SPEARHOLDER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SPEARHOLDER), 1, "Tributed: {}", s.context());
    assert_eq!(s.count(0, "hand", BLACK_PENDANT), 1, "{}", s.context());
}

// ---- Ocean of Regeneration

/// Ocean of Regeneration: "You can target 1 WATER monster with 1000 or less ATK in your GY; Special Summon it, but destroy it
/// during the End Phase."
#[test]
fn ocean_revives_a_water_monster_for_a_turn() {
    let spawns = [sp(0, Spell, OCEAN), sp(0, Grave, PSYCHIC_KAPPA)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(OCEAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PSYCHIC_KAPPA), 1, "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PSYCHIC_KAPPA), 0, "destroyed in the End Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", PSYCHIC_KAPPA), 1, "{}", s.context());
}

/// Ocean of Regeneration only takes monsters of 1000 ATK or less: Blue-Eyes is not a target (and it is not WATER).
#[test]
fn ocean_refuses_a_strong_monster() {
    let spawns = [sp(0, Spell, OCEAN), sp(0, Grave, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    assert!(s.run(&[Activate(OCEAN)]).is_err(), "{}", s.context());
    assert_eq!(s.count(0, "mzone", BLUE_EYES), 0, "{}", s.context());
}

/// The anime Ocean: "Once per turn: You can target 1 Aqua-Type monster with 1000 or less ATK in your Graveyard; Special
/// Summon it. That monster is destroyed at the end of this turn."
#[test]
fn anime_ocean_revives_an_aqua_monster_for_a_turn() {
    let spawns = [sp(0, Spell, OCEAN), sp(0, Grave, PSYCHIC_KAPPA)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(OCEAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PSYCHIC_KAPPA), 1, "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    assert_eq!(s.count(0, "grave", PSYCHIC_KAPPA), 1, "{}", s.context());
}

// ---- Orgoth the Relentless

/// Orgoth the Relentless: "Once per turn, during your Main Phase: You can roll a six-sided die 3 times and this card gains
/// ATK/DEF equal to the total x 100 (until the end of your opponent's turn), then, if 2 of the results were the same, apply
/// the appropriate effect. If all 3 were the same, apply all of these effects." Over several seeds: the gain is always
/// 300-1800 in hundreds, and Dark Hole afterwards destroys Orgoth in some seeds and not in others (1 or 2: "cannot be
/// destroyed by battle or card effects"), and a pair of 3 or 4 draws 2 cards in some.
#[test]
fn orgoth_rolls_three_dice() {
    let (mut drew_two, mut survived, mut destroyed, mut gains) = (false, false, false, std::collections::BTreeSet::new());
    for seed in 1..=24u64 {
        let Some(dir) = engine_dir() else { return };
        let spawns = [sp(0, Monster, ORGOTH), sp(0, Hand, DARK_HOLE)];
        let mut s = Scenario::start_field(&dir, seed, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns).unwrap();
        let before = hand(&s, 0);
        s.run(&[Activate(ORGOTH)]).unwrap();
        clean(&s);
        let gain = s.attack_of(0, ORGOTH).unwrap() - 2500;
        assert!((300..=1800).contains(&gain) && gain % 100 == 0, "seed {seed}: gain {gain}: {}", s.context());
        assert_eq!(s.monster(0, ORGOTH).unwrap()["defense"].as_i64().unwrap() - 2450, gain, "DEF gains the same: {}", s.context());
        gains.insert(gain);
        drew_two |= hand(&s, 0) == before + 2;
        s.run(&[Activate(DARK_HOLE)]).unwrap();
        clean(&s);
        match s.count(0, "mzone", ORGOTH) {
            1 => survived = true,
            _ => destroyed = true,
        }
    }
    assert!(gains.len() > 3, "the total varies with the dice: {gains:?}");
    assert!(drew_two, "a pair of 3 or 4 draws 2 cards, in some seed");
    assert!(survived && destroyed, "a pair of 1 or 2 makes it indestructible, in some seeds only: {survived} {destroyed}");
}

// ---- Orichalcos Sword of Sealing

/// Orichalcos Sword of Sealing: "The equipped monster's effects are negated." Metaion cannot be destroyed by card effects,
/// until the Sword is on it: Dark Hole then destroys it.
#[test]
fn sword_of_sealing_negates_the_equipped_monster() {
    let spawns = [sp(0, Hand, SWORD), sp(0, Hand, DARK_HOLE), sp(1, Monster, METAION)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(SWORD), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", METAION), 0, "its protection was negated: {}", s.context());
    assert_eq!(s.count(1, "grave", METAION), 1, "{}", s.context());
}

/// Sword of Sealing: "Once per turn (Quick Effect): You can send 1 card from your hand to the GY, then target 1 face-up
/// card on the field; destroy that target." Given by its first effect to an Effect Monster of mine, with a card in my Field
/// Zone.
#[test]
fn sword_of_sealing_gives_a_quick_destruction() {
    let spawns = [sp(0, Hand, SWORD), sp(0, Hand, MOUND), sp(0, Monster, KURIBOH), sp(0, Hand, WARWOLF), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(1, BLUE_EYES), (0, WARWOLF)]);
    s.run(&[Activate(MOUND), Activate(SWORD), Activate(SWORD), Activate(KURIBOH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the discarded card: {}", s.context());
}

/// The anime Sword: "Activate only by discarding 1 card that you have drawn this turn. Negate the effects of the equipped
/// monster." The card drawn for the turn pays; the negated Metaion then falls to Dark Hole.
#[test]
fn anime_sword_negates_the_equipped_monster() {
    let spawns = [sp(0, Hand, SWORD), sp(0, Hand, DARK_HOLE), sp(1, Monster, METAION)];
    let Some(mut s) = duel(true, &spawns) else { return };
    let before = count_all(&s, 0, "grave");
    s.run(&[Activate(SWORD), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", METAION), 0, "negated, then destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", SWORD), 1, "the Sword followed its target: {}", s.context());
    assert_eq!(count_all(&s, 0, "grave"), before + 3, "Dark Hole, the Sword and the discarded card: {}", s.context());
}
