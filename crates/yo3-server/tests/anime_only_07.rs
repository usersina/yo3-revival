//! Anime-only cards, group 7 (rows 5437-5466 of engine/added/cards.tsv): Valon's Armor monsters and cards, and the last Duel
//! Monsters decks (Tristan Taylor, Rebecca Hawkins, Rex Raptor, Crump, Bandit Keith, Mako Tsunami, Strings, Seto Kaiba and
//! Yugi Muto). These cards exist only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`).
//! Each effect goes through the real core with both seats' client models checked after every step (`duelcheck::scenario`);
//! the texts are the anime ones. Every Trap and every Quick-Play Spell answering the opponent is Set first, and neither
//! player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! Covered: the cards the per-card fuzz flagged (Rare Metal Soul, Jurassic Heart, Quick Attack, Phoenix Gravitation: it
//! activated none of them, each needing a board the self-play never builds: a Fusion Monster, a Dinosaur under a targeting
//! Trap, a Fusion Summon that turn, four Armor monsters in the Graveyard), the signature cards (Full Armor Gravitation,
//! Psychic Armor Head, Judgment Blaster, Chain of the Underworld, Command Angel, Defender Iceberg, Data Brain) and a seeded
//! sample of the rest (Active Guard, Attack Guidance Barrier, Big Bang Blow, Buster Pyle, Magical Academy, Pillager,
//! Revenge Sacrifice, Trap Buster), with Armored Gravitation and Magnet Armor on the way.
//! Defender Iceberg has an override (`engine/overrides/c511000643.lua`: upstream read the attack target's DEF before
//! checking there was one, so every direct attack stopped on a Lua error: batch 7's fuzz). The fuzz also found a translator
//! bug: a card the core holds in no place between two moves (Cemetery Change's swap of the two Graveyards: a move to
//! location 0, then from it) left the client's Graveyards out of step (`translate/events.rs`, `limbo`); its case is
//! `cemetery_change_swaps_the_two_graveyards_in_the_clients_view`.
//! Not coverable: "ignoring the Battle City or Virtual World Rules" (Quick Attack): the game plays no such rules, so every
//! Special Summoned monster may attack the turn it comes and the clause has nothing to lift.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const POLYMERIZATION: u16 = 4837;
const RAIGEKI_BREAK: u16 = 5544; // Trap: discard 1 card; target 1 card on the field; destroy it
const GILASAURUS: u16 = 5256; // Dinosaur, Level 3, 1400/400

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, team 0's Main Deck being `deck` and team 1's 40 Feral Imps. The
/// core shuffles the Decks (the seed fixes it), so a test finds a card it wants in a Deck by what the Deck holds, not by
/// where it lies.
fn duel_with(seed: u64, deck: &[u16], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(deck, extra[0]), (&[FILLER; 40][..], extra[1])];
    Some(Scenario::start_anime_field(&dir, seed, 8000, decks, spawns).unwrap())
}

