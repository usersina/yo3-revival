//! Anime-only cards, group 25 (rows 6024-6054 of engine/added/cards.tsv): Bashford's, Commander Koda's, Haley's, Leo's and
//! Misty Tredwell's cards. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play Spell answering the
//! opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! The batch makes no Token and has no script note.
//! Covered: the cards the per-card fuzz flagged (Junk Spirit, Maternal Junk, Crime and Punishment, Lightlow Protection, Mind
//! Monster, Circle of Life, Hollow Ghost, Mirror Prison, Respect Synchron, Sorrowful Chain), the signature cards (the Junk
//! family with Kid Junk, Scrap-Iron Pitfall and Barricade, Agent of Hatred, Hollow Spirit, Circle of Terror, Block Lock,
//! Remake Puzzle, Damage Eraser, Synchro Big Tornado, the three Sad Stories, Mirror Labyrinth) and the seeded sample of the
//! rest (Doom Gazer, Junk Factory, Level Mirroring, Mad Profiler, Paternal Junk, Sleepy Beauty), with Destruction Insurance.
//! Why the fuzz could not reach what it flagged: a Trap that answers one exact event (Junk Spirit: a Junk monster lost in
//! battle; Crime and Punishment: battle damage from a direct attack; Respect Synchron: the opponent's own Synchro Monster
//! sent to the Graveyard by its own effect), a card that needs another one on the field (Maternal Junk: Paternal Junk;
//! Sorrowful Chain: the three Sad Stories; Hollow Ghost: only Hollow Spirit summons it), a declaration the fuzz makes at
//! random (Mind Monster names one card of the opponent's Extra Deck: most guesses miss, and a miss deals no damage), a
//! passive effect (Mirror Prison, Lightlow Protection's trigger needs the opponent to change the ATK), and Circle of Life's
//! "send a Spell" is its cost (its Category says TOGRAVE, but the card goes to the Graveyard as the cost, never in the
//! operation, so "promised, never done" is the fuzz's report of a harmless Category). Level Mirroring has an override
//! (`engine/overrides/c511001304.lua`: the Level lasts only until the End Phase, as the text says).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const HINOTAMA: u16 = 4350; // 500 damage
const MYSTICAL_SPACE_TYPHOON: u16 = 4909;
const TRAP_HOLE: u16 = 4836;
const PYRAMID_TURTLE: u16 = 5423; // a Level 4 Zombie
const VAMPIRE_LORD: u16 = 5410; // a Level 5 Zombie, 2000/1500
const JUNK_SYNCHRON: u16 = 7687; // Level 3 Tuner, 1300/500, a "Junk"
const JUNK_WARRIOR: u16 = 7696; // Level 5 Synchro, 2300/1300, a "Junk"
const GOYO_GUARDIAN: u16 = 7736; // a Synchro Monster, 2800/2000
const REMOTEN: u16 = 8166; // Morphtronic Remoten: Level 3, 300/1200
const VIDEON: u16 = 8167; // Morphtronic Videon: Level 4, 1000/1000
const CELFON: u16 = 7874; // Morphtronic Celfon: Level 1, 100/100
const GAIA_KNIGHT: u16 = 7697; // Gaia Knight, the Force of Earth: a Synchro Monster, 2600/800

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
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

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

// ---------------------------------------------------------------------------------------------------------------
// Bashford: the Junk family, Junk Factory, Junk Spirit, Scrap-Iron Barricade and Pitfall
// ---------------------------------------------------------------------------------------------------------------

