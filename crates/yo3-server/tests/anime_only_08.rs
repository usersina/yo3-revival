//! Anime-only cards, group 8 (rows 5467-5516 of engine/added/cards.tsv): Jaden Yuki's cards. These cards exist only in
//! the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real
//! core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones.
//! Every Trap and every Quick-Play Spell answering the opponent is Set first, and neither player has a Battle Phase in
//! turn 1, so team 0's attacks wait for turn 3.
//! Covered: the Token (Half Token, made by Wild Half), the cards the per-card fuzz flagged (Bubble Rod, Clay Wrap, Contact
//! Soul, Cross Change, Cross Heart, Death and Rebirth, Elemental HERO Clay Guardian, Elemental Mirage, Elemental Storm, Feather
//! Storm, Fusion Shot, Hero Dice, Hero Shield, Hero's Backup, Hero's Guard, Hope for the Future, Impact Flip, Mud Max, Neospace
//! Road, Relationship, Tag Request), the cards `--find` noted (Courageous Charge, Impact Flip, Spy Hero), the signature cards
//! (Wild Half, Neos Energy, Neo Signal, Astral Shift, Take Over 5, Neospace Wave) and a seeded sample of the rest (Battle x 2,
//! Climax Hour, Cyclone Wing, Damage Off Zone, Dummy Marker, Hero Call, Hero's Guild, Skill Shock). The fuzz flagged those cards
//! for boards the self-play never builds (an Elemental HERO next to its equip card, a Neos with a Polymerization in the hand, a
//! Fusion Monster whose Material lies in the Graveyard, a Normal Monster destroyed in the Battle Phase...), and all of them work:
//! none needed an override or a change to `translate/`.
//! The script notes: Courageous Charge, Impact Flip and Spy Hero take their first target without a check, which could stop the
//! effect on a Lua error if a response removed the target. The core hands the card over anyway, so the effect finds nothing to
//! do and the engine reports no error (`courageous_charge_with_its_target_destroyed_in_answer_does_nothing`,
//! `impact_flip_with_its_target_destroyed_in_answer_does_nothing`); Spy Hero's cost only mills its own Deck, never the target in
//! the opponent's Graveyard, and no card of the game answers by taking a Spell out of a Graveyard (Soul Release is a Normal
//! Spell), so that one has no answered case.
//! Harness notes: the CPU answers every single optional trigger "yes", so a test that declines one (Damage Off Zone's second
//! effect) answers the prompt itself; a `Chain` action is taken at the first chance its team gets, so a card answering one
//! effect in particular (Raigeki Break against a target) is chained by `answer_with` to that effect's own window.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DARK_HOLE: u16 = 4342;
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const POLYMERIZATION: u16 = 4837;
const RAIGEKI_BREAK: u16 = 5544; // Trap: discard 1 card; target 1 card on the field; destroy it

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }
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

