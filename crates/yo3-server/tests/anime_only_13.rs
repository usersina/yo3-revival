//! Anime-only cards, group 13 (rows 5619-5657 of engine/added/cards.tsv): Brron, Mad King of Dark World's, Mad Dog's, Adrian
//! Gecko's and Yubel's cards (Darkness Half, the Sinister Doctrines and Wicked Canon, the Slimes, the Cloud cards, the
//! Forbidden One cards, Fog King's Royal Sword). These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after every
//! step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play Spell answering the opponent is
//! Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3. A continuous Spell or Trap
//! spawned face-up has its first Standby Phase before the plan starts (Big Summon Cloud's would discard the hand) and its
//! effects work only once it is activated (Gravity Bind), so such cards are played from Set or from the hand.
//! Covered: the Tokens (Dark Token, made by Darkness Half; Slime Token, made by Multiple Slime), the cards the per-card fuzz
//! flagged (Cobal, the five Sinister Doctrines, Wicked Canon, Acid Slime, Crest Burn, Defense Seal Wall, Rage of the Forbidden
//! One, Ultimate Ritual of the Forbidden Lord, Royal Sword, Dual Gate), the script notes (Darkness Half, Big Summon Cloud at ∞,
//! Eternal Reverse), the signature cards (Darkness Half, Wicked Canon with the Doctrines, Exodius with its Ritual, Wall and
//! Rage, Royal Sword, the Cloud cards) and a seeded sample of the rest (Clone Slime, Contingency Fee, Draw Slime, House of
//! Sand, Jammer Slime, Last Trick and Multiple Slime), with one case each for Thousand Buster, Fiend Rose, Revenge Soul, Dark
//! Tournament, Slime Base, Trap Trip and Despised Reality.
//! Why the fuzz flagged them: the Doctrines' banishing finds nothing without a namesake in the Graveyard; Wicked Canon's second
//! effect needs Doctrines in the Graveyard and a Fusion Material in the Deck; Acid Slime, Crest Burn, Defense Seal Wall, Rage,
//! the Ritual, Royal Sword and Dual Gate need a battle, a Fog Castle, an Exodius, Forbidden One cards or a second Dual Gate
//! that its self-played Decks rarely had; Cobal's effects needed a turn with battle damage.
//! Overrides written for the batch: Cobal (`engine/overrides/c511000290.lua`: the check for a turn's battle damage flagged only
//! the player who controlled the card that loaded it) and Crest Burn (`c511011002.lua`: it put Beacon Counters on the Fog
//! Castle where its text removes them). Darkness Half's note ("its cost can leave nothing to choose") came to nothing: its
//! cost only keeps it on the field, and with its target gone the effect ends quietly.
//! Not covered: Delta Barrier, Akashic Record and Triangle Force (not in the sample: no flag, no note, no signature).
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

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: team 0's has `bottom` out of its
/// opening hand (the Decks are shuffled by the seed, so the first seed from the given one that leaves them in the Deck is
/// used), for a card an effect fetches from the Deck.
fn duel(seed: u64, lp: u32, fillers: [u16; 2], bottom: &[u16], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(fillers[0], 40 - bottom.len())).collect();
    let decks = [(&mine[..], extra[0]), (&vec![fillers[1]; 40][..], extra[1])];
    for seed in seed..seed + 60 {
        let s = Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap();
        let spawned = |cid: u16| spawns.iter().any(|p| p.cid == cid);
        if bottom.iter().all(|&cid| spawned(cid) || s.count(0, "hand", cid) == 0) {
            return Some(s);
        }
    }
    panic!("no seed keeps {bottom:?} out of the opening hand");
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [FILLER, FILLER], bottom, [&[], &[]], spawns) }

/// A duel with the Extra Decks given: `extra[team]`, and team 0's Deck bottom.
fn field_extra(bottom: &[u16], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    duel(7, 8000, [FILLER, FILLER], bottom, extra, spawns)
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

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

const FIEND: i64 = 0x8;
const AQUA: i64 = 0x40;
const DARK: i64 = 0x20;
const WATER: i64 = 0x2;

// ---------------------------------------------------------------------------------------------------------------
// Darkness Half and the Dark Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Darkness Half: "Target 1 face-up monster you control with the highest ATK; equip this card to that monster. Its original
/// ATK is halved, then Special Summon 2 "Dark Tokens" (Fiend-Type/DARK/Level 2/ATK 1000/DEF 1000) in Attack Position to
/// your opponent's side of the field." Warwolf (2000) beside Thunder Kid (700): Warwolf is the only target and halves to
/// 1000 (Thunder Kid, not the highest, is no target), the card stays equipped in the Spell & Trap Zone, and team 1 gets two
/// Tokens of those stats in face-up Attack Position, none for team 0.
#[test]
fn darkness_half_halves_the_strongest_monster_and_gives_the_opponent_two_dark_tokens() {
    let [half, token] = ["Darkness Half", "Dark Token"].map(card);
    let spawns = [set(0, half), mon(0, WARWOLF), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(half)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "2000 halved: {}", s.context());
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(700), "{}", s.context());
    assert_eq!(s.count(0, "szone", half), 1, "Darkness Half stays equipped: {}", s.context());
    assert_eq!((s.count(1, "mzone", token), s.count(0, "mzone", token)), (2, 0), "{}", s.context());
    let t = stats(&s, 1, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(1000), Some(1000), Some(2), FIEND, DARK), "Fiend, DARK, Level 2, 1000/1000: {}", s.context());
    assert_eq!(s.position_of(1, token), Some(1), "face-up Attack: {}", s.context());
}

/// Darkness Half, with a tie for the highest ATK: both monsters qualify and the player picks one (Warwolf here); and with
/// only one free Monster Zone on the opponent's side the Tokens cannot both come, so none do (the card is still equipped).
#[test]
fn darkness_half_needs_two_free_zones_on_the_opponents_side_for_its_tokens() {
    let [half, token] = ["Darkness Half", "Dark Token"].map(card);
    let spawns = [set(0, half), mon(0, WARWOLF), mon(1, DARK_BLADE), mon(1, DARK_BLADE), mon(1, DARK_BLADE), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(half)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "{}", s.context());
    assert_eq!(s.count(1, "mzone", token), 0, "one free zone: no Token: {}", s.context());
    assert_eq!(s.count(0, "szone", half), 1, "{}", s.context());
}

