//! Anime-only cards, group 30 (rows 6217-6243 of engine/added/cards.tsv): the Video Game's versions of anime cards (the
//! Tag Force texts ProjectIgnis marks Video Game, which the anime showed too) and Bastion Misawa's Magnet Warriors Σ+, Ω− and
//! Σ−: Arkana, Weevil Underwood, Adrian Gecko, Aster Phoenix, Bastion Misawa, Blair Flannigan, Chazz Princeton, Don Zaloog,
//! Jaden Yuki, Jean-Louis Bonaparte, Jim Crocodile Cook, Lorenzo, Mad Dog and Mr. Huffington. These cards exist only in the
//! anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core with
//! both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap is Set
//! first (a Trap cannot be activated from the hand), and neither player has a Battle Phase in turn 1, so team 0's attacks wait
//! for turn 3.
//! Covered: the cards the per-card fuzz flagged (Fog Castle, Cupid's Kiss, Pride Shout, Dark Scorpion Retreat, Crowning of the
//! Emperor, Hot Sauce Bottle: the fuzz never had the cards they need, Fog Castle's destroyed monster, a Maiden Counter, a
//! monster with 0 ATK and some DEF, five Dark Scorpions, a Toy Soldier, a Set Spell destroyed), the Magnet Warriors' poles through the client's view (the client shows Effect Monsters; the
//! engine keeps Plus and Minus: attraction and the negated attack), Dizzy Tiger (`engine/overrides/c100000321.lua`: upstream
//! used EVENT_BATTLE_END, which the core no longer raises, and negated after the Flip effect had run), the signature cards
//! (Toy Soldier, Toy Emperor, Burst Impact, D - Mind, Happy Marriage, Flipping the Table with Dizzy Angel) and a seeded sample
//! of the rest (Dark Scorpion - Tragedy of Love, Doll of Demise, Happy Marriage, Last Machine Acid Virus, Plasma Warrior Eitom
//! (VG) and Soldier Ari), with Sacred Defense Barrier, B.E.S. Assault Core, Shield Recovery, Defense Maiden and The Rival's Name
//! on the way. There is no Token in the batch.
//! Not coverable: Defense Maiden's redirect. It switches an attack to a "Maiden in Love (VG)", a card the game does not have,
//! and the game's printed Maiden in Love makes every opposing monster attack it already, so no attack is ever declared on
//! another monster to be switched.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DARK_HOLE: u16 = 4342;
const MYSTICAL_SPACE_TYPHOON: u16 = 4909;
// Two cards of the client are named "Toy Soldier": the anime one (added row 6235, Konami id 16176) is the one tested here.
const TOY_SOLDIER: u16 = 16176;

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

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `special_summons`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// How many counters of one kind a card of a team's Monster Zones holds.
fn counters(s: &Scenario, team: usize, cid: u16, kind: &str) -> i64 {
    s.monster(team, cid).map_or(-1, |m| m["counters"][kind].as_i64().unwrap_or(0))
}

// ---------------------------------------------------------------------------------------------------------------
// Fog Castle
// ---------------------------------------------------------------------------------------------------------------

