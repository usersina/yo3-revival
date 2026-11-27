//! Anime-only cards, group 6 (rows 5404-5436 of engine/added/cards.tsv): Battle City's and the Big Five's cards (Umbra,
//! Arkana, Gansley, Odion, Lumis, Leichter, Johnson, Nezbitt, Gurimo, Duke Devlin and Yami Yugi). These cards exist only in
//! the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real
//! core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones.
//! Every Trap and every Quick-Play Spell answering the opponent is Set first, and neither player has a Battle Phase in
//! turn 1, so team 0's attacks wait for turn 3.
//! Covered: the Tokens (Bronze Knight, made by Bronze Knights; Shield, made by Shield Wall), the cards the per-card fuzz
//! flagged (Masquerade, Ruklamba The Spirit King, Adamantine Sword Revival, Sinister Justice, Recycling Plant, Dragon
//! Revival Ritual), the signature cards (Diamond Head Dragon, Seal of Serket, Masked Doll, Eye of Ujat), Spell Transfer
//! (today's override against a Spell Transfer on both sides) and a seeded sample of the rest (Ambush Shield, Card Exchange,
//! Curse Transfer, Rainbow Snake Eingana and Spider Web (GX)), with Beckon to Darkness, Leading Question and Cup of Sealed
//! Soul on the way. Dragon Revival Ritual has an override (`engine/overrides/c120000016.lua`: Five-Headed Dragon is never
//! its own DARK Tribute). The Mystical Beast of Serket, a Token made with a card's own id, reaches the client as that card
//! (`a_token_with_a_card_id_reaches_the_client`). Soul Shield's case at ∞ LP is in `infinite_lp.rs`.
//! Not coverable: "ignoring the Battle City or Virtual World Rules" (Sinister Justice, Recycling Plant): the game plays no
//! such rules, so every Special Summoned monster may attack the turn it comes and the clause has nothing to lift.
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
// Bronze Knights and the Bronze Knight Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Bronze Knights: "Discard any number of cards; Special Summon an equal number of "Bronze Knight Tokens" (Warrior-Type/
/// EARTH/Level 1/ATK 500/DEF 500) in face-up Attack Position." Choosing the fewest takes one discard and makes one Token;
/// choosing every Feral Imp in the hand makes one Token for each, and both discards go to the Graveyard.
#[test]
fn bronze_knights_makes_a_token_for_each_discarded_card() {
    let [knights, token] = ["Bronze Knights", "Bronze Knight Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, knights)]) else { return };
    let (imps, grave_before) = (s.state()[0]["hand"].as_array().unwrap().iter().filter(|c| c["cid"] == FILLER).count(), s.count(0, "grave", FILLER));
    assert!(imps >= 3, "the opening hand holds Feral Imps: {}", s.context());
    let imps = imps.min(5); // at most as many as there are free Monster Zones
    s.run(&[Activate(knights)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 1, "one discard, one Token: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), grave_before + 1, "{}", s.context());
    // Every Imp discarded.
    let Some(mut s) = field(&[], &[hand(0, knights)]) else { return };
    s.prefer(&[(0, FILLER)]);
    s.prefer_most(true);
    s.run(&[Activate(knights)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), imps, "{imps} discards, {imps} Tokens: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), grave_before + imps, "{}", s.context());
    assert_eq!(s.count(0, "grave", knights), 1, "{}", s.context());
    // Warrior, EARTH, Level 1, 500/500, face-up Attack Position.
    let warrior = (stats(&s, 0, token).3, stats(&s, 0, token).4);
    assert_eq!((stats(&s, 0, token).0, stats(&s, 0, token).1, stats(&s, 0, token).2), (Some(500), Some(500), Some(1)), "{}", s.context());
    assert_eq!(s.position_of(0, token), Some(1), "face-up Attack: {}", s.context());
    let Some(s2) = field(&[], &[mon(0, DARK_BLADE)]) else { return };
    assert_eq!(warrior.0, stats(&s2, 0, DARK_BLADE).3, "Warrior-Type (as Dark Blade)");
    assert_eq!(warrior.1, 1, "EARTH");
}

// ---------------------------------------------------------------------------------------------------------------
// Shield Wall and the Shield Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Shield Wall: "Special Summon 4 "Shield Tokens" (Rock-Type/EARTH/Level 1/ATK 0/DEF 1000) in face-up Defense Position.
/// These Tokens cannot attack or be Tributed for a Tribute Summon. If a "Shield Token" is changed to Attack Position,
/// destroy it." Four Tokens of those stats in Defense; in turn 3 none of them can attack (Warwolf can), and with them as my
/// only monsters Dark Magician (Level 7, two Tributes) cannot be Tribute Summoned, where two Dark Blades in their place allow it.
#[test]
fn shield_wall_makes_four_tokens_that_cannot_attack_or_be_tributed() {
    let [wall, token] = ["Shield Wall", "Shield Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, wall), mon(0, WARWOLF), hand(0, DARK_MAGICIAN)]) else { return };
    // Four free Monster Zones are needed: with Warwolf on the field 5 - 1 = 4 are.
    s.run(&[Activate(wall)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 4, "four Tokens: {}", s.context());
    assert_eq!(s.count(0, "grave", wall), 1, "{}", s.context());
    let t = stats(&s, 0, token);
    assert_eq!((t.0, t.1, t.2, t.3), (Some(0), Some(1000), Some(1), 0x100), "Rock, 0/1000, Level 1: {}", s.context());
    assert_eq!(t.4, 1, "EARTH: {}", s.context());
    assert_eq!(s.position_of(0, token), Some(4), "face-up Defense: {}", s.context());
    // They cannot be Tributed for a Tribute Summon: Dark Magician needs two Tributes and only Warwolf is left.
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "no Tribute Summon: {:?}", offered(&s, "summons"));
    // In turn 3: the Tokens cannot attack, Warwolf can.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let err = s.run(&[Attack(token)]).expect_err("a Shield Token cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{WARWOLF}")) && !err.contains(&format!("\"cid\":{token}")), "{err}");
    clean(&s);
    // The control: two Dark Blades are Tributes enough.
    let Some(mut s) = field(&[], &[hand(0, DARK_MAGICIAN), mon(0, DARK_BLADE), mon(0, DARK_BLADE)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "{:?}", offered(&s, "summons"));
}

/// Shield Wall: "If a "Shield Token" is changed to Attack Position, destroy it." In turn 3 one Token is switched to
/// Attack Position by its controller and is destroyed (a Token is gone, not in the Graveyard); the other three stay.
#[test]
fn a_shield_token_changed_to_attack_position_is_destroyed() {
    let [wall, token] = ["Shield Wall", "Shield Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, wall)]) else { return };
    s.run_both([&[Activate(wall), EndTurn, Reposition(token)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 3, "one Token was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", token), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Spell Transfer (engine/overrides/c511000920.lua) and Curse Transfer
// ---------------------------------------------------------------------------------------------------------------

/// Spell Transfer: "When your opponent activates a Spell Card: Negate its effect, and if you do, activate this card's effect
/// as the negated effect." Team 1's Dian Keto (gain 1000 LP) is negated, and Spell Transfer's controller gains the 1000:
/// 9000 and 8000, not 8000 and 9000.
#[test]
fn spell_transfer_negates_a_spell_and_runs_its_effect_as_its_own() {
    let transfer = card("Spell Transfer");
    let Some(mut s) = field(&[], &[set(0, transfer), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(transfer)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", transfer), 1, "Spell Transfer was activated: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (9000, 8000), "Spell Transfer's controller gained: {}", s.context());
    // The control: without it the Spell's own controller gains.
    let Some(mut s) = field(&[], &[hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 9000), "{}", s.context());
}

/// Spell Transfer on a Spell that picks its own target: Monster Reborn is negated and Spell Transfer's controller revives
/// with it: Warwolf from team 0's own Graveyard comes to team 0's field, and team 1's Dark Blade stays where it lies.
#[test]
fn spell_transfer_revives_with_the_negated_monster_reborn() {
    let transfer = card("Spell Transfer");
    let spawns = [set(0, transfer), hand(1, MONSTER_REBORN), grave(0, WARWOLF), grave(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (1, FILLER)]);
    s.run_both([&[Chain(transfer)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Spell Transfer's controller revived Warwolf: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "team 1 revived nothing: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 0, "{}", s.context());
}

/// Spell Transfer against another Spell Transfer, which is a Spell Card itself while it is activated: team 1's Dian Keto is
/// answered by team 0's Spell Transfer, and team 1's own Spell Transfer answers that one. The second negates the first and
/// would "activate its effect as the negated effect", which was Spell Transfer's own again, over and over with the same
/// chain values until the Lua stack overflowed (engine/overrides/c511000920.lua, batch 6's fuzz). Now it only negates:
/// the first Spell Transfer does nothing, so Dian Keto resolves for team 1 (8000 and 9000), and both Spell Transfers are
/// in the Graveyard.
#[test]
fn a_spell_transfer_answering_a_spell_transfer_negates_it_without_endless_recursion() {
    let transfer = card("Spell Transfer");
    let Some(mut s) = field(&[], &[set(0, transfer), set(1, transfer), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(transfer)], &[Activate(DIAN_KETO), Chain(transfer)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", transfer), s.count(1, "grave", transfer)), (1, 1), "both were activated: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 9000), "the second Spell Transfer only negated: Dian Keto helped team 1: {}", s.context());
}

/// Curse Transfer: "When your opponent activates a Trap Card: Negate its effect, and if you do, activate this card's effect
/// as the negated effect." Team 1's Just Desserts (500 damage for each monster the opponent controls) is negated and
/// Curse Transfer's controller inflicts it: team 1, with 2 monsters, takes 1000 and team 0 takes nothing, where without
/// Curse Transfer team 0, with 1 monster, takes 500.
#[test]
fn curse_transfer_negates_a_trap_and_runs_its_effect_as_its_own() {
    let [curse, desserts] = ["Curse Transfer", "Just Desserts"].map(card);
    let spawns = [set(0, curse), mon(0, THUNDER_KID), set(1, desserts), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(curse)], &[Activate(desserts)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", curse), 1, "Curse Transfer was activated: {}", s.context());
    assert_eq!(s.count(1, "grave", desserts), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1000), "team 1 took 500 for each of its 2 monsters: {}", s.context());
    // The control.
    let spawns = [mon(0, THUNDER_KID), set(1, desserts), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(desserts)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 500, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Ambush Shield, Card Exchange, Spider Web (GX), Rainbow Snake Eingana and Beckon to Darkness
// ---------------------------------------------------------------------------------------------------------------

/// Ambush Shield: "Tribute 1 monster, then target 1 monster; it gains ATK equal to the DEF of the Tributed monster, until
/// the End Phase." In turn 3 Dark Blade (DEF 1500) is Tributed and Thunder Kid gains 1500 (700 to 2200) and beats Warwolf
/// (2000): 200 damage. With Warwolf (DEF 100) as the Tribute the gain is 100 only.
#[test]
fn ambush_shield_gives_the_defense_of_the_tributed_monster_as_atk() {
    let shield = card("Ambush Shield");
    for (tribute, gain) in [(DARK_BLADE, 1500), (WARWOLF, 100)] {
        let spawns = [set(0, shield), mon(0, DARK_BLADE), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, FILLER)];
        let Some(mut s) = field(&[], &spawns) else { return };
        keep_hands(&mut s);
        s.prefer(&[(0, tribute), (0, THUNDER_KID)]);
        s.prefer_most(true); // the Tribute is chosen one card at a time, with a way to finish
        s.run_both([&[EndTurn, Activate(shield)], &[EndTurn]]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "grave", tribute), 1, "{tribute} was Tributed: {}", s.context());
        assert_eq!(s.count(0, "grave", shield), 1, "{}", s.context());
        assert_eq!(s.attack_of(0, THUNDER_KID), Some(700 + gain), "gain {gain}: {}", s.context());
        assert_eq!(s.attack_of(0, if tribute == DARK_BLADE { WARWOLF } else { DARK_BLADE }), Some(if tribute == DARK_BLADE { 2000 } else { 1800 }), "only the target gains: {}", s.context());
    }
    // It wins the battle it was activated for, and the gain ends with the turn.
    let spawns = [set(0, shield), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE), (0, THUNDER_KID)]);
    s.prefer_most(true);
    s.run_both([&[EndTurn, Activate(shield), Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "2200 against 2000: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 200), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(700), "until the End Phase: {}", s.context());
}

/// Ambush Shield: "Tribute 1 monster, then target 1 monster": with a single monster there is no second to target, so it
/// is not offered.
#[test]
fn ambush_shield_needs_a_second_monster_to_target() {
    let shield = card("Ambush Shield");
    let Some(mut s) = field(&[], &[set(0, shield), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(shield)], &[EndTurn]]).expect_err("nothing to target");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
}

/// Card Exchange: "Exchange all the cards in your hand with your opponent." Team 1's whole hand is Thunder Kids (and one Dark
/// Magician), team 0's Feral Imps (and a Warwolf): afterwards team 0 holds team 1's cards and the other way round, and
/// Card Exchange itself is in the Graveyard.
#[test]
fn card_exchange_swaps_the_two_hands() {
    let exchange = card("Card Exchange");
    let spawns = [hand(0, exchange), hand(0, WARWOLF), hand(1, DARK_MAGICIAN)];
    let Some(mut s) = duel(7, 8000, [FILLER, THUNDER_KID], &[], &spawns) else { return };
    let (mine, theirs) = (hand_size(&s, 0) - 1, hand_size(&s, 1));
    assert_eq!((s.count(0, "hand", FILLER), s.count(1, "hand", THUNDER_KID), s.count(1, "hand", FILLER)), (mine - 1, theirs - 1, 0), "{}", s.context());
    s.run(&[Activate(exchange)]).unwrap();
    clean(&s);
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), (theirs, mine), "the hands changed sides: {}", s.context());
    assert_eq!((s.count(0, "hand", THUNDER_KID), s.count(0, "hand", DARK_MAGICIAN), s.count(0, "hand", FILLER)), (theirs - 1, 1, 0), "{}", s.context());
    assert_eq!((s.count(1, "hand", FILLER), s.count(1, "hand", WARWOLF), s.count(1, "hand", THUNDER_KID)), (mine - 1, 1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", exchange), 1, "{}", s.context());
}

/// Spider Web (GX): "Target 1 card in your opponent's Graveyard that was sent there the previous turn; add it to your
/// hand." Team 1's Dian Keto is activated in turn 2; in turn 3 team 0 takes it from team 1's Graveyard into its hand.
#[test]
fn spider_web_takes_a_card_sent_to_the_opponents_graveyard_the_previous_turn() {
    let web = card("Spider Web (GX)");
    let Some(mut s) = field(&[], &[hand(0, web), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(web)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DIAN_KETO), 1, "Dian Keto is in my hand: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 0, "and not in team 1's Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", web), 1, "{}", s.context());
}

/// Spider Web (GX): "sent there the previous turn": a card that went to the Graveyard this very turn (Dark Hole's victim),
/// and one that went there two turns ago, are not targets, so the Spell is not offered in turn 1 after Dark Hole nor in
/// turn 3. (A card the scenario spawns into a Graveyard counts as sent the previous turn: the spawn is a move to the
/// Graveyard in the core, so the test sends Warwolf there by an effect. Feral Imps discarded at team 1's End Phase would
/// become targets from turn 5 on, so the turns stop at 3.)
#[test]
fn spider_web_leaves_a_card_sent_this_turn_or_two_turns_ago() {
    let web = card("Spider Web (GX)");
    let spawns = [hand(0, web), hand(0, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole destroyed Warwolf: {}", s.context());
    assert!(!offered(&s, "activates").contains(&web.into()), "sent this turn: {:?}", offered(&s, "activates"));
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "hand", web), 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&web.into()), "sent two turns ago: {:?}", offered(&s, "activates"));
    clean(&s);
}

/// Rainbow Snake Eingana: "When this card is sent to the Graveyard: Destroy all monsters your opponent controls." and
/// Beckon to Darkness: "Send 1 monster your opponent controls to the Graveyard." Team 1's Beckon to Darkness sends team 0's
/// Eingana (2200/2400) to the Graveyard, and its effect destroys team 1's two monsters.
#[test]
fn eingana_destroys_every_monster_of_the_opponent_when_it_is_sent_to_the_graveyard() {
    let [eingana, beckon] = ["Rainbow Snake Eingana", "Beckon to Darkness"].map(card);
    let spawns = [mon(0, eingana), hand(1, beckon), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(beckon)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", eingana), 1, "Beckon to Darkness sent Eingana to the Graveyard: {}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "both were destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", beckon), 1, "{}", s.context());
}

/// Rainbow Snake Eingana, the control: Beckon to Darkness on a monster without such an effect (Dark Blade) leaves the
/// sender's own monsters alone.
#[test]
fn beckon_to_darkness_sends_only_one_monster() {
    let beckon = card("Beckon to Darkness");
    let spawns = [mon(0, DARK_BLADE), mon(0, THUNDER_KID), hand(1, beckon), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[EndTurn], &[Activate(beckon)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(0, "mzone", THUNDER_KID)), (1, 1), "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Rainbow Snake Eingana: it is sent to the Graveyard by battle too ("sent to the Graveyard", whatever the way): Dark
/// Magician (2500) attacks it (2200), and both of the attacker's controller's monsters are destroyed after the battle.
#[test]
fn eingana_destroyed_in_battle_takes_the_attackers_side_with_it() {
    let eingana = card("Rainbow Snake Eingana");
    let spawns = [mon(0, eingana), mon(1, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", eingana), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", DARK_MAGICIAN), s.count(1, "grave", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 300, "2500 against 2200: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Masquerade
// ---------------------------------------------------------------------------------------------------------------

/// Masquerade: "Take control of 1 monster your opponent controls that is equipped with a "Mask" card." Team 1 equips its
/// Warwolf with Mask of the Accursed in turn 2; in turn 3 Masquerade takes Warwolf, and the Mask stays with it (on team 1's
/// side of the field). Warwolf without a Mask, or with another Equip Spell (Axe of Despair), is no target.
#[test]
fn masquerade_takes_a_monster_equipped_with_a_mask() {
    let [masquerade, accursed] = ["Masquerade", "Mask of the Accursed"].map(card);
    let spawns = [hand(0, masquerade), hand(1, accursed), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(masquerade)], &[Activate(accursed)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "the Masked Warwolf is mine: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "only the Masked monster was taken: {}", s.context());
    assert_eq!(s.count(0, "grave", masquerade), 1, "{}", s.context());
    // The controls: no Mask on any monster, and an Equip Spell that is no Mask.
    let Some(mut s) = field(&[], &[hand(0, masquerade), mon(1, WARWOLF)]) else { return };
    let err = s.run_both([&[EndTurn, Activate(masquerade)], &[EndTurn]]).expect_err("no Mask");
    assert!(err.contains("never became possible"), "{err}");
    let axe = card("Axe of Despair");
    let Some(mut s) = field(&[], &[hand(0, masquerade), hand(1, axe), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(masquerade)], &[Activate(axe)]]).expect_err("an Axe is no Mask");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Ruklamba The Spirit King
// ---------------------------------------------------------------------------------------------------------------

/// Ruklamba The Spirit King: "When you take 1000 or more damage: You can Special Summon this card from your hand."
/// Team 1's Warwolf attacks directly for 2000: Ruklamba (1000/2000, Level 8) comes from the hand in face-up Attack
/// Position. Thunder Kid's 700 is too little: it is not offered.
#[test]
fn ruklamba_comes_from_the_hand_when_its_controller_takes_1000_damage() {
    let ruklamba = card("Ruklamba The Spirit King");
    let Some(mut s) = field(&[], &[hand(0, ruklamba), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(ruklamba)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ruklamba), 1, "Ruklamba was Special Summoned: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    let r = stats(&s, 0, ruklamba);
    assert_eq!((r.0, r.1, r.2), (Some(1000), Some(2000), Some(8)), "{}", s.context());
    assert_eq!(s.position_of(0, ruklamba), Some(1), "face-up Attack: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, ruklamba), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(ruklamba)], &[Attack(THUNDER_KID)]]).expect_err("700 damage");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000 - 700, "{}", s.context());
    assert_eq!(s.count(0, "hand", ruklamba), 1, "{}", s.context());
}

/// Ruklamba: "When you take 1000 or more damage": the damage must be mine. Team 0's own Warwolf deals 2000 to team 1, and
/// Ruklamba in team 0's hand is not offered.
#[test]
fn ruklamba_ignores_damage_to_the_opponent() {
    let ruklamba = card("Ruklamba The Spirit King");
    let Some(mut s) = field(&[], &[hand(0, ruklamba), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(WARWOLF), Chain(ruklamba)], &[EndTurn]]).expect_err("the opponent took it");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
}

/// Ruklamba: "(Quick Effect): You can Tribute this card; Special Summon monsters from your hand whose combined ATK is
/// 2000 or less, in face-up Attack Position." Ruklamba is Tributed; Thunder Kid (700) and a Feral Imp (1300) come from the
/// hand, 2000 in all, and Dark Magician (2500) stays in the hand: more would pass 2000.
#[test]
fn ruklamba_tributes_itself_for_monsters_from_the_hand_up_to_2000_atk() {
    let ruklamba = card("Ruklamba The Spirit King");
    let Some(mut s) = field(&[], &[mon(0, ruklamba), hand(0, THUNDER_KID), hand(0, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(0, THUNDER_KID), (0, FILLER)]);
    s.prefer_most(true);
    s.run(&[Activate(ruklamba)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ruklamba), 1, "Ruklamba was Tributed: {}", s.context());
    assert_eq!((s.count(0, "mzone", THUNDER_KID), s.count(0, "mzone", FILLER)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!((s.position_of(0, THUNDER_KID), s.position_of(0, FILLER)), (Some(1), Some(1)), "face-up Attack: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Adamantine Sword Revival and Diamond Head Dragon
// ---------------------------------------------------------------------------------------------------------------

/// Adamantine Sword Revival: "When a Dragon-Type monster is Special Summoned from the Graveyard: You can Tribute it; Special
/// Summon 1 "Diamond Head Dragon" from your Deck in face-up Attack Position to the controller of the Special Summoned
/// Dragon-Type monster's side of the field." and Diamond Head Dragon: "This card's ATK becomes equal to the ATK of the
/// Tributed Dragon-Type monster +1000." Team 1's Monster Reborn revives its own Blue-Eyes White Dragon (3000) and team 0's
/// Sword Revival Tributes it: Diamond Head Dragon (Level 8, 4000/2800) comes from team 0's Deck to team 1's side. A Luster
/// Dragon (1900) gives 2900.
#[test]
fn adamantine_sword_revival_tributes_the_revived_dragon_for_a_diamond_head_dragon_on_its_side() {
    let [revival, head] = ["Adamantine Sword Revival", "Diamond Head Dragon"].map(card);
    for (dragon, atk) in [(card("Blue-Eyes White Dragon"), 4000), (card("Luster Dragon"), 2900)] {
        let spawns = [set(0, revival), hand(1, MONSTER_REBORN), grave(1, dragon)];
        let Some(mut s) = field(&[head], &spawns) else { return };
        keep_hands(&mut s);
        s.prefer(&[(1, dragon), (0, FILLER), (1, FILLER)]);
        s.run_both([&[Chain(revival)], &[Activate(MONSTER_REBORN)]]).unwrap();
        clean(&s);
        assert_eq!(s.count(1, "grave", dragon), 1, "the revived Dragon was Tributed: {}", s.context());
        assert_eq!(s.count(1, "mzone", head), 1, "Diamond Head Dragon is on team 1's side: {}", s.context());
        assert_eq!(s.count(0, "mzone", head), 0, "{}", s.context());
        let h = stats(&s, 1, head);
        assert_eq!((h.0, h.1, h.2), (Some(atk), Some(2800), Some(8)), "{}", s.context());
        assert_eq!(s.position_of(1, head), Some(1), "face-up Attack: {}", s.context());
        assert_eq!(s.count(0, "grave", revival), 1, "{}", s.context());
    }
}

/// Adamantine Sword Revival: the same on my own side, in turn 3: my Monster Reborn revives my Blue-Eyes White Dragon and the
/// Sword Revival, chained to it, brings the Diamond Head Dragon (4000) to my side. A revived monster that is no Dragon
/// (Warwolf) does not set it off.
#[test]
fn adamantine_sword_revival_brings_the_diamond_head_dragon_to_its_own_controller_too() {
    let [revival, head, dragon] = ["Adamantine Sword Revival", "Diamond Head Dragon", "Blue-Eyes White Dragon"].map(card);
    let spawns = [set(0, revival), hand(0, MONSTER_REBORN), grave(0, dragon)];
    let Some(mut s) = field(&[head], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, dragon), (0, FILLER), (1, FILLER)]);
    s.run(&[EndTurn, Activate(MONSTER_REBORN), Chain(revival)]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(0, "mzone", head), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, head), Some(4000), "{}", s.context());
    assert_eq!(s.count(0, "grave", dragon), 1, "{}", s.context());
    let spawns = [set(0, revival), hand(0, MONSTER_REBORN), grave(0, WARWOLF)];
    let Some(mut s) = field(&[head], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    let err = s.run(&[EndTurn, Activate(MONSTER_REBORN), Chain(revival)]).expect_err("no Dragon");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Diamond Head Dragon: "Cannot be Normal Summoned/Set. Must be Special Summoned with "Adamantine Sword Revival", and cannot
/// be Special Summoned by other ways." In the Graveyard it is no target of Monster Reborn, and in the hand it has no Normal
/// Summon or Set offered.
#[test]
fn diamond_head_dragon_cannot_be_summoned_any_other_way() {
    let head = card("Diamond Head Dragon");
    let Some(mut s) = field(&[], &[hand(0, MONSTER_REBORN), grave(0, head)]) else { return };
    let err = s.run(&[Activate(MONSTER_REBORN)]).expect_err("nothing to revive");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", head), 1, "{}", s.context());
    clean(&s);
    let Some(mut s) = field(&[], &[hand(0, head)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&head.into()) && !offered(&s, "monster_sets").contains(&head.into()), "{}", s.context());
    assert!(!offered(&s, "special_summons").contains(&head.into()), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Sinister Justice and Leading Question
// ---------------------------------------------------------------------------------------------------------------

/// Sinister Justice: "If a Level 4 or lower Fusion Monster inflicts battle damage: Gain 800 LP." Darkfire Dragon (Level 4,
/// 1500) attacks directly in turn 3: 1500 damage and 800 LP (8800 and 6500). Gaia the Dragon Champion (Level 7) and
/// Warwolf (not a Fusion Monster) inflict damage without it.
#[test]
fn sinister_justice_gains_800_lp_for_a_low_level_fusion_monster_dealing_battle_damage() {
    let [justice, darkfire, champion] = ["Sinister Justice", "Darkfire Dragon", "Gaia the Dragon Champion"].map(card);
    for (attacker, damage, gain) in [(darkfire, 1500, 800), (champion, 2600, 0), (WARWOLF, 2000, 0)] {
        let spawns = [spell(0, justice), mon(0, attacker)];
        let Some(mut s) = field_extra([&[darkfire, champion], &[]], &spawns) else { return };
        keep_hands(&mut s);
        s.run_both([&[EndTurn, Attack(attacker)], &[EndTurn]]).unwrap();
        clean(&s);
        assert_eq!(s.lp(1), 8000 - damage, "{attacker}: {}", s.context());
        assert_eq!(s.lp(0), 8000 + gain, "{attacker}: gain {gain}: {}", s.context());
    }
}

/// Leading Question: "If a Level 4 or lower Fusion Monster you control battles another monster, it gains 800 ATK during the
/// Damage Step only." Darkfire Dragon (1500, Level 4) attacks Warwolf (2000): at 2300 it wins (300 damage) and is back at
/// 1500 afterwards. Without the Spell it loses (500 damage to me), and a Level 7 Fusion Monster (Gaia the Dragon Champion,
/// 2600) against Blue-Eyes White Dragon (3000) gets nothing and loses.
#[test]
fn leading_question_gives_800_atk_in_the_damage_step_to_a_low_level_fusion_monster() {
    let [question, darkfire, champion, blue_eyes] =
        ["Leading Question", "Darkfire Dragon", "Gaia the Dragon Champion", "Blue-Eyes White Dragon"].map(card);
    let Some(mut s) = field_extra([&[darkfire, champion], &[]], &[spell(0, question), mon(0, darkfire), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(darkfire)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "2300 against 2000: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 300), "{}", s.context());
    assert_eq!(s.attack_of(0, darkfire), Some(1500), "the 800 lasted for the Damage Step only: {}", s.context());
    // Without the Spell.
    let Some(mut s) = field_extra([&[darkfire, champion], &[]], &[mon(0, darkfire), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(darkfire)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", darkfire), 1, "1500 against 2000: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
    // A Level 7 Fusion Monster gets nothing.
    let Some(mut s) = field_extra([&[darkfire, champion], &[]], &[spell(0, question), mon(0, champion), mon(1, blue_eyes)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(champion)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", champion), 1, "2600 against 3000: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 400, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Recycling Plant
// ---------------------------------------------------------------------------------------------------------------

/// Recycling Plant: "Fusion Summon 1 Machine-Type Fusion Monster from your Extra Deck, by banishing monsters from your side
/// of the field or Graveyard as Fusion Materials". Two Cyber Dragons in the Graveyard are banished and Cyber Twin Dragon
/// comes from the Extra Deck. With the Cyber Dragons only in the hand it is not offered.
#[test]
fn recycling_plant_fusion_summons_a_machine_by_banishing_materials_from_the_graveyard() {
    let [plant, twin, cyber] = ["Recycling Plant", "Cyber Twin Dragon", "Cyber Dragon"].map(card);
    let spawns = [hand(0, plant), grave(0, cyber), grave(0, cyber)];
    let Some(mut s) = field_extra([&[twin], &[]], &spawns) else { return };
    s.run(&[Activate(plant)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", twin), 1, "Cyber Twin Dragon is on the field: {}", s.context());
    assert_eq!((s.count(0, "removed", cyber), s.count(0, "grave", cyber)), (2, 0), "both were banished: {}", s.context());
    assert_eq!(s.count(0, "grave", plant), 1, "{}", s.context());
    let spawns = [hand(0, plant), hand(0, cyber), hand(0, cyber)];
    let Some(mut s) = field_extra([&[twin], &[]], &spawns) else { return };
    let err = s.run(&[Activate(plant)]).expect_err("no materials on the field or in the Graveyard");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Recycling Plant: "by banishing monsters from your side of the field": Cyber Dragons on the field are banished too.
#[test]
fn recycling_plant_banishes_materials_from_the_field() {
    let [plant, twin, cyber] = ["Recycling Plant", "Cyber Twin Dragon", "Cyber Dragon"].map(card);
    let spawns = [hand(0, plant), mon(0, cyber), mon(0, cyber)];
    let Some(mut s) = field_extra([&[twin], &[]], &spawns) else { return };
    s.run(&[Activate(plant)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", twin), s.count(0, "mzone", cyber), s.count(0, "removed", cyber)), (1, 0, 2), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Eye of Ujat, Seal of Serket and Masked Doll
// ---------------------------------------------------------------------------------------------------------------

/// Eye of Ujat: "When your opponent declares an attack: Change the attack target to a monster your opponent controls,
/// except the attacking monster." Team 1's Warwolf (2000) attacks my Thunder Kid and is turned on its own Dark Blade
/// (1800): Dark Blade is destroyed, team 1 takes 200 and I take nothing.
#[test]
fn eye_of_ujat_turns_the_attack_on_another_monster_of_the_attacker() {
    let eye = card("Eye of Ujat");
    let spawns = [set(0, eye), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(eye)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "Warwolf destroyed its own Dark Blade: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 200), "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", eye), 1, "{}", s.context());
}

/// Eye of Ujat: "a monster your opponent controls, except the attacking monster": with the attacker alone there is no
/// other monster, so the Trap is not offered and the attack lands (1300 damage, Thunder Kid destroyed).
#[test]
fn eye_of_ujat_needs_a_second_monster_on_the_attackers_side() {
    let eye = card("Eye of Ujat");
    let Some(mut s) = field(&[], &[set(0, eye), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(eye)], &[Attack(WARWOLF)]]).expect_err("only one attacker");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!((s.lp(0), s.count(0, "grave", THUNDER_KID)), (8000 - 1300, 1), "{}", s.context());
}

/// Seal of Serket and Cup of Sealed Soul: "When you activate this card, it stays face-up on the field. Only once, if you also
/// control a "Cup of Sealed Soul" and "Temple of the Kings": You can Special Summon 1 "Mystical Beast of Serket" ignoring its
/// Summoning Conditions. "Mystical Beast of Serket" cannot be destroyed by card effects while this card and "Cup of Sealed
/// Soul" are face-up on the field." Both Spells are activated and stay, Temple of the Kings is in place: Seal's effect makes
/// the Beast (Level 6, Fairy, EARTH, 2500/2000), once only, and team 1's Dark Hole leaves it standing. The script makes the
/// Beast with `Duel.CreateToken` (a Token with the card's own id), which the client shows as that card:
/// `a_token_with_a_card_id_reaches_the_client` below checks the client's side.
#[test]
fn seal_of_serket_summons_the_mystical_beast_of_serket_that_cannot_be_destroyed_by_effects() {
    let [seal, cup, temple, beast] = ["Seal of Serket", "Cup of Sealed Soul", "Temple of the Kings", "Mystical Beast of Serket"].map(card);
    let spawns = [hand(0, seal), hand(0, cup), spell(0, temple), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(seal), Activate(cup), Activate(seal)]).unwrap();
    assert_eq!((s.count(0, "szone", seal), s.count(0, "szone", cup), s.count(0, "szone", temple)), (1, 1, 1), "they stay on the field: {}", s.context());
    assert_eq!(s.count(0, "mzone", beast), 1, "the Mystical Beast of Serket: {}", s.context());
    let b = stats(&s, 0, beast);
    assert_eq!((b.0, b.1, b.2, b.3, b.4), (Some(2500), Some(2000), Some(6), 0x4, 1), "2500/2000, Level 6, Fairy, EARTH: {}", s.context());
    // Only once: Seal's effect is not offered again (the Cup's own "only once" is its own).
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&seal.into()), "{:?}", offered(&s, "activates"));
    // Dark Hole in team 1's turn destroys nothing of mine.
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", beast), 1, "cannot be destroyed by card effects: {}", s.context());
    assert_eq!((s.count(0, "szone", seal), s.count(0, "szone", cup)), (1, 1), "{}", s.context());
}

/// Seal of Serket's Beast is a Token made with a card's own id (`Duel.CreateToken(tp, 89194033)`): the client shows it as
/// that card, which a real duel's card table always has (the server's holds every card the client knows). A scenario's
/// table holds only the decks' cards, so the Beast sits at the bottom of team 0's Deck here, never drawn.
#[test]
fn a_token_with_a_card_id_reaches_the_client() {
    let [seal, cup, temple, beast] = ["Seal of Serket", "Cup of Sealed Soul", "Temple of the Kings", "Mystical Beast of Serket"].map(card);
    let Some(mut s) = field(&[beast], &[hand(0, seal), hand(0, cup), spell(0, temple)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(seal), Activate(cup), Activate(seal)]).unwrap();
    assert_eq!(s.count(0, "mzone", beast), 1, "{}", s.context());
    clean(&s);
}

/// Seal of Serket: "if you also control a "Cup of Sealed Soul" and "Temple of the Kings"": without the Cup the effect is not
/// offered, and the Beast of the other Spell's own effect likewise.
#[test]
fn seal_of_serket_needs_the_cup_and_the_temple() {
    let [seal, temple] = ["Seal of Serket", "Temple of the Kings"].map(card);
    let Some(mut s) = field(&[], &[hand(0, seal), spell(0, temple)]) else { return };
    let err = s.run(&[Activate(seal), Activate(seal)]).expect_err("no Cup of Sealed Soul");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", seal), 1, "{}", s.context());
    clean(&s);
}

/// Masked Doll: "You do not have to pay LP for the effect of "Mask of Brutality" that you control." Mask of Brutality (+1000
/// ATK, -1000 DEF; "once per turn, during your Standby Phase, pay 1000 LP or destroy this card") is on Warwolf in turn 1; at
/// turn 3's Standby Phase LP are still 8000 and the Mask is on the field with Masked Doll. Without the Doll 1000 LP are paid.
#[test]
fn masked_doll_pays_the_cost_of_mask_of_brutality() {
    let [doll, brutality] = ["Masked Doll", "Mask of Brutality"].map(card);
    let Some(mut s) = field(&[], &[spell(0, doll), hand(0, brutality), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(brutality), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(3000), "Warwolf has the Mask (+1000): {}", s.context());
    assert_eq!(s.count(0, "szone", brutality), 1, "the Mask stays: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no LP paid: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, brutality), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(brutality), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000, "without the Doll the cost is paid: {}", s.context());
    assert_eq!(s.count(0, "szone", brutality), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dragon Revival Ritual
// ---------------------------------------------------------------------------------------------------------------

/// The five monsters of the five Attributes DRR asks for (DARK, EARTH, WATER, WIND and FIRE), vanilla ones of the game.
fn five_attributes() -> [u16; 5] {
    ["Dark Blade", "Battle Ox", "Hyosube", "Flying Kamakiri #2", "Darkfire Soldier #1"].map(card)
}

/// Dragon Revival Ritual: "This card is used to Ritual Summon "Five-Headed Dragon". You must also Tribute 1 DARK, EARTH,
/// FIRE, WATER, and WIND monster from your hand or field within the next 5 turns." With the five monsters on the field and
/// the anime Five-Headed Dragon (a Main Deck Ritual Monster, 5000/5000, Level 12) in the hand, the Spell is activated and the
/// Dragon is Ritual Summoned by Tributing one monster of each Attribute: all five are in the Graveyard, and the Dragon's
/// own effect destroys the opponent's cards.
#[test]
fn dragon_revival_ritual_summons_five_headed_dragon_by_tributing_the_five_attributes() {
    let [ritual, dragon] = ["Dragon Revival Ritual", "Five-Headed Dragon"].map(card);
    let mats = five_attributes();
    let mut spawns = vec![hand(0, ritual), hand(0, dragon), mon(1, WARWOLF)];
    spawns.extend(mats.iter().map(|&m| mon(0, m)));
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&mats.map(|m| (0, m)));
    s.run(&[Activate(ritual)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ritual), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", dragon), 1, "the Dragon waits in the hand: {}", s.context());
    s.run(&[SpSummon(dragon)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dragon), 1, "Five-Headed Dragon is on the field: {}", s.context());
    for m in mats {
        assert_eq!(s.count(0, "grave", m), 1, "{m} was Tributed: {}", s.context());
    }
    let d = stats(&s, 0, dragon);
    assert_eq!((d.0, d.1, d.2), (Some(5000), Some(5000), Some(12)), "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "its own effect destroyed the opponent's monster: {}", s.context());
}

/// Dragon Revival Ritual: "within the next 5 turns": the Tributes may be made over several turns. In turn 1 only the DARK
/// and EARTH monsters are Tributed (the answer to "Tribute more?" is no), and the Dragon stays in the hand; in turn 3 the
/// other three are Tributed and it is summoned, without the first two again.
#[test]
fn dragon_revival_ritual_may_tribute_over_several_turns() {
    let [ritual, dragon] = ["Dragon Revival Ritual", "Five-Headed Dragon"].map(card);
    let mats = five_attributes();
    let mut spawns = vec![hand(0, ritual), hand(0, dragon)];
    spawns.extend(mats.iter().map(|&m| mon(0, m)));
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, mats[0]), (0, mats[1]), (0, mats[2]), (0, mats[3]), (0, mats[4]), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(ritual)]).unwrap();
    // Two Tributes, then "no".
    s.run_to(&[SpSummon(dragon)], |p| p["type"] == msg::SELECT_YESNO).unwrap();
    s.respond(serde_json::json!({"type": 3, "yes": true})).unwrap();
    s.run_to(&[], |p| p["type"] == msg::SELECT_YESNO).unwrap();
    s.respond(serde_json::json!({"type": 3, "yes": false})).unwrap();
    for _ in 0..8 {
        if s.prompt()["type"] == msg::SELECT_IDLECMD {
            break;
        }
        s.pass().unwrap();
    }
    clean(&s);
    assert_eq!(s.count(0, "hand", dragon), 1, "the Dragon is still in the hand: {}", s.context());
    let tributed: usize = mats.iter().map(|&m| s.count(0, "grave", m)).sum();
    assert_eq!(tributed, 2, "two monsters were Tributed so far: {}", s.context());
    // Turn 3: the rest.
    s.run_both([&[EndTurn, SpSummon(dragon)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dragon), 1, "{}", s.context());
    for m in mats {
        assert_eq!(s.count(0, "grave", m), 1, "{m}: {}", s.context());
    }
}

/// Dragon Revival Ritual: "Tribute 1 DARK, EARTH, FIRE, WATER, and WIND monster from your hand or field": Five-Headed Dragon
/// is itself DARK and in the hand, and must not count as the DARK Tribute for itself. With the EARTH, WATER, WIND and FIRE
/// monsters on the field and no other DARK one, those four are Tributed and the DARK one stays owed: the Dragon is not
/// offered again, and stays in the hand (engine/overrides/c120000016.lua: upstream offered the Dragon as its own DARK
/// Tribute, which left the hand and failed the Ritual Summon on a Lua error half way, the Dragon and the Tributes lost).
#[test]
fn five_headed_dragon_is_not_its_own_dark_tribute() {
    let [ritual, dragon] = ["Dragon Revival Ritual", "Five-Headed Dragon"].map(card);
    let mats = five_attributes();
    let mut spawns = vec![hand(0, ritual), hand(0, dragon)];
    spawns.extend(mats[1..].iter().map(|&m| mon(0, m)));
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &spawns) else { return };
    s.run(&[Activate(ritual)]).unwrap();
    s.run(&[SpSummon(dragon)]).unwrap();
    clean(&s);
    for m in &mats[1..] {
        assert_eq!(s.count(0, "grave", *m), 1, "{m} was Tributed: {}", s.context());
    }
    assert_eq!(s.count(0, "hand", dragon), 1, "the Dragon is still in the hand: {}", s.context());
    assert_eq!(s.count(0, "grave", dragon), 0, "{}", s.context());
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&dragon.into()), "no DARK monster is left to Tribute: {:?}", offered(&s, "special_summons"));
    clean(&s);
}