/// Dark Token: the Tokens are the opponent's: Dark Hole destroys them with the rest and they are gone, not in the Graveyard.
#[test]
fn dark_tokens_are_destroyed_like_any_monster_and_leave_no_card() {
    let [half, token] = ["Darkness Half", "Dark Token"].map(card);
    let spawns = [set(0, half), mon(0, WARWOLF), hand(0, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(half), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", token), 0, "Dark Hole destroyed them: {}", s.context());
    assert_eq!((s.count(1, "grave", token), s.count(0, "grave", token)), (0, 0), "Tokens leave no card: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// Darkness Half's script reads its target after the chain: with the equip target gone before it resolves (my own Raigeki
/// Break, chained to it, destroys Warwolf) the effect has nothing to equip and must not stop on a Lua error: the Tokens do
/// not come, and the card is put into the Graveyard. (The note on its script, "its cost can leave nothing to choose", comes
/// to nothing: the cost only keeps it on the field, and this is the nearest case, which runs without an engine error.)
#[test]
fn darkness_half_with_its_target_gone_does_nothing_and_does_not_break() {
    let [half, token] = ["Darkness Half", "Dark Token"].map(card);
    let raigeki = card("Raigeki Break");
    let spawns = [set(0, half), set(0, raigeki), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(half), Chain(raigeki)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Raigeki Break destroyed the target: {}", s.context());
    assert_eq!(s.count(1, "mzone", token), 0, "{}", s.context());
    assert_eq!(s.count(0, "szone", half), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Slimes: Multiple Slime and its Tokens, Acid Slime, Draw Slime, Clone Slime, Slime Ball, Slime Base and Jammer Slime
// ---------------------------------------------------------------------------------------------------------------

/// Multiple Slime: "When this card is destroyed by battle and sent to the Graveyard: Special Summon 3 "Slime Tokens"
/// (Aqua-Type/WATER/Level 1/ATK 500/DEF 500) in Attack Position." Team 1's Warwolf (2000) beats it (1500): 500 damage, and
/// three Tokens of those stats come in Attack Position to its controller, Multiple Slime itself in the Graveyard. Destroyed
/// by an effect (Dark Hole) it makes none.
#[test]
fn multiple_slime_destroyed_by_battle_makes_three_slime_tokens() {
    let [multiple, token] = ["Multiple Slime", "Slime Token"].map(card);
    let Some(mut s) = field(&[], &[mon(0, multiple), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", multiple), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", token), 3, "three Tokens: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "2000 against 1500: {}", s.context());
    let t = stats(&s, 0, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(500), Some(500), Some(1), AQUA, WATER), "Aqua, WATER, Level 1, 500/500: {}", s.context());
    assert_eq!(s.position_of(0, token), Some(1), "face-up Attack: {}", s.context());
    // Dark Hole is no battle: no Token.
    let Some(mut s) = field(&[], &[mon(0, multiple), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", multiple), s.count(0, "mzone", token)), (1, 0), "{}", s.context());
}

/// Acid Slime: "When this card is destroyed by battle and sent to the Graveyard: Inflict 800 damage to your opponent."
/// Team 1's Warwolf (2000) destroys Acid Slime (800/1000): 1200 battle damage to team 0, then 800 to team 1 (the attacker).
/// With Dark Hole it is no battle, and nobody takes damage.
#[test]
fn acid_slime_destroyed_by_battle_inflicts_800_damage_to_the_opponent() {
    let acid = card("Acid Slime");
    let Some(mut s) = field(&[], &[mon(0, acid), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", acid), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1200, 8000 - 800), "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, acid), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", acid), s.lp(0), s.lp(1)), (1, 8000, 8000), "{}", s.context());
}

/// Draw Slime: "When this card is sent to the Graveyard: Draw 1 card." Dark Hole (team 1's) destroys it and team 0 draws 1.
#[test]
fn draw_slime_draws_a_card_when_it_goes_to_the_graveyard() {
    let draw = card("Draw Slime");
    let Some(mut s) = field(&[], &[mon(0, draw), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    let (hand_before, deck_before) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", draw), 1, "{}", s.context());
    assert_eq!((hand_size(&s, 0), deck_size(&s, 0)), (hand_before + 1, deck_before - 1), "drew one card: {}", s.context());
}

/// Clone Slime: "When this face-up card is targeted for an attack: You can send this card to the Graveyard to target 1
/// "Slime" monster in your Graveyard; Special Summon that monster in the same Battle Position as this card, and switch the
/// attack target to that monster." Team 1's Warwolf attacks Clone Slime (Attack Position): Draw Slime comes from the
/// Graveyard in Attack Position and takes the attack (2000 against 300: 1700 damage to team 0, where an unswitched attack
/// would have had no target) and Draw Slime's own trigger then draws a card. Repositioned to Defense first, a Multiple Slime
/// (2000 DEF) comes in Defense, and the Warwolf's attack on it is a tie.
#[test]
fn clone_slime_sends_itself_for_a_slime_from_the_graveyard_that_takes_the_attack() {
    let [clone, draw, multiple] = ["Clone Slime", "Draw Slime", "Multiple Slime"].map(card);
    let Some(mut s) = field(&[], &[mon(0, clone), grave(0, draw), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let hand_before = hand_size(&s, 0);
    s.run_both([&[Chain(clone)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", clone), 1, "Clone Slime was sent as the cost: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1700, "the attack went on against Draw Slime (300): {}", s.context());
    assert_eq!(s.count(0, "grave", draw), 1, "Draw Slime fought and lost: {}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before + 1, "and drew a card: {}", s.context());
    // The same Position: Defense.
    let Some(mut s) = field(&[], &[mon(0, clone), grave(0, multiple), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Reposition(clone), Chain(clone)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", multiple), 1, "Multiple Slime came back: {}", s.context());
    assert_eq!(s.position_of(0, multiple), Some(4), "in face-up Defense, as Clone Slime was: {}", s.context());
    assert_eq!(s.lp(0), 8000, "2000 against 2000 DEF: nobody is hurt: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Clone Slime: with no "Slime" monster in the Graveyard (a Warwolf only) the effect is not offered, and the attack goes
/// through.
#[test]
fn clone_slime_needs_a_slime_in_the_graveyard() {
    let clone = card("Clone Slime");
    let Some(mut s) = field(&[], &[mon(0, clone), grave(0, WARWOLF), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(clone)], &[Attack(WARWOLF)]]).expect_err("nothing to revive");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", clone), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "the Warwolf hit Clone Slime (0 ATK) as it was: {}", s.context());
}

/// Slime Ball: "When a monster is Special Summoned: Destroy that monster, and gain LP equal to that monster's ATK." Team 1's
/// Monster Reborn revives its Warwolf (2000); Slime Ball destroys it and its controller, team 0, gains 2000 (10000).
#[test]
fn slime_ball_destroys_a_special_summoned_monster_and_gains_its_atk() {
    let ball = card("Slime Ball");
    let Some(mut s) = field(&[], &[set(0, ball), hand(1, MONSTER_REBORN), grave(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[Chain(ball)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "destroyed again: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 + 2000, 8000), "{}", s.context());
    assert_eq!(s.count(0, "grave", ball), 1, "{}", s.context());
}

/// Jammer Slime: "When an opponent's Spell/Trap Card is activated: Discard 1 "Slime" monster; negate the activation, and if
/// you do, destroy it." Team 1's Dian Keto is negated (no LP gained) and destroyed, and the discarded Draw Slime draws a card
/// for team 0 on its way to the Graveyard. With only Feral Imps in the hand it is not offered.
#[test]
fn jammer_slime_discards_a_slime_to_negate_a_spell() {
    let [jammer, draw] = ["Jammer Slime", "Draw Slime"].map(card);
    let Some(mut s) = field(&[], &[set(0, jammer), hand(0, draw), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    let hand_before = hand_size(&s, 0);
    s.run_both([&[Chain(jammer)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "Dian Keto was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", draw), 1, "Draw Slime was discarded: {}", s.context());
    assert_eq!(s.count(0, "grave", jammer), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before - 1, "-1 Draw Slime, +1 drawn by it: {}", s.context());
    let Some(mut s) = field(&[], &[set(0, jammer), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(jammer)], &[Activate(DIAN_KETO)]]).expect_err("no Slime to discard");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 9000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Contingency Fee, House of Sand and Last Trick
// ---------------------------------------------------------------------------------------------------------------

/// Contingency Fee: "Your opponent draw cards until they have 6 cards in their hand, then, gain 1000 LP for each card they
/// drew." (The script gives the 1000 LP to the card's controller.) Team 1 starts with fewer than 6 cards: it draws up to 6 and
/// team 0 gains 1000 for each. With 6 cards in its hand already, the Spell is not offered.
#[test]
fn contingency_fee_refills_the_opponents_hand_to_six_and_gains_1000_for_each_card() {
    let fee = card("Contingency Fee");
    let Some(mut s) = field(&[], &[hand(0, fee)]) else { return };
    let (theirs, deck) = (hand_size(&s, 1), deck_size(&s, 1));
    assert!(theirs < 6, "team 1 starts below 6 cards: {}", s.context());
    let drawn = 6 - theirs;
    s.run(&[Activate(fee)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 1), 6, "{}", s.context());
    assert_eq!(deck_size(&s, 1), deck - drawn as i64, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 + 1000 * drawn as i64, 8000), "{}", s.context());
    assert_eq!(s.count(0, "grave", fee), 1, "{}", s.context());
    // Already 6 cards: not offered.
    let Some(mut s) = field(&[], &[hand(0, fee), hand(1, DARK_BLADE), hand(1, DARK_BLADE)]) else { return };
    assert!(hand_size(&s, 1) >= 6, "{}", s.context());
    let err = s.run(&[Activate(fee)]).expect_err("6 cards or more");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// House of Sand: "When a player Special Summons a monster(s) from their Deck, add this face-up card to their hand and send
/// the top 3 cards of their Deck to the Graveyard." Team 0's Mystic Tomato is destroyed by battle (Warwolf, team 1's turn)
/// and fetches a Feral Imp from the Deck: House of Sand returns to team 0's hand and 3 more cards of its Deck go to the
/// Graveyard (the Imp and 3 gone from the Deck: 4). (A card goes to its owner's hand, so a summon by the opponent mills
/// the opponent but the card goes back to its owner: not tried.)
#[test]
fn house_of_sand_returns_to_the_hand_and_mills_three_when_a_monster_comes_from_the_deck() {
    let [sand, tomato] = ["House of Sand", "Mystic Tomato"].map(card);
    let Some(mut s) = field(&[], &[spell(0, sand), mon(0, tomato), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let (deck_before, grave_before) = (deck_size(&s, 0), s.count(0, "grave", FILLER));
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", tomato), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "Mystic Tomato fetched a Feral Imp: {}", s.context());
    assert_eq!(s.count(0, "hand", sand), 1, "House of Sand is in the hand: {}", s.context());
    assert_eq!(s.count(0, "szone", sand), 0, "{}", s.context());
    assert_eq!(deck_size(&s, 0), deck_before - 4, "the Imp and 3 milled: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), grave_before + 3, "{}", s.context());
}

/// House of Sand: a monster Special Summoned from the Graveyard (Monster Reborn) is no summon from the Deck: nothing happens.
#[test]
fn house_of_sand_ignores_a_summon_from_the_graveyard() {
    let sand = card("House of Sand");
    let Some(mut s) = field(&[], &[spell(0, sand), hand(0, MONSTER_REBORN), grave(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let deck_before = deck_size(&s, 0);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(MONSTER_REBORN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.count(0, "szone", sand), deck_size(&s, 0)), (1, deck_before), "{}", s.context());
}

/// Last Trick: "Activate only when your opponent activates a Spell Card and places it in the Graveyard. Add that card to
/// your hand." Team 1's Dian Keto resolves (team 1 gains 1000) and goes to the Graveyard; team 0 takes it into its own hand.
/// A Spell team 0 itself played is no target: Set with Dian Keto of its own, Last Trick is not used.
#[test]
fn last_trick_takes_the_spell_the_opponent_just_played() {
    let trick = card("Last Trick");
    let Some(mut s) = field(&[], &[set(0, trick), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(trick)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DIAN_KETO), 1, "Dian Keto is in my hand: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 0, "{}", s.context());
    assert_eq!(s.lp(1), 9000, "it resolved first: {}", s.context());
    assert_eq!(s.count(0, "grave", trick), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, trick), hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Activate(DIAN_KETO), Chain(trick)], &[EndTurn]]).expect_err("my own Spell");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Wicked Canon and the five Sinister Doctrines
// ---------------------------------------------------------------------------------------------------------------

const DOCTRINES: [&str; 5] =
    ["Sinister Doctrine - Anger", "Sinister Doctrine - Anguish", "Sinister Doctrine - Doubt", "Sinister Doctrine - Hatred", "Sinister Doctrine - Sadness"];

/// Wicked Canon: "When you take battle damage: You can send 1 "Sinister Doctrine" Spell from your hand or Deck to the
/// Graveyard." and each Sinister Doctrine: "When this card is sent to the Graveyard: Banish all other cards with the same
/// name as this card from your Graveyard." For each of the five: a copy already in the Graveyard and the next Doctrine (a
/// different name) beside it; team 1's Warwolf hits team 0 for 2000, Canon sends the copy from the Deck bottom, and the
/// older copy is banished while the other name stays in the Graveyard. (The per-card fuzz flagged Anger, Doubt and Sadness
/// as never doing it: with no second copy in the Graveyard a Doctrine's banishing finds nothing, and its fuzz duels never
/// had one.)
#[test]
fn wicked_canon_sends_a_doctrine_and_it_banishes_its_namesakes_from_the_graveyard() {
    let canon = card("Wicked Canon (Anime)");
    for (i, name) in DOCTRINES.iter().enumerate() {
        let (doctrine, other) = (card(name), card(DOCTRINES[(i + 1) % 5]));
        let Some(mut s) = field(&[doctrine], &[spell(0, canon), grave(0, doctrine), grave(0, other), mon(1, WARWOLF)]) else { return };
        keep_hands(&mut s);
        s.run_both([&[Chain(canon)], &[Attack(WARWOLF)]]).unwrap();
        clean(&s);
        assert_eq!(s.lp(0), 8000 - 2000, "{name}: {}", s.context());
        assert_eq!(s.count(0, "grave", doctrine), 1, "{name}: the new copy is in the Graveyard: {}", s.context());
        assert_eq!(s.count(0, "removed", doctrine), 1, "{name}: the older copy was banished: {}", s.context());
        assert_eq!((s.count(0, "grave", other), s.count(0, "removed", other)), (1, 0), "{name}: another name stays: {}", s.context());
        assert_eq!(s.count(0, "szone", canon), 1, "{name}: Canon stays: {}", s.context());
    }
}

/// A Doctrine sent with no namesake in the Graveyard banishes nothing (the fuzz's "promised, never done"): Anger alone is
/// sent by Canon and is in the Graveyard afterwards.
#[test]
fn a_sinister_doctrine_alone_in_the_graveyard_stays_there() {
    let [canon, anger] = ["Wicked Canon (Anime)", "Sinister Doctrine - Anger"].map(card);
    let Some(mut s) = field(&[anger], &[spell(0, canon), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(canon)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", anger), s.count(0, "removed", anger)), (1, 0), "{}", s.context());
}

/// Wicked Canon: "You can banish this face-up card you control and all "Sinister Doctrine" cards from your Graveyard;
/// Special Summon 1 Fusion Material from your Deck whose Level is equal to or less than the number of "Sinister Doctrine"
/// Spells banished x2, that is listed on a Fusion Monster in your Extra Deck." Three Doctrines in the Graveyard: Canon and
/// all three are banished and Curse of Dragon (Level 5, a Material of Gaia the Dragon Champion, in the Extra Deck) comes
/// from the Deck: 3 x 2 = 6. Gaia The Fierce Knight (Level 7, its other Material) in the Deck too is too high and stays
/// there. With two Doctrines banished (4) Curse of Dragon is too high and the effect is not offered. (Flame Swordsman, a
/// better fit, stays in the Main Deck when given as an Extra Deck card: the engine's data has it as a different passcode.)
#[test]
fn wicked_canon_banishes_itself_and_the_doctrines_for_a_fusion_material_from_the_deck() {
    let [canon, anger, doubt, sadness] = ["Wicked Canon (Anime)", DOCTRINES[0], DOCTRINES[2], DOCTRINES[4]].map(card);
    let [curse, gaia, champion] = ["Curse of Dragon", "Gaia The Fierce Knight", "Gaia the Dragon Champion"].map(card);
    let spawns = [spell(0, canon), grave(0, anger), grave(0, doubt), grave(0, sadness)];
    let Some(mut s) = field_extra(&[gaia, curse], [&[champion], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(canon)]).unwrap();
    clean(&s);
    for doctrine in [anger, doubt, sadness, canon] {
        assert_eq!(s.count(0, "removed", doctrine), 1, "{doctrine} was banished: {}", s.context());
    }
    assert_eq!(s.count(0, "mzone", curse), 1, "Curse of Dragon came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "hand", gaia) + s.count(0, "mzone", gaia), 0, "Level 7 is above 6, and stays in the Deck: {}", s.context());
    assert_eq!(s.position_of(0, curse), Some(1), "face-up Attack: {}", s.context());
    // Two Doctrines: 4 only.
    let Some(mut s) = field_extra(&[curse], [&[champion], &[]], &[spell(0, canon), grave(0, anger), grave(0, doubt)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(canon)]).expect_err("Level 5 is above 4");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", canon), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Cobal, Excavator of Dark World
// ---------------------------------------------------------------------------------------------------------------

/// Cobal, Excavator of Dark World: "During the End Phase of a turn you took battle damage: You can Special Summon this card
/// from your hand." Team 1's Warwolf hits team 0 for 2000 in turn 2 and Cobal (300/500, Level 2) comes in the End Phase,
/// face-up in Attack Position. When team 0 took no damage that turn it is not offered.
#[test]
fn cobal_comes_from_the_hand_in_the_end_phase_of_a_turn_its_player_took_battle_damage() {
    let cobal = card("Cobal, Excavator of Dark World");
    let Some(mut s) = field(&[], &[hand(0, cobal), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(cobal)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", cobal), 1, "Cobal was Special Summoned: {}", s.context());
    let c = stats(&s, 0, cobal);
    assert_eq!((c.0, c.1, c.2), (Some(300), Some(500), Some(2)), "{}", s.context());
    assert_eq!(s.position_of(0, cobal), Some(1), "face-up Attack: {}", s.context());
    // No damage, no effect.
    let Some(mut s) = field(&[], &[hand(0, cobal), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(cobal)], &[EndTurn]]).expect_err("nobody was hurt");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "hand", cobal), 1, "{}", s.context());
}

/// Cobal: "a turn you took battle damage": the damage must be mine. Team 0 hurts team 1 in turn 3: team 0's own Cobal is
/// not offered at that End Phase, team 1's is. (The script's check for the damage was one card's effect for both players,
/// flagging only the player who controlled the card that loaded it: engine/overrides/c511000290.lua.)
#[test]
fn cobal_of_the_player_who_was_hurt_comes_whichever_side_the_damage_check_belongs_to() {
    let cobal = card("Cobal, Excavator of Dark World");
    // Team 0's Cobal is the first one loaded; team 1 is the one hurt.
    let Some(mut s) = field(&[], &[hand(0, cobal), hand(1, cobal), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Attack(WARWOLF)], &[Chain(cobal)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(1, "mzone", cobal), 1, "team 1's Cobal came: {}", s.context());
    assert_eq!((s.count(0, "mzone", cobal), s.count(0, "hand", cobal)), (0, 1), "team 0's stays in the hand: {}", s.context());
}

/// Cobal: "If this card inflicts Battle Damage to your opponent by a direct attack: You can send this face-up card to the
/// Graveyard; add 1 "Dark World" card from your Graveyard to your hand." Cobal attacks directly in turn 3 (300 damage),
/// goes to the Graveyard and Beiige, Vanguard of Dark World comes from the Graveyard to the hand. With no Dark World card
/// in the Graveyard it is not offered.
#[test]
fn cobal_sends_itself_to_the_graveyard_for_a_dark_world_card_after_a_direct_attack() {
    let [cobal, beiige] = ["Cobal, Excavator of Dark World", "Beiige, Vanguard of Dark World"].map(card);
    let Some(mut s) = field(&[], &[mon(0, cobal), grave(0, beiige)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, beiige), (1, FILLER)]); // Cobal itself is a "Dark World" card in the Graveyard by then: not that one
    s.run_both([&[EndTurn, Attack(cobal), Chain(cobal)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 300, "{}", s.context());
    assert_eq!(s.count(0, "grave", cobal), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", beiige), s.count(0, "grave", beiige)), (1, 0), "Beiige was added to the hand: {}", s.context());
    // No Dark World card in the Graveyard.
    let Some(mut s) = field(&[], &[mon(0, cobal)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(cobal), Chain(cobal)], &[EndTurn]]).expect_err("nothing to add");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!((s.lp(1), s.count(0, "mzone", cobal)), (8000 - 300, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Royal Sword and Crest Burn (Fog King)
// ---------------------------------------------------------------------------------------------------------------

/// The Crest Counters on a team's Royal Sword (the engine's counter 0x95).
fn crest_counters(s: &Scenario, team: usize, sword: u16) -> i64 {
    s.state()[team]["szone"].as_array().and_then(|z| z.iter().find(|c| c["cid"] == sword)).and_then(|c| c["counters"]["149"].as_i64()).unwrap_or(0)
}

/// Royal Sword: "Equip only to "Fog King". At the end of the Battle Phase, if it battles: Place 1 Crest Counter on this card
/// (max 4). The equipped monster gains 800 ATK for each Crest Counter on this card. While there are 4 Crest Counters on
/// this card: You can send the equipped monster to the Graveyard; inflict 4000 damage to your opponent." Fog King (0 ATK)
/// is the only monster it can equip (Warwolf alone: not offered). Fog King attacks Dark Magician in Defense Position in four
/// turns (3, 5, 7, 9): after each battle the Sword has one more counter and Fog King 800 more ATK (800, 1600, 2400, 3200),
/// none of them past 4. With 4 counters, its effect sends Fog King and the Sword to the Graveyard and team 1 takes 4000.
#[test]
fn royal_sword_gains_a_crest_counter_for_each_battle_and_ends_in_4000_damage() {
    let [fog, sword] = ["Fog King", "Royal Sword"].map(card);
    let defending = Spawn { team: 1, place: MonsterDefense, cid: DARK_MAGICIAN };
    let Some(mut s) = duel(7, 20000, [FILLER, FILLER], &[], [&[], &[]], &[mon(0, fog), hand(0, sword), defending]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(sword)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", sword), 1, "{}", s.context());
    assert_eq!((crest_counters(&s, 0, sword), s.attack_of(0, fog)), (0, Some(0)), "{}", s.context());
    for battles in 1..=4 {
        s.run(&[Attack(fog), EndTurn]).unwrap();
        clean(&s);
        assert_eq!(crest_counters(&s, 0, sword), battles, "{battles} battles: {}", s.context());
        assert_eq!(s.attack_of(0, fog), Some(800 * battles), "800 for each counter: {}", s.context());
    }
    // Four counters: the equipped monster is sent to the Graveyard for 4000 damage.
    s.run(&[Activate(sword)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", fog), s.count(0, "grave", sword)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 20000 - 4000, "{}", s.context());
}

/// Royal Sword: "Equip only to "Fog King"": with Warwolf as the only monster it is not offered.
#[test]
fn royal_sword_equips_only_to_fog_king() {
    let sword = card("Royal Sword");
    let Some(mut s) = field(&[], &[hand(0, sword), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(sword)]).expect_err("no Fog King");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "hand", sword), 1, "{}", s.context());
}

/// Crest Burn: "When a "Royal Sword" you control is destroyed and sent to the Graveyard: Remove Beacon Counters from a "Fog
/// Castle" you control equal to the number of Crest Counters that were on the destroyed card." After one battle the Sword has
/// 1 Crest Counter; team 1's Mystical Space Typhoon destroys it in turn 4 and Crest Burn answers it. (The Fog Castle the game
/// has gets no Beacon Counter from anything, so there is none to remove: the script's own counter on it, which it never
/// permits, is not there either, and the card is simply used.) A Sword without a counter does not set it off.
#[test]
fn crest_burn_answers_a_royal_sword_with_crest_counters_that_is_destroyed() {
    let [fog, sword, castle, burn] = ["Fog King", "Royal Sword", "Fog Castle", "Crest Burn"].map(card);
    let mst = card("Mystical Space Typhoon");
    let defending = Spawn { team: 1, place: MonsterDefense, cid: DARK_MAGICIAN };
    let spawns = [mon(0, fog), hand(0, sword), spell(0, castle), set(0, burn), hand(1, mst), defending];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, sword), (0, FILLER), (1, FILLER)]);
    s.run_both([&[Activate(sword), Attack(fog), Chain(burn)], &[EndTurn, Activate(mst)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", sword), s.count(0, "grave", burn)), (1, 1), "Royal Sword was destroyed and Crest Burn used: {}", s.context());
    assert_eq!(s.count(0, "szone", castle), 1, "{}", s.context());
    let counters = s.state()[0]["szone"].as_array().unwrap().iter().find(|c| c["cid"] == castle).unwrap()["counters"].clone();
    assert!(counters.as_object().unwrap().is_empty(), "no counters on Fog Castle: {counters}");
    // The same without a battle: the Sword has no Crest Counter and Crest Burn is not offered.
    let spawns = [mon(0, fog), hand(0, sword), spell(0, castle), set(0, burn), hand(1, mst)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, sword), (0, FILLER), (1, FILLER)]);
    let err = s.run_both([&[Activate(sword), Chain(burn)], &[Activate(mst)]]).expect_err("no counter");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!((s.count(0, "grave", sword), s.count(0, "szone", burn)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Forbidden One cards: Defense Seal Wall, Rage of the Forbidden One, Ultimate Ritual of the Forbidden Lord and Exodius
// ---------------------------------------------------------------------------------------------------------------

const GRAVITY_BIND: u16 = 5134; // Level 4 or higher monsters cannot attack
const CARD_DESTRUCTION: u16 = 4821;
const MIRROR_FORCE: u16 = 4887;
const EXODIA: u16 = 4027;
const FORBIDDEN_ARMS_LEGS: [u16; 4] = [4023, 4024, 4025, 4026]; // Right Leg, Left Leg, Right Arm, Left Arm of the Forbidden One

/// Defense Seal Wall: "Activate only when an opponent's monster declares an attack, if you control an "Exodius the Ultimate
/// Forbidden Lord": Negate that attack and end the Battle Phase." Team 1's Warwolf attacks in turn 2: the attack is negated,
/// the turn goes to Main Phase 2 where, without the Wall, team 1's Dark Blade would be asked for its attack, nobody loses LP
/// and the Wall stays face-up. Without an Exodius it cannot be activated.
#[test]
fn defense_seal_wall_negates_an_attack_and_ends_the_battle_phase() {
    let [wall, exodius] = ["Defense Seal Wall", "Exodius the Ultimate Forbidden Lord"].map(card);
    let spawns = [set(0, wall), mon(0, exodius), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(wall)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "the Battle Phase is over: {}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "szone", wall), 1, "the Wall stays on the field: {}", s.context());
    // The control: Exodius is immune to Warwolf's effects but not a wall of its own: with no Wall the Battle Phase goes on
    // (the second Monster has its attack asked for) and Exodius' controller takes the hit.
    let Some(mut s) = field(&[], &[mon(0, exodius), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.prompt()["type"], msg::SELECT_BATTLECMD, "the Battle Phase goes on: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "Exodius is 0 ATK: Warwolf's 2000 is all damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", exodius), 1, "and it cannot be destroyed: {}", s.context());
    // No Exodius, no activation.
    let spawns = [set(0, wall), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(wall)], &[Attack(WARWOLF)]]).expect_err("no Exodius");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000 - 1300, "Warwolf hit Thunder Kid: {}", s.context());
}

/// Defense Seal Wall: "If an opponent's monster declares an attack while you control an "Exodius the Ultimate Forbidden
/// Lord": You can negate the attack and end the Battle Phase." Face-up on the field it does so again: the Wall negates team 1's
/// attacks in turns 2 and 4. "If "Exodius the Ultimate Forbidden Lord" leaves the field, destroy this card": Curse of Dragon
/// is Tribute Summoned in turn 5 with Exodius as the Tribute and the Wall is destroyed with it.
#[test]
fn defense_seal_wall_keeps_negating_while_exodius_is_there_and_is_destroyed_with_it() {
    let [wall, exodius, curse] = ["Defense Seal Wall", "Exodius the Ultimate Forbidden Lord", "Curse of Dragon"].map(card);
    let spawns = [set(0, wall), mon(0, exodius), hand(0, curse), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(wall), Chain(wall)], &[Attack(WARWOLF), EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "both attacks were negated: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", wall), 1, "{}", s.context());
    s.prefer(&[(0, exodius), (0, FILLER)]);
    s.run(&[Summon(curse)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", curse), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", exodius), 0, "Exodius was the Tribute: {}", s.context());
    assert_eq!(s.count(0, "grave", wall), 1, "Exodius left, so the Wall was destroyed: {}", s.context());
}

/// Rage of the Forbidden One: "While you control an "Exodius the Ultimate Forbidden Lord": Until the End Phase, negate the
/// effects of all cards your opponent controls and/or is played this turn." Team 1's Gravity Bind, activated in turn 2 (Level
/// 4 or higher monsters cannot attack), stops Warwolf (Level 4) from attacking, and in turn 3 Rage negates it: Warwolf attacks
/// directly and team 1 takes 2000. Without Rage it cannot attack ("can't attack"). Without Exodius Rage is not offered.
/// (Exodius itself is unaffected by the opponent's effects, so Gravity Bind never stopped it.)
#[test]
fn rage_of_the_forbidden_one_negates_the_effects_of_the_opponents_cards_this_turn() {
    let [rage, exodius] = ["Rage of the Forbidden One", "Exodius the Ultimate Forbidden Lord"].map(card);
    let spawns = [hand(0, rage), mon(0, exodius), mon(0, WARWOLF), set(1, GRAVITY_BIND)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(rage), Attack(WARWOLF)], &[Activate(GRAVITY_BIND)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "Warwolf attacked: {}", s.context());
    assert_eq!(s.count(0, "grave", rage), 1, "{}", s.context());
    // Without Rage: no attack for Warwolf.
    let Some(mut s) = field(&[], &[mon(0, exodius), mon(0, WARWOLF), set(1, GRAVITY_BIND)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(WARWOLF)], &[Activate(GRAVITY_BIND)]]).expect_err("Gravity Bind");
    assert!(err.contains("can't attack"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    // Without Exodius, Rage is not offered.
    let Some(mut s) = field(&[], &[hand(0, rage), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(rage)]).expect_err("no Exodius");
    assert!(err.contains("never became possible"), "{err}");
}

/// Ultimate Ritual of the Forbidden Lord: "Must be activated from the Graveyard. While there are 5 different "Forbidden One"
/// cards in your hand or Graveyard: Reveal all "Forbidden One" cards in your hand, if any; return all "Forbidden One" cards in
/// your Graveyard to your Deck, also send 2 "Forbidden One" cards from your hand to the Graveyard and Special Summon 1
/// "Exodius the Ultimate Forbidden Lord" from your hand or Deck." Three pieces in the Graveyard and two in the hand are five
/// different: the three go back to the Deck, the two in the hand are sent to the Graveyard and Exodius comes from the hand
/// (the Deck's size is up by 3). Four different are not enough: it is not offered.
#[test]
fn ultimate_ritual_activates_from_the_graveyard_for_exodius() {
    let [ritual, exodius] = ["Ultimate Ritual of the Forbidden Lord", "Exodius the Ultimate Forbidden Lord"].map(card);
    let [right_leg, left_leg, right_arm, left_arm] = FORBIDDEN_ARMS_LEGS;
    let spawns = [grave(0, ritual), grave(0, left_leg), grave(0, right_arm), grave(0, left_arm), hand(0, EXODIA), hand(0, right_leg), hand(0, exodius)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let deck_before = deck_size(&s, 0);
    s.run(&[Activate(ritual)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", exodius), 1, "Exodius was Special Summoned: {}", s.context());
    for returned in [left_leg, right_arm, left_arm] {
        assert_eq!(s.count(0, "grave", returned), 0, "{returned} went back to the Deck: {}", s.context());
    }
    assert_eq!(deck_size(&s, 0), deck_before + 3, "{}", s.context());
    assert_eq!((s.count(0, "grave", EXODIA), s.count(0, "grave", right_leg)), (1, 1), "the two in the hand were sent: {}", s.context());
    assert_eq!(s.count(0, "grave", ritual), 1, "the Ritual is back in the Graveyard: {}", s.context());
    // Only four different pieces.
    let spawns = [grave(0, ritual), grave(0, left_leg), grave(0, right_arm), hand(0, EXODIA), hand(0, right_leg), hand(0, exodius)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(ritual)]).expect_err("four different pieces");
    assert!(err.contains("never became possible"), "{err}");
}

/// Ultimate Ritual: "While this card is in the Graveyard and you control an "Exodius the Ultimate Forbidden Lord", when a
/// "Forbidden One" card(s) is sent to the Graveyard by a card effect, except by the effect of "Exodius the Ultimate Forbidden
/// Lord", return them to the Deck." Team 1's Card Destruction makes team 0 discard a Left Arm of the Forbidden One, which goes
/// back to the Deck instead of staying in the Graveyard. Without the Ritual in the Graveyard it stays there.
#[test]
fn ultimate_ritual_in_the_graveyard_returns_forbidden_one_cards_sent_by_card_effects() {
    let [ritual, exodius] = ["Ultimate Ritual of the Forbidden Lord", "Exodius the Ultimate Forbidden Lord"].map(card);
    let arm = FORBIDDEN_ARMS_LEGS[3];
    let spawns = [grave(0, ritual), mon(0, exodius), hand(0, arm), hand(1, CARD_DESTRUCTION)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let deck_before = deck_size(&s, 0);
    s.run_both([&[EndTurn], &[Activate(CARD_DESTRUCTION)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", arm), 0, "the Arm did not stay in the Graveyard: {}", s.context());
    // (It went to the Deck, which Card Destruction then draws from: the Deck is down by the cards drawn, one fewer.)
    let drawn = hand_size(&s, 0) as i64;
    assert_eq!(deck_size(&s, 0), deck_before + 1 - drawn, "it was shuffled into the Deck: {}", s.context());
    let spawns = [mon(0, exodius), hand(0, arm), hand(1, CARD_DESTRUCTION)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(CARD_DESTRUCTION)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", arm), 1, "without the Ritual it stays: {}", s.context());
}

/// Exodius the Ultimate Forbidden Lord: "Win the Duel when there are 5 "Forbidden One" cards with different names in your GY"
/// and "Send 1 "Forbidden One" monster from your hand or Deck to the GY when this card attacks". Four different pieces in the
/// Graveyard and Exodia the Forbidden One in the hand: Exodius attacks in turn 3, sends Exodia, and team 0 wins the Duel.
#[test]
fn exodius_wins_the_duel_with_five_different_forbidden_one_cards_in_the_graveyard() {
    let exodius = card("Exodius the Ultimate Forbidden Lord");
    let spawns = [mon(0, exodius), grave(0, FORBIDDEN_ARMS_LEGS[0]), grave(0, FORBIDDEN_ARMS_LEGS[1]), grave(0, FORBIDDEN_ARMS_LEGS[2]),
        grave(0, FORBIDDEN_ARMS_LEGS[3]), hand(0, EXODIA)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[EndTurn, Attack(exodius)]).expect_err("the Duel ends");
    assert!(err.contains("the duel is over"), "{err}");
    let win = s.messages().iter().find(|m| m["type"] == msg::WIN).unwrap_or_else(|| panic!("a win: {}", s.context()));
    assert_eq!(win["player"], 0, "team 0 won: {win}");
    assert_eq!(s.count(0, "grave", EXODIA), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dual Gate and Eternal Reverse
// ---------------------------------------------------------------------------------------------------------------

/// Dual Gate: "Banish this face-up card and 1 "Dual Gate" from your Graveyard; Draw 2 cards." With a Dual Gate in the
/// Graveyard, the one in the hand and that one are both banished and 2 cards are drawn (the hand is one card more than it
/// was). With none in the Graveyard it is not offered.
#[test]
fn dual_gate_banishes_itself_and_a_dual_gate_from_the_graveyard_to_draw_two() {
    let gate = card("Dual Gate");
    let Some(mut s) = field(&[], &[hand(0, gate), grave(0, gate)]) else { return };
    keep_hands(&mut s);
    let (hand_before, deck_before) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run(&[Activate(gate)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", gate), s.count(0, "grave", gate), s.count(0, "hand", gate)), (2, 0, 0), "{}", s.context());
    assert_eq!((hand_size(&s, 0), deck_size(&s, 0)), (hand_before + 1, deck_before - 2), "-1 played, +2 drawn: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, gate)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(gate)]).expect_err("no Dual Gate to banish");
    assert!(err.contains("never became possible"), "{err}");
}

/// Eternal Reverse: "Once per turn: You can Set 1 Spell/Trap Card on your opponent's side of the field face-down, and if you
/// do, it cannot be activated for the rest of this turn." Equipped to Warwolf, it Sets team 1's Gravity Bind (Level 4 or
/// higher monsters cannot attack, from turn 2 on) face-down in turn 3: Warwolf attacks directly for 2000 in the same turn,
/// as a face-down Gravity Bind stops nothing. (The script's note on Pendulum cards, a filter, matters to nothing here.)
#[test]
fn eternal_reverse_sets_an_opponents_spell_or_trap_face_down() {
    let reverse = card("Eternal Reverse");
    let spawns = [hand(0, reverse), mon(0, WARWOLF), set(1, GRAVITY_BIND)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (1, GRAVITY_BIND)]);
    s.run_both([&[Activate(reverse), EndTurn, Activate(reverse), Attack(WARWOLF)], &[Activate(GRAVITY_BIND)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", reverse), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "Warwolf attacked past a Set Gravity Bind: {}", s.context());
    let gb = s.state()[1]["szone"].as_array().unwrap().iter().find(|c| c["cid"] == GRAVITY_BIND).unwrap();
    assert_eq!(gb["position"], 10, "Gravity Bind is face-down again: {gb}");
}

/// Eternal Reverse: "If the equipped monster would be destroyed by battle, you can send this card to the Graveyard instead."
/// Team 1's Dark Magician (2500) beats Warwolf (2000) in turn 2, and Eternal Reverse goes to the Graveyard in its place:
/// Warwolf stays, team 0 takes the 500.
#[test]
fn eternal_reverse_is_sent_to_the_graveyard_in_place_of_the_equipped_monster() {
    let reverse = card("Eternal Reverse");
    let Some(mut s) = field(&[], &[hand(0, reverse), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Activate(reverse), Chain(reverse)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf was not destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", reverse), 1, "Eternal Reverse went instead: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Cloud cards: Big Summon Cloud, Exploding Cloud, Mirage Target and Wonder Cloud
// ---------------------------------------------------------------------------------------------------------------

/// Big Summon Cloud: "Once per turn, during your Standby Phase, discard your entire hand or destroy this card. You can pay
/// half your LP; Special Summon 1 "Cloudian" monster from your hand." Cloudian - Smoke Ball comes from the hand for 4000 of
/// 8000 LP.
#[test]
fn big_summon_cloud_pays_half_the_lp_for_a_cloudian_from_the_hand() {
    let [cloud, smoke] = ["Big Summon Cloud", "Cloudian - Smoke Ball"].map(card);
    // (A Cloud spawned on the field would have its first Standby Phase before the plan starts: it is played from the hand.)
    let Some(mut s) = field(&[], &[hand(0, cloud), hand(0, smoke)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(cloud), Activate(cloud)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", smoke), 1, "{}", s.context());
    assert_eq!(s.lp(0), 4000, "half of 8000: {}", s.context());
    assert_eq!(s.position_of(0, smoke), Some(1), "{}", s.context());
    assert_eq!(s.count(0, "szone", cloud), 1, "{}", s.context());
}

/// Big Summon Cloud at ∞ (LP over 9,999,999 is exactly 19,999,998, which a finite cost leaves as it is: half of ∞ is ∞,
/// crates/yo3-engine/src/guards.lua): the Cloudian comes and the LP stay ∞. (The case of `infinite_lp.rs` is this one, kept
/// here with the batch: the script works the amount out from the LP, `Duel.GetLP(tp)/2`.)
#[test]
fn big_summon_cloud_at_infinite_lp_pays_half_of_infinity_and_keeps_it() {
    const INFINITE: i64 = 19_999_998;
    let [cloud, smoke] = ["Big Summon Cloud", "Cloudian - Smoke Ball"].map(card);
    let Some(mut s) = duel(7, 20_000_000, [FILLER, FILLER], &[], [&[], &[]], &[hand(0, cloud), hand(0, smoke)]) else { return };
    keep_hands(&mut s);
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, INFINITE), "{}", s.context());
    s.run(&[Activate(cloud), Activate(cloud)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", smoke), 1, "{}", s.context());
    assert_eq!(s.lp(0), INFINITE, "half of ∞ is ∞: {}", s.context());
}

/// Big Summon Cloud: "Once per turn, during your Standby Phase, discard your entire hand or destroy this card." In turn 3's
/// Standby Phase one of the two happens: the hand goes to the Graveyard (the Cloud stays) or the Cloud is destroyed.
#[test]
fn big_summon_cloud_costs_the_hand_or_itself_in_the_standby_phase() {
    let cloud = card("Big Summon Cloud");
    let Some(mut s) = field(&[], &[hand(0, cloud)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(cloud)]).unwrap();
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    let (discarded, destroyed) = (hand_size(&s, 0) == 0 && s.count(0, "szone", cloud) == 1, s.count(0, "grave", cloud) == 1);
    assert!(discarded != destroyed, "the hand was discarded, or the Cloud destroyed, not both or neither: {}", s.context());
}

/// Exploding Cloud: "Target 1 face-up Attack Position monster on the field; Destroy it, then take damage equal to that
/// monster's ATK." It takes either side's monster: team 1's Warwolf (2000) is destroyed and team 0 takes 2000; its own Thunder
/// Kid (700) is destroyed for 700.
#[test]
fn exploding_cloud_destroys_a_monster_and_hurts_its_player_for_its_atk() {
    let cloud = card("Exploding Cloud");
    let Some(mut s) = field(&[], &[hand(0, cloud), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(cloud)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, cloud), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(cloud)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "{}", s.context());
    // A monster in Defense Position is no target.
    let defending = Spawn { team: 1, place: MonsterDefense, cid: WARWOLF };
    let Some(mut s) = field(&[], &[hand(0, cloud), defending]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(cloud)]).expect_err("only Attack Position");
    assert!(err.contains("never became possible"), "{err}");
}

/// Mirage Target: "When your opponent's monster declares an attack: Send 1 "Cloudian" monster you control to the Graveyard;
/// Negate the attack, then gain LP equal to the original ATK of the sent monster." Team 1's Warwolf attacks in turn 2:
/// Cloudian - Altus (1300) is sent, the attack is negated and team 0 gains 1300. Without a Cloudian it is not offered.
#[test]
fn mirage_target_sends_a_cloudian_to_negate_the_attack_and_gain_its_atk() {
    let [mirage, altus] = ["Mirage Target", "Cloudian - Altus"].map(card);
    let Some(mut s) = field(&[], &[set(0, mirage), mon(0, altus), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(mirage)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", altus), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 + 1300, "the attack was negated and 1300 gained: {}", s.context());
    assert_eq!(s.count(0, "grave", mirage), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, mirage), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(mirage)], &[Attack(WARWOLF)]]).expect_err("no Cloudian");
    assert!(err.contains("never became possible"), "{err}");
}

/// Wonder Cloud: "Banish all cards you control and all cards in your hand; draw 1 card for each card your opponent controls,
/// then banish all cards in your Deck." Team 1 controls two monsters: team 0 banishes its Warwolf and its whole hand, draws 2
/// and banishes the Deck, holding just the two drawn cards, with nothing left in the Deck.
#[test]
fn wonder_cloud_banishes_everything_and_draws_a_card_for_each_card_of_the_opponent() {
    let wonder = card("Wonder Cloud");
    let Some(mut s) = field(&[], &[hand(0, wonder), mon(0, WARWOLF), mon(1, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    let (hand_before, deck_before) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run(&[Activate(wonder)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), 2, "two cards drawn for the opponent's two: {}", s.context());
    assert_eq!(deck_size(&s, 0), 0, "{}", s.context());
    let removed = s.state()[0]["removed"].as_array().unwrap().len();
    assert_eq!(removed, hand_before - 1 + 1 + (deck_before as usize - 2), "the hand, Warwolf and the rest of the Deck: {}", s.context());
    assert_eq!((s.count(0, "removed", WARWOLF), s.count(0, "grave", wonder)), (1, 1), "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE) + s.count(1, "mzone", THUNDER_KID), 2, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The rest of the batch's cards, one case each: Thousand Buster, Fiend Rose, Revenge Soul, Dark Tournament, Slime Base,
// Trap Trip and Despised Reality
// ---------------------------------------------------------------------------------------------------------------

/// Thousand Buster: "Pay 1000 LP; Destroy all face-up Defense Position monsters on the field with 1000 or less DEF, and all
/// Attack Position monsters on the field with 1000 or less ATK, then, both players take 1000 damage." Thunder Kid on each side
/// (700 ATK) and team 1's Warwolf in Defense (DEF 100) are destroyed, Dark Blade (1800) is not, and team 0 is at 8000 - 1000
/// (cost) - 1000, team 1 at 7000.
#[test]
fn thousand_buster_destroys_the_weak_monsters_and_hurts_both_players() {
    let buster = card("Thousand Buster");
    let defending = Spawn { team: 1, place: MonsterDefense, cid: WARWOLF };
    let spawns = [set(0, buster), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, THUNDER_KID), defending];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(buster)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", THUNDER_KID), s.count(1, "grave", THUNDER_KID), s.count(1, "grave", WARWOLF)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1000 - 1000, 8000 - 1000), "{}", s.context());
}

/// Fiend Rose: "Activate this card by targeting 1 monster you control; equip this card to that target. The equipped monster
/// cannot be destroyed by battle and you take no battle damage from battles involving the equipped monster." Team 1's Dark
/// Magician (2500) attacks Dark Blade (1800) in turn 2: Dark Blade lives and nobody is hurt. Without the Rose it is destroyed
/// and its controller takes 700.
#[test]
fn fiend_rose_keeps_the_equipped_monster_alive_and_its_controller_unhurt() {
    let rose = card("Fiend Rose");
    let Some(mut s) = field(&[], &[set(0, rose), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(rose)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", rose), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.lp(0)), (1, 8000 - 700), "{}", s.context());
}

/// Revenge Soul: "Return 1 monster destroyed by battle this turn to its owner's hand." Team 1's Dark Magician destroys team
/// 0's Warwolf in turn 2 and team 0's Set Revenge Soul brings Warwolf back to the hand.
#[test]
fn revenge_soul_returns_a_monster_destroyed_by_battle_this_turn() {
    let soul = card("Revenge Soul");
    let Some(mut s) = field(&[], &[set(0, soul), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(soul)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", WARWOLF), s.count(0, "grave", WARWOLF)), (1, 0), "Warwolf is back in the hand: {}", s.context());
    assert_eq!(s.count(0, "grave", soul), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
}

/// Dark Tournament: "Change all monsters on the field to face-up Attack Position. This turn, all face-up Attack Position
/// monsters must declare an attack, if able, and they cannot be destroyed by battle." In turn 3 team 1's Dark Blade (1800)
/// goes from Defense to Attack Position, Warwolf (2000) attacks it and neither is destroyed, with 200 damage to team 1.
#[test]
fn dark_tournament_puts_every_monster_in_attack_position_and_keeps_them_from_dying() {
    let tournament = card("Dark Tournament");
    let defending = Spawn { team: 1, place: MonsterDefense, cid: DARK_BLADE };
    let Some(mut s) = field(&[], &[set(0, tournament), mon(0, WARWOLF), defending]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(tournament), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, DARK_BLADE), Some(1), "Dark Blade is in Attack Position: {}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", DARK_BLADE)), (1, 1), "nobody was destroyed: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 200), "{}", s.context());
}

/// Slime Base: "Special Summon 1 "Slime" monster from your hand." Draw Slime comes from the hand in face-up Attack Position.
#[test]
fn slime_base_summons_a_slime_from_the_hand() {
    let [base, draw] = ["Slime Base", "Draw Slime"].map(card);
    let Some(mut s) = field(&[], &[hand(0, base), hand(0, draw)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(base)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", draw), 1, "{}", s.context());
    assert_eq!(s.position_of(0, draw), Some(1), "{}", s.context());
    assert_eq!(s.count(0, "grave", base), 1, "{}", s.context());
}

/// Trap Trip: "Target 1 Trap Card in your Graveyard; add it to your hand." Mirror Force comes from the Graveyard to the hand
/// (a Spell in the Graveyard is no target).
#[test]
fn trap_trip_adds_a_trap_from_the_graveyard_to_the_hand() {
    let trip = card("Trap Trip");
    let Some(mut s) = field(&[], &[set(0, trip), grave(0, MIRROR_FORCE), grave(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(trip)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", MIRROR_FORCE), s.count(0, "grave", MIRROR_FORCE)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", trip), 1, "{}", s.context());
}

/// Despised Reality: "Activate only during your opponent's turn. Your opponent can Special Summon 1 Level 4 or lower monster
/// from their Deck. If they do, you draw 2 cards. If not, you take 1000 damage." Activated in turn 2, one of the two
/// happens (the CPU answers the question): the opponent brings a Feral Imp and team 0 draws 2, or team 0 takes 1000.
#[test]
fn despised_reality_gives_the_opponent_a_choice_of_a_summon_for_two_cards_or_1000_damage() {
    let reality = card("Despised Reality");
    let Some(mut s) = field(&[], &[set(0, reality)]) else { return };
    keep_hands(&mut s);
    let hand_before = hand_size(&s, 0);
    s.run_both([&[Chain(reality)], &[EndTurn]]).unwrap(); // ends in team 0's turn 3, after its draw for the turn
    clean(&s);
    assert_eq!(s.count(0, "grave", reality), 1, "{}", s.context());
    let summoned = s.count(1, "mzone", FILLER) == 1;
    if summoned {
        assert_eq!((s.lp(0), hand_size(&s, 0)), (8000, hand_before + 2 + 1), "two cards drawn: {}", s.context());
    } else {
        assert_eq!((s.lp(0), hand_size(&s, 0)), (8000 - 1000, hand_before + 1), "1000 damage: {}", s.context());
    }
}