/// Fog Castle: "When a monster you control is destroyed and sent to the Graveyard: The Monster Zone it occupied cannot be
/// used, then Special Summon it to your side of the field." Warwolf is destroyed by Dark Hole (the opponent's Dark Blade
/// too, which Fog Castle does not bring back): Warwolf is on the field again and Dark Blade is in the Graveyard. (That the
/// zone is closed shows in the next test: five of them close all five zones.) Without the Field Spell the same Dark Hole leaves both in the Graveyard.
#[test]
fn fog_castle_brings_a_destroyed_monster_back_and_closes_its_zone() {
    let fog = card("Fog Castle");
    let Some(mut s) = field(&[], &[hand(0, fog), hand(0, DARK_HOLE), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.run(&[Activate(fog)]).unwrap();
    clean(&s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "the opponent's monster stays destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf came back: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 0, "{}", s.context());
    // The control.
    let Some(mut s) = field(&[], &[hand(0, DARK_HOLE), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "mzone", WARWOLF)), (1, 0), "{}", s.context());
}

/// Fog Castle: "If you no longer have a usable Monster Zone with this effect: You can send this card and your entire hand to
/// the Graveyard to target up to 4 monsters in your Graveyard; add them to your hand. You cannot conduct your Battle Phase
/// the turn you activate this effect." Five Dark Holes in a row close all five zones (each destroys the one monster, which
/// comes back in another zone); then Fog Castle and the rest of the hand go to the Graveyard and the four monsters of the
/// Graveyard return to the hand, and the turn has no Battle Phase.
#[test]
fn fog_castle_with_no_usable_zone_returns_monsters_from_the_graveyard() {
    let fog = card("Fog Castle");
    let mut spawns = vec![hand(0, fog), mon(0, WARWOLF), grave(0, DARK_BLADE), grave(0, DARK_MAGICIAN), grave(0, THUNDER_KID), grave(0, FILLER)];
    spawns.extend([hand(0, DARK_HOLE); 5]);
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DARK_BLADE), (0, DARK_MAGICIAN), (0, THUNDER_KID), (0, FILLER)]);
    s.prefer_most(true);
    s.run(&[Activate(fog)]).unwrap();
    s.run(&[Activate(DARK_HOLE); 5]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_HOLE), 5, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the last zone closed, with no zone left to return to: {}", s.context());
    s.run(&[Activate(fog)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", fog), 1, "Fog Castle was sent to the Graveyard: {}", s.context());
    for cid in [DARK_BLADE, DARK_MAGICIAN, THUNDER_KID, FILLER] {
        assert_eq!(s.count(0, "hand", cid), 1, "{cid} was added to the hand: {}", s.context());
    }
    assert!(s.prompt()["to_bp"] != true, "no Battle Phase this turn: {}", s.prompt());
}

// ---------------------------------------------------------------------------------------------------------------
// Cupid's Kiss, Happy Marriage and Defense Maiden
// ---------------------------------------------------------------------------------------------------------------

/// Cupid's Kiss: "At the end of the Damage Step, if the equipped monster you control attacked an opponent's monster with a
/// Maiden Counter, and you took battle damage: Take control of the opponent's monster." Maiden in Love (the game's printed
/// card) is attacked by Warwolf in turn 2 (opposing monsters must attack it) and puts the Maiden Counter on it; in turn 3
/// Thunder Kid, equipped with Cupid's Kiss, attacks Warwolf, loses 1300 LP of battle damage and takes control of Warwolf.
#[test]
fn cupids_kiss_takes_control_of_a_monster_with_a_maiden_counter() {
    let [maiden, kiss] = ["Maiden in Love", "Cupid's Kiss"].map(card);
    let Some(mut s) = field(&[], &[mon(0, maiden), mon(0, THUNDER_KID), hand(0, kiss), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (1, WARWOLF)]);
    s.choose_options(&[0]);
    s.run_both([&[Activate(kiss), EndTurn, Chain(maiden)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", kiss), 1, "Cupid's Kiss is equipped to Thunder Kid: {}", s.context());
    assert_eq!(counters(&s, 1, WARWOLF, "4240"), 1, "Warwolf has the Maiden Counter: {:?}", s.monster(1, WARWOLF));
    let lp = s.lp(0);
    s.run(&[Attack(THUNDER_KID)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), lp - 1300, "700 against 2000: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "I control Warwolf: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
}

/// Cupid's Kiss without a Maiden Counter on the monster attacked: nothing is taken.
#[test]
fn cupids_kiss_needs_the_maiden_counter() {
    let kiss = card("Cupid's Kiss");
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), hand(0, kiss), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[Activate(kiss), EndTurn, Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "Warwolf stays with its owner: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
}

/// Happy Marriage: "Must be activated while you control a face-up monster owned by your opponent. The equipped monster gains
/// ATK equal to the combined ATK of those monsters you control." Change of Heart takes Dark Blade (1800): Happy Marriage on
/// Warwolf makes it 2000 + 1800. Without a monster of the opponent's among mine it cannot be activated.
#[test]
fn happy_marriage_adds_the_attack_of_the_opponents_monsters_i_control() {
    let [marriage, change] = ["Happy Marriage", "Change of Heart"].map(card);
    let Some(mut s) = field(&[], &[hand(0, marriage), hand(0, change), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DARK_BLADE), (0, WARWOLF)]);
    let err = s.run(&[Activate(marriage)]).expect_err("no monster of the opponent's among mine");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = field(&[], &[hand(0, marriage), hand(0, change), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DARK_BLADE), (0, WARWOLF)]);
    s.run(&[Activate(change), Activate(marriage)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade is mine for the turn: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000 + 1800), "{}", s.context());
    assert_eq!(s.count(0, "szone", marriage), 1, "{}", s.context());
}

/// Defense Maiden (a Continuous Trap): activated from Set, it stays on the field face-up, with no "Maiden in Love (VG)" to
/// switch an attack to (see the header).
#[test]
fn defense_maiden_activates_and_stays_on_the_field() {
    let maiden = card("Defense Maiden");
    let Some(mut s) = field(&[], &[set(0, maiden), mon(0, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Activate(maiden)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", maiden), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Pride Shout
// ---------------------------------------------------------------------------------------------------------------

/// Pride Shout: "At the end of the Damage Step, if your monster with 0 ATK attacked directly while this card was already
/// face-up in your Spell & Trap Zone: Inflict damage to your opponent equal to the DEF of that monster you control."
/// Millennium Shield (0/3000) attacks directly in turn 3: 3000 damage. A Warwolf attacking directly does only its own 2000.
#[test]
fn pride_shout_inflicts_the_defense_of_a_zero_attack_monster() {
    let [pride, shield] = ["Pride Shout", "Millennium Shield"].map(card);
    let Some(mut s) = field(&[], &[set(0, pride), mon(0, shield)]) else { return };
    s.run_both([&[EndTurn, Activate(pride), Attack(shield)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 3000, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, pride), mon(0, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Activate(pride), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "only the battle damage: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dark Scorpion - Tragedy of Love and Dark Scorpion Retreat
// ---------------------------------------------------------------------------------------------------------------

/// Dark Scorpion - Tragedy of Love: "While you control a "Don Zaloog" and "Dark Scorpion - Meanae the Thorn": Send 1 "Dark
/// Scorpion - Meanae the Thorn" you control to the Graveyard; destroy all monsters your opponent controls." Meanae goes to
/// the Graveyard as the cost and both of the opponent's monsters are destroyed; Don Zaloog stays. Without Don Zaloog it is
/// not offered.
#[test]
fn tragedy_of_love_sends_meanae_and_destroys_the_opponents_monsters() {
    let [tragedy, don, meanae] = ["Dark Scorpion - Tragedy of Love", "Don Zaloog", "Dark Scorpion - Meanae the Thorn"].map(card);
    let spawns = [hand(0, tragedy), mon(0, don), mon(0, meanae), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(tragedy)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", meanae), 1, "Meanae was the cost: {}", s.context());
    assert_eq!(s.count(0, "mzone", don), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", tragedy), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, tragedy), mon(0, meanae), mon(1, WARWOLF)]) else { return };
    let err = s.run(&[Activate(tragedy)]).expect_err("no Don Zaloog");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Dark Scorpion Retreat: "If you control a "Don Zaloog", "Dark Scorpion - Cliff the Trap Remover", "Dark Scorpion - Chick the
/// Yellow", "Dark Scorpion - Gorg the Strong" and "Dark Scorpion - Meanae the Thorn": Return them to your hand." With all five
/// on the field the Trap returns the five to the hand; with one missing it cannot be activated.
#[test]
fn dark_scorpion_retreat_returns_the_five_to_the_hand() {
    let retreat = card("Dark Scorpion Retreat");
    let five = ["Don Zaloog", "Cliff the Trap Remover", "Dark Scorpion - Chick the Yellow", "Dark Scorpion - Gorg the Strong", "Dark Scorpion - Meanae the Thorn"].map(card);
    let mut spawns = vec![set(0, retreat)];
    spawns.extend(five.map(|c| mon(0, c)));
    let Some(mut s) = field(&[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run_both([&[EndTurn, Activate(retreat)], &[EndTurn]]).unwrap();
    clean(&s);
    for c in five {
        assert_eq!((s.count(0, "hand", c), s.count(0, "mzone", c)), (1, 0), "{c} returned to the hand: {}", s.context());
    }
    assert_eq!(s.count(0, "grave", retreat), 1, "{}", s.context());
    assert!(hand_size(&s, 0) >= before + 5, "{}", s.context());
    let mut spawns = vec![set(0, retreat)];
    spawns.extend(five[..4].iter().map(|&c| mon(0, c)));
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[EndTurn, Activate(retreat)], &[EndTurn]]).expect_err("Meanae is missing");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Toy Soldier, Toy Emperor and Crowning of the Emperor
// ---------------------------------------------------------------------------------------------------------------

/// Crowning of the Emperor: "Tribute 1 "Toy Soldier"; Special Summon 1 "Toy Emperor" from your hand or Deck." Toy Soldier is
/// Tributed and Toy Emperor (Level 6) comes from the Deck, with no Tribute of its own: it is face-up on the field, Soldier
/// in the Graveyard. With no Toy Soldier on the field it is not offered.
#[test]
fn crowning_of_the_emperor_tributes_a_toy_soldier_for_a_toy_emperor() {
    let [crowning, emperor] = ["Crowning of the Emperor", "Toy Emperor"].map(card);
    let soldier = TOY_SOLDIER;
    let Some(mut s) = field(&[emperor], &[hand(0, crowning), mon(0, soldier)]) else { return };
    s.run(&[Activate(crowning)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", soldier), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", emperor), 1, "Toy Emperor came from the Deck: {}", s.context());
    assert_eq!(s.position_of(0, emperor), Some(1), "{}", s.context());
    assert_eq!(s.count(0, "grave", crowning), 1, "{}", s.context());
    // From the hand too.
    let Some(mut s) = field(&[], &[hand(0, crowning), mon(0, soldier), hand(0, emperor)]) else { return };
    s.run(&[Activate(crowning)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", emperor), 1, "Toy Emperor came from the hand: {}", s.context());
    // Nothing to Tribute.
    let Some(mut s) = field(&[emperor], &[hand(0, crowning), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(crowning)]).expect_err("no Toy Soldier");
    assert!(err.contains("never became possible"), "{err}");
}

/// Toy Soldier: "Once per turn, during your Standby Phase: You can Special Summon 1 "Toy Soldier" from your Deck in Attack
/// Position." In turn 1 team 0 uses it: a second Toy Soldier is on the field, face-up in Attack Position, and none is left
/// in the Deck for a third.
#[test]
fn toy_soldier_summons_another_in_the_standby_phase() {
    let soldier = TOY_SOLDIER;
    let Some(mut s) = field(&[soldier], &[mon(0, soldier)]) else { return };
    s.run(&[Chain(soldier)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", soldier), 2, "{}", s.context());
    assert_eq!(s.position_of(0, soldier), Some(1), "{}", s.context());
}

/// Toy Emperor: "When this card destroys an opponent's monster by battle: You can add 1 Trap from your Deck to your hand."
/// In turn 3 Toy Emperor (2300) destroys Thunder Kid and Trap Hole comes from the bottom of the Deck.
#[test]
fn toy_emperor_adds_a_trap_after_destroying_a_monster_by_battle() {
    let [emperor, trap_hole] = ["Toy Emperor", "Trap Hole"].map(card);
    let Some(mut s) = field(&[trap_hole], &[mon(0, emperor), mon(1, THUNDER_KID)]) else { return };
    s.run_both([&[EndTurn, Attack(emperor), Chain(emperor)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", trap_hole), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Hot Sauce Bottle
// ---------------------------------------------------------------------------------------------------------------

/// Hot Sauce Bottle: "If this face-down card is destroyed and sent to the Graveyard: Inflict 500 damage to your opponent."
/// Team 1's Mystical Space Typhoon destroys the Set bottle in turn 2: its controller's opponent, team 1, takes 500.
/// Destroyed face-up (it is activated first, as a Spell) it does nothing.
#[test]
fn hot_sauce_bottle_burns_the_opponent_when_destroyed_face_down() {
    let bottle = card("Hot Sauce Bottle");
    let Some(mut s) = field(&[], &[set(0, bottle), hand(1, MYSTICAL_SPACE_TYPHOON)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(MYSTICAL_SPACE_TYPHOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bottle), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 500), "{}", s.context());
    // Face-up it is only a Spell: no damage.
    let Some(mut s) = field(&[], &[spell(0, bottle), hand(1, MYSTICAL_SPACE_TYPHOON)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(MYSTICAL_SPACE_TYPHOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bottle), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dizzy Tiger (engine/overrides/c100000321.lua), Dizzy Angel and Flipping the Table
// ---------------------------------------------------------------------------------------------------------------

/// Dizzy Tiger: "Negate the effects of face-down Defense Position monsters attacked by this card." Team 1's face-down Needle
/// Worm (Flip: send the top 5 cards of the opponent's Deck to the Graveyard) is attacked by Dizzy Tiger in turn 3: its Flip
/// effect does nothing and no card leaves team 0's Deck. The control is Warwolf attacking it: 5 cards go.
#[test]
fn dizzy_tiger_negates_a_flip_monster_it_attacks() {
    let [tiger, worm] = ["Dizzy Tiger", "Needle Worm"].map(card);
    let mut milled = vec![];
    for attacker in [tiger, WARWOLF] {
        let spawns = [Spawn { team: 1, place: MonsterSet, cid: worm }, mon(0, attacker)];
        let Some(mut s) = field(&[], &spawns) else { return };
        s.run_both([&[EndTurn, Attack(attacker)], &[EndTurn]]).unwrap();
        clean(&s);
        assert_eq!(s.count(1, "grave", worm), 1, "the Needle Worm (DEF 600) was destroyed: {}", s.context());
        milled.push(s.state()[0]["grave"].as_array().map_or(0, Vec::len));
    }
    assert_eq!(milled, [0, 5], "Dizzy Tiger's negation stopped the Flip effect: {milled:?}");
}

/// Flipping the Table: "If you control a "Dizzy" card: You can destroy all other cards you control. If you do, destroy cards
/// on your opponent's field equal to the number of cards you destroyed by this effect. You cannot Normal Summon/Set the turn
/// you activate this effect." Dizzy Angel "cannot be destroyed by the effect of Flipping the Table": with Warwolf and Thunder
/// Kid beside it two cards are destroyed and so two of the opponent's; Dizzy Angel stays. With Dizzy Tiger in its place Tiger
/// is destroyed too, and so are three of the opponent's cards.
#[test]
fn flipping_the_table_destroys_my_cards_and_as_many_of_the_opponents_but_not_dizzy_angel() {
    let [table, angel, tiger] = ["Flipping the Table", "Dizzy Angel", "Dizzy Tiger"].map(card);
    for (dizzy, mine, theirs) in [(angel, 2, 2), (tiger, 3, 3)] {
        let spawns = [hand(0, table), mon(0, dizzy), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, DARK_BLADE), mon(1, WARWOLF), mon(1, DARK_MAGICIAN), mon(1, FILLER)];
        let Some(mut s) = field(&[], &spawns) else { return };
        s.prefer(&[(1, DARK_BLADE), (1, WARWOLF), (1, DARK_MAGICIAN), (1, FILLER)]);
        s.prefer_most(true);
        s.run(&[Activate(table)]).unwrap();
        s.run(&[Activate(table)]).unwrap();
        clean(&s);
        let mine_gone = [WARWOLF, THUNDER_KID, dizzy].iter().filter(|&&c| s.count(0, "grave", c) == 1).count();
        let theirs_gone = [DARK_BLADE, WARWOLF, DARK_MAGICIAN, FILLER].iter().map(|&c| s.count(1, "grave", c)).sum::<usize>();
        assert_eq!((mine_gone, theirs_gone), (mine, theirs), "{}", s.context());
        assert_eq!(s.count(0, "mzone", angel), (dizzy == angel) as usize, "{}", s.context());
        assert_eq!(s.count(0, "szone", table), 1, "Flipping the Table itself stays: {}", s.context());
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Burst Impact, D - Mind and The Rival's Name
// ---------------------------------------------------------------------------------------------------------------

/// Burst Impact: "If you control an "Elemental HERO Burstinatrix": Destroy all monsters on the field, except "Elemental HERO
/// Burstinatrix", and if you do, players take damage equal to the number of monsters they controlled that were destroyed
/// x300." Team 0 has Burstinatrix and Warwolf, team 1 three monsters: team 0 takes 300, team 1 takes 900, Burstinatrix stays.
/// Without Burstinatrix it is not offered.
#[test]
fn burst_impact_destroys_all_but_burstinatrix_and_burns_300_for_each() {
    let [impact, burstinatrix] = ["Burst Impact", "Elemental Hero Burstinatrix"].map(card);
    let spawns = [hand(0, impact), mon(0, burstinatrix), mon(0, WARWOLF), mon(1, DARK_BLADE), mon(1, THUNDER_KID), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(impact)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", burstinatrix), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.state()[1]["grave"].as_array().map_or(0, Vec::len), 3, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 300, 8000 - 900), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, impact), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    let err = s.run(&[Activate(impact)]).expect_err("no Burstinatrix");
    assert!(err.contains("never became possible"), "{err}");
}

/// D - Mind: "If you control no "Destiny HERO" monsters: Special Summon 1 Level 3 or lower "Destiny HERO" monster from your
/// Deck." Doom Lord (Level 3) comes from the Deck; with a Destiny HERO already on the field it is not offered.
#[test]
fn d_mind_summons_a_low_level_destiny_hero_from_the_deck() {
    let [mind, doom_lord] = ["D - Mind", "Destiny Hero - Doom Lord"].map(card);
    let Some(mut s) = field(&[doom_lord], &[hand(0, mind)]) else { return };
    s.run(&[Activate(mind)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", doom_lord), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", mind), 1, "{}", s.context());
    let Some(mut s) = field(&[doom_lord], &[hand(0, mind), mon(0, doom_lord)]) else { return };
    let err = s.run(&[Activate(mind)]).expect_err("a Destiny HERO is on the field");
    assert!(err.contains("never became possible"), "{err}");
}

/// The Rival's Name: "Target 1 monster in your opponent's Graveyard; Special Summon this card in Defense Position and it is
/// treated as a Normal Monster (Spellcaster-Type/LIGHT/Level 1/ATK 0/DEF 0), whose name is treated as the targeted monster's
/// name." Dark Blade is in team 1's Graveyard: the card is a face-up Defense Position monster on team 0's field (a Spellcaster,
/// LIGHT, Level 1, 0/0) and not in the Spell & Trap Zone.
#[test]
fn the_rivals_name_becomes_a_normal_monster() {
    let rival = card("The Rival's Name");
    let Some(mut s) = field(&[], &[hand(0, rival), grave(1, DARK_BLADE)]) else { return };
    s.run(&[Activate(rival)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", rival), 1, "it is a monster now: {}", s.context());
    assert_eq!(s.position_of(0, rival), Some(4), "face-up Defense Position: {}", s.context());
    let m = s.monster(0, rival).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64(), m["attribute"].as_i64()), (Some(0), Some(0), Some(1), Some(2), Some(16)), "{m}");
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "the target stays in the Graveyard: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Sacred Defense Barrier
// ---------------------------------------------------------------------------------------------------------------

/// Sacred Defense Barrier: "Each time a Rock-Type monster is Normal or Flip Summoned: Change it to Defense Position, and if
/// you do, place 1 Protection Counter on it. If a monster with a Protection Counter would be destroyed by battle or card
/// effect while this card is on the field, remove 1 Protection Counter from it instead." Giant Soldier of Stone is Normal
/// Summoned: Defense Position and 1 counter; Dark Blade (not a Rock) stays in Attack Position with none. In turn 2 Dark
/// Magician (2500) attacks the Giant Soldier (DEF 2000), which would die: it stays, its counter is used up.
#[test]
fn sacred_defense_barrier_protects_a_summoned_rock() {
    let [barrier, giant] = ["Sacred Defense Barrier", "Giant Soldier of Stone"].map(card);
    let Some(mut s) = field(&[], &[hand(0, barrier), hand(0, giant), mon(1, DARK_MAGICIAN)]) else { return };
    s.run(&[Activate(barrier), Summon(giant)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, giant), Some(4), "changed to Defense Position: {}", s.context());
    assert_eq!(counters(&s, 0, giant, "4246"), 1, "a Protection Counter: {:?}", s.monster(0, giant));
    // Turn 2.
    let Some(mut s) = field(&[], &[hand(0, barrier), hand(0, giant), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[Activate(barrier), Summon(giant), EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", giant), 1, "the counter saved it: {}", s.context());
    assert_eq!(counters(&s, 0, giant, "4246"), 0, "and was used up: {:?}", s.monster(0, giant));
    // Not a Rock.
    let Some(mut s) = field(&[], &[hand(0, barrier), hand(0, DARK_BLADE)]) else { return };
    s.run(&[Activate(barrier), Summon(DARK_BLADE)]).unwrap();
    clean(&s);
    assert_eq!((s.position_of(0, DARK_BLADE), counters(&s, 0, DARK_BLADE, "4246")), (Some(1), 0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// B.E.S. Assault Core and Shield Recovery
// ---------------------------------------------------------------------------------------------------------------

/// B.E.S. Assault Core: "If only your opponent controls a monster, you can Special Summon this card (from your hand)." With
/// Warwolf alone on the other side it is offered and comes face-up; with a monster of mine too, it is not.
#[test]
fn bes_assault_core_special_summons_itself_when_only_the_opponent_has_a_monster() {
    let core = card("B.E.S. Assault Core");
    let Some(mut s) = field(&[], &[hand(0, core), mon(1, WARWOLF)]) else { return };
    s.run(&[SpSummon(core)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", core), 1, "{}", s.context());
    let m = s.monster(0, core).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64()), (Some(1300), Some(2000), Some(4)), "{m}");
    let Some(mut s) = field(&[], &[hand(0, core), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&core.into()), "{:?}", offered(&s, "special_summons"));
    let Some(mut s) = field(&[], &[hand(0, core)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&core.into()), "an empty field on both sides: {:?}", offered(&s, "special_summons"));
}

/// Shield Recovery: "Target 1 "B.E.S." monster you control; place 3 counters on it." B.E.S. Crystal Core gets 3 B.E.S. counters
/// (counter 0x1f); a Warwolf is no target, so with it alone the Spell is not offered.
#[test]
fn shield_recovery_places_three_counters_on_a_bes_monster() {
    let [recovery, crystal] = ["Shield Recovery", "B.E.S. Crystal Core"].map(card);
    let Some(mut s) = field(&[], &[hand(0, recovery), mon(0, crystal), mon(0, WARWOLF)]) else { return };
    s.prefer(&[(0, crystal)]);
    s.run(&[Activate(recovery)]).unwrap();
    clean(&s);
    assert_eq!(counters(&s, 0, crystal, "31"), 3, "{:?}", s.monster(0, crystal));
    assert_eq!(counters(&s, 0, WARWOLF, "31"), 0, "{:?}", s.monster(0, WARWOLF));
    let Some(mut s) = field(&[], &[hand(0, recovery), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(recovery)]).expect_err("no B.E.S. monster");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Last Machine Acid Virus
// ---------------------------------------------------------------------------------------------------------------

/// Last Machine Acid Virus (a Trap): "Tribute 1 WATER monster; look at your opponent's hand, all monsters they control, and
/// all cards they draw until the end of their 3rd turn after this card's activation, and destroy all Machine monsters.
/// Inflict 500 damage to your opponent for each monster destroyed by this effect." Team 1's Deck is all Mechanicalchasers
/// (Machines): activated in turn 3, its hand and the Mechanicalchaser on its field are destroyed for 500 each, the Tributed
/// Hyosube is in the Graveyard, and the Machine team 1 draws in turn 4 is destroyed at once for 500 more. Its Warwolf (not a
/// Machine) is not touched.
#[test]
fn last_machine_acid_virus_destroys_machines_in_the_hand_and_on_the_field_and_those_drawn() {
    let [virus, hyosube, chaser] = ["Last Machine Acid Virus", "Hyosube", "Mechanicalchaser"].map(card);
    let spawns = [set(0, virus), mon(0, hyosube), mon(1, chaser), mon(1, WARWOLF)];
    let Some(mut s) = duel(7, 8000, [FILLER, chaser], &[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn]).unwrap();
    let hand_before = hand_size(&s, 1);
    assert!(hand_before >= 5, "{}", s.context());
    s.run(&[Activate(virus)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", hyosube), 1, "the WATER monster was Tributed: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", chaser), 0, "{}", s.context());
    assert_eq!(hand_size(&s, 1), 0, "the hand was all Machines: {}", s.context());
    let destroyed = hand_before + 1;
    assert_eq!(s.lp(1), 8000 - 500 * destroyed as i64, "500 for each: {}", s.context());
    // The card team 1 draws in its turn is destroyed too.
    let lp = s.lp(1);
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), lp - 500, "the drawn Machine was destroyed: {}", s.context());
    assert_eq!(hand_size(&s, 1), 0, "{}", s.context());
}

/// Last Machine Acid Virus: "Tribute 1 WATER monster": with no WATER monster to Tribute it is not offered.
#[test]
fn last_machine_acid_virus_needs_a_water_monster_to_tribute() {
    let virus = card("Last Machine Acid Virus");
    let Some(mut s) = field(&[], &[set(0, virus), mon(0, WARWOLF)]) else { return };
    let err = s.run_both([&[EndTurn, Activate(virus)], &[]]).expect_err("no WATER monster");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Doll of Demise and Soldier Ari
// ---------------------------------------------------------------------------------------------------------------

/// Doll of Demise (Level 5, Fiend, DARK, 1600/1700) and Soldier Ari (Level 3, Insect, EARTH, 500/1200) are Normal Monsters:
/// the client's view of them has those stats, Doll of Demise is Tribute Summoned with one Tribute and Ari with none, and both
/// stand face-up in Attack Position.
#[test]
fn doll_of_demise_and_soldier_ari_are_normal_monsters() {
    let [doll, ari] = ["Doll of Demise", "Soldier Ari"].map(card);
    let Some(mut s) = field(&[], &[hand(0, doll), hand(0, ari), mon(0, THUNDER_KID)]) else { return };
    s.run(&[Summon(doll)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was the Tribute: {}", s.context());
    let m = s.monster(0, doll).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64(), m["attribute"].as_i64()), (Some(1600), Some(1700), Some(5), Some(8), Some(32)), "{m}");
    assert_eq!(s.position_of(0, doll), Some(1), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, ari)]) else { return };
    s.run(&[Summon(ari)]).unwrap();
    clean(&s);
    let m = s.monster(0, ari).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64(), m["attribute"].as_i64()), (Some(500), Some(1200), Some(3), Some(2048), Some(1)), "{m}");
}

// ---------------------------------------------------------------------------------------------------------------
// Plasma Warrior Eitom (VG)
// ---------------------------------------------------------------------------------------------------------------

/// Plasma Warrior Eitom (VG): "Cannot be Normal Summoned/Set. Must be Special Summoned (from your hand) by Tributing 1 Level 7
/// or higher monster. This card can attack directly, but when it does so using this effect, the battle damage inflicted to
/// your opponent is halved." Dark Magician (Level 7) is Tributed; in turn 3 Eitom (3000) attacks directly past Warwolf and
/// inflicts 1500; against an empty field it attacks directly in the ordinary way for the whole 3000.
#[test]
fn eitom_halves_its_direct_attack_past_a_monster() {
    let eitom = card("Plasma Warrior Eitom (VG)");
    let Some(mut s) = field(&[], &[hand(0, eitom), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&eitom.into()), "cannot be Normal Summoned: {:?}", offered(&s, "summons"));
    s.run(&[SpSummon(eitom)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "the Tribute: {}", s.context());
    assert_eq!(s.attack_of(0, eitom), Some(3000), "{}", s.context());
    s.run_to(&[EndTurn, Attack(eitom)], |p| p["type"] == msg::SELECT_BATTLECMD && p["player"] == 0).unwrap();
    let attack = s.prompt()["attacks"].as_array().and_then(|a| a.iter().find(|x| x["cid"] == eitom)).cloned().expect("Eitom can attack");
    assert_eq!(attack["can_direct"], true, "a direct attack is offered: {attack}");
    s.run(&[Attack(eitom)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1500, "half of 3000: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    // No monster on the other side: the usual direct attack, not halved.
    let Some(mut s) = field(&[], &[hand(0, eitom), mon(0, DARK_MAGICIAN)]) else { return };
    s.run(&[SpSummon(eitom), EndTurn, Attack(eitom)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 3000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Magnet Warrior Σ+, Ω− and Σ− (Bastion Misawa): the poles the client does not show
// ---------------------------------------------------------------------------------------------------------------

/// Magnet Warrior Σ+ (Plus, 1800/1500) and Σ−/Ω− (Minus, 1500/1800 and 1900/800): "If your opponent controls a Minus monster,
/// this card must attack that monster, if able" (the Minus ones the other way round). Team 1 holds Ω− in Defense Position
/// (800 DEF, a Minus monster that cannot attack itself) and a Dark Blade in Attack Position: in turn 3 Σ+ may not skip its
/// Battle Phase and has to attack Ω−, although the attack on the Dark Blade is picked first when team 0 prefers it; Ω− is
/// destroyed, the Dark Blade stays and nobody loses LP.
#[test]
fn sigma_plus_must_attack_the_opponents_minus_monster() {
    let [plus, omega] = ["Magnet Warrior Σ＋", "Magnet Warrior Ω－"].map(card);
    let spawns = [mon(0, plus), Spawn { team: 1, place: MonsterDefense, cid: omega }, mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, DARK_BLADE), (1, omega)]);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["to_ep"], false, "the End Phase is not offered while Σ+ must attack: {}", s.prompt());
    s.run(&[Attack(plus)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", omega), 1, "Ω- was the target: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    assert_eq!(s.count(0, "mzone", plus), 1, "{}", s.context());
}

/// The same from the Minus side: Ω− (1900/800) must attack the opponent's Σ+ (Defense Position, 1500 DEF) although a Dark
/// Blade stands beside it, and destroys it. And with no Plus monster on the other side there is no attraction: the Battle
/// Phase may be skipped.
#[test]
fn omega_minus_must_attack_the_opponents_plus_monster() {
    let [plus, omega] = ["Magnet Warrior Σ＋", "Magnet Warrior Ω－"].map(card);
    let spawns = [mon(0, omega), Spawn { team: 1, place: MonsterDefense, cid: plus }, mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, DARK_BLADE), (1, plus)]);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["to_ep"], false, "Ω- must attack: {}", s.prompt());
    s.run(&[Attack(omega)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", plus), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, omega), mon(1, DARK_BLADE)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["to_ep"], true, "{}", s.prompt());
}

/// "During the Battle Step, while an attack involving this card and a Minus monster is occurring, negate the attack." Ω−
/// (1900) attacks the opponent's Σ− (1500, both Minus, so no pole pulls): the attack is negated, so both stay on the field
/// and nobody loses LP, where a plain 1900 against 1500 would have destroyed Σ− and cost 400. The same goes for two Plus
/// monsters: Σ+ against Σ+.
#[test]
fn an_attack_between_two_monsters_of_one_pole_is_negated() {
    let [plus, minus, omega] = ["Magnet Warrior Σ＋", "Magnet Warrior Σ－", "Magnet Warrior Ω－"].map(card);
    for (mine, theirs) in [(omega, minus), (plus, plus)] {
        let Some(mut s) = field(&[], &[mon(0, mine), mon(1, theirs)]) else { return };
        s.run_both([&[EndTurn, Attack(mine)], &[EndTurn]]).unwrap();
        clean(&s);
        assert_eq!((s.count(0, "mzone", mine), s.count(1, "mzone", theirs)), (1, 1), "both stay: {}", s.context());
        assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "no damage: {}", s.context());
    }
}