/// Junk Factory: "All "Junk" monsters gain 500 ATK. If a "Junk" monster destroys an opponent's monster by battle, you can
/// return that monster to the Deck instead of sending it to the Graveyard." Both sides' Junk monsters gain (Junk Synchron
/// 1300 to 1800, the opponent's Junk Warrior 2300 to 2800); Junk Synchron then beats Thunder Kid (700) in turn 3 and the Kid
/// goes back to its Deck, not the Graveyard.
#[test]
fn junk_factory_raises_every_junk_monster_and_returns_what_they_destroy() {
    let [factory] = ["Junk Factory"].map(card);
    let spawns = [hand(0, factory), mon(0, JUNK_SYNCHRON), mon(1, JUNK_WARRIOR)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(factory)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, JUNK_SYNCHRON), Some(1800), "{}", s.context());
    assert_eq!(s.attack_of(1, JUNK_WARRIOR), Some(2800), "the opponent's Junk monsters gain too: {}", s.context());
    // The battle: only Thunder Kid is there to attack.
    let spawns = [hand(0, factory), mon(0, JUNK_SYNCHRON), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let deck = deck_size(&s, 1);
    s.run(&[Activate(factory), EndTurn, Attack(JUNK_SYNCHRON), Chain(factory)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1100, "1800 against 700: {}", s.context());
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "mzone", THUNDER_KID)), (0, 0), "not in the Graveyard: {}", s.context());
    assert_eq!(deck_size(&s, 1), deck, "drew a card in turn 2 and got Thunder Kid back: {}", s.context());
    // The control: a non-Junk attacker sends it to the Graveyard.
    let Some(mut s) = field(&[], &[hand(0, factory), mon(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let deck = deck_size(&s, 1);
    s.run(&[Activate(factory), EndTurn, Attack(DARK_BLADE)]).unwrap();
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(deck_size(&s, 1), deck - 1, "{}", s.context());
}

/// Junk Spirit: "Activate only when a "Junk" monster you control is destroyed by battle and sent to the Graveyard. During
/// the End Phase, Special Summon that monster." Team 1's Warwolf (2000) destroys Junk Synchron (1300) in turn 2; at the
/// End Phase the Synchron is back on team 0's field and Junk Spirit is in the Graveyard. (The per-card fuzz never got a
/// Junk monster lost in battle with this Trap Set.)
#[test]
fn junk_spirit_brings_back_the_junk_monster_lost_in_battle_at_the_end_phase() {
    let spirit = card("Junk Spirit");
    let Some(mut s) = field(&[], &[set(0, spirit), mon(0, JUNK_SYNCHRON), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(spirit)], &[Attack(WARWOLF)]]).unwrap();
    s.run_both([&[], &[EndTurn]]).unwrap(); // the End Phase
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_SYNCHRON), 1, "Junk Synchron came back: {}", s.context());
    assert_eq!(s.count(0, "grave", JUNK_SYNCHRON), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", spirit), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "the battle was fought: {}", s.context());
    // Not a Junk monster: it is not offered.
    let Some(mut s) = field(&[], &[set(0, spirit), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", spirit), 1, "Junk Spirit stays Set: {}", s.context());
}

/// Maternal Junk, Paternal Junk and Kid Junk: "Once per turn, if you control a "Paternal Junk": You can add 1 "Kid Junk"
/// from your Deck or Graveyard to your hand." and Paternal Junk "Gains 500 ATK while you control a "Maternal Junk". Gains
/// 1000 ATK while you control a "Kid Junk"." With both parents Maternal Junk fetches Kid Junk from the Deck (once a turn);
/// Paternal Junk is 1600 + 500 = 2100, and 3100 when Kid Junk is Summoned. From the Graveyard when it is not in the Deck;
/// without Paternal Junk the effect is not offered.
#[test]
fn maternal_junk_fetches_kid_junk_and_paternal_junk_grows_with_the_family() {
    let [maternal, paternal, kid] = ["Maternal Junk", "Paternal Junk", "Kid Junk"].map(card);
    let spawns = [mon(0, maternal), mon(0, paternal)];
    let Some(mut s) = field(&[kid], &spawns) else { return };
    keep_hands(&mut s);
    assert_eq!(s.attack_of(0, paternal), Some(1600 + 500), "{}", s.context());
    let deck = deck_size(&s, 0);
    s.run(&[Activate(maternal)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", kid), 1, "Kid Junk is in my hand: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 1, "from the Deck: {}", s.context());
    assert!(!offered(&s, "activates").contains(&maternal.into()), "once per turn: {:?}", offered(&s, "activates"));
    s.run(&[Summon(kid)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, paternal), Some(1600 + 500 + 1000), "{}", s.context());
    assert_eq!(s.attack_of(0, kid), Some(300), "{}", s.context());
    // From the Graveyard.
    let Some(mut s) = field(&[], &[mon(0, maternal), mon(0, paternal), grave(0, kid)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(maternal)]).unwrap();
    assert_eq!((s.count(0, "hand", kid), s.count(0, "grave", kid)), (1, 0), "{}", s.context());
    // No Paternal Junk: nothing to activate.
    let Some(mut s) = field(&[kid], &[mon(0, maternal)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&maternal.into()), "{:?}", offered(&s, "activates"));
    // Only Maternal Junk: Paternal Junk has its 500 and not the 1000.
    let Some(s) = field(&[], &[mon(0, maternal), mon(0, paternal), mon(0, kid)]) else { return };
    assert_eq!(s.attack_of(0, paternal), Some(3100), "{}", s.context());
    let Some(s) = field(&[], &[mon(0, paternal)]) else { return };
    assert_eq!(s.attack_of(0, paternal), Some(1600), "alone: {}", s.context());
}

/// Scrap-Iron Barricade: "Target 1 face-up monster on the field; Set this card face-down instead of sending it to the
/// Graveyard. Also, that monster cannot be destroyed by your opponent's card effects." Team 0 answers team 1's Dark Hole
/// with it on Warwolf: Warwolf survives, team 1's own monster does not, and the Barricade is Set again, not in the Graveyard.
#[test]
fn scrap_iron_barricade_protects_a_monster_and_sets_itself_again() {
    let barricade = card("Scrap-Iron Barricade");
    let spawns = [set(0, barricade), mon(0, WARWOLF), hand(1, DARK_HOLE), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[EndTurn]).unwrap(); // to team 1's turn: the Trap answers its Dark Hole, not a phase before it
    s.run_both([&[Chain(barricade)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf was not destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "the opponent's own monster was: {}", s.context());
    assert_eq!(s.count(0, "szone", barricade), 1, "Set again: {}", s.context());
    assert_eq!(s.count(0, "grave", barricade), 0, "{}", s.context());
    // The control: without it Warwolf goes too.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// Scrap-Iron Pitfall: "When your opponent Special Summons exactly 1 monster: Return that monster to your opponent's hand
/// and Set this card face-down again instead of sending it to the Graveyard." Team 1's Monster Reborn brings Warwolf back;
/// Pitfall returns it to team 1's hand and is Set again.
#[test]
fn scrap_iron_pitfall_returns_a_special_summoned_monster_and_sets_itself_again() {
    let pitfall = card("Scrap-Iron Pitfall");
    let spawns = [set(0, pitfall), hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn]).unwrap();
    s.run_both([&[Chain(pitfall)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", WARWOLF), 1, "back in team 1's hand: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(0, "szone", pitfall), 1, "Set again: {}", s.context());
    assert_eq!(s.count(0, "grave", pitfall), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Commander Koda: Agent of Hatred, Crime and Punishment, Destruction Insurance, Lightlow Protection, Mad Profiler, Mind Monster
// ---------------------------------------------------------------------------------------------------------------

/// Agent of Hatred: "Once per turn, at the end of your opponent's Battle Phase: You can gain LP equal to the battle damage
/// you took from 1 attack this turn." Team 1's Dark Magician (2500) attacks in turn 2, Agent of Hatred beside Thunder Kid
/// takes no part in it: whatever battle damage team 0 took (the control without Agent shows how much) it gets back.
#[test]
fn agent_of_hatred_gives_back_the_battle_damage_of_one_attack() {
    let agent = card("Agent of Hatred");
    // The control: no Agent, team 0 takes the damage.
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    let lost = 8000 - s.lp(0);
    assert_eq!(lost, 1800, "2500 against Thunder Kid's 700: {}", s.context());
    // With Agent of Hatred.
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), mon(0, agent), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(agent)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "the 1800 came back at the end of the Battle Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the battle was fought: {}", s.context());
    assert_eq!(s.count(0, "mzone", agent), 1, "{}", s.context());
}

/// Crime and Punishment: "When you take battle damage from a direct attack: Destroy 1 monster your opponent controls, also,
/// at the end of the Battle Phase, you can Special Summon 1 Level 4 or lower monster from your hand." Team 1's Warwolf
/// attacks the empty field in turn 2 (2000 damage); team 0 destroys Warwolf, and at the end of the Battle Phase a Level 4
/// Feral Imp from its hand is Special Summoned. Team 1's other monster stays.
#[test]
fn crime_and_punishment_destroys_the_direct_attacker_and_summons_from_the_hand() {
    let crime = card("Crime and Punishment");
    let Some(mut s) = field(&[], &[set(0, crime), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn, Chain(crime)], &[Attack(WARWOLF)]]).unwrap();
    s.run_both([&[], &[EndTurn]]).unwrap(); // the Battle Phase ends: "you can Special Summon" is asked and answered yes
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "the direct attack hit: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "only one monster: {}", s.context());
    assert_eq!(s.count(0, "grave", crime), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "a Level 4 from the hand at the end of the Battle Phase: {}", s.context());
}

/// Crime and Punishment answers a direct attack only: with a monster to attack, damage from battle with it does not count.
#[test]
fn crime_and_punishment_is_not_for_an_attack_on_a_monster() {
    let crime = card("Crime and Punishment");
    let Some(mut s) = field(&[], &[set(0, crime), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "szone", crime), 1, "still Set: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Destruction Insurance: "When the equipped monster is destroyed, inflict damage to your opponent equal to half the
/// original ATK of the equipped monster." Warwolf (2000) wears it and is destroyed by Dark Hole: the opponent takes 1000.
#[test]
fn destruction_insurance_hurts_the_opponent_for_half_the_atk() {
    let insurance = card("Destruction Insurance");
    let spawns = [hand(0, insurance), mon(0, WARWOLF), hand(0, DARK_HOLE), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(insurance), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", insurance), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1000), "half of 2000: {}", s.context());
}

/// Lightlow Protection: "If the ATK of the equipped monster is changed by the effect of an opponent's card or effect:
/// Destroy it." Team 1's Shrink on Warwolf destroys it (and the Protection with it); the control, Rush Recklessly of its
/// own controller on the same monster, changes the ATK and leaves it alone.
#[test]
fn lightlow_protection_destroys_a_monster_whose_atk_the_opponent_changes() {
    let protection = card("Lightlow Protection");
    let (shrink, rush) = (card("Shrink"), card("Rush Recklessly"));
    let spawns = [hand(0, protection), mon(0, WARWOLF), hand(1, shrink)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Activate(protection), EndTurn], &[Activate(shrink)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", shrink), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", protection), 1, "{}", s.context());
    // Its own controller's Rush Recklessly: no destruction.
    let Some(mut s) = field(&[], &[hand(0, protection), mon(0, WARWOLF), hand(0, rush)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(protection), Activate(rush)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2700), "{}", s.context());
    assert_eq!(s.count(0, "szone", protection), 1, "{}", s.context());
}

/// Mad Profiler: "You can Special Summon this card (from your hand) by Tributing 3 monsters you control. If you do, this
/// card gains the following effect. You can send 1 card from your hand to the Graveyard, then target 1 card on the field
/// of the same card type as the card you sent; banish it." Three Tributes bring it out (2600/1600); discarding a monster
/// banishes Warwolf, and discarding a Spell (Dian Keto) banishes the Spell team 1 controls.
#[test]
fn mad_profiler_banishes_a_card_of_the_type_it_discards() {
    let profiler = card("Mad Profiler");
    let spawns = [hand(0, profiler), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(0, FILLER), mon(1, WARWOLF), spell(1, DIAN_KETO), hand(0, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, FILLER), (1, WARWOLF)]);
    s.run(&[SpSummon(profiler)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, profiler), Some(2600), "{}", s.context());
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(0, "grave", THUNDER_KID), s.count(0, "grave", FILLER)), (1, 1, 1), "three Tributes: {}", s.context());
    s.run(&[Activate(profiler)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "a monster for a monster: {}", s.context());
    assert_eq!(s.count(1, "szone", DIAN_KETO), 1, "{}", s.context());
    // A Spell for a Spell.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DIAN_KETO), (1, DIAN_KETO)]);
    s.run(&[SpSummon(profiler), Activate(profiler)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", DIAN_KETO), 1, "the Spell: {}", s.context());
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "the one it discarded: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Mind Monster: "Declare 1 monster name; if that card is in your opponent's Extra Deck, inflict damage to your opponent
/// equal to half of that monster's ATK." Goyo Guardian (2800) in team 1's Extra Deck, declared: 1400 damage; Gaia Knight (2600)
/// there too: 1300. Feral Imp declared (a card of the duel, not in the Extra Deck): none. Not offered at all when the Extra
/// Deck is empty.
#[test]
fn mind_monster_hurts_for_half_the_atk_of_the_declared_monster_in_the_extra_deck() {
    let mind = card("Mind Monster");
    for (declared, damage) in [(GOYO_GUARDIAN, 1400), (GAIA_KNIGHT, 1300), (FILLER, 0)] {
        let Some(mut s) = field_extra([&[], &[GOYO_GUARDIAN, GAIA_KNIGHT]], &[hand(0, mind)]) else { return };
        s.declare_card(Some(declared));
        s.run(&[Activate(mind)]).unwrap();
        clean(&s);
        assert_eq!(s.lp(1), 8000 - damage, "declared {declared}: {}", s.context());
        assert_eq!(s.count(0, "grave", mind), 1, "{}", s.context());
    }
    let Some(mut s) = field(&[], &[hand(0, mind)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&mind.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Haley: Circle of Life, Circle of Terror, Hollow Spirit and Hollow Ghost, Mirror Prison, Sleepy Beauty
// ---------------------------------------------------------------------------------------------------------------

/// Circle of Life: "During each of their turns, the turn player can send 1 Spell Card from their hand to the Graveyard.
/// Neither player can activate or Set Spell Cards from the hand unless they have sent a card to the Graveyard with this
/// effect during this turn." With it on the field no Spell in team 0's hand can be activated or Set; sending Dian Keto
/// with the effect (no LP gained: it is only sent) lets Dark Hole be activated. (The per-card fuzz flags "TOGRAVE
/// promised, never done": the card is sent as the effect's cost, which is the text; the Category is just its label.)
#[test]
fn circle_of_life_lets_the_turn_player_activate_spells_only_after_sending_one() {
    let circle = card("Circle of Life");
    let spawns = [hand(0, circle), hand(0, DIAN_KETO), hand(0, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DIAN_KETO)]);
    s.run(&[Activate(circle)]).unwrap();
    clean(&s);
    assert!(!offered(&s, "activates").contains(&DARK_HOLE.into()), "Spells are locked: {:?}", offered(&s, "activates"));
    assert!(!offered(&s, "activates").contains(&DIAN_KETO.into()), "{:?}", offered(&s, "activates"));
    assert!(!offered(&s, "spell_sets").contains(&DARK_HOLE.into()), "or Set: {:?}", offered(&s, "spell_sets"));
    s.run(&[Activate(circle)]).unwrap(); // its own effect: send Dian Keto
    clean(&s);
    assert_eq!((s.count(0, "grave", DIAN_KETO), s.lp(0)), (1, 8000), "sent, not activated: {}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole is allowed now: {}", s.context());
    // The opponent's turn: its Spells are locked until it sends one.
    let spawns = [hand(0, circle), hand(1, DIAN_KETO), hand(1, DARK_HOLE), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, DIAN_KETO)]);
    s.run_both([&[Activate(circle), EndTurn], &[]]).unwrap();
    assert!(!offered(&s, "activates").contains(&DARK_HOLE.into()), "{:?}", offered(&s, "activates"));
    s.run_both([&[], &[Activate(circle), Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", DIAN_KETO), s.count(0, "grave", WARWOLF)), (1, 1), "{}", s.context());
}

/// Circle of Terror: "During each of their turns, the turn player can send 1 Monster Card from their hand to the Graveyard.
/// Neither player can Normal or Special Summon monsters from the hand unless they have sent a card to the Graveyard with
/// this effect during this turn." With it Dark Blade cannot be Normal Summoned; after discarding a Feral Imp with the
/// effect it can, and the discard is once per turn.
#[test]
fn circle_of_terror_lets_the_turn_player_summon_only_after_sending_a_monster() {
    let terror = card("Circle of Terror");
    let Some(mut s) = field(&[], &[hand(0, terror), hand(0, DARK_BLADE)]) else { return };
    s.prefer(&[(0, FILLER)]);
    s.run(&[Activate(terror)]).unwrap();
    clean(&s);
    assert!(!offered(&s, "summons").contains(&DARK_BLADE.into()), "no Normal Summon: {:?}", offered(&s, "summons"));
    let imps = s.count(0, "hand", FILLER);
    s.run(&[Activate(terror)]).unwrap(); // its own effect
    clean(&s);
    assert_eq!((s.count(0, "hand", FILLER), s.count(0, "grave", FILLER)), (imps - 1, 1), "{}", s.context());
    assert!(offered(&s, "summons").contains(&DARK_BLADE.into()), "{:?}", offered(&s, "summons"));
    assert!(!offered(&s, "activates").contains(&terror.into()), "once per turn: {:?}", offered(&s, "activates"));
    s.run(&[Summon(DARK_BLADE)]).unwrap();
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
}

/// Hollow Spirit: "Once per turn, if there is a Zombie-Type monster in your Graveyard: you can inflict 800 damage to your
/// opponent." Pyramid Turtle (a Zombie) in the Graveyard: 800 damage once; with no Zombie in the Graveyard it is not offered.
#[test]
fn hollow_spirit_inflicts_800_with_a_zombie_in_the_graveyard() {
    let spirit = card("Hollow Spirit");
    let Some(mut s) = field(&[], &[mon(0, spirit), grave(0, PYRAMID_TURTLE)]) else { return };
    s.run(&[Activate(spirit)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&spirit.into()), "once per turn: {:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[mon(0, spirit)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&spirit.into()), "no Zombie in the Graveyard: {:?}", offered(&s, "activates"));
}

/// Hollow Spirit and Hollow Ghost: "When this card is destroyed and sent to the Graveyard: you can Special Summon 1
/// "Hollow Ghost" from your hand, Deck, or Graveyard." and Hollow Ghost's "Each time a card(s) is sent to the Graveyard:
/// Inflict 600 damage to your opponent. If "Hollow Spirit" is not in your Graveyard, this card's ATK becomes 0." Dark Hole
/// destroys Hollow Spirit, Hollow Ghost comes from the Deck (2600 ATK, Spirit being in the Graveyard); no damage for the
/// cards that went to the Graveyard before it; Dian Keto, sent after, costs team 1 600 LP (and gains 1000 for team 0).
/// (The per-card fuzz never summoned Hollow Ghost: only Hollow Spirit can.)
#[test]
fn hollow_spirit_calls_hollow_ghost_which_hurts_for_every_card_sent_to_the_graveyard() {
    let [spirit, ghost] = ["Hollow Spirit", "Hollow Ghost"].map(card);
    let spawns = [mon(0, spirit), hand(0, DARK_HOLE), hand(0, DIAN_KETO), mon(1, FILLER)];
    let Some(mut s) = field(&[ghost], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE), Chain(spirit)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ghost), 1, "Hollow Ghost came out of the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", spirit), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, ghost), Some(2600), "Hollow Spirit is in the Graveyard: {}", s.context());
    assert_eq!(s.lp(1), 8000, "nothing sent after it came: {}", s.context());
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (9000, 8000 - 600), "{}", s.context());
}

/// Hollow Ghost: "If "Hollow Spirit" is not in your Graveyard, this card's ATK becomes 0." and "Cannot be Normal Summoned/
/// Set. Must be Special Summoned with "Hollow Spirit", and cannot be Special Summoned by other ways.": 0 ATK alone, 2600
/// with Hollow Spirit in the Graveyard; neither Monster Reborn nor a Normal Summon can bring it.
#[test]
fn hollow_ghost_has_no_atk_without_hollow_spirit_in_the_graveyard_and_comes_only_from_it() {
    let [spirit, ghost] = ["Hollow Spirit", "Hollow Ghost"].map(card);
    let Some(s) = field(&[], &[mon(0, ghost)]) else { return };
    assert_eq!(s.attack_of(0, ghost), Some(0), "{}", s.context());
    let Some(s) = field(&[], &[mon(0, ghost), grave(0, spirit)]) else { return };
    assert_eq!(s.attack_of(0, ghost), Some(2600), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, ghost), grave(0, ghost), hand(0, MONSTER_REBORN)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&ghost.into()), "no Normal Summon: {:?}", offered(&s, "summons"));
    assert!(!offered(&s, "monster_sets").contains(&ghost.into()), "no Set: {:?}", offered(&s, "monster_sets"));
    assert!(!offered(&s, "activates").contains(&MONSTER_REBORN.into()), "no Monster Reborn: {:?}", offered(&s, "activates"));
}

/// Mirror Prison: "While this card is in your Graveyard, monsters your opponent controls, except Machine-Type and Zombie-
/// Type monsters, cannot declare an attack unless they Tribute 1 monster of the same Type." Team 1's Warwolf alone cannot
/// attack; Pyramid Turtle (a Zombie) can; a Warrior (Dark Blade) attacks by Tributing the other Warrior beside it.
#[test]
fn mirror_prison_makes_an_attacker_tribute_a_monster_of_its_type_unless_machine_or_zombie() {
    let prison = card("Mirror Prison");
    let marauding_captain = 5318; // a Warrior
    let Some(mut s) = field(&[], &[grave(0, prison), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).expect_err("nothing of its Type to Tribute");
    assert!(err.contains("can't attack"), "{err}");
    // A Zombie attacks freely.
    let Some(mut s) = field(&[], &[grave(0, prison), mon(0, THUNDER_KID), mon(1, PYRAMID_TURTLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(PYRAMID_TURTLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", PYRAMID_TURTLE) + s.count(1, "grave", PYRAMID_TURTLE), 1, "{}", s.context());
    assert!(s.lp(0) < 8000 || s.count(0, "grave", THUNDER_KID) == 1, "the attack was made: {}", s.context());
    // A Warrior with another Warrior beside it: the Tribute is the cost.
    let spawns = [grave(0, prison), mon(0, THUNDER_KID), mon(1, DARK_BLADE), mon(1, marauding_captain)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, marauding_captain)]);
    s.run_both([&[EndTurn], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", marauding_captain), 1, "Tributed to attack: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "and the attack was made: {}", s.context());
    // The control: without Mirror Prison in the Graveyard Warwolf attacks.
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
}

/// Sleepy Beauty: "While this card is in your Graveyard, reduce the Level of all Zombie-Type monsters in your hand by 1."
/// Vampire Lord (Level 5, two Tributes) is Level 4 in the hand and can be Normal Summoned with none; without Sleepy
/// Beauty in the Graveyard it cannot.
#[test]
fn sleepy_beauty_lowers_the_level_of_zombies_in_the_hand() {
    let sleepy = card("Sleepy Beauty");
    let Some(mut s) = field(&[], &[grave(0, sleepy), hand(0, VAMPIRE_LORD)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "summons").contains(&VAMPIRE_LORD.into()), "Level 4: {:?}", offered(&s, "summons"));
    s.run(&[Summon(VAMPIRE_LORD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", VAMPIRE_LORD), 1, "{}", s.context());
    assert_eq!(s.level_of(0, VAMPIRE_LORD), Some(5), "on the field it is Level 5 again: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, VAMPIRE_LORD)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&VAMPIRE_LORD.into()), "Level 5: {:?}", offered(&s, "summons"));
}

// ---------------------------------------------------------------------------------------------------------------
// Leo: Block Lock, Damage Eraser, Level Mirroring, Remake Puzzle, Respect Synchron, Synchro Big Tornado
// ---------------------------------------------------------------------------------------------------------------

/// Block Lock: "When your opponent targets a monster on the field for an attack: You can change the target to another
/// "Morphtronic" monster you control." Warwolf attacks one of Remoten and Videon (the control shows which); with Block
/// Lock the other is attacked and destroyed instead (and its controller takes the other's damage).
#[test]
fn block_lock_changes_the_attack_target_to_another_morphtronic() {
    let lock = card("Block Lock");
    let Some(mut s) = field(&[], &[mon(0, REMOTEN), mon(0, VIDEON), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    let (first, other) = if s.count(0, "grave", REMOTEN) == 1 { (REMOTEN, VIDEON) } else { (VIDEON, REMOTEN) };
    assert_eq!(s.count(0, "grave", first), 1, "one of them was attacked: {}", s.context());
    let Some(mut s) = field(&[], &[set(0, lock), mon(0, REMOTEN), mon(0, VIDEON), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, ChainOnAttack(lock)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", other), 1, "the other one was attacked now: {}", s.context());
    assert_eq!(s.count(0, "mzone", first), 1, "and {first} was spared: {}", s.context());
    assert_eq!(s.count(0, "grave", lock), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - if other == REMOTEN { 1700 } else { 1000 }, "damage by the new target's ATK: {}", s.context());
}

/// Damage Eraser: "If you would take damage from an opponent's card effect: Gain an equal amount of LP, instead." Team 1's
/// Hinotama (500 damage) is answered: team 0 gains 500 (8500) where without Damage Eraser it loses them (7500).
#[test]
fn damage_eraser_turns_effect_damage_into_life_points() {
    let eraser = card("Damage Eraser");
    let Some(mut s) = field(&[], &[set(0, eraser), hand(1, HINOTAMA)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(eraser)], &[Activate(HINOTAMA)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8500, "{}", s.context());
    assert_eq!(s.count(0, "grave", eraser), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(1, HINOTAMA)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(HINOTAMA)]]).unwrap();
    assert_eq!(s.lp(0), 7500, "{}", s.context());
}

/// Level Mirroring: "Target 1 monster you control; Its Level becomes the same as 1 monster your opponent controls until the
/// End Phase." Dark Magician (Level 7) becomes Level 2 like Thunder Kid, and is Level 7 again once the turn is over
/// (engine/overrides/c511001304.lua: upstream's Level never wore off).
#[test]
fn level_mirroring_copies_a_level_until_the_end_phase() {
    let mirroring = card("Level Mirroring");
    let Some(mut s) = field(&[], &[hand(0, mirroring), mon(0, DARK_MAGICIAN), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(mirroring)]).unwrap();
    clean(&s);
    assert_eq!(s.level_of(0, DARK_MAGICIAN), Some(2), "{}", s.context());
    assert_eq!(s.level_of(1, THUNDER_KID), Some(2), "{}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    assert_eq!(s.level_of(0, DARK_MAGICIAN), Some(7), "until the End Phase: {}", s.context());
    // Equal Levels have nothing to mirror.
    let Some(mut s) = field(&[], &[hand(0, mirroring), mon(0, WARWOLF), mon(1, FILLER)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&mirroring.into()), "{:?}", offered(&s, "activates"));
}

/// Remake Puzzle: "During your opponent's Battle Phase: Destroy 1 "Morphtronic" monster you control, and if you do, end the
/// Battle Phase, then, if that monster is sent to the Graveyard, Special Summon 1 "Morphtronic" monster from your Graveyard
/// whose Level is less than the Level of the monster destroyed by this effect." Warwolf attacks Videon (Level 4), Remake
/// Puzzle destroys it: no damage, the turn goes on to Main Phase 2 (no Battle Phase prompt, Dark Blade's attack never
/// comes) and Celfon (Level 1) returns from the Graveyard.
#[test]
fn remake_puzzle_destroys_a_morphtronic_ends_the_battle_phase_and_revives_a_lower_one() {
    let puzzle = card("Remake Puzzle");
    let spawns = [set(0, puzzle), mon(0, VIDEON), grave(0, CELFON), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, ChainOnAttack(puzzle)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", VIDEON), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", CELFON), 1, "a lower Level Morphtronic came back: {}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was over: {}", s.context());
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "the Battle Phase ended, no more attacks: {}", s.context());
    // The control: without it the Battle Phase goes on to the next attacker.
    let Some(mut s) = field(&[], &[mon(0, VIDEON), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.prompt()["type"], msg::SELECT_BATTLECMD, "{}", s.context());
}

/// Respect Synchron: "Activate only when a Synchro Monster your opponent controls is sent to the Graveyard by your
/// opponent's card effect: Special Summon that monster to your side of the field, ignoring its Summoning conditions, even
/// if it was not properly Special Summoned first. When that monster leaves the field, destroy this card. When this card
/// leaves the field, destroy that monster." Team 1's own Dark Hole destroys its Gaia Knight, which comes to team 0's field;
/// a second Dark Hole destroys it there and Respect Synchron goes with it. (The per-card fuzz found no Synchro Monster
/// of the opponent's destroyed by the opponent itself.)
#[test]
fn respect_synchron_takes_the_opponents_synchro_monster_and_goes_when_it_goes() {
    let respect = card("Respect Synchron");
    let spawns = [set(0, respect), mon(1, GAIA_KNIGHT), hand(1, DARK_HOLE), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn]).unwrap();
    s.run_both([&[Chain(respect)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GAIA_KNIGHT), 1, "Gaia Knight is on team 0's side: {}", s.context());
    assert_eq!(s.count(0, "szone", respect), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", GAIA_KNIGHT), 0, "{}", s.context());
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", GAIA_KNIGHT), 1, "destroyed again, to its owner's Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", respect), 1, "Respect Synchron was destroyed with it: {}", s.context());
}

/// Respect Synchron: "When this card leaves the field, destroy that monster." Team 1's Mystical Space Typhoon on the Trap
/// destroys the Gaia Knight it holds.
#[test]
fn respect_synchron_destroys_its_monster_when_it_leaves_the_field() {
    let respect = card("Respect Synchron");
    let spawns = [set(0, respect), mon(1, GAIA_KNIGHT), hand(1, DARK_HOLE), hand(1, MYSTICAL_SPACE_TYPHOON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, respect)]);
    s.run(&[EndTurn]).unwrap();
    s.run_both([&[Chain(respect)], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!(s.count(0, "mzone", GAIA_KNIGHT), 1, "{}", s.context());
    s.run_both([&[], &[Activate(MYSTICAL_SPACE_TYPHOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", respect), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", GAIA_KNIGHT), 1, "its monster was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", GAIA_KNIGHT), 0, "{}", s.context());
}

/// Synchro Big Tornado: "Target 1 Synchro Monster you control and 1 monster your opponent controls; the monster your
/// opponent controls lose ATK equal to the DEF of your Synchro Monster." Gaia Knight (DEF 800) takes 800 from Warwolf
/// (2000 to 1200). With no Synchro Monster it is not offered.
#[test]
fn synchro_big_tornado_takes_the_synchro_monsters_defense_from_the_opponents_atk() {
    let tornado = card("Synchro Big Tornado");
    let Some(mut s) = field(&[], &[set(0, tornado), mon(0, GAIA_KNIGHT), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, GAIA_KNIGHT), (1, WARWOLF)]);
    s.run(&[EndTurn, Activate(tornado)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, WARWOLF), Some(1200), "{}", s.context());
    assert_eq!(s.attack_of(0, GAIA_KNIGHT), Some(2600), "{}", s.context());
    assert_eq!(s.count(0, "grave", tornado), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, tornado), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn]).unwrap();
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert!(!offered(&s, "activates").contains(&tornado.into()), "no Synchro Monster: {:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Misty Tredwell: Doom Gazer, Mirror Labyrinth, the Sad Stories and Sorrowful Chain
// ---------------------------------------------------------------------------------------------------------------

/// Doom Gazer: "When a card effect that would destroy a card(s) on the field is activated: Inflict 300 damage to your
/// opponent for each card that would be destroyed by that effect." Team 1's Dark Hole would destroy three monsters (two of
/// team 0's, one of its own): 900 damage to team 1, and Dark Hole still resolves.
#[test]
fn doom_gazer_hurts_for_each_card_a_destroying_effect_would_destroy() {
    let gazer = card("Doom Gazer");
    let spawns = [set(0, gazer), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(1, THUNDER_KID), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(gazer)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 900, "3 cards: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", DARK_BLADE), s.count(1, "grave", THUNDER_KID)), (1, 1, 1), "not negated: {}", s.context());
    assert_eq!(s.count(0, "grave", gazer), 1, "{}", s.context());
}

/// Mirror Labyrinth: "While you control no face-up Level 5 or higher monster, and the only monster you control is a face-up
/// Level 4 or lower monster, that monster can attack twice during each Battle Phase." Dark Blade alone attacks Thunder Kid
/// and Feral Imp in one Battle Phase; with a second monster of mine it can attack once only.
#[test]
fn mirror_labyrinth_lets_a_lone_low_level_monster_attack_twice() {
    let labyrinth = card("Mirror Labyrinth");
    let spawns = [hand(0, labyrinth), mon(0, DARK_BLADE), mon(1, THUNDER_KID), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(labyrinth), EndTurn, Attack(DARK_BLADE), Attack(DARK_BLADE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "grave", FILLER)), (1, 1), "two attacks: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1100 - 500, "{}", s.context());
    // With another monster: one attack.
    let spawns = [hand(0, labyrinth), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, FILLER), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(labyrinth), EndTurn, Attack(DARK_BLADE), Attack(DARK_BLADE)]).expect_err("not the only monster");
    assert!(err.contains("can't attack"), "{err}");
    // A Level 7 monster alone does not either.
    let Some(mut s) = field(&[], &[hand(0, labyrinth), mon(0, DARK_MAGICIAN), mon(1, THUNDER_KID), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(labyrinth), EndTurn, Attack(DARK_MAGICIAN), Attack(DARK_MAGICIAN)]).expect_err("Level 7");
    assert!(err.contains("can't attack"), "{err}");
}

/// The Sad Stories: "You cannot activate Spell Cards the turn you activate this card." Activating one locks every Spell
/// for both players that turn, and a Spell activated earlier in the turn keeps it from being activated.
#[test]
fn a_sad_story_and_spells_exclude_each_other_in_a_turn() {
    let [atrocious, memories] = ["Sad Story - Atrocious Day", "Sad Story - Sorrowful Memories"].map(card);
    let Some(mut s) = field(&[], &[hand(0, atrocious), hand(0, DIAN_KETO)]) else { return };
    s.run(&[Activate(atrocious)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", atrocious), 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&DIAN_KETO.into()), "no Spell this turn: {:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[hand(0, memories), hand(0, DIAN_KETO)]) else { return };
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    assert_eq!(s.lp(0), 9000, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&memories.into()), "a Spell was activated this turn: {:?}", offered(&s, "activates"));
}

/// The Sad Stories: "When the turn player draws a card(s) during the Draw Phase: They must reveal all cards they draw, and
/// shuffle all revealed [Trap Cards - Atrocious Day, monsters - Sorrowful Memories, Spell Cards - Unwavering Truth] into the
/// Deck." Team 1 draws in turn 2 from a Deck of 40 identical cards (a Trap, a monster or a Spell): its hand stays at 5
/// exactly when the card drawn is of the Story's kind and grows to 6 otherwise, whichever Story is on the field.
#[test]
fn the_sad_stories_shuffle_back_the_drawn_cards_of_their_kind() {
    let stories = [("Sad Story - Atrocious Day", TRAP_HOLE), ("Sad Story - Sorrowful Memories", FILLER), ("Sad Story - Unwavering Truth", DIAN_KETO)];
    for (name, kind) in stories {
        let story = card(name);
        for drawn in [TRAP_HOLE, FILLER, DIAN_KETO] {
            let Some(mut s) = duel(7, 8000, [FILLER, drawn], &[], &[spell(0, story)]) else { return };
            let (hand_before, deck_before) = (hand_size(&s, 1), deck_size(&s, 1));
            s.run_both([&[EndTurn], &[]]).unwrap();
            clean(&s);
            let shuffled = drawn == kind;
            assert_eq!(hand_size(&s, 1), hand_before + usize::from(!shuffled), "{name} against {drawn}: {}", s.context());
            assert_eq!(deck_size(&s, 1), deck_before - i64::from(!shuffled), "{name} against {drawn}: {}", s.context());
        }
    }
}

/// The Sad Stories: "the turn player": team 0's own draws are shuffled back too, Sorrowful Memories against a Deck of
/// monsters (the duel starts in turn 1's Draw Phase, so even that first draw is): its hand stays at 5 through turns 1 and 3,
/// where the control's grows by one each.
#[test]
fn a_sad_story_shuffles_the_controllers_own_draw_too() {
    let memories = card("Sad Story - Sorrowful Memories");
    for (spawns, kept) in [(vec![spell(0, memories)], true), (vec![], false)] {
        let Some(mut s) = field(&[], &spawns) else { return };
        s.run(&[]).unwrap();
        let turn_1 = hand_size(&s, 0);
        s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
        clean(&s);
        let (first, third) = if kept { (5, 5) } else { (6, 7) };
        assert_eq!((turn_1, hand_size(&s, 0)), (first, third), "Story on the field: {kept}: {}", s.context());
    }
}

/// Sorrowful Chain: "If you control 1 "Sad Story - Sorrowful Memories", "Sad Story - Unwavering Truth", and "Sad Story -
/// Dreadful Day": Special Summon 1 "Dragon Queen of Tragic Endings" from your hand or Deck." (The anime's third Story is
/// this batch's Sad Story - Atrocious Day, passcode 100000098.) With the three Stories Sorrowful Chain brings the Queen
/// from the Deck; with two it is not offered.
#[test]
fn sorrowful_chain_summons_the_dragon_queen_with_the_three_sad_stories() {
    let [chain, atrocious, memories, truth, queen] =
        ["Sorrowful Chain", "Sad Story - Atrocious Day", "Sad Story - Sorrowful Memories", "Sad Story - Unwavering Truth", "Dragon Queen of Tragic Endings"].map(card);
    let spawns = [set(0, chain), spell(0, atrocious), spell(0, memories), spell(0, truth)];
    let Some(mut s) = field(&[queen], &spawns) else { return };
    s.run(&[EndTurn, Activate(chain)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", queen), 1, "the Queen (in the Deck only) is on the field: {}", s.context());
    assert_eq!(s.attack_of(0, queen), Some(1900), "{}", s.context());
    assert_eq!(s.count(0, "grave", chain), 1, "{}", s.context());
    // Only two Stories.
    let spawns = [set(0, chain), spell(0, atrocious), spell(0, memories)];
    let Some(mut s) = field(&[queen], &spawns) else { return };
    let err = s.run(&[EndTurn, Activate(chain)]).expect_err("a Story missing");
    assert!(err.contains("never became possible"), "{err}");
}