/// Team 0's Main Deck of `cards` and then Feral Imps up to 40.
fn deck_of(cards: &[u16]) -> Vec<u16> {
    cards.iter().copied().chain(std::iter::repeat(FILLER)).take(40).collect()
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(spawns: &[Spawn]) -> Option<Scenario> { duel_with(7, &[FILLER; 40], [&[], &[]], spawns) }

/// A duel with the Extra Decks given: `extra[team]`.
fn field_extra(extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> { duel_with(7, &[FILLER; 40], extra, spawns) }

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// The monsters on a team's field.
fn monsters(s: &Scenario, team: usize) -> usize {
    s.state()[team]["mzone"].as_array().map_or(0, |z| z.iter().filter(|c| c["cid"].as_u64().is_some_and(|c| c != 0)).count())
}

fn deck_size(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap()
}

// ---------------------------------------------------------------------------------------------------------------
// Rare Metal Soul and Quick Attack (the Fusion Monsters' Spells)
// ---------------------------------------------------------------------------------------------------------------

/// Rare Metal Soul: "Target 1 face-up Fusion Monster you control; It gains 1000 ATK until the End Phase." Gaia the Dragon
/// Champion (2600) on the field gains 1000 (3600) and loses it with the turn; Warwolf, not a Fusion Monster, is no target:
/// with only Warwolf on the field the Spell is not offered.
#[test]
fn rare_metal_soul_gives_a_fusion_monster_1000_atk_until_the_end_phase() {
    let [soul, champion] = ["Rare Metal Soul", "Gaia the Dragon Champion"].map(card);
    let Some(mut s) = field_extra([&[champion], &[]], &[hand(0, soul), mon(0, champion), mon(0, WARWOLF)]) else { return };
    assert_eq!(s.attack_of(0, champion), Some(2600), "{}", s.context());
    s.run(&[Activate(soul)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, champion), Some(3600), "+1000: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "only the target gains: {}", s.context());
    assert_eq!(s.count(0, "grave", soul), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, champion), Some(2600), "until the End Phase: {}", s.context());
    // No Fusion Monster, no target.
    let Some(mut s) = field(&[hand(0, soul), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(soul)]).expect_err("no Fusion Monster to target");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Quick Attack: "1 Fusion Monster you control can attack the turn it is Fusion Summoned, ignoring the Battle City or Virtual
/// World Rules." It needs a Fusion Monster Fusion Summoned this turn: team 1 Fusion Summons Gaia the Dragon Champion with
/// Polymerization, plays Quick Attack (it goes to the Graveyard) and attacks Feral Imp in the same turn (2600 against 1300:
/// 1300 damage). In a turn with no Fusion Summon, or with a Fusion Monster that came earlier, it is not offered.
#[test]
fn quick_attack_follows_a_fusion_summon_of_the_turn() {
    let [quick, champion, knight, dragon] = ["Quick Attack", "Gaia the Dragon Champion", "Gaia The Fierce Knight", "Curse of Dragon"].map(card);
    let spawns = [mon(0, FILLER), hand(1, POLYMERIZATION), hand(1, knight), hand(1, dragon), hand(1, quick)];
    let Some(mut s) = field_extra([&[], &[champion]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(POLYMERIZATION), Activate(quick), Attack(champion)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", quick), 1, "Quick Attack was played: {}", s.context());
    assert_eq!(s.count(1, "mzone", champion), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "the Fusion Monster attacked the turn it came: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
    // No Fusion Summon this turn.
    let Some(mut s) = field_extra([&[], &[champion]], &[hand(0, quick), mon(0, champion)]) else { return };
    let err = s.run(&[Activate(quick)]).expect_err("no Fusion Summon this turn");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Jurassic Heart
// ---------------------------------------------------------------------------------------------------------------

/// Jurassic Heart: "Negate the activation and effect(s) of a Trap Card that targets 1 face-up Dinosaur-Type monster you control
/// and destroy it." Team 1's Raigeki Break (discard 1; target 1 card on the field; destroy it) aims at team 0's Gilasaurus: it
/// is negated and destroyed, Gilasaurus stays. Against a target that is no Dinosaur (Warwolf) Jurassic Heart is not played and
/// Raigeki Break destroys it.
#[test]
fn jurassic_heart_negates_a_trap_aimed_at_a_dinosaur() {
    let heart = card("Jurassic Heart");
    let spawns = [set(0, heart), mon(0, GILASAURUS), set(1, RAIGEKI_BREAK)];
    let Some(mut s) = field(&spawns) else { return };
    s.prefer(&[(0, GILASAURUS), (1, FILLER)]);
    s.run_both([&[Chain(heart)], &[Activate(RAIGEKI_BREAK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GILASAURUS), 1, "Gilasaurus is safe: {}", s.context());
    assert_eq!(s.count(1, "grave", RAIGEKI_BREAK), 1, "Raigeki Break was negated and destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", heart), 1, "{}", s.context());
    // The control: no Dinosaur aimed at, no answer.
    let spawns = [set(0, heart), mon(0, WARWOLF), set(1, RAIGEKI_BREAK)];
    let Some(mut s) = field(&spawns) else { return };
    s.prefer(&[(0, WARWOLF), (1, FILLER)]);
    let err = s.run_both([&[Chain(heart)], &[Activate(RAIGEKI_BREAK)]]).expect_err("Warwolf is no Dinosaur");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Raigeki Break destroyed Warwolf: {}", s.context());
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Phoenix Gravitation, Armored Gravitation, Magnet Armor, Full Armor Gravitation
// ---------------------------------------------------------------------------------------------------------------

/// Phoenix Gravitation: "Special Summon 4 Armor monsters from your Graveyard. All monsters Special Summoned by this card's
/// effect are sent to the Graveyard at the end of this turn." Four Armor monsters in the Graveyard (Active Guard, Trap Buster,
/// Over Boost, Jet Gauntlet) come to the field, and the End Phase takes them back. With only three in the Graveyard it is not
/// offered.
#[test]
fn phoenix_gravitation_brings_four_armor_monsters_back_for_a_turn() {
    let [phoenix, guard, buster, boost, gauntlet] = ["Phoenix Gravitation", "Active Guard", "Trap Buster", "Over Boost", "Jet Gauntlet"].map(card);
    let spawns = [hand(0, phoenix), grave(0, guard), grave(0, buster), grave(0, boost), grave(0, gauntlet)];
    let Some(mut s) = field(&spawns) else { return };
    s.run(&[Activate(phoenix)]).unwrap();
    clean(&s);
    for c in [guard, buster, boost, gauntlet] {
        assert_eq!((s.count(0, "mzone", c), s.count(0, "grave", c)), (1, 0), "{c} came back: {}", s.context());
    }
    assert_eq!(s.count(0, "grave", phoenix), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    for c in [guard, buster, boost, gauntlet] {
        assert_eq!((s.count(0, "mzone", c), s.count(0, "grave", c)), (0, 1), "{c} went back at the End Phase: {}", s.context());
    }
    // Three are not enough.
    let Some(mut s) = field(&[hand(0, phoenix), grave(0, guard), grave(0, buster), grave(0, boost)]) else { return };
    let err = s.run(&[Activate(phoenix)]).expect_err("only three Armor monsters");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Cemetery Change (batch 9's, which found the translator bug here): "Pay 1000 LP; destroy all monsters on the field, then send
/// all cards in each player's Graveyard to the Deck and from there to the opponent's Graveyard" (the core moves them through a
/// place of no location). Team 0's Graveyard (Warwolf, Thunder Kid) and team 1's (Dark Blade) change sides in the client's
/// view as in the engine's: the check after the step would flag the piles otherwise.
#[test]
fn cemetery_change_swaps_the_two_graveyards_in_the_clients_view() {
    let change = card("Cemetery Change");
    let spawns = [hand(0, change), grave(0, WARWOLF), grave(0, THUNDER_KID), grave(1, DARK_BLADE), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&spawns) else { return };
    s.run(&[Activate(change)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", THUNDER_KID)), (1, 1), "my cards are in team 1's Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "and team 1's Dark Blade in mine: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN) + s.count(0, "grave", change) , 2, "the destroyed monster and the Spell went on: {}", s.context());
    assert_eq!(s.lp(0), 7000, "{}", s.context());
}

/// Armored Gravitation: "Special Summon up to 4 Level 4 or lower Armor monsters from your Deck, in face-up Attack Position. Send
/// them to the Graveyard during the End Phase." A Deck of Armor monsters (Active Guard, Trap Buster, Over Boost, Jet Gauntlet and
/// Double Cloth Armor, Level 6) brings four Level 4 or lower ones to the field in Attack Position, never Double Cloth Armor, and
/// the End Phase sends them to the Graveyard.
#[test]
fn armored_gravitation_summons_armor_monsters_from_the_deck_for_a_turn() {
    let [gravitation, guard, buster, boost, gauntlet, cloth] =
        ["Armored Gravitation", "Active Guard", "Trap Buster", "Over Boost", "Jet Gauntlet", "Double Cloth Armor"].map(card);
    let armor = [guard, buster, boost, gauntlet, cloth];
    let deck: Vec<u16> = armor.iter().cycle().take(30).copied().chain([FILLER; 10]).collect();
    let Some(mut s) = duel_with(7, &deck, [&[], &[]], &[hand(0, gravitation)]) else { return };
    s.prefer(&[(0, guard), (0, buster), (0, boost), (0, gauntlet), (0, cloth)]);
    s.prefer_most(true);
    let before = deck_size(&s, 0);
    s.run(&[Activate(gravitation)]).unwrap();
    clean(&s);
    let level_4: [u16; 4] = [guard, buster, boost, gauntlet];
    let summoned: usize = level_4.iter().map(|&c| s.count(0, "mzone", c)).sum();
    let on_field = monsters(&s, 0);
    assert_eq!(on_field, 4, "up to 4 Armor monsters: {}", s.context());
    assert_eq!(summoned, on_field, "all Level 4 or lower: {}", s.context());
    assert_eq!(s.count(0, "mzone", cloth), 0, "Double Cloth Armor is Level 6: {}", s.context());
    assert!(level_4.iter().all(|&c| s.count(0, "mzone", c) == 0 || s.position_of(0, c) == Some(1)), "face-up Attack Position: {}", s.context());
    assert_eq!(deck_size(&s, 0), before - 4, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(monsters(&s, 0), 0, "sent to the Graveyard at the End Phase: {}", s.context());
    assert_eq!(level_4.iter().map(|&c| s.count(0, "grave", c)).sum::<usize>(), 4, "{}", s.context());
}

/// Magnet Armor, a Trap: "Special Summon 1 Armor monster from your Graveyard in face-up Attack Position. Destroy it during the End Phase."
/// Jet Gauntlet comes back from the Graveyard (Warwolf next to it stays where it is) and is destroyed at the End Phase.
#[test]
fn magnet_armor_revives_an_armor_monster_for_a_turn() {
    let [magnet, gauntlet] = ["Magnet Armor", "Jet Gauntlet"].map(card);
    let Some(mut s) = field(&[set(0, magnet), grave(0, gauntlet), grave(0, WARWOLF)]) else { return };
    s.run(&[Activate(magnet)]).unwrap();
    assert_eq!(s.count(0, "grave", magnet), 1, "{}", s.context());
    clean(&s);
    assert_eq!(s.count(0, "mzone", gauntlet), 1, "{}", s.context());
    assert_eq!(s.position_of(0, gauntlet), Some(1), "face-up Attack Position: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", gauntlet), s.count(0, "grave", gauntlet)), (0, 1), "destroyed at the End Phase: {}", s.context());
}

/// Full Armor Gravitation: "Excavate the top 10 cards of your Deck. Special Summon as many Armor monsters as possible from among
/// those cards, in the order they were excavated, then banish the remaining cards." The Deck (shuffled by the core: its top is
/// what the seed made it) holds Armor monsters among Feral Imps; for each of five seeds the Armor monsters among the ten are
/// Special Summoned up to the five free Monster Zones, and every other excavated card (Armor monsters left over as well) is
/// banished: ten cards leave the Deck in all. One of the seeds has more than five Armor monsters on top, one fewer.
#[test]
fn full_armor_gravitation_summons_the_armor_monsters_and_banishes_the_rest() {
    let [gravitation, guard, boost, gauntlet] = ["Full Armor Gravitation", "Active Guard", "Over Boost", "Jet Gauntlet"].map(card);
    let armor = [guard, boost, gauntlet];
    let deck: Vec<u16> = armor.iter().cycle().take(21).copied().chain([FILLER; 19]).collect();
    let mut seen = (false, false); // seeds with more than five Armor monsters on top, with fewer
    for seed in 1..=8 {
        let Some(mut s) = duel_with(seed, &deck, [&[], &[]], &[hand(0, gravitation)]) else { return };
        let before = deck_size(&s, 0);
        s.run(&[Activate(gravitation)]).unwrap();
        clean(&s);
        let summoned: usize = armor.iter().map(|&c| s.count(0, "mzone", c)).sum();
        let banished_armor: usize = armor.iter().map(|&c| s.count(0, "removed", c)).sum();
        let top = summoned + banished_armor;
        assert_eq!(summoned, top.min(5), "seed {seed}: as many as there is room for: {}", s.context());
        assert_eq!(s.state()[0]["removed"].as_array().unwrap().len(), 10 - summoned, "seed {seed}: the rest is banished: {}", s.context());
        assert_eq!(deck_size(&s, 0), before - 10, "seed {seed}: ten cards were excavated: {}", s.context());
        assert_eq!(s.count(0, "grave", gravitation), 1, "{}", s.context());
        assert!(armor.iter().all(|&c| s.count(0, "mzone", c) == 0 || s.position_of(0, c) == Some(1)), "face-up Attack: {}", s.context());
        seen = (seen.0 || top > 5, seen.1 || top < 5);
    }
    assert_eq!(seen, (true, true), "the seeds cover both cases");
}

/// Full Armor Gravitation: with only two Monster Zones free (Warwolf, Dark Blade and Thunder Kid on the field), two Armor
/// monsters are Special Summoned and everything else of the ten, any others Armor monsters too, is banished.
#[test]
fn full_armor_gravitation_summons_only_as_many_as_there_is_room_for() {
    let [gravitation, guard, boost, gauntlet] = ["Full Armor Gravitation", "Active Guard", "Over Boost", "Jet Gauntlet"].map(card);
    let armor = [guard, boost, gauntlet];
    let deck: Vec<u16> = armor.iter().cycle().take(33).copied().chain([FILLER; 7]).collect();
    let spawns = [hand(0, gravitation), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(0, THUNDER_KID)];
    let Some(mut s) = duel_with(7, &deck, [&[], &[]], &spawns) else { return };
    s.run(&[Activate(gravitation)]).unwrap();
    clean(&s);
    let summoned: usize = armor.iter().map(|&c| s.count(0, "mzone", c)).sum();
    assert_eq!(summoned, 2, "two free zones, two Armor monsters: {}", s.context());
    assert_eq!(s.state()[0]["removed"].as_array().unwrap().len(), 8, "the other eight are banished: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Psychic Armor Head and Data Brain
// ---------------------------------------------------------------------------------------------------------------

/// Psychic Armor Head: "During your Draw Phase, instead of conducting your normal draw: You can add 1 Armor monster from your Deck
/// to your hand." In turn 3 the draw is replaced: Over Boost, the Deck's only Armor monster, comes to the hand and the hand grows
/// by one card, as with a draw. Without the Head, the same duel's draw takes a Feral Imp.
#[test]
fn psychic_armor_head_takes_an_armor_monster_in_place_of_the_draw() {
    let [head, boost] = ["Psychic Armor Head", "Over Boost"].map(card);
    let deck = deck_of(&[boost]);
    let Some(mut s) = duel_with(7, &deck, [&[], &[]], &[mon(0, head)]) else { return };
    assert_eq!(s.count(0, "hand", boost), 0, "Over Boost starts in the Deck: {}", s.context());
    let (hand0, deck0) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", boost), 1, "the Armor monster is in my hand: {}", s.context());
    assert_eq!((hand_size(&s, 0), deck_size(&s, 0)), (hand0 + 2, deck0 - 2), "turn 1's draw and one card instead of turn 3's: {}", s.context());
    // Without the Head the draw is a normal one: a Feral Imp, and the same count.
    let Some(mut s) = duel_with(7, &deck, [&[], &[]], &[]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "hand", boost), 0, "a normal draw takes a Feral Imp: {}", s.context());
}

/// Psychic Armor Head: "Once per turn, during your Standby Phase, if this card is in your Graveyard: You can Special Summon it
/// in face-up Attack Position." In turn 3's Standby Phase the Head comes back from the Graveyard.
#[test]
fn psychic_armor_head_comes_back_from_the_graveyard_in_the_standby_phase() {
    let head = card("Psychic Armor Head");
    let Some(mut s) = field(&[grave(0, head)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", head), s.count(0, "grave", head)), (1, 0), "{}", s.context());
    assert_eq!(s.position_of(0, head), Some(1), "face-up Attack Position: {}", s.context());
}

/// Data Brain: "Equip only to "Psychic Armor Head". If the equipped monster is destroyed by an opponent's Spell effect, during
/// your next turn: You can activate this card from your Graveyard as that Spell, and its effect is the same as that Spell's. If
/// this card is activated this way, destroy it during the End Phase." Team 0 equips it to the Head; team 1's Dark Hole destroys
/// the Head (which comes back in turn 3's Standby Phase) and the Data Brain goes to the Graveyard; in turn 3 team 0 Summons
/// Warwolf and activates Data Brain from the Graveyard as Dark Hole: Warwolf and team 1's Feral Imp are destroyed, and the Data
/// Brain is back in the Graveyard.
#[test]
fn data_brain_repeats_the_spell_that_destroyed_its_monster() {
    let [brain, head] = ["Data Brain", "Psychic Armor Head"].map(card);
    let dark_hole = card("Dark Hole");
    let spawns = [hand(0, brain), hand(0, WARWOLF), mon(0, head), hand(1, dark_hole), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(brain), EndTurn, Summon(WARWOLF)], &[Activate(dark_hole), Summon(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "Dark Hole destroyed team 1's monster: {}", s.context());
    assert_eq!(s.count(0, "grave", brain), 1, "and the Data Brain went to the Graveyard with the Head: {}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", FILLER)), (1, 1), "turn 3: Warwolf and team 1's Imp stand: {}", s.context());
    s.run(&[Activate(brain)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", FILLER)), (0, 0), "the Data Brain was a Dark Hole: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 1, "{}", s.context());
    // A copy of a Normal Spell goes to the Graveyard as it resolves (Data Brain keeps itself on the field only as a copy of a
    // Continuous, Equip or Field Spell): the Graveyard holds it again, in that turn already.
    assert_eq!((s.count(0, "szone", brain), s.count(0, "grave", brain)), (0, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Defender Iceberg (engine/overrides/c511000643.lua)
// ---------------------------------------------------------------------------------------------------------------

/// Defender Iceberg: "Your opponent cannot target other Aqua, Fish, and Sea Serpent-Type monsters you control for attacks, except
/// "Defender Iceberg". If a monster attacks this Defense Position card and its ATK is lower than this card's DEF, damage
/// calculation is not applied." Team 0 holds Defender Iceberg (0/2450) in Defense Position and Great White Terror (Aqua): in turn
/// 2 Warwolf (2000) can only attack the Iceberg, and 2000 is lower than 2450, so nobody is hurt and nothing is destroyed, where
/// without it the same attack would cost team 1 450 Life Points. Without Iceberg the Aqua monster is attacked and destroyed.
#[test]
fn defender_iceberg_protects_the_other_sea_monsters_and_stops_weaker_attackers() {
    let [iceberg, terror] = ["Defender Iceberg", "Great White Terror"].map(card);
    let Some(mut s) = field(&[mon(0, iceberg), mon(0, terror), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Reposition(iceberg)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, iceberg), Some(4), "face-up Defense Position: {}", s.context());
    assert_eq!((s.count(0, "mzone", iceberg), s.count(0, "mzone", terror)), (1, 1), "nothing was destroyed: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "damage calculation is not applied: {}", s.context());
    // The control: with no Iceberg the Aqua monster is a target and dies.
    let Some(mut s) = field(&[mon(0, terror), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", terror), 1, "{}", s.context());
}

/// Defender Iceberg: the damage rule is for an attack on the Iceberg itself (its script read the target's DEF before looking for
/// a target and stopped on a Lua error at every direct attack on the field: the override). With the Iceberg on the field a
/// direct attack by Warwolf (team 1 has no monster) takes 2000 Life Points, and an attacker with more ATK than its DEF (a
/// Defense Position Iceberg attacked by Dark Magician, 2500 against 2450) destroys it and its controller is not damaged either.
#[test]
fn defender_iceberg_lets_a_direct_attack_through_and_loses_to_a_stronger_attacker() {
    let iceberg = card("Defender Iceberg");
    let Some(mut s) = field(&[mon(0, iceberg), mon(0, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6000, "a direct attack: {}", s.context());
    let Some(mut s) = field(&[mon(0, iceberg), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[Reposition(iceberg)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", iceberg), 1, "2500 beats 2450: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "a Defense Position monster costs no damage: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Judgment Blaster, Chain of the Underworld and Command Angel
// ---------------------------------------------------------------------------------------------------------------

/// Judgment Blaster: "Discard 5 cards from your hand; destroy all monsters on the field." Five Feral Imps are discarded and
/// Warwolf and Dark Blade, team 1's Dark Magician and team 1's Warwolf are destroyed, the Blaster being in the Graveyard too.
#[test]
fn judgment_blaster_discards_five_and_destroys_every_monster() {
    let blaster = card("Judgment Blaster");
    let spawns = [hand(0, blaster), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    let before = (s.count(0, "hand", FILLER), s.count(0, "grave", FILLER));
    assert!(before.0 > 5, "the hand holds the Imps: {}", s.context());
    s.run(&[Activate(blaster)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FILLER), before.1 + 5, "five discarded: {}", s.context());
    assert_eq!(s.count(0, "hand", FILLER), before.0 - 5, "{}", s.context());
    assert_eq!((monsters(&s, 0), monsters(&s, 1)), (0, 0), "all monsters destroyed: {}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", DARK_BLADE), s.count(1, "grave", DARK_MAGICIAN), s.count(1, "grave", WARWOLF)), (1, 1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", blaster), 1, "{}", s.context());
}

/// Chain of the Underworld: "When an opponent's monster is destroyed and sent to the Graveyard: Special Summon that monster to
/// your opponent's side of the field in Attack Position, but it loses 700 ATK, also your opponent draws 1 card." Team 1's Thunder
/// Kid (700) attacks Warwolf (2000) and is destroyed; it returns to team 1's side in Attack Position at 0 ATK, and team 1 draws a
/// card.
#[test]
fn chain_of_the_underworld_returns_the_destroyed_monster_weaker_and_gives_a_draw() {
    let chain = card("Chain of the Underworld");
    let Some(mut s) = field(&[set(0, chain), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let before = hand_size(&s, 1);
    s.run_both([&[Chain(chain)], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", chain), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "Thunder Kid is back on team 1's side: {}", s.context());
    assert_eq!(s.attack_of(1, THUNDER_KID), Some(0), "700 - 700: {}", s.context());
    assert_eq!(s.position_of(1, THUNDER_KID), Some(1), "Attack Position: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 0, "{}", s.context());
    assert_eq!(hand_size(&s, 1), before + 1 + 1, "team 1 drew for the turn and for the effect: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1300, "the battle itself cost team 1 1300: {}", s.context());
}

/// Command Angel (Anime): "If you control another monster, monsters your opponent controls cannot target this card for attacks.
/// All Fairy-Type monsters you control gain 400 ATK." Team 0's Angel (1200) is a Fairy and stands at 1600. With Warwolf next to
/// it, Dark Magician (team 1, 2500) can only attack Warwolf: Warwolf dies and the Angel is untouched; alone the Angel can be
/// attacked and is destroyed.
#[test]
fn command_angel_cannot_be_attacked_while_another_monster_stands() {
    let angel = card("Command Angel (Anime)");
    let Some(mut s) = field(&[mon(0, angel), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    assert_eq!(s.attack_of(0, angel), Some(1600), "a Fairy gains 400: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "{}", s.context());
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", angel), s.count(0, "grave", WARWOLF)), (1, 1), "only Warwolf could be attacked: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
    // Alone it can be.
    let Some(mut s) = field(&[mon(0, angel), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", angel), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 900, "2500 against 1600: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Armor monsters: Active Guard, Big Bang Blow, Buster Pyle and Trap Buster
// ---------------------------------------------------------------------------------------------------------------

/// Active Guard: "During damage calculation, if you would take battle damage, OR when a card or effect is activated that would
/// inflict damage to you (Quick Effect): You can activate this effect; you take no damage for the rest of this turn, also this
/// card is destroyed during the End Phase." Team 0's Thunder Kid (700) attacks Warwolf (2000) in turn 3: Active Guard is
/// activated before the damage calculation and team 0 takes none (Thunder Kid still dies); where without it team 0 would take 1300.
/// Active Guard is destroyed at the End Phase.
#[test]
fn active_guard_prevents_the_battle_damage_and_is_destroyed_at_the_end_phase() {
    let guard = card("Active Guard");
    let spawns = [mon(0, guard), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    s.run_both([&[EndTurn, Attack(THUNDER_KID), Chain(guard)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID) + s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid lost the battle: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", guard), 1, "Active Guard waits for the End Phase: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", guard), 1, "destroyed during the End Phase: {}", s.context());
    // The control: with no Active Guard the damage is dealt.
    let Some(mut s) = field(&[mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}

/// Big Bang Blow: "If this card is destroyed by battle while you control 2 or more monsters: Destroy as many monsters on the field
/// as possible, and if you do, both players take damage equal to the total ATK those destroyed monsters had while face-up on the
/// field." Team 0 holds Big Bang Blow (0 ATK) with Thunder Kid (700) and Dark Blade (1800); team 1's Warwolf (2000) attacks and
/// destroys it. Everything else is destroyed (Dark Blade, Thunder Kid and Warwolf) and both players take 700 + 1800 + 2000 = 4500
/// (Big Bang Blow's own 0 adding nothing).
#[test]
fn big_bang_blow_destroys_the_field_and_both_players_take_the_total_atk() {
    let blow = card("Big Bang Blow");
    let Some(mut s) = field(&[mon(0, blow), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, blow)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", blow), 1, "destroyed by battle: {}", s.context());
    assert_eq!((monsters(&s, 0), monsters(&s, 1)), (0, 0), "all monsters destroyed: {}", s.context());
    assert_eq!((s.count(0, "grave", THUNDER_KID), s.count(0, "grave", DARK_BLADE), s.count(1, "grave", WARWOLF)), (1, 1, 1), "{}", s.context());
    // Warwolf's 2000 against the Blow's 0 first, then 4500 for each.
    assert_eq!(s.lp(1), 8000 - 4500, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000 - 4500, "{}", s.context());
    // Alone (after the battle only one other monster) it does nothing.
    let Some(mut s) = field(&[mon(0, blow), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, blow)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", blow), s.count(0, "mzone", THUNDER_KID), s.count(1, "mzone", WARWOLF)), (1, 1, 1), "one other monster is not enough: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 2000, 8000), "{}", s.context());
}

/// Buster Pyle: "Cannot be destroyed by battle. You take no battle damage from battles involving this card. After damage
/// calculation, if this card battles an opponent's monster: Destroy that monster, and if you do, inflict 500 damage to your
/// opponent." Team 1's Dark Magician (2500) attacks Buster Pyle (0 ATK, team 0's only monster): the Pyle stays, Dark Magician is
/// destroyed, team 1 takes 500 and team 0 takes nothing.
#[test]
fn buster_pyle_destroys_the_monster_it_battles_and_burns_500() {
    let pyle = card("Buster Pyle");
    let Some(mut s) = field(&[mon(0, pyle), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", pyle), 1, "not destroyed by battle: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 500), "{}", s.context());
}

/// Trap Buster: "Negate the effects of your opponent's Traps that target exactly 1 Armor monster you control or would destroy 2 or
/// more monsters on the field." Team 1's Raigeki Break aimed at the Trap Buster itself is negated (it is in the Graveyard,
/// the Trap Buster stays); aimed at Warwolf, no Armor monster, it destroys it.
#[test]
fn trap_buster_negates_a_trap_aimed_at_an_armor_monster() {
    let buster = card("Trap Buster");
    let Some(mut s) = field(&[mon(0, buster), mon(0, WARWOLF), set(1, RAIGEKI_BREAK)]) else { return };
    s.prefer(&[(0, buster), (1, FILLER)]);
    s.run_both([&[EndTurn], &[Activate(RAIGEKI_BREAK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", RAIGEKI_BREAK), 1, "it was activated: {}", s.context());
    assert_eq!((s.count(0, "mzone", buster), s.count(0, "mzone", WARWOLF)), (1, 1), "its effect was negated: {}", s.context());
    // Aimed at a plain monster it works.
    let Some(mut s) = field(&[mon(0, buster), mon(0, WARWOLF), set(1, RAIGEKI_BREAK)]) else { return };
    s.prefer(&[(0, WARWOLF), (1, FILLER)]);
    s.run_both([&[EndTurn], &[Activate(RAIGEKI_BREAK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Attack Guidance Barrier, Revenge Sacrifice, Magical Academy and Pillager
// ---------------------------------------------------------------------------------------------------------------

/// Attack Guidance Barrier: "Activate only when an opponent's monster declares an attack. Change the attack target to a Defense
/// Position monster you control. It gains 600 DEF until the End Phase. After that battle, end the Battle Phase." Team 1's
/// Thunder Kid (700) attacks Dark Blade (Attack Position, 1800); the Barrier turns the attack on team 0's Defense Position
/// Thunder Kid (DEF 600 + 600 = 1200): it survives, the attacker takes 500, Dark Blade is untouched and the Battle Phase ends
/// (team 1's second attacker, Feral Imp, never gets to attack).
#[test]
fn attack_guidance_barrier_redirects_the_attack_to_a_defense_monster_with_600_more() {
    let barrier = card("Attack Guidance Barrier");
    let spawns = [set(0, barrier), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, THUNDER_KID), mon(1, FILLER)];
    let Some(mut s) = field(&spawns) else { return };
    s.prefer(&[(0, DARK_BLADE), (0, THUNDER_KID)]);
    s.run_both([&[Reposition(THUNDER_KID), ChainOnAttack(barrier)], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", barrier), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "DEF 1200 against 700: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "the attacker took 1200 - 700: {}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over: {}", s.context());
}

/// Revenge Sacrifice: "If a monster you control is destroyed by battle: Tribute the monster that destroyed it, and if you do,
/// Special Summon 1 monster from your hand." Team 1's Warwolf destroys team 0's Thunder Kid; the Trap Tributes Warwolf (it is in
/// team 1's Graveyard) and Dark Blade comes from team 0's hand.
#[test]
fn revenge_sacrifice_tributes_the_attacker_and_summons_from_the_hand() {
    let sacrifice = card("Revenge Sacrifice");
    let Some(mut s) = field(&[set(0, sacrifice), mon(0, THUNDER_KID), hand(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, DARK_BLADE), (0, THUNDER_KID)]);
    s.run_both([&[Summon(FILLER), Chain(sacrifice)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", WARWOLF)), (1, 0), "the attacker was Tributed: {}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(0, "hand", DARK_BLADE)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", sacrifice), 1, "{}", s.context());
}

/// Magical Academy: "Discard 2 cards; Special Summon 1 Spellcaster-Type monster from your hand, that can be Normal Summoned/Set,
/// by Tributing the same number of monsters needed for a Tribute Summon, then increase its Level by 2 and ATK by 500, also,
/// negate the effects of your opponent's Spells/Traps that target the Summoned monster this turn." Dark Magician (Level 7, two
/// Tributes) comes from the hand over Warwolf and Thunder Kid, as a Level 9 monster with 3000 ATK; two Feral Imps are discarded.
/// Team 1's Raigeki Break, in answer, is negated and Dark Magician stays.
#[test]
fn magical_academy_summons_a_spellcaster_over_its_tributes_and_protects_it() {
    let academy = card("Magical Academy");
    let spawns = [hand(0, academy), hand(0, DARK_MAGICIAN), mon(0, WARWOLF), mon(0, THUNDER_KID), set(1, RAIGEKI_BREAK)];
    let Some(mut s) = field(&spawns) else { return };
    s.prefer(&[(0, FILLER), (0, WARWOLF), (0, THUNDER_KID), (0, DARK_MAGICIAN), (1, FILLER)]);
    s.prefer_most(true);
    let imps = s.count(0, "grave", FILLER);
    // Raigeki Break is chained at the first chance after Dark Magician is on the field (earlier chances come at the turn's
    // start and with Magical Academy on the chain, when it would have destroyed a Tribute).
    let listed = |p: &serde_json::Value| p["selects"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == RAIGEKI_BREAK));
    let mut answered = false;
    for _ in 0..12 {
        s.run_to(&[Activate(academy)], |p| p["type"] == msg::SELECT_CHAIN && p["player"] == 1 && listed(p).is_some()).unwrap();
        if s.prompt()["type"] != msg::SELECT_CHAIN {
            break;
        }
        if s.count(0, "mzone", DARK_MAGICIAN) == 1 {
            let index = listed(s.prompt()).unwrap();
            s.respond(Response::Chain(Some(index)).json()).unwrap();
            answered = true;
            break;
        }
        s.pass().unwrap();
    }
    assert!(answered, "team 1 had a chance to answer with Dark Magician on the field: {}", s.context());
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "Dark Magician came to the field and stays: {}", s.context());
    assert_eq!((s.attack_of(0, DARK_MAGICIAN), s.level_of(0, DARK_MAGICIAN)), (Some(3000), Some(9)), "+500 ATK, +2 Levels: {}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", THUNDER_KID)), (1, 1), "both Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), imps + 2, "two cards discarded: {}", s.context());
    assert_eq!(s.count(0, "grave", academy), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", RAIGEKI_BREAK), 1, "Raigeki Break was activated and had no effect: {}", s.context());
}

/// Pillager: "Look at your opponent's hand. Choose 1 card in it and add it to your hand." Team 1's hand holds a Dark Magician
/// among its Feral Imps: it is the card chosen, and moves to team 0's hand; team 1's hand is one card shorter.
#[test]
fn pillager_takes_the_chosen_card_from_the_opponents_hand() {
    let pillager = card("Pillager");
    let Some(mut s) = field(&[hand(0, pillager), hand(1, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(1, DARK_MAGICIAN)]);
    let (mine, theirs) = (hand_size(&s, 0), hand_size(&s, 1));
    assert_eq!(s.count(1, "hand", DARK_MAGICIAN), 1, "{}", s.context());
    s.run(&[Activate(pillager)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", DARK_MAGICIAN), s.count(1, "hand", DARK_MAGICIAN)), (1, 0), "{}", s.context());
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), (mine, theirs - 1), "the Spell left my hand, a card came: {}", s.context());
    assert_eq!(s.count(0, "grave", pillager), 1, "{}", s.context());
}