fn monsters(s: &Scenario, team: usize) -> usize {
    s.state()[team]["mzone"].as_array().map_or(0, |z| z.iter().filter(|c| c["cid"].as_u64().is_some_and(|c| c != 0)).count())
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// Plays `plan` for team 0 and stops at the first answering window in which team `team` can chain `cid`: the card is then
/// chained to the effect just activated, not to the first chance its team gets (a Set card offered at a phase). Returns
/// with the card's chain link added and the duel carried on to the next idle prompt.
fn answer_with(s: &mut Scenario, plan: &[yo3_server::duelcheck::scenario::Action], team: u64, cid: u16) {
    s.run(&[]).unwrap(); // to the first Main Phase prompt (the duel opens on a chain window)
    s.run_to(plan, |p| {
        p["type"] == msg::SELECT_CHAIN && p["player"] == team && p["selects"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
    })
    .unwrap();
    let i = s.prompt()["selects"].as_array().unwrap().iter().position(|c| c["cid"] == cid).unwrap();
    s.respond(Response::Chain(Some(i)).json()).unwrap();
    s.run(&[]).unwrap();
}

// ---------------------------------------------------------------------------------------------------------------
// Wild Half and the Half Token
// ---------------------------------------------------------------------------------------------------------------

/// Wild Half: "If you control an "Elemental Hero Wildheart": Target 1 monster your opponent controls; halve its ATK,
/// then, Special Summon 1 "Half Token" to your opponent's side of the field with the same Level, Type, Attribute, ATK,
/// DEF and effect(s) as the targeted monster." Dark Blade (1800/1500, Level 4 Warrior, EARTH) is halved to 900 and a Half
/// Token with those stats stands on team 1's side; team 0 gets nothing.
#[test]
fn wild_half_halves_the_target_and_makes_a_half_token_on_the_opponents_side() {
    let [wild_half, wildheart, token] = ["Wild Half", "Elemental Hero Wildheart", "Half Token"].map(card);
    let Some(mut s) = field(&[], &[mon(0, wildheart), hand(0, wild_half), mon(1, DARK_BLADE)]) else { return };
    let blade = (s.monster(1, DARK_BLADE).unwrap()["level"].as_i64(), s.monster(1, DARK_BLADE).unwrap()["race"].as_i64(), s.monster(1, DARK_BLADE).unwrap()["attribute"].as_i64());
    assert_eq!(s.attack_of(1, DARK_BLADE), Some(1800));
    s.run(&[Activate(wild_half)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, DARK_BLADE), Some(900), "the target's ATK is halved: {}", s.context());
    assert_eq!(s.count(1, "mzone", token), 1, "the Token is on the opponent's side: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 0, "{}", s.context());
    let t = s.monster(1, token).unwrap();
    assert_eq!((t["level"].as_i64(), t["race"].as_i64(), t["attribute"].as_i64()), blade, "Level, Type and Attribute of the target: {}", s.context());
    assert_eq!((t["attack"].as_i64(), t["defense"].as_i64()), (Some(900), Some(1500)), "the target's stats: {}", s.context());
    assert_eq!(s.count(0, "grave", wild_half), 1, "{}", s.context());
}

/// The Half Token leaves the field like any Token: with Dark Hole afterwards it is destroyed and is not in the Graveyard,
/// where Dark Blade (a card) is.
#[test]
fn the_half_token_is_gone_when_it_leaves_the_field() {
    let [wild_half, wildheart, token] = ["Wild Half", "Elemental Hero Wildheart", "Half Token"].map(card);
    let Some(mut s) = field(&[], &[mon(0, wildheart), hand(0, wild_half), hand(0, DARK_HOLE), mon(1, DARK_BLADE)]) else { return };
    s.run(&[Activate(wild_half), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((monsters(&s, 0), monsters(&s, 1)), (0, 0), "{}", s.context());
    assert_eq!(s.count(1, "grave", token), 0, "a Token is never in the Graveyard: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Bubble Rod, Clay Wrap
// ---------------------------------------------------------------------------------------------------------------

/// Bubble Rod: "Equip only to "Elemental Hero Bubbleman". It gains 800 ATK. When the equipped monster is destroyed: draw
/// 1 card." Equipped to Bubbleman it adds 800 ATK, it is not offered with only another monster on the field, and when
/// Dark Hole (team 1's) destroys Bubbleman team 0 draws 1 card.
#[test]
fn bubble_rod_gives_800_atk_to_bubbleman_and_draws_when_it_is_destroyed() {
    let [rod, bubbleman] = ["Bubble Rod", "Elemental Hero Bubbleman"].map(card);
    let Some(mut s) = field(&[], &[mon(0, bubbleman), hand(0, rod), hand(1, DARK_HOLE)]) else { return };
    let before = s.attack_of(0, bubbleman).unwrap();
    s.run(&[Activate(rod)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", rod), 1, "equipped: {}", s.context());
    assert_eq!(s.attack_of(0, bubbleman), Some(before + 800), "{}", s.context());
    let hand_before = hand_size(&s, 0);
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bubbleman), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", rod), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before + 1, "Bubble Rod drew a card: {}", s.context());
    // Bubble Rod is for Bubbleman only: with another monster alone it is not offered.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), hand(0, rod)]) else { return };
    let err = s.run(&[Activate(rod)]).expect_err("no Bubbleman to equip");
    assert!(err.contains("never became possible"), "{err}");
}

/// Clay Wrap: "Equip only to an "Elemental HERO Clayman" you control. When this card is sent to the Graveyard while
/// equipped, send 1 Spell or Trap Card your opponent controls to the Graveyard." Team 1 destroys the equip card with
/// Mystical Space Typhoon and team 0 sends team 1's Set Trap Hole to the Graveyard. (When the equipped Clayman is
/// destroyed instead the Wrap goes with it and the effect does not happen: the Set Trap stays.)
#[test]
fn clay_wrap_sends_a_spell_or_trap_when_it_is_destroyed_while_equipped() {
    let [wrap, clayman, trap_hole, typhoon] = ["Clay Wrap", "Elemental Hero Clayman", "Trap Hole", "Mystical Space Typhoon"].map(card);
    let spawns = [mon(0, clayman), hand(0, wrap), hand(1, typhoon), set(1, trap_hole)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, wrap), (1, trap_hole)]);
    s.run_both([&[Activate(wrap), EndTurn], &[Activate(typhoon)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", wrap), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", trap_hole), 1, "Clay Wrap sent the Set Trap Hole: {}", s.context());
    assert_eq!(s.count(1, "szone", trap_hole), 0, "{}", s.context());
    // The equipped Clayman is destroyed (Dark Hole): the Wrap goes to the Graveyard with it, no effect.
    let spawns = [mon(0, clayman), hand(0, wrap), hand(1, DARK_HOLE), set(1, trap_hole)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Activate(wrap), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", wrap), 1, "{}", s.context());
    assert_eq!(s.count(1, "szone", trap_hole), 1, "the Set Trap stayed: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Contact Soul, Cross Change
// ---------------------------------------------------------------------------------------------------------------

/// Contact Soul: "If you control a "Elemental HERO Neos": Special Summon 1 "Neo-Spacian" monster from your hand, Deck, or
/// Graveyard. During the End Phase, return all monsters you control to the Deck." Aqua Dolphin comes from the Graveyard,
/// and at the end of the turn both it and Neos are in the Deck (the field is empty, the Deck 2 bigger). Without Neos it
/// is not offered.
#[test]
fn contact_soul_summons_a_neo_spacian_and_returns_every_monster_at_the_end_of_the_turn() {
    let [soul, neos, dolphin] = ["Contact Soul", "Elemental Hero Neos", "Neo-Spacian Aqua Dolphin"].map(card);
    let Some(mut s) = field(&[], &[mon(0, neos), hand(0, soul), grave(0, dolphin)]) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Activate(soul)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dolphin), 1, "Aqua Dolphin came from the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", dolphin), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", neos), 1, "{}", s.context());
    // The End Phase: every monster goes back to the Deck.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(monsters(&s, 0), 0, "the field is empty: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck + 2 - 1, "both monsters are in the Deck (less team 0's turn 3 draw): {}", s.context());
    assert_eq!((s.count(0, "grave", dolphin), s.count(0, "grave", neos)), (0, 0), "{}", s.context());
    // Without Neos on the field the card is not offered; a Neo-Spacian from the hand works as well.
    let Some(mut s) = field(&[], &[hand(0, soul), hand(0, dolphin)]) else { return };
    let err = s.run(&[Activate(soul)]).expect_err("no Neos");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = field(&[], &[mon(0, neos), hand(0, soul), hand(0, dolphin)]) else { return };
    s.run(&[Activate(soul)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dolphin), 1, "{}", s.context());
}

/// Cross Change, first effect: "Return 1 "Elemental HERO" you control to the Deck, and if you do, Special Summon 1
/// "Neo-Spacian" monster from your Deck with a Level equal to the returned monster's." Elemental HERO Avian (Level 3)
/// goes back, Neo-Spacian Aqua Dolphin (Level 3) comes from the bottom of the Deck.
#[test]
fn cross_change_trades_a_hero_for_a_neo_spacian_of_the_same_level() {
    let [change, avian, dolphin] = ["Cross Change", "Elemental Hero Avian", "Neo-Spacian Aqua Dolphin"].map(card);
    let Some(mut s) = field(&[dolphin], &[mon(0, avian), hand(0, change)]) else { return };
    s.run(&[Activate(change)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dolphin), 1, "the Neo-Spacian came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "mzone", avian), 0, "Avian went back: {}", s.context());
    assert_eq!(s.count(0, "grave", avian), 0, "to the Deck, not the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", change), 1, "{}", s.context());
}

/// Cross Change, second effect: "Return 1 "Neo-Spacian" monster you control to the Deck, and if you do, Special Summon 1
/// "Elemental HERO" monster from your Deck with a Level equal to the returned monster's." With both on the field, the
/// second option is chosen: Aqua Dolphin (Level 3) goes back and Avian (Level 3) comes, while Elemental HERO Burstinatrix
/// on the field stays.
#[test]
fn cross_change_trades_a_neo_spacian_for_a_hero_of_the_same_level() {
    let [change, avian, dolphin, burstinatrix] = ["Cross Change", "Elemental Hero Avian", "Neo-Spacian Aqua Dolphin", "Elemental Hero Burstinatrix"].map(card);
    let Some(mut s) = field(&[avian], &[mon(0, dolphin), mon(0, burstinatrix), hand(0, change)]) else { return };
    s.choose_options(&[1]);
    s.prefer(&[(0, dolphin)]);
    s.run(&[Activate(change)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", avian), 1, "the HERO came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "mzone", dolphin), 0, "Aqua Dolphin went back: {}", s.context());
    assert_eq!(s.count(0, "mzone", burstinatrix), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Cross Heart, Death and Rebirth, Dummy Marker
// ---------------------------------------------------------------------------------------------------------------

/// Cross Heart: "Activate only when an opponent's monster has attacked you directly. Take control of that opponent's
/// monster." Team 1's Dark Blade attacks team 0 directly in turn 2 (1800 damage); after the damage team 0 chains Cross
/// Heart and Dark Blade is on team 0's field. When team 1's monster attacks a monster instead, it is not an answer.
#[test]
fn cross_heart_takes_the_monster_that_attacked_directly() {
    let heart = card("Cross Heart");
    let Some(mut s) = field(&[], &[set(0, heart), mon(1, DARK_BLADE)]) else { return };
    s.run_both([&[Chain(heart)], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1800, "the direct attack hit: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "team 0 controls Dark Blade now: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", heart), 1, "{}", s.context());
    // Against a monster of team 0's it is no answer: the Trap stays Set.
    let Some(mut s) = field(&[], &[set(0, heart), mon(0, THUNDER_KID), mon(1, DARK_BLADE)]) else { return };
    s.run_both([&[], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was attacked: {}", s.context());
    assert_eq!(s.count(0, "szone", heart), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "{}", s.context());
}

/// Death and Rebirth: "When a Normal Monster you control is destroyed during your Battle Phase: Destroy 1 monster your
/// opponent controls and Special Summon your destroyed Normal Monster in face-up Attack Position. The monster Special
/// Summoned by this effect must attack this turn." In turn 3 Thunder Kid (Normal, 700) attacks Warwolf (2000) and is
/// destroyed; the Trap destroys Warwolf and brings Thunder Kid back in Attack Position, and the Battle Phase cannot end
/// without its attack.
#[test]
fn death_and_rebirth_destroys_a_monster_and_revives_the_normal_monster_which_must_attack() {
    let rebirth = card("Death and Rebirth");
    let Some(mut s) = field(&[], &[set(0, rebirth), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Attack(THUNDER_KID), Chain(rebirth)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid is back: {}", s.context());
    assert_eq!(s.position_of(0, THUNDER_KID), Some(1), "in face-up Attack Position: {}", s.context());
    assert_eq!(s.count(0, "grave", rebirth), 1, "{}", s.context());
    // It must attack: the Battle Phase cannot be left, and the revived monster is the one that can attack.
    let p = s.prompt();
    assert_eq!((p["type"].as_u64(), p["to_ep"].as_bool()), (Some(msg::SELECT_BATTLECMD), Some(false)), "must attack: {}", s.context());
    assert_eq!(offered(&s, "attacks"), vec![serde_json::json!(THUNDER_KID)], "{}", s.context());
    s.pass().unwrap();
    s.run(&[]).unwrap(); // the attack's chain window and its damage
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 700, "the revived Thunder Kid attacked directly: {}", s.context());
}

/// Death and Rebirth: "Normal Monster": an Effect Monster destroyed the same way is not enough, the Trap is not answered. Team
/// 0's Marauding Captain attacks Warwolf and is destroyed; the Trap stays Set and Warwolf stays too.
#[test]
fn death_and_rebirth_needs_a_normal_monster() {
    let [rebirth, captain] = ["Death and Rebirth", "Marauding Captain"].map(card);
    let Some(mut s) = field(&[], &[set(0, rebirth), mon(0, captain), mon(1, WARWOLF)]) else { return };
    let err = s.run_both([&[EndTurn, Attack(captain), Chain(rebirth)], &[EndTurn]]).expect_err("not a Normal Monster");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "grave", captain), 1, "the Effect Monster was destroyed: {}", s.context());
    assert_eq!(s.count(0, "szone", rebirth), 1, "the Trap stayed Set: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Dummy Marker: "When this Set card is destroyed: Draw 1 card. If this Set card was destroyed by the effect of an
/// opponent's Spell, draw 1 more card." Mystical Space Typhoon (a Spell) destroys the Set Dummy Marker: team 0 draws 2.
/// Raigeki Break (a Trap): team 0 draws 1.
#[test]
fn dummy_marker_draws_one_card_or_two_when_a_spell_destroys_it() {
    let [marker, typhoon] = ["Dummy Marker", "Mystical Space Typhoon"].map(card);
    let Some(mut s) = field(&[], &[set(0, marker), hand(1, typhoon)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, marker)]);
    let before = hand_size(&s, 0);
    s.run_both([&[EndTurn], &[Activate(typhoon)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", marker), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 2, "destroyed by a Spell: 2 cards: {}", s.context());
    let Some(mut s) = field(&[], &[set(0, marker), set(1, RAIGEKI_BREAK)]) else { return };
    s.prefer(&[(0, marker), (1, FILLER)]);
    let before = hand_size(&s, 0);
    s.run_both([&[EndTurn], &[Activate(RAIGEKI_BREAK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", marker), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "destroyed by a Trap: 1 card: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Elemental HERO Clay Guardian, Elemental Mirage, Elemental Storm, Feather Storm
// ---------------------------------------------------------------------------------------------------------------

/// Elemental HERO Clay Guardian (Anime): "This card can only be Special Summoned with "Metamorphosis" by Tributing
/// "Elemental HERO Clayman". This card's name becomes "Elemental HERO Clayman" while it is on the field. If this card is
/// Special Summoned: Inflict 200 damage to your opponent for each card they control." Metamorphosis Tributes Clayman for
/// the Guardian (Level 4 too), and team 1, controlling 2 monsters and a Set card, takes 600.
#[test]
fn clay_guardian_comes_with_metamorphosis_and_burns_200_for_each_card_the_opponent_controls() {
    let [guardian, metamorphosis, clayman] = ["Elemental HERO Clay Guardian (Anime)", "Metamorphosis", "Elemental Hero Clayman"].map(card);
    let spawns = [mon(0, clayman), hand(0, metamorphosis), mon(1, DARK_BLADE), mon(1, WARWOLF), set(1, card("Trap Hole"))];
    let Some(mut s) = field_extra([&[guardian], &[]], &spawns) else { return };
    s.run(&[Activate(metamorphosis)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", guardian), 1, "Clay Guardian is on the field: {}", s.context());
    assert_eq!(s.count(0, "grave", clayman), 1, "Clayman was Tributed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 600, "200 for each of the 3 cards: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    // Its name is Clayman's while it is on the field: Mud Max ("Equip only to a Clayman") can equip it.
    let max = card("Mud Max");
    let spawns = [mon(0, clayman), hand(0, metamorphosis), hand(0, max)];
    let Some(mut s) = field_extra([&[guardian], &[]], &spawns) else { return };
    s.run(&[Activate(metamorphosis), Activate(max)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", max), 1, "Mud Max is on Clay Guardian: {}", s.context());
    assert_eq!(s.count(0, "mzone", guardian), 1, "{}", s.context());
}

/// Elemental HERO Clay Guardian (Anime): it "can only be Special Summoned with Metamorphosis": it is never offered as a
/// Special Summon of its own.
#[test]
fn clay_guardian_cannot_be_special_summoned_on_its_own() {
    let [guardian, clayman] = ["Elemental HERO Clay Guardian (Anime)", "Elemental Hero Clayman"].map(card);
    let Some(mut s) = field_extra([&[guardian], &[]], &[mon(0, clayman), hand(0, POLYMERIZATION)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&guardian.into()), "{:?}", offered(&s, "special_summons"));
    assert_eq!(s.count(0, "mzone", guardian), 0);
}

/// Elemental Mirage: "When an "Elemental HERO" monster you control is destroyed by an opponent's card effect: Return all
/// monsters that you control that were destroyed by that effect to the field in the same battle positions they were in."
/// Team 1's Dark Hole destroys Elemental HERO Avian (Attack Position) and Warwolf (Defense Position) of team 0: both come
/// back in the positions they had. Team 1's own monster stays destroyed.
#[test]
fn elemental_mirage_returns_the_monsters_destroyed_by_the_effect_in_their_positions() {
    let [mirage, avian] = ["Elemental Mirage", "Elemental Hero Avian"].map(card);
    let spawns = [set(0, mirage), mon(0, avian), mon_def(0, WARWOLF), hand(1, DARK_HOLE), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(mirage)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", avian), 1, "Avian is back: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf is back: {}", s.context());
    assert_eq!(s.position_of(0, avian), Some(1), "Attack Position: {}", s.context());
    assert_eq!(s.position_of(0, WARWOLF), Some(4), "Defense Position: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "team 1's monster stays destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", mirage), 1, "{}", s.context());
}

/// Elemental Storm: "Target 1 face-up "Elemental HERO" monster you control. It cannot attack this turn. Destroy 1 monster
/// your opponent controls with an original ATK lower than the original ATK of the targeted monster." Elemental HERO Neos
/// (2500) as the target destroys Dark Blade (1800); Neos cannot attack that turn (Warwolf can). Against a monster of 2500
/// it is not offered.
#[test]
fn elemental_storm_destroys_a_weaker_monster_and_stops_the_target_attacking() {
    let [storm, neos] = ["Elemental Storm", "Elemental Hero Neos"].map(card);
    let Some(mut s) = field(&[], &[mon(0, neos), mon(0, WARWOLF), hand(0, storm), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(storm)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "Dark Blade was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", storm), 1, "{}", s.context());
    let err = s.run(&[Attack(neos)]).expect_err("Neos cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{WARWOLF}")) && !err.contains(&format!("\"cid\":{neos}")), "{err}");
    clean(&s);
    // Not offered against an equally strong monster: Dark Magician's 2500 is not lower than Neos' 2500.
    let Some(mut s) = field(&[], &[mon(0, neos), hand(0, storm), mon(1, DARK_MAGICIAN)]) else { return };
    let err = s.run(&[Activate(storm)]).expect_err("no weaker monster");
    assert!(err.contains("never became possible"), "{err}");
}

/// Feather Storm: "If you control a "Elemental HERO Avian": Switch the original ATK/DEF of all monsters your opponent
/// controls, until the End Phase." Warwolf (2000/100) is 100/2000 until the End Phase and Avian (1000) destroys it in
/// battle; Dark Blade (1800/1500) is 1500/1800 and back to normal the next turn.
#[test]
fn feather_storm_switches_the_original_atk_and_def_of_the_opponents_monsters() {
    let [storm, avian] = ["Feather Storm", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[], &[set(0, storm), mon(0, avian), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.run_both([&[EndTurn, Activate(storm)], &[EndTurn]]).unwrap();
    clean(&s);
    let (w, b) = (s.monster(1, WARWOLF).unwrap(), s.monster(1, DARK_BLADE).unwrap());
    assert_eq!((w["attack"].as_i64(), w["defense"].as_i64()), (Some(100), Some(2000)), "{}", s.context());
    assert_eq!((b["attack"].as_i64(), b["defense"].as_i64()), (Some(1500), Some(1800)), "{}", s.context());
    s.run(&[Attack(avian)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Avian (1000) beat Warwolf (100): {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let b = s.monster(1, DARK_BLADE).unwrap();
    assert_eq!((b["attack"].as_i64(), b["defense"].as_i64()), (Some(1800), Some(1500)), "back after the End Phase: {}", s.context());
}

/// Feather Storm needs Avian: without it the Trap is not offered.
#[test]
fn feather_storm_needs_elemental_hero_avian() {
    let storm = card("Feather Storm");
    let Some(mut s) = field(&[], &[set(0, storm), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    let err = s.run_both([&[EndTurn, Activate(storm)], &[EndTurn]]).expect_err("no Avian");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Hero Dice
// ---------------------------------------------------------------------------------------------------------------

/// The face of the die in a step's messages (MSG_TOSS_DICE, 131).
fn die_in(messages: &[serde_json::Value]) -> Option<u64> {
    messages.iter().find(|m| m["type"] == 131).and_then(|m| m["results"][0].as_u64())
}

/// Plays `plan` and then answers prompts until the die has been rolled, returning its face.
fn roll_after(s: &mut Scenario, plan: &[yo3_server::duelcheck::scenario::Action]) -> u64 {
    s.run(&[]).unwrap(); // to the first Main Phase prompt (the duel opens on a chain window)
    s.run_to(plan, |p| p["type"] != msg::SELECT_IDLECMD).unwrap();
    for _ in 0..30 {
        if let Some(face) = die_in(s.messages()) {
            return face;
        }
        s.pass().unwrap();
    }
    panic!("no die was rolled: {}", s.context());
}

/// Hero Dice: "Target 1 "Elemental HERO" monster you control; roll a six-sided die and apply an appropriate effect. 1:
/// Take damage equal to the targeted monster's ATK. 2: Destroy 1 Spell Card you control. 3: Destroy 1 monster you
/// control. 4: Destroy 1 monster your opponent controls. 5: Destroy 1 Spell Card your opponent controls. 6: The targeted
/// monster can attack directly this turn." Team 0 has Avian (the target, 1000 ATK) and Sparkman, team 1 Warwolf, and each
/// has a face-up Dian Keto. Across seeds every face is rolled (read from the engine's message) and each does what it
/// says: Avian's 1000 as damage, team 0's Dian Keto gone, a monster of team 0's gone (Avian: the choices take the first
/// card on offer), Warwolf gone, team 1's Dian Keto gone, and a direct attack for 1000 past Warwolf in turn 3.
#[test]
fn hero_dice_applies_each_result() {
    use yo3_server::duelcheck::scenario::Action;
    let [dice, avian, sparkman] = ["Hero Dice", "Elemental Hero Avian", "Elemental Hero Sparkman"].map(card);
    let mut seen = [false; 6];
    for seed in 1..200u64 {
        let spawns = [mon(0, avian), mon(0, sparkman), spell(0, DIAN_KETO), hand(0, dice), mon(1, WARWOLF), spell(1, DIAN_KETO)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        keep_hands(&mut s);
        s.prefer(&[(0, avian), (0, FILLER), (1, FILLER)]);
        let plan: [Action; 2] = [EndTurn, Activate(dice)];
        let face = {
            // Turn 3, so that the direct attack of the sixth face can be tried.
            s.run(&[]).unwrap();
            s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
            roll_after(&mut s, &plan[1..])
        };
        s.run(&[]).unwrap();
        clean(&s);
        let who = format!("seed {seed}, face {face}: {}", s.context());
        assert_eq!(s.count(0, "grave", dice), 1, "{who}");
        let (spells0, spells1) = (s.count(0, "szone", DIAN_KETO), s.count(1, "szone", DIAN_KETO));
        match face {
            1 => assert_eq!((s.lp(0), s.lp(1)), (7000, 8000), "{who}"),
            2 => assert_eq!((spells0, spells1, s.lp(0)), (0, 1, 8000), "{who}"),
            3 => assert_eq!((s.count(0, "grave", avian), s.count(0, "mzone", sparkman), s.count(1, "mzone", WARWOLF)), (1, 1, 1), "{who}"),
            4 => assert_eq!((s.count(1, "grave", WARWOLF), s.count(0, "mzone", avian), s.count(0, "mzone", sparkman)), (1, 1, 1), "{who}"),
            5 => assert_eq!((spells0, spells1), (1, 0), "{who}"),
            _ => {
                assert_eq!((s.lp(0), s.lp(1), s.count(0, "mzone", sparkman), s.count(1, "mzone", WARWOLF)), (8000, 8000, 1, 1), "nothing else happened: {who}");
                s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
                s.run_to(&[], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
                assert_eq!(s.prompt()["attacks"][0]["can_direct"], true, "{who}");
                s.run(&[Attack(avian)]).unwrap();
                clean(&s);
                assert_eq!(s.lp(1), 8000 - 1000, "Avian attacked directly past Warwolf: {}", s.context());
                assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
            }
        }
        seen[face as usize - 1] = true;
        if seen == [true; 6] {
            return;
        }
    }
    panic!("not every face was rolled: {seen:?}");
}

// ---------------------------------------------------------------------------------------------------------------
// Fusion Shot, Hero Shield, Hero's Backup, Hero's Guard
// ---------------------------------------------------------------------------------------------------------------

/// Fusion Shot: "Equip only to a Fusion Monster. You can banish 1 monster with 1000 or less ATK from your Graveyard that
/// was used in the equipped monster's Fusion Summon; inflict damage to your opponent equal to the banished monster's
/// ATK." Elemental HERO Flame Wingman is Fusion Summoned from Avian (1000 ATK) and Burstinatrix (1200 ATK); equipped, the
/// Shot banishes Avian (Burstinatrix is too strong) and team 1 takes 1000.
#[test]
fn fusion_shot_banishes_a_weak_material_for_damage_equal_to_its_atk() {
    let [shot, wingman, avian, burstinatrix] = ["Fusion Shot", "Elemental Hero Flame Wingman", "Elemental Hero Avian", "Elemental Hero Burstinatrix"].map(card);
    let spawns = [hand(0, POLYMERIZATION), hand(0, avian), hand(0, burstinatrix), hand(0, shot)];
    let Some(mut s) = field_extra([&[wingman], &[]], &spawns) else { return };
    s.run(&[Activate(POLYMERIZATION), Activate(shot), Activate(shot)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", wingman), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", avian), 1, "Avian was banished: {}", s.context());
    assert_eq!(s.count(0, "grave", burstinatrix), 1, "Burstinatrix (1200) is left: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    assert_eq!(s.count(0, "szone", shot), 1, "the equip card stays: {}", s.context());
}

/// Hero Shield: "Target 1 face-up "HERO" monster you control, and equip this card to it. If the equipped monster would be
/// destroyed by battle, you can send this face-up card to the Graveyard instead. During damage calculation, if the
/// equipped monster battles an opponent's monster, you can draw 1 card for each multiple of 1000 Battle Damage you would
/// take." Team 1's Warwolf (2000) attacks Avian (1000); Hero Shield is chained to the attack, equipped to Avian, takes
/// the destruction in its place, and draws 1 card for the 1000 damage.
#[test]
fn hero_shield_saves_the_equipped_hero_and_draws_for_the_battle_damage() {
    let [shield, avian] = ["Hero Shield", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[], &[set(0, shield), mon(0, avian), mon(1, WARWOLF)]) else { return };
    let before = hand_size(&s, 0);
    s.run_both([&[ChainOnAttack(shield), Chain(shield)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", avian), 1, "Avian was not destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", shield), 1, "the Shield went in its place: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "one card for the 1000 damage: {}", s.context());
}

/// Hero's Backup: "During the Battle Phase: Target 1 monster you control. If the targeted monster battles a monster your
/// opponent controls whose ATK is 1000 or more higher than the targeted monster's, remove from play 1 "Elemental HERO"
/// monster from your Graveyard to have the targeted monster gain ATK equal to the removed monster's ATK." Avian (1000)
/// attacks Warwolf (2000); with Sparkman (1600) removed from the Graveyard Avian is 2600 and destroys it.
#[test]
fn heros_backup_gives_the_atk_of_the_banished_hero() {
    let [backup, avian, sparkman] = ["Hero's Backup", "Elemental Hero Avian", "Elemental Hero Sparkman"].map(card);
    let Some(mut s) = field(&[], &[set(0, backup), mon(0, avian), grave(0, sparkman), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Attack(avian), ChainOnAttack(backup)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", sparkman), 1, "Sparkman was banished: {}", s.context());
    assert_eq!(s.attack_of(0, avian), Some(2600), "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 600, "{}", s.context());
}

/// Hero's Backup: "ATK is 1000 or more higher": against a monster only 500 higher it is not offered.
#[test]
fn heros_backup_needs_an_opponent_1000_stronger() {
    let [backup, avian, sparkman] = ["Hero's Backup", "Elemental Hero Avian", "Elemental Hero Sparkman"].map(card);
    let Some(mut s) = field(&[], &[set(0, backup), mon(0, avian), grave(0, sparkman), mon(1, THUNDER_KID)]) else { return };
    // Thunder Kid is 700: no stronger than Avian at all.
    let err = s.run_both([&[EndTurn, Attack(avian), ChainOnAttack(backup)], &[EndTurn]]).expect_err("not offered");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", backup), 1, "not activated: {}", s.context());
    assert_eq!(s.count(0, "grave", sparkman), 1, "{}", s.context());
}

/// Hero's Guard: "When an opponent's monster declares an attack: "HERO" monsters you control cannot be destroyed by
/// battle by this battle. Also, you can banish 1 "HERO" monster in your Graveyard to halve the Battle Damage you take
/// from this battle and draw 1 card." Warwolf (2000) attacks Avian (1000): Avian survives, Sparkman is banished from the
/// Graveyard, the damage is 500 and team 0 draws 1.
#[test]
fn heros_guard_protects_the_hero_halves_the_damage_and_draws() {
    let [guard, avian, sparkman] = ["Hero's Guard", "Elemental Hero Avian", "Elemental Hero Sparkman"].map(card);
    let Some(mut s) = field(&[], &[set(0, guard), mon(0, avian), grave(0, sparkman), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, sparkman)]);
    let before = hand_size(&s, 0);
    s.run_both([&[ChainOnAttack(guard)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", avian), 1, "Avian survived the battle: {}", s.context());
    assert_eq!(s.count(0, "removed", sparkman), 1, "Sparkman was banished: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "half of 1000: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Courageous Charge, Spy Hero, Impact Flip
// ---------------------------------------------------------------------------------------------------------------

/// Courageous Charge: "Pay 1000 LP, then target 1 monster you control with 1000 or less ATK; when that monster inflicts
/// battle damage to your opponent this turn, inflict damage to your opponent equal to the DEF of that monster." Thunder
/// Kid (700/600) attacks directly in turn 3: 700 of battle damage and 600 more of effect damage; the 1000 LP was paid.
#[test]
fn courageous_charge_adds_the_defense_as_damage_when_the_monster_hits() {
    let charge = card("Courageous Charge");
    let Some(mut s) = field(&[], &[hand(0, charge), mon(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(charge), Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000, "1000 LP paid: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 700 - 600, "700 battle damage and 600 (its DEF): {}", s.context());
}

/// Courageous Charge, the target answered: Thunder Kid, the target, is destroyed by team 1's Raigeki Break in response.
/// The effect finds no target when it resolves (upstream's script then stops on a Lua error,
/// `engine/overrides/c511000999.lua` checks).
#[test]
fn courageous_charge_with_its_target_destroyed_in_answer_does_nothing() {
    let charge = card("Courageous Charge");
    let Some(mut s) = field(&[], &[hand(0, charge), mon(0, THUNDER_KID), mon(0, WARWOLF), set(1, RAIGEKI_BREAK)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (0, FILLER), (1, FILLER)]);
    answer_with(&mut s, &[Activate(charge)], 1, RAIGEKI_BREAK);
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the target was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", charge), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000, "{}", s.context());
}

/// Spy Hero: "Send 2 random cards from your Deck to the Graveyard, then target 1 Spell in your opponent's Graveyard; add
/// it to your hand. You can use it as your hand until the end of this turn." Dian Keto comes from team 1's Graveyard into
/// team 0's hand and is used (1000 LP, and it ends in its owner's Graveyard); when it is not used it goes back to team
/// 1's Graveyard at the End Phase.
#[test]
fn spy_hero_takes_a_spell_from_the_opponents_graveyard_for_the_turn() {
    let spy = card("Spy Hero");
    let spawns = [hand(0, spy), grave(1, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Activate(spy), Activate(DIAN_KETO)]).unwrap();
    clean(&s);
    assert_eq!(deck_size(&s, 0), deck - 2, "2 cards from the Deck: {}", s.context());
    assert_eq!(s.lp(0), 9000, "Dian Keto was used from the hand: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "and went to its owner's Graveyard: {}", s.context());
    // Not used: back in the opponent's Graveyard at the End Phase.
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(spy)]).unwrap();
    assert_eq!(s.count(0, "hand", DIAN_KETO), 1, "in my hand for the turn: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 0, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "back where it came from: {}", s.context());
    assert_eq!(s.count(0, "hand", DIAN_KETO), 0, "{}", s.context());
}

/// Impact Flip: "Once per turn, you can flip 1 face-down Defense Position monster your opponent controls face-up. (Flip
/// Effects are not activated at this time). The equipped monster cannot battle the turn you use this effect." Team 1's
/// Set Morphing Jar is turned face-up Defense without its flip effect (the hands are untouched), and the equipped Warwolf
/// cannot attack that turn (Dark Blade can).
#[test]
fn impact_flip_turns_a_monster_face_up_without_its_flip_effect_and_stops_the_equipped_attacking() {
    let [flip, jar] = ["Impact Flip", "Morphing Jar"].map(card);
    let Some(mut s) = field(&[], &[hand(0, flip), mon(0, WARWOLF), mon(0, DARK_BLADE), mon_set(1, jar)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(flip)], &[EndTurn]]).unwrap();
    let hands = (hand_size(&s, 0), hand_size(&s, 1));
    s.run(&[Activate(flip)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, jar), Some(4), "face-up Defense: {}", s.context());
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), hands, "Morphing Jar's flip effect was not activated: {}", s.context());
    let err = s.run(&[Attack(WARWOLF)]).expect_err("the equipped monster cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{DARK_BLADE}")) && !err.contains(&format!("\"cid\":{WARWOLF}")), "{err}");
}

/// Impact Flip: "When the equipped monster destroys an opponent's monster by battle, send the top card of your opponent's
/// Deck to the Graveyard." Warwolf (2000) destroys Thunder Kid: team 1's Deck is 1 card smaller and its Graveyard has
/// Thunder Kid and that card.
#[test]
fn impact_flip_sends_the_top_card_of_the_opponents_deck_when_the_equipped_monster_destroys_one() {
    let [flip] = ["Impact Flip"].map(card);
    let Some(mut s) = field(&[], &[hand(0, flip), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(flip)], &[EndTurn]]).unwrap();
    let (deck, grave_before) = (deck_size(&s, 1), s.state()[1]["grave"].as_array().unwrap().len());
    s.run(&[Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(deck_size(&s, 1), deck - 1, "{}", s.context());
    assert_eq!(s.state()[1]["grave"].as_array().unwrap().len(), grave_before + 2, "Thunder Kid and the top card: {}", s.context());
}

/// Impact Flip: "When this card is sent to the Graveyard, both players draw 1 card." Team 1's Mystical Space Typhoon
/// destroys it: team 0 draws 1, and team 1 draws 1 after spending the Typhoon (so its hand holds its turn's draw and no
/// more).
#[test]
fn impact_flip_makes_both_players_draw_when_it_goes_to_the_graveyard() {
    let [flip, typhoon] = ["Impact Flip", "Mystical Space Typhoon"].map(card);
    let Some(mut s) = field(&[], &[hand(0, flip), mon(0, WARWOLF), hand(1, typhoon)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, flip), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(flip)]).unwrap();
    let hands = (hand_size(&s, 0), hand_size(&s, 1));
    s.run_both([&[EndTurn], &[Activate(typhoon)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", flip), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), hands.0 + 1, "{}", s.context());
    assert_eq!(hand_size(&s, 1), hands.1 + 1, "its turn's draw, the Typhoon out and 1 card in: {}", s.context());
}

/// Impact Flip, the target answered: team 1's Raigeki Break destroys the Set monster in response to the flip, which finds
/// no target when it resolves (upstream's script used the target without a check; the core hands it over nonetheless, so
/// no error).
#[test]
fn impact_flip_with_its_target_destroyed_in_answer_does_nothing() {
    let [flip, jar] = ["Impact Flip", "Morphing Jar"].map(card);
    let Some(mut s) = field(&[], &[hand(0, flip), mon(0, WARWOLF), mon_set(1, jar), set(1, RAIGEKI_BREAK)]) else { return };
    s.prefer(&[(1, jar), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(flip)]).unwrap();
    answer_with(&mut s, &[Activate(flip)], 1, RAIGEKI_BREAK);
    clean(&s);
    assert_eq!(s.count(1, "grave", jar), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", flip), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Hope for the Future, Mud Max, Neospace Road, Relationship, Tag Request
// ---------------------------------------------------------------------------------------------------------------

/// Hope for the Future (Anime): "While you control an "Elemental HERO Neos": Discard 1 "Polymerization"; send, from your
/// Deck to your Graveyard, as many Fusion Materials listed on a "Neos" Fusion monster in your Extra Deck as possible."
/// With Elemental HERO Flare Neos in the Extra Deck (Neos and Neo-Spacian Flare Scarab) and Flare Scarab, a second Neos
/// and Aqua Dolphin in the Deck, Polymerization is discarded and Scarab and the second Neos go to the Graveyard; Aqua
/// Dolphin (no Neos Fusion Monster lists it) stays in the Deck.
#[test]
fn hope_for_the_future_sends_the_fusion_materials_from_the_deck() {
    let [hope, neos, flare_neos, scarab, dolphin] =
        ["Hope for the Future (Anime)", "Elemental Hero Neos", "Elemental Hero Flare Neos", "Neo-Spacian Flare Scarab", "Neo-Spacian Aqua Dolphin"].map(card);
    let spawns = [set(0, hope), mon(0, neos), hand(0, POLYMERIZATION)];
    let dir = engine_dir();
    let Some(dir) = dir else { return };
    let mine: Vec<u16> = [scarab, neos, dolphin].into_iter().chain(std::iter::repeat_n(FILLER, 37)).collect();
    let decks = [(&mine[..], &[flare_neos][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(hope)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", POLYMERIZATION), 1, "Polymerization was discarded: {}", s.context());
    assert_eq!(s.count(0, "grave", scarab), 1, "Flare Scarab was sent: {}", s.context());
    assert_eq!(s.count(0, "grave", neos), 1, "the second Neos was sent: {}", s.context());
    assert_eq!(s.count(0, "grave", dolphin), 0, "Aqua Dolphin stays in the Deck: {}", s.context());
    assert_eq!(s.count(0, "mzone", neos), 1, "{}", s.context());
}

/// Hope for the Future needs a Neos on the field (the Trap is not offered without) and a Polymerization to discard.
#[test]
fn hope_for_the_future_needs_neos_and_a_polymerization() {
    let [hope, neos, flare_neos, scarab] = ["Hope for the Future (Anime)", "Elemental Hero Neos", "Elemental Hero Flare Neos", "Neo-Spacian Flare Scarab"].map(card);
    let dir = engine_dir();
    let Some(dir) = dir else { return };
    let mine: Vec<u16> = [scarab].into_iter().chain(std::iter::repeat_n(FILLER, 39)).collect();
    for spawns in [[set(0, hope), hand(0, POLYMERIZATION), mon(0, WARWOLF)], [set(0, hope), mon(0, neos), hand(0, FILLER)]] {
        let decks = [(&mine[..], &[flare_neos][..]), (&[FILLER; 40][..], &[][..])];
        let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
        keep_hands(&mut s);
        let err = s.run_both([&[EndTurn, Activate(hope)], &[EndTurn]]).expect_err("not possible");
        assert!(err.contains("never became possible"), "{err}");
    }
}

/// Mud Max: "Equip only to a "Elemental HERO Clayman". It gains 300 ATK. When your opponent's monster declares an attack:
/// You can Tribute this card; Special Summon 1 Level 4 or lower "Elemental HERO" monster from your hand, and if you do,
/// negate that attack." Clayman (800) with Mud Max is 1100; when Warwolf attacks, Mud Max is Tributed, Elemental HERO
/// Avian comes from the hand and the attack is negated: no damage, Warwolf stays and Clayman is not destroyed.
#[test]
fn mud_max_gives_300_atk_and_summons_a_hero_to_negate_an_attack() {
    let [max, clayman, avian] = ["Mud Max", "Elemental Hero Clayman", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[], &[mon(0, clayman), hand(0, max), hand(0, avian), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let base = s.attack_of(0, clayman).unwrap();
    s.run(&[Activate(max)]).unwrap();
    assert_eq!(s.attack_of(0, clayman), Some(base + 300), "{}", s.context());
    s.run_both([&[EndTurn, Chain(max)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", max), 1, "Mud Max was Tributed: {}", s.context());
    assert_eq!(s.count(0, "mzone", avian), 1, "Avian came from the hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", clayman), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
}

/// Neospace Road: "When an "Elemental HERO Neos" you control is destroyed by battle and sent to the Graveyard: Draw 1
/// card, also, end the Battle Phase." Team 1's Blue-Eyes White Dragon destroys Neos (500 damage); team 0 draws 1 and team
/// 1's Dark Blade has no attack to follow with: the Battle Phase ended, where without the Trap team 1 is back at its
/// Battle Phase prompt.
#[test]
fn neospace_road_draws_and_ends_the_battle_phase() {
    let [road, neos, dragon] = ["Neospace Road", "Elemental Hero Neos", "Blue-Eyes White Dragon"].map(card);
    let spawns = [set(0, road), mon(0, neos), mon(1, dragon), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run_both([&[Chain(road)], &[Attack(dragon)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", neos), 1, "Neos was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", road), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
    assert_eq!(s.prompt()["type"].as_u64(), Some(msg::SELECT_IDLECMD), "the Battle Phase is over: {}", s.context());
    // The control: without the Trap the Battle Phase goes on.
    let Some(mut s) = field(&[], &[mon(0, neos), mon(1, dragon), mon(1, DARK_BLADE)]) else { return };
    s.run_both([&[], &[Attack(dragon)]]).unwrap();
    assert_eq!(s.prompt()["type"].as_u64(), Some(msg::SELECT_BATTLECMD), "{}", s.context());
}

/// Relationship: "Activate only if a monster(s) you controlled was destroyed this turn: Special Summon 1 "Elemental HERO"
/// monster with ATK equal to the ATK 1 of those destroyed monsters had on the field from your Deck in Defense Position."
/// Elemental HERO Sparkman (1600) is destroyed by Dark Hole; a second Sparkman comes from the Deck in Defense Position.
/// With only Avian (1000) in the Deck it is not offered, nor before anything of mine was destroyed.
#[test]
fn relationship_summons_a_hero_with_the_atk_of_the_destroyed_monster() {
    let [relationship, sparkman, avian] = ["Relationship", "Elemental Hero Sparkman", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[sparkman], &[mon(0, sparkman), hand(0, DARK_HOLE), hand(0, relationship)]) else { return };
    s.run(&[Activate(DARK_HOLE), Activate(relationship)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", sparkman), 1, "the second Sparkman came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", sparkman), 1, "{}", s.context());
    assert_eq!(s.position_of(0, sparkman), Some(4), "in Defense Position: {}", s.context());
    // Another ATK in the Deck: not offered.
    let Some(mut s) = field(&[avian], &[mon(0, sparkman), hand(0, DARK_HOLE), hand(0, relationship)]) else { return };
    let err = s.run(&[Activate(DARK_HOLE), Activate(relationship)]).expect_err("no HERO of that ATK");
    assert!(err.contains("never became possible"), "{err}");
    // Nothing of mine destroyed: not offered either.
    let Some(mut s) = field(&[sparkman], &[hand(0, relationship)]) else { return };
    let err = s.run(&[Activate(relationship)]).expect_err("nothing destroyed");
    assert!(err.contains("never became possible"), "{err}");
}

/// Tag Request: "Target 1 face-up monster you control; Special Summon 1 "Elemental HERO" monster from your hand with an
/// ATK equal to that monster's." Clayman (800) is the target and Elemental HERO Bubbleman (800) comes from the hand; a
/// hand of Avian (1000) only does not make it possible.
#[test]
fn tag_request_summons_a_hero_of_equal_atk_from_the_hand() {
    let [request, clayman, bubbleman, avian] = ["Tag Request", "Elemental Hero Clayman", "Elemental Hero Bubbleman", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[], &[mon(0, clayman), hand(0, request), hand(0, bubbleman), hand(0, avian)]) else { return };
    s.run(&[Activate(request)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", bubbleman), 1, "Bubbleman has Clayman's 800: {}", s.context());
    assert_eq!(s.count(0, "hand", avian), 1, "Avian (1000) stays: {}", s.context());
    assert_eq!(s.count(0, "mzone", clayman), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, clayman), hand(0, request), hand(0, avian)]) else { return };
    let err = s.run(&[Activate(request)]).expect_err("no equal ATK");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// The signature cards: Neos Energy, Neo Signal, Astral Shift, Take Over 5, Neospace Wave
// ---------------------------------------------------------------------------------------------------------------

/// Neos Energy: "Equip only to a "Neo-Spacian" monster. It gains 800 ATK. Once per turn, during your Standby Phase: It
/// loses 300 ATK." Aqua Dolphin (600) is 1400 when equipped, 1100 after the first of its controller's Standby Phases and
/// 800 after the second (the loss grows: 300, then 600 in all). Warwolf, no Neo-Spacian, cannot take it.
#[test]
fn neos_energy_gives_800_atk_and_takes_300_more_each_standby_phase() {
    let [energy, dolphin] = ["Neos Energy", "Neo-Spacian Aqua Dolphin"].map(card);
    let Some(mut s) = field(&[], &[mon(0, dolphin), hand(0, energy)]) else { return };
    keep_hands(&mut s);
    assert_eq!(s.attack_of(0, dolphin), Some(600));
    s.run(&[Activate(energy)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, dolphin), Some(1400), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, dolphin), Some(1100), "turn 3's Standby Phase: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, dolphin), Some(800), "turn 5's: {}", s.context());
    clean(&s);
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), hand(0, energy)]) else { return };
    let err = s.run(&[Activate(energy)]).expect_err("no Neo-Spacian");
    assert!(err.contains("never became possible"), "{err}");
}

/// Neo Signal: "When a monster you control is destroyed and sent to the Graveyard: Special Summon 1 Level 4 or lower
/// "Neo-Spacian" monster from your hand or Deck." Team 1's Dark Hole destroys Warwolf; Aqua Dolphin comes from team 0's
/// Deck (the bottom).
#[test]
fn neo_signal_summons_a_neo_spacian_when_a_monster_is_destroyed() {
    let [signal, dolphin] = ["Neo Signal", "Neo-Spacian Aqua Dolphin"].map(card);
    let Some(mut s) = field(&[dolphin], &[set(0, signal), mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(signal)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", dolphin), 1, "Aqua Dolphin came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", signal), 1, "{}", s.context());
}

/// Astral Shift: "When a monster you control is targeted for an attack: It becomes a direct attack. After damage
/// calculation, draw 1 card." Warwolf (2000) attacks Avian (1000): chained to the attack the Trap makes it direct, so
/// team 0 takes 2000, Avian is not destroyed, and team 0 draws 1 card.
#[test]
fn astral_shift_makes_the_attack_direct_and_draws_after_damage_calculation() {
    let [shift, avian] = ["Astral Shift", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[], &[set(0, shift), mon(0, avian), mon(1, WARWOLF)]) else { return };
    let before = hand_size(&s, 0);
    s.run_both([&[ChainOnAttack(shift)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "a direct attack: {}", s.context());
    assert_eq!(s.count(0, "mzone", avian), 1, "Avian was not attacked: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "one card after damage calculation: {}", s.context());
}

/// Take Over 5: "Send the top 5 cards of your Deck to the Graveyard. During your Standby Phase, if this card is in your
/// Graveyard: You can banish 1 card with the same name as this card from your hand, Deck, or Graveyard; draw 1 card.
/// While this card is in your Graveyard, negate your card effects that would send cards from the Deck to the Graveyard."
/// The Deck loses 5 cards and the Graveyard gains them and Take Over 5.
#[test]
fn take_over_5_sends_the_top_five_cards_of_the_deck_to_the_graveyard() {
    let take_over = card("Take Over 5");
    let Some(mut s) = field(&[], &[hand(0, take_over)]) else { return };
    let (deck, grave_before) = (deck_size(&s, 0), s.state()[0]["grave"].as_array().unwrap().len());
    s.run(&[Activate(take_over)]).unwrap();
    clean(&s);
    assert_eq!(deck_size(&s, 0), deck - 5, "{}", s.context());
    assert_eq!(s.state()[0]["grave"].as_array().unwrap().len(), grave_before + 6, "5 cards and Take Over 5: {}", s.context());
    assert_eq!(deck_size(&s, 1), 35, "the opponent's Deck is untouched: {}", s.context());
}

/// Take Over 5, in the Graveyard: "negate your card effects that would send cards from the Deck to the Graveyard". Hero's
/// Guild (each player sends the top card of their Deck) is used after Take Over 5 has gone to the Graveyard: its effect
/// is negated and no Deck loses more than Take Over 5's own 5 cards; used without it, both Decks lose 1.
#[test]
fn take_over_5_in_the_graveyard_negates_the_owners_deck_to_graveyard_effects() {
    let [take_over, guild] = ["Take Over 5", "Hero's Guild"].map(card);
    let Some(mut s) = field(&[], &[hand(0, take_over), hand(0, guild)]) else { return };
    let decks = (deck_size(&s, 0), deck_size(&s, 1));
    s.run(&[Activate(take_over), Activate(guild), Activate(guild)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", take_over), 1, "{}", s.context());
    assert_eq!((deck_size(&s, 0), deck_size(&s, 1)), (decks.0 - 5, decks.1), "Hero's Guild did nothing: {}", s.context());
    // The control: with no Take Over 5 the effect sends one card from each Deck.
    let Some(mut s) = field(&[], &[hand(0, guild)]) else { return };
    let decks = (deck_size(&s, 0), deck_size(&s, 1));
    s.run(&[Activate(guild), Activate(guild)]).unwrap();
    clean(&s);
    assert_eq!((deck_size(&s, 0), deck_size(&s, 1)), (decks.0 - 1, decks.1 - 1), "{}", s.context());
}

/// Take Over 5, in the Graveyard: "During your Standby Phase: You can banish 1 card with the same name as this card from
/// your hand, Deck, or Graveyard; draw 1 card." With a lone copy in the Graveyard, at the first Standby Phase it banishes
/// itself and team 0 draws a card (a hand of one more than a duel without the copy).
#[test]
fn take_over_5_in_the_graveyard_banishes_a_copy_and_draws_at_the_standby_phase() {
    let take_over = card("Take Over 5");
    let Some(mut base) = field(&[], &[]) else { return };
    base.run(&[]).unwrap();
    let Some(mut s) = field(&[], &[grave(0, take_over)]) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", take_over), 1, "the copy was banished as the cost: {}", s.context());
    assert_eq!(hand_size(&s, 0), hand_size(&base, 0) + 1, "and 1 card was drawn: {}", s.context());
}

/// Neospace Wave: "When you have more "Neo-Spacian" monsters in your Deck than the number of monsters you control: Send
/// all monsters you control to the Graveyard, and if you do, Special Summon random "Neo-Spacian" monsters from your Deck,
/// equal to the number sent." Warwolf goes to the Graveyard and one of the two Neo-Spacians in the Deck (Aqua Dolphin,
/// Flare Scarab) takes its place. With only one in the Deck against one monster it is not offered.
#[test]
fn neospace_wave_trades_each_monster_for_a_random_neo_spacian() {
    let [wave, dolphin, scarab] = ["Neospace Wave", "Neo-Spacian Aqua Dolphin", "Neo-Spacian Flare Scarab"].map(card);
    let Some(mut s) = field(&[dolphin, scarab], &[mon(0, WARWOLF), hand(0, wave)]) else { return };
    s.run(&[Activate(wave)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(monsters(&s, 0), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", dolphin) + s.count(0, "mzone", scarab), 1, "a Neo-Spacian: {}", s.context());
    let Some(mut s) = field(&[dolphin], &[mon(0, WARWOLF), hand(0, wave)]) else { return };
    let err = s.run(&[Activate(wave)]).expect_err("not more Neo-Spacians than monsters");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// A seeded sample of the rest: Battle x 2, Climax Hour, Cyclone Wing, Damage Off Zone, Hero Call, Hero's Guild, Skill Shock
// ---------------------------------------------------------------------------------------------------------------

/// Battle x 2: "During the Battle Step, when a monster you control declares an attack, and the attack target's ATK is
/// higher than or equal to twice the ATK of your attacking monster: Double your monster's original ATK during damage
/// calculation only." Avian (1000) attacks Warwolf (2000): doubled to 2000 it ties, so both are destroyed and nobody
/// takes damage, where without the Spell only Avian falls and team 0 takes 1000. Against Dark Blade (1800 < 2 x 1000) it
/// is not offered.
#[test]
fn battle_x_2_doubles_the_attacking_monsters_atk_for_the_damage_calculation() {
    let [battle, avian] = ["Battle x 2", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[], &[hand(0, battle), mon(0, avian), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(avian), ChainOnAttack(battle)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", battle), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", avian), s.count(1, "grave", WARWOLF)), (1, 1), "2000 against 2000: both fall: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, battle), mon(0, avian), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(avian)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", avian), s.count(1, "mzone", WARWOLF), s.lp(0)), (1, 1, 7000), "the control: {}", s.context());
    // Not offered against a target below twice the ATK.
    let Some(mut s) = field(&[], &[hand(0, battle), mon(0, avian), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(avian)], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "hand", battle), 1, "{}", s.context());
}

/// Climax Hour: "During your 3rd turn (counting your Draw Phases starting at your next turn) after this card's
/// activation, during that Draw Phase, if you draw a monster, Special Summon it, ignoring its Summoning conditions."
/// Activated in turn 1, nothing happens at the draws of turns 3 and 5 (Feral Imps go to the hand) and at turn 7's the
/// monster drawn is on the field.
#[test]
fn climax_hour_summons_the_monster_drawn_in_the_third_draw_phase_after_it() {
    let climax = card("Climax Hour");
    let Some(mut s) = field(&[], &[hand(0, climax)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(climax)]).unwrap();
    clean(&s);
    for turn in [3, 5] {
        s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
        assert_eq!(monsters(&s, 0), 0, "turn {turn}: nothing yet: {}", s.context());
    }
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", FILLER), 1, "turn 7: the drawn Feral Imp was Special Summoned: {}", s.context());
}

/// Cyclone Wing: "When the equipped monster declares an attack: You can target 1 Spell/Trap your opponent controls;
/// destroy it." Warwolf equipped with it attacks in turn 3 and team 1's Set Trap Hole is destroyed; a different monster's
/// attack does not set it off.
#[test]
fn cyclone_wing_destroys_a_spell_or_trap_when_the_equipped_monster_attacks() {
    let [wing, trap_hole] = ["Cyclone Wing", "Trap Hole"].map(card);
    let Some(mut s) = field(&[], &[hand(0, wing), mon(0, WARWOLF), mon(0, DARK_BLADE), set(1, trap_hole)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[Activate(wing), EndTurn, Attack(WARWOLF), Chain(wing)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", trap_hole), 1, "the Set Trap was destroyed: {}", s.context());
    assert_eq!(s.count(0, "szone", wing), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "Warwolf's attack went through: {}", s.context());
    // The unequipped Dark Blade attacking does not set it off.
    let Some(mut s) = field(&[], &[hand(0, wing), mon(0, WARWOLF), mon(0, DARK_BLADE), set(1, trap_hole)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[Activate(wing), EndTurn, Attack(DARK_BLADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", trap_hole), 1, "{}", s.context());
}

/// Damage Off Zone: "Halve the battle damage both player takes. During damage calculation, if a player(s) would take
/// battle damage: You can pay half your LP and send this card to the Graveyard; that damage becomes 0." Team 1 has the
/// Zone face-up; Warwolf's direct attack in turn 3 asks team 1 whether to use the second effect. Declined, the damage is
/// 1000 (half of 2000) and the Zone stays; accepted, team 1 pays 4000 LP, the Zone goes to the Graveyard and there is no
/// damage.
#[test]
fn damage_off_zone_halves_battle_damage_or_negates_it_for_half_the_lp() {
    let zone = card("Damage Off Zone");
    for (accept, lp) in [(false, 8000 - 1000), (true, 8000 / 2)] {
        let Some(mut s) = field(&[], &[mon(0, WARWOLF), spell(1, zone)]) else { return };
        keep_hands(&mut s);
        s.run_to(&[EndTurn, Attack(WARWOLF)], |p| p["type"] == msg::SELECT_EFFECTYN && p["player"] == 1).unwrap();
        s.respond(Response::yes(yo3_server::ocg::resp::EFFECTYN, accept).json()).unwrap();
        s.run(&[]).unwrap();
        clean(&s);
        assert_eq!(s.lp(1), lp, "accepted: {accept}: {}", s.context());
        assert_eq!(s.lp(0), 8000, "{}", s.context());
        assert_eq!((s.count(1, "szone", zone), s.count(1, "grave", zone)), if accept { (0, 1) } else { (1, 0) }, "{}", s.context());
    }
}

/// Hero Call: "Place 1 monster in your Deck to the top of your Deck." Avian (at the bottom of the Deck) is placed on top,
/// so team 0 draws it at its next Draw Phase (turn 3).
#[test]
fn hero_call_puts_a_monster_of_the_deck_on_top() {
    let [call, avian] = ["Hero Call", "Elemental Hero Avian"].map(card);
    let Some(mut s) = field(&[avian], &[set(0, call)]) else { return };
    s.prefer(&[(0, avian), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(call)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", call), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", avian), 0, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", avian), 1, "drawn at turn 3: {}", s.context());
}

/// Hero's Guild: "Once per turn: You can have both players send the top card of their Decks to the Graveyard, then if a
/// Warrior-Type monster is sent to the Graveyard this way, the player who sent it can add it to their hand. If a card is
/// added to a player's hand by this effect, destroy this card." With a Deck of Dark Blades (Warriors) team 0 sends one
/// and takes it back: the Guild is destroyed, and team 1's Feral Imp (a Fiend) stays in its Graveyard. With Decks of
/// Feral Imps both only lose a card and the Guild stays.
#[test]
fn heros_guild_sends_both_top_cards_and_returns_a_warrior_destroying_itself() {
    let guild = card("Hero's Guild");
    let Some(mut s) = duel(7, 8000, [DARK_BLADE, FILLER], &[], &[hand(0, guild)]) else { return };
    let (hand0, decks) = (hand_size(&s, 0), (deck_size(&s, 0), deck_size(&s, 1)));
    s.run(&[Activate(guild), Activate(guild)]).unwrap();
    clean(&s);
    assert_eq!((deck_size(&s, 0), deck_size(&s, 1)), (decks.0 - 1, decks.1 - 1), "{}", s.context());
    assert_eq!(hand_size(&s, 0), hand0 - 1 + 1, "the Warrior is back in the hand (the Guild left it): {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", guild), 1, "the Guild was destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, guild)]) else { return };
    s.run(&[Activate(guild), Activate(guild)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", guild), 1, "no Warrior: the Guild stays: {}", s.context());
    assert_eq!((s.count(0, "grave", FILLER), s.count(1, "grave", FILLER)), (1, 1), "{}", s.context());
}

/// Skill Shock: "When your opponent activates an Effect Monster's effect: Tribute 1 monster; Negate that effect. That
/// monster cannot attack or be Tributed this turn." Team 1 flips its Set Morphing Jar; team 0 Tributes Warwolf and the
/// flip effect (both hands to the Graveyard, 5 cards drawn) is negated; the Jar cannot attack that turn (Dark Blade can).
#[test]
fn skill_shock_tributes_a_monster_to_negate_an_effect_monsters_effect() {
    let [shock, jar] = ["Skill Shock", "Morphing Jar"].map(card);
    let spawns = [set(0, shock), mon(0, WARWOLF), mon_set(1, jar), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let hands = (hand_size(&s, 0), hand_size(&s, 1));
    s.run_both([&[Chain(shock)], &[Reposition(jar), Attack(jar)]]).expect_err("the Jar cannot attack");
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", shock), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", jar), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), hands.0, "Morphing Jar's flip effect was negated: {}", s.context());
    assert_eq!(hand_size(&s, 1), hands.1 + 1, "(team 1 only drew for its turn): {}", s.context());
}

/// Wild Half: the Half Token has "the same ... effect(s) as the targeted monster". Neo-Spacian Flare Scarab ("gains 400
/// ATK for each Spell/Trap your opponent controls") is the target while team 0 controls a Spell: with Wild Half itself on
/// the field it has 500 + 2 x 400 = 1300 ATK, halved to 650; the Half Token comes with that 650 as its ATK and, with the
/// Scarab's effect copied, gains 400 for the one Spell team 0 still controls (1050).
#[test]
fn the_half_token_has_the_effects_of_the_targeted_monster() {
    let [wild_half, wildheart, scarab, token] = ["Wild Half", "Elemental Hero Wildheart", "Neo-Spacian Flare Scarab", "Half Token"].map(card);
    let Some(mut s) = field(&[], &[mon(0, wildheart), hand(0, wild_half), spell(0, DIAN_KETO), mon(1, scarab)]) else { return };
    assert_eq!(s.attack_of(1, scarab), Some(500 + 400), "400 for the Spell: {}", s.context());
    s.run(&[Activate(wild_half)]).unwrap();
    clean(&s);
    let (target, t) = (s.monster(1, scarab).unwrap().clone(), s.monster(1, token).unwrap().clone());
    assert_eq!(target["attack"].as_i64(), Some(650), "half of 1300: {}", s.context());
    assert_eq!(t["base_attack"].as_i64(), Some(650), "the Token has the target's ATK: {}", s.context());
    assert_eq!(t["attack"].as_i64(), Some(650 + 400), "and its effect: 400 for team 0's Spell: {}", s.context());
}
