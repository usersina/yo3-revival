//! Anime-only cards, group 29 (rows 6151-6179 of engine/added/cards.tsv): Z-one's, Atticus Rhodes', Crow Hogan's, Ghost's,
//! Jack Atlas' and Marco's cards (and the 5D's duelists after them down to Vizor). These cards exist only in the anime, so
//! every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core with both
//! seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap is Set
//! first (a Trap cannot be activated from the hand, except Foolish Decision and Magician's Adage: "from your hand during
//! your turn"), and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3. A Trap given to a
//! team's plan with `Chain` is used at the first chance it is offered, so a test that needs a later one starts the plan
//! with `EndTurn` or steps the prompts itself.
//! Covered: the cards the per-card fuzz flagged (Offensive Guard, Wise A3, Smile Kid, Dromi the Sacred Shackles, Raging
//! Sacred Curse, Doom Ray: it could not reach them, they need a direct attack, a Wisel Attack beside a Wisel Attack 3 and a
//! Meklord Emperor, a Divine-Beast or a Gate Blocker's partner), the script notes (Shockwave at ∞ LP and ∞ ATK; Spiderweb
//! Castle with its target gone), the signature cards (Foolish Decision and Magician's Adage for Z-one, Shadow Dance for Crow,
//! Shockwave for Jack, Baton of the Hero for Yusei, Smile Kid for Marco, Trap Recycle for Ghost), the sample (Brilliant Shrine
//! Art, Illegal Keeper, Magnetic Storm, Power Bind, Return Damage, Trap Recycle) and, being cheap, the rest. No card of this
//! batch makes a Token.
//! Two overrides: Shadow Dance (`engine/overrides/c511000945.lua`: only the damage the opponent takes counts) and Dromi the
//! Sacred Shackles (`engine/overrides/c511000963.lua`: the damage upstream never dealt, and only for a card effect).
//! Not coverable: Smile Kid's Graveyard effect (a "Smile Angel" from the Deck: the client has no such card, so there is
//! nothing to fetch) and Gate Blocker 2's "Speed World" clause (no such card). Brilliant Shrine Art battles the attacker
//! with the Earthbound Immortal directly (`engine/overrides/c511002698.lua`: an Immortal can't be chosen as an attack target,
//! so upstream's redirect made the core ask the attacker to replay, and a CPU attacker picked its old target again).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 2, 700/600 vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const FISSURE: u16 = 4835; // destroy the opponent's face-up monster with the lowest ATK
const TRUNADE: u16 = 4910; // Giant Trunade: every Spell and Trap Card on the field returns to the hand
const OBELISK: u16 = 4998; // Obelisk the Tormentor: a Divine-Beast, 4000/4000
const YAMI: u16 = 4341; // a Field Spell
const GRANEL: u16 = 9265; // the anime Meklord Emperor Granel: a Meklord Emperor on the field keeps the Wisel Attacks alive

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card.
fn duel(seed: u64, lp: u32, spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, spawns) }

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    offered_in(s.prompt(), list)
}

fn offered_in(prompt: &serde_json::Value, list: &str) -> Vec<serde_json::Value> {
    prompt[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

#[test]
fn foolish_decision_from_the_hand_takes_no_battle_damage_in_my_own_turn() {
    let fd = card("Foolish Decision");
    let Some(mut s) = field(&[hand(0, fd), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(THUNDER_KID), Chain(fd)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", fd), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let Some(mut s) = field(&[mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}

#[test]
fn foolish_decision_is_not_offered_from_the_hand_in_the_opponents_turn() {
    // Team 1's Warwolf attacks me directly in turn 2: the card in the hand cannot answer in a turn that is not mine.
    let fd = card("Foolish Decision");
    let Some(mut s) = field(&[hand(0, fd), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(fd)], &[Attack(WARWOLF)]]).expect_err("never offered");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "hand", fd), 1, "{}", s.context());
}

#[test]
fn magicians_adage_burns_300_for_each_card_returned_to_the_hand_this_turn() {
    let adage = card("Magician's Adage");
    let spawns = [hand(0, adage), hand(0, TRUNADE), set(1, DIAN_KETO), set(1, DARK_HOLE)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(adage), Activate(TRUNADE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", adage), 1, "{}", s.context());
    assert_eq!((s.count(1, "hand", DIAN_KETO), s.count(1, "hand", DARK_HOLE)), (1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 600), "{}", s.context());
    // It lasts this turn only.
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(adage), EndTurn, Activate(TRUNADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

#[test]
fn offensive_guard_halves_a_direct_attacker_and_draws() {
    let guard = card("Offensive Guard");
    let Some(mut s) = field(&[mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    let (control_hand, control_lp) = (hand_size(&s, 0), s.lp(0));
    assert_eq!(control_lp, 6000, "{}", s.context());
    let Some(mut s) = field(&[set(0, guard), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(guard)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", guard), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(1000), "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000, "{}", s.context());
    assert_eq!(hand_size(&s, 0), control_hand + 1, "{}", s.context());
}

#[test]
fn shadow_dance_burns_1000_when_the_opponent_takes_damage_from_a_direct_attack() {
    let dance = card("Shadow Dance");
    // My Thunder Kid attacks directly in turn 3: team 1 takes 700 and then Shadow Dance's 1000.
    let Some(mut s) = field(&[set(0, dance), mon(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(THUNDER_KID), Chain(dance)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 700 - 1000), "{}", s.context());
    assert_eq!(s.count(0, "grave", dance), 1, "{}", s.context());
}

/// A Shadow Dance that waits on the side taking the damage is not "your opponent takes damage": team 1's Set Shadow Dance
/// stays Set when my Thunder Kid hits team 1 (engine/overrides/c511000945.lua: upstream let it hit back at me).
#[test]
fn shadow_dance_does_not_answer_damage_its_own_controller_takes() {
    let dance = card("Shadow Dance");
    let Some(mut s) = field(&[mon(0, THUNDER_KID), set(1, dance)]) else { return };
    keep_hands(&mut s);
    // Team 1 would chain it at the first chance: it never gets one.
    let err = s.run_both([&[EndTurn, Attack(THUNDER_KID)], &[Chain(dance)]]).expect_err("never offered");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 700), "{}", s.context());
    assert_eq!((s.count(1, "szone", dance), s.count(1, "grave", dance)), (1, 0), "{}", s.context());
}

/// Team 1's Warwolf attacks (turn 2) with team 0's Brilliant Shrine Art active (turn 1): when the attack is changed to an
/// Earthbound Immortal, the core asks its attacker whether to replay the attack ("the target is not one it may select": the
/// monster cannot be attacked by selection), and `replay` answers that.
fn warwolf_attacks_through_the_shrine(s: &mut Scenario, replay: bool) -> bool {
    use yo3_server::ocg::{Response, battle, idle, resp};
    let mut asked = false;
    s.run(&[Activate(card("Brilliant Shrine Art"))]).unwrap();
    for _ in 0..60 {
        let p = s.prompt().clone();
        let (ty, player) = (p["type"].as_u64().unwrap(), p["player"].as_u64());
        if ty == 11 && player == Some(1) && p["to_bp"] == true {
            s.respond(serde_json::json!({"type": 1, "action": idle::TO_BP, "index": null})).unwrap();
        } else if ty == 10 && player == Some(1) && p["attacks"].as_array().is_some_and(|a| !a.is_empty()) && !asked {
            s.respond(serde_json::json!({"type": 1, "action": battle::ATTACK, "index": 0})).unwrap();
        } else if ty == 12 {
            s.respond(Response::yes(resp::EFFECTYN, true).json()).unwrap();
        } else if ty == 13 {
            asked = true;
            s.respond(Response::yes(resp::YESNO, replay).json()).unwrap();
        } else {
            s.pass().unwrap();
        }
        if asked && ty == 11 { break; }
    }
    asked
}

#[test]
fn brilliant_shrine_art_turns_an_attack_on_to_an_earthbound_immortal() {
    let [art, ccapac] = ["Brilliant Shrine Art", "Earthbound Immortal Ccapac Apu"].map(card);
    let spawns = [set(0, art), mon(0, ccapac), mon(0, DARK_BLADE), spell(0, YAMI), mon(1, WARWOLF)];
    // Without it Warwolf (2000) destroys Dark Blade (1800) and I take 200.
    let Some(mut s) = field(&spawns[1..]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.lp(0)), (1, 7800), "{}", s.context());
    // With it the attack becomes a battle with Ccapac Apu (engine/overrides/c511002698.lua battles them directly, as an
    // Immortal can't be chosen as an attack target): Warwolf's controller is never asked to attack again, Warwolf is
    // destroyed and its controller takes the damage. Dark Blade lives.
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    assert!(!warwolf_attacks_through_the_shrine(&mut s, false), "the attacker is not asked to attack again: {}", s.context());
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade was not attacked: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf lost the battle with Ccapac Apu: {}", s.context());
    assert!(s.lp(0) == 8000 && s.lp(1) < 8000, "{}", s.context());
    assert_eq!(s.count(0, "szone", art), 1, "{}", s.context());
}

#[test]
fn brilliant_shrine_art_is_destroyed_when_the_earthbound_immortal_leaves_the_field() {
    // Team 1's Mystical Space Typhoon destroys the Field Spell: Ccapac Apu is destroyed in the End Phase, and the Art with it.
    let [art, ccapac] = ["Brilliant Shrine Art", "Earthbound Immortal Ccapac Apu"].map(card);
    let spawns = [set(0, art), mon(0, ccapac), spell(0, YAMI), hand(1, 4909)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, YAMI)]);
    s.run_both([&[Activate(art)], &[Activate(4909), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", YAMI), s.count(0, "grave", ccapac), s.count(0, "grave", art)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "szone", art), 0, "{}", s.context());
}

#[test]
fn wise_a3_sends_a_wisel_attack_to_the_graveyard_for_a_wisel_attack_3_from_the_hand() {
    let [wise, attack, attack3] = ["Wise A3", "Wisel Attack (TF6)", "Wisel Attack 3 (TF6)"].map(card);
    let spawns = [set(0, wise), mon(0, GRANEL), mon(0, attack), hand(0, attack3)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(wise)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", attack), 1, "the cost: {}", s.context());
    assert_eq!(s.count(0, "mzone", attack3), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", wise), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", GRANEL), 1, "{}", s.context());
}


// ---------------------------------------------------------------------------------------------------------------
// Shockwave, at finite LP and at ∞
// ---------------------------------------------------------------------------------------------------------------

const INFINITE: i64 = 19_999_998;
const KURIBOH: u16 = 4064;
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK

/// Team 0 at ∞ LP (Mystik Wok on one of two anime Gehs: 8000 and ∞ is ∞, the other Geh stays), team 1 at 8000, with `spawns`.
fn infinite(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(0, WOK), mon(0, GEH), mon(0, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &field).expect("the duel starts");
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!((s.lp(0), s.count(0, "mzone", GEH)), (INFINITE, 1), "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[]);
    Some(s)
}

/// Shockwave: "While your LP are lower than your opponent's: Target 1 monster on the field; destroy it, then, inflict damage
/// to both players equal to its ATK." Team 1 gains 1000 from Dian Keto in turn 2; in turn 3 team 0 (8000 against 9000) destroys
/// Warwolf (2000): 6000 and 7000. At equal LP it is not offered.
#[test]
fn shockwave_destroys_a_monster_and_hurts_both_players_by_its_atk_while_behind() {
    let shock = card("Shockwave");
    let spawns = [set(0, shock), mon(0, THUNDER_KID), hand(1, DIAN_KETO), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn, Activate(shock)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(0, "mzone", THUNDER_KID), s.count(0, "grave", shock)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 2000, 9000 - 2000), "{}", s.context());
    // Level LP: not behind, so it cannot be activated.
    let Some(mut s) = field(&spawns[..2]) else { return };
    let err = s.run(&[EndTurn, Activate(shock)]).expect_err("equal LP");
    assert!(err.contains("never became possible"), "{err}");
}

/// Shockwave at ∞ (engine/vendor/CardScripts/unofficial/c500000008.lua works its damage out from the target's ATK). Team 0 has
/// ∞ LP and team 1 8000, so team 1 is the one behind. A finite target (Warwolf, 2000) hurts team 1 only: ∞ LP stay ∞. An ∞
/// ATK target (a Geh) is ∞ damage to both players, which takes all of them: both at 0, a draw. Team 0, at ∞, is never behind.
#[test]
fn shockwave_at_infinite_lp_and_with_an_infinite_atk() {
    let shock = card("Shockwave");
    let Some(mut s) = infinite(&[set(1, shock), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn], &[Activate(shock)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1), s.count(1, "grave", WARWOLF)), (INFINITE, 6000, 1), "{}", s.context());
    let Some(mut s) = infinite(&[set(1, shock), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, GEH)]);
    let result = s.run_both([&[EndTurn], &[Activate(shock)]]);
    assert!(result.is_err_and(|e| e.contains("the duel is over")), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1), s.count(0, "grave", GEH)), (0, 0, 2), "{}", s.context());
    clean(&s);
    // At ∞ team 0 is not behind: its own Shockwave is not offered.
    let Some(mut s) = infinite(&[set(0, shock), mon(1, WARWOLF)]) else { return };
    let err = s.run(&[EndTurn, Activate(shock)]).expect_err("∞ is not lower than 8000");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Smile Kid, Bewitching Butterfly
// ---------------------------------------------------------------------------------------------------------------

/// Smile Kid: "You can Special Summon this card (from your hand) by discarding 1 card." One card is discarded and Smile Kid
/// comes to the field (Level 1, 0/0). Its Graveyard effect (a "Smile Angel" from the Deck) has nothing to fetch: the game has
/// no Smile Angel, so Dark Hole destroying it just sends it to the Graveyard.
#[test]
fn smile_kid_is_special_summoned_by_discarding_a_card() {
    let kid = card("Smile Kid");
    let Some(mut s) = field(&[hand(0, kid)]) else { return };
    keep_hands(&mut s);
    let (hand_before, imps_in_grave) = (hand_size(&s, 0), s.count(0, "grave", FILLER));
    s.run(&[SpSummon(kid)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", kid), 1, "{}", s.context());
    assert_eq!((hand_size(&s, 0), s.count(0, "grave", FILLER)), (hand_before - 2, imps_in_grave + 1), "{}", s.context());
    assert_eq!((s.level_of(0, kid), s.attack_of(0, kid)), (Some(1), Some(0)), "{}", s.context());
    // Destroyed by a card effect: no "Smile Angel" to fetch.
    let Some(mut s) = field(&[hand(0, kid), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(kid)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", kid), 1, "{}", s.context());
}

/// Bewitching Butterfly: "If only your opponent controls a monster, you can Special Summon this card (from your hand). If
/// your opponent controls a monster that has higher ATK than this card, destroy this card." Against Thunder Kid (700) the
/// Butterfly (800/1800, Level 2) stays; against Warwolf (2000) it is destroyed at once; it cannot be summoned with a monster
/// of its own side or with none on the other.
#[test]
fn bewitching_butterfly_comes_when_only_the_opponent_has_a_monster_and_dies_to_a_stronger_one() {
    let butterfly = card("Bewitching Butterfly");
    let Some(mut s) = field(&[hand(0, butterfly), mon(1, THUNDER_KID)]) else { return };
    s.run(&[SpSummon(butterfly)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, butterfly), s.monster(0, butterfly).and_then(|m| m["defense"].as_i64()), s.level_of(0, butterfly)), (Some(800), Some(1800), Some(2)), "{}", s.context());
    let Some(mut s) = field(&[hand(0, butterfly), mon(1, WARWOLF)]) else { return };
    s.run(&[SpSummon(butterfly)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", butterfly), s.count(0, "grave", butterfly)), (0, 1), "{}", s.context());
    for spawns in [vec![hand(0, butterfly)], vec![hand(0, butterfly), mon(0, DARK_BLADE), mon(1, THUNDER_KID)]] {
        let Some(mut s) = field(&spawns) else { return };
        let err = s.run(&[SpSummon(butterfly)]).expect_err("not offered");
        assert!(err.contains("never became possible"), "{err}");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Dromi the Sacred Shackles, Raging Sacred Curse (a Divine-Beast on my side)
// ---------------------------------------------------------------------------------------------------------------

/// Dromi the Sacred Shackles: "Target 1 Divine-Beast-Type monster you control and 1 face-up monster your opponent controls;
/// when the targeted opponent's monster leaves the field because of a card effect, inflict damage to your opponent equal to the
/// difference in ATK between that monster and the targeted Divine-Beast-Type monster had when this card resolved."
/// Obelisk (4000) and Warwolf (2000): Fissure destroys Warwolf and team 1 takes the difference, 2000 (engine/overrides/c511000963.lua:
/// upstream asked whether the leaving monster was still related to the effect, which it never is, so no damage came).
#[test]
fn dromi_hurts_the_opponent_by_the_atk_difference_when_the_targeted_monster_leaves_by_a_card_effect() {
    let dromi = card("Dromi the Sacred Shackles");
    let spawns = [set(0, dromi), mon(0, OBELISK), hand(0, FISSURE), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(dromi), Activate(FISSURE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", dromi), s.count(1, "grave", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2000), "{}", s.context());
}

/// Dromi's damage is for a monster that leaves "because of a card effect": Warwolf destroyed by battle against Obelisk costs
/// its controller the battle damage (2000) and nothing more (engine/overrides/c511000963.lua).
#[test]
fn dromi_does_nothing_when_the_targeted_monster_is_destroyed_by_battle() {
    let dromi = card("Dromi the Sacred Shackles");
    let Some(mut s) = field(&[set(0, dromi), mon(0, OBELISK), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(dromi)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2000), "only the battle damage: {}", s.context());
}

/// Raging Sacred Curse: "If you control a Divine-Beast-Type monster: Target 1 monster your opponent controls; until the End
/// Phase, that monster loses 2000 ATK/DEF, and is unaffected by your opponent's Spell/Trap effects." With Obelisk on my side
/// Warwolf (2000/100) is at 0/0 in turn 3, and back at 2000 after the turn. The "opponent's" Spell/Trap effects are those of
/// the target's own controller, team 1: its Book of Moon, chained to my next summon, turns Warwolf face-down when there was no
/// curse and leaves it face-up when there was.
#[test]
fn raging_sacred_curse_takes_2000_and_shuts_out_the_targets_own_spells() {
    let [curse, book] = [card("Raging Sacred Curse"), 5432]; // Book of Moon: a Quick-Play Spell turning a face-up monster face-down
    let spawns = [mon(0, OBELISK), set(0, curse), mon(1, WARWOLF), set(1, book), hand(0, DARK_BLADE)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(curse)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", curse), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(0), "2000 - 2000: {}", s.context());
    assert_eq!(s.monster(1, WARWOLF).and_then(|m| m["defense"].as_i64()), Some(0), "{}", s.context());
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[Summon(DARK_BLADE)], &[Chain(book)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", book), 1, "Book of Moon was activated: {}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(1), "and did nothing to Warwolf: {}", s.context());
    // The control: with no curse the same Book of Moon turns it face-down (position 8).
    let Some(mut s) = field(&[mon(1, WARWOLF), set(1, book), hand(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[Summon(DARK_BLADE)], &[Chain(book)]]).unwrap();
    assert_eq!(s.position_of(1, WARWOLF), Some(8), "{}", s.context());
    // Until the End Phase.
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(curse), EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(1, WARWOLF), Some(2000), "{}", s.context());
}

/// Doom Ray: "Activate only when an opponent's monster declares a direct attack. Inflict 800 damage to both players for each
/// card in your hand." The damage is 800 times the cards in my hand, to both; the attack goes on (2000 on top for me).
#[test]
fn doom_ray_hurts_both_players_800_for_each_card_in_my_hand() {
    let ray = card("Doom Ray");
    let Some(mut s) = field(&[set(0, ray), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(ray)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    let cards = hand_size(&s, 0) as i64;
    assert!((5..=7).contains(&cards), "{}", s.context());
    assert_eq!(s.count(0, "grave", ray), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 800 * cards - 2000, 8000 - 800 * cards), "{cards} cards: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Spiderweb Castle
// ---------------------------------------------------------------------------------------------------------------

/// Spiderweb Castle: "Activate only when your opponent activates a Trap Card. Once per turn, when an opponent's monster
/// declares an attack: Halve its ATK. During your Main Phase 1: You can send this face-up card to the Graveyard to target 1
/// monster you control; it can attack directly this turn." Team 1's Just Desserts (500 for my one monster) is answered by the
/// Castle; Warwolf (2000) then attacks Dark Blade (1800) at 1000 and is destroyed (800 damage to team 1). In turn 3 the
/// Castle goes to the Graveyard and Dark Blade attacks directly past Thunder Kid: 1800.
#[test]
fn spiderweb_castle_halves_an_attacker_and_gives_a_direct_attack() {
    let [castle, desserts] = ["Spiderweb Castle", "Just Desserts"].map(card);
    let spawns = [set(0, castle), mon(0, DARK_BLADE), set(1, desserts), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[Chain(castle)], &[Activate(desserts), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", castle), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf at 1000 lost to Dark Blade: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 500, 8000 - 800), "{}", s.context());
    // Turn 3: the Castle's second effect.
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[EndTurn, Activate(castle), Attack(DARK_BLADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", castle), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 800 - 1800, "a direct attack past Thunder Kid: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
}

/// Spiderweb Castle's second effect targets a monster. If the target is gone when it resolves (Raigeki Break destroys it in
/// response, after the Castle was sent to the Graveyard as the cost) nothing is left to give a direct attack: the effect does
/// nothing instead of stopping on a Lua error (the script's first line, `tc:IsFaceup()`, never checks the target).
#[test]
fn spiderweb_castle_whose_target_is_destroyed_in_response_does_nothing() {
    use yo3_server::ocg::{Response, idle, msg};
    let [castle, desserts, raigeki] = ["Spiderweb Castle", "Just Desserts", "Raigeki Break"].map(card);
    let spawns = [set(0, castle), mon(0, DARK_BLADE), set(1, desserts), hand(1, raigeki), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[Chain(castle)], &[Activate(desserts), SetSpell(raigeki)]]).unwrap();
    assert_eq!((s.count(0, "szone", castle), s.count(1, "szone", raigeki), s.count(0, "mzone", DARK_BLADE)), (1, 1, 1), "{}", s.context());
    // Turn 3, Main Phase 1: the Castle's effect by hand, answered by Raigeki Break when (and only when) it is on the chain.
    s.run_to(&[EndTurn], |p| p["type"] == msg::SELECT_IDLECMD && p["player"] == 0 && offered_in(p, "activates").contains(&castle.into())).unwrap();
    let index = offered(&s, "activates").iter().position(|c| *c == castle).unwrap();
    s.respond(serde_json::json!({"type": 1, "action": idle::ACTIVATE, "index": index})).unwrap();
    let mut chained = false;
    for _ in 0..30 {
        let p = s.prompt().clone();
        if p["type"] == msg::SELECT_IDLECMD && p["player"] == 0 { break; }
        match offered(&s, "selects").iter().position(|c| *c == raigeki) {
            Some(i) if p["type"] == msg::SELECT_CHAIN && p["player"] == 1 && !chained => {
                chained = true;
                s.respond(Response::Chain(Some(i)).json()).unwrap();
            }
            _ => s.pass().unwrap(),
        }
    }
    assert!(chained, "Raigeki Break answered the Castle: {}", s.context());
    clean(&s);
    assert_eq!((s.count(0, "grave", castle), s.count(0, "grave", DARK_BLADE), s.count(1, "grave", raigeki)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Baton of the Hero, Return Damage, Trap Recycle, Power Bind, Illegal Keeper, Magnetic Storm
// ---------------------------------------------------------------------------------------------------------------

/// Baton of the Hero: "During your opponent's turn: During this turn, you choose the targets of your opponent's attacks."
/// Team 1's Warwolf attacks Dark Blade or Thunder Kid, as its own choice without the Baton (the victim of the control run);
/// with the Baton, I name the other one.
#[test]
fn baton_of_the_hero_lets_me_choose_what_the_opponent_attacks() {
    let baton = card("Baton of the Hero");
    let spawns = [mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    let default_victim = if s.count(0, "grave", DARK_BLADE) == 1 { DARK_BLADE } else { THUNDER_KID };
    assert_eq!(s.count(0, "grave", default_victim), 1, "{}", s.context());
    let mine = if default_victim == DARK_BLADE { THUNDER_KID } else { DARK_BLADE };
    let Some(mut s) = field(&[set(0, baton), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, mine)]);
    s.run_both([&[EndTurn, Chain(baton)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", baton), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", mine), 1, "I chose {mine}: {}", s.context());
    assert_eq!(s.count(0, "mzone", default_victim), 1, "{}", s.context());
}

/// Return Damage: "If you would take battle damage: You take no battle damage, and if you do, inflict damage to your opponent
/// equal to the battle damage you would have taken." Warwolf (2000) attacks my Thunder Kid in Attack Position (700): 1300
/// battle damage is mine to take, and goes to team 1 instead.
#[test]
fn return_damage_sends_my_battle_damage_to_the_opponent() {
    let rd = card("Return Damage");
    let Some(mut s) = field(&[set(0, rd), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(rd)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", rd), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1300), "{}", s.context());
}

/// Trap Recycle: "Send 1 card from your hand to the Graveyard. Target 1 Trap Card in your Graveyard that successfully resolved
/// its effect this turn and add it to your hand. You cannot Set that Trap Card this turn." Just Desserts is used and taken back
/// in turn 3 (one discard, Desserts in the hand again and not offered to Set); one used in turn 1 is no target in turn 3.
#[test]
fn trap_recycle_takes_back_a_trap_used_this_turn() {
    let [recycle, desserts] = ["Trap Recycle", "Just Desserts"].map(card);
    let spawns = [set(0, desserts), set(0, recycle), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(desserts)]).unwrap();
    let (hand_before, imps) = (hand_size(&s, 0), s.count(0, "grave", FILLER));
    s.run(&[Activate(recycle)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", desserts), s.count(0, "grave", desserts), s.count(0, "grave", recycle)), (1, 0, 1), "{}", s.context());
    assert_eq!((hand_size(&s, 0), s.count(0, "grave", FILLER)), (hand_before, imps + 1), "one discarded, one taken: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
    assert!(!offered(&s, "spell_sets").contains(&desserts.into()), "it cannot be Set this turn: {:?}", offered(&s, "spell_sets"));
    // The control: a Trap used two turns ago.
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(desserts), EndTurn, Activate(recycle)]).expect_err("no Trap of this turn");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", desserts), 1, "{}", s.context());
}

/// Power Bind: "When your opponent Normal Summons a monster: That monster cannot attack this turn, also its ATK becomes 0
/// until the Standby Phase of the opponent's next turn." Team 1's Warwolf is Normal Summoned: 0 ATK, and it cannot attack in
/// that turn.
#[test]
fn power_bind_leaves_a_normal_summoned_monster_with_no_atk_and_no_attack() {
    let bind = card("Power Bind");
    let Some(mut s) = field(&[set(0, bind), hand(1, WARWOLF), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(bind)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", bind), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(0), "{}", s.context());
    let Some(mut s) = field(&[set(0, bind), hand(1, WARWOLF), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(bind)], &[Summon(WARWOLF), Attack(WARWOLF)]]).expect_err("cannot attack");
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
    assert_eq!((s.lp(0), s.count(0, "mzone", DARK_BLADE)), (8000, 1), "{}", s.context());
}

/// Illegal Keeper: "If your opponent draws a card(s) (except during the Draw Phase): Return the card(s) to the Deck, also,
/// inflict 1000 damage to your opponent." Team 1's Pot of Greed draws 2, both go back to the Deck, and team 1 takes 1000; the
/// draw of its Draw Phase stays (8000 until then).
#[test]
fn illegal_keeper_returns_drawn_cards_and_burns_1000() {
    let keeper = card("Illegal Keeper");
    let pot = 4844; // Pot of Greed: draw 2
    let Some(mut s) = field(&[hand(1, pot)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(pot)]]).unwrap();
    let control = hand_size(&s, 1);
    let Some(mut s) = field(&[mon(0, keeper), hand(1, pot)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(pot)]]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 1), control - 2, "both drawn cards went back: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7000), "{}", s.context());
    assert_eq!(s.count(1, "grave", pot), 1, "{}", s.context());
}

/// Magnetic Storm: "If this is the only card in your Spell & Trap Zone. Target 1 Set Spell/Trap your opponent controls; destroy
/// it. Your opponent cannot activate Spell/Trap Cards in response to this card's activation." Team 1's Trap Hole is destroyed
/// and its Set Mystical Space Typhoon is never offered in answer; with another card of mine in the zone it cannot be activated.
#[test]
fn magnetic_storm_destroys_a_set_card_and_cannot_be_answered() {
    use yo3_server::ocg::{idle, msg};
    let storm = card("Magnetic Storm");
    let (trap_hole, mst) = (4836, 4909);
    let Some(mut s) = field(&[set(0, storm), set(1, trap_hole), set(1, mst)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, trap_hole)]);
    s.run_to(&[EndTurn], |p| p["type"] == msg::SELECT_IDLECMD && p["player"] == 0 && offered_in(p, "activates").contains(&storm.into())).unwrap();
    let index = offered(&s, "activates").iter().position(|c| *c == storm).unwrap();
    s.respond(serde_json::json!({"type": 1, "action": idle::ACTIVATE, "index": index})).unwrap();
    for _ in 0..30 {
        let p = s.prompt().clone();
        if p["type"] == msg::SELECT_IDLECMD && p["player"] == 0 { break; }
        if p["type"] == msg::SELECT_CHAIN {
            assert!(!offered_in(&p, "selects").contains(&mst.into()), "Typhoon was offered in answer: {p}");
        }
        s.pass().unwrap();
    }
    clean(&s);
    assert_eq!((s.count(1, "grave", trap_hole), s.count(1, "szone", mst), s.count(0, "grave", storm)), (1, 1, 1), "{}", s.context());
    // Another card in my Spell & Trap Zone: not the only one.
    let Some(mut s) = field(&[set(0, storm), set(0, DIAN_KETO), set(1, trap_hole)]) else { return };
    let err = s.run(&[EndTurn, Activate(storm)]).expect_err("not alone");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// The rest of the batch
// ---------------------------------------------------------------------------------------------------------------

fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }

/// Class Change: "Target 1 monster on the field; Increase its Level by 1." Dark Blade (4) becomes Level 5, and Warwolf (4),
/// a monster of the opponent's, would too.
#[test]
fn class_change_raises_a_level_by_1() {
    let change = card("Class Change");
    for (team, cid) in [(0, DARK_BLADE), (1, WARWOLF)] {
        let Some(mut s) = field(&[set(0, change), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
        keep_hands(&mut s);
        s.prefer(&[(team, cid)]);
        s.run(&[EndTurn, Activate(change)]).unwrap();
        clean(&s);
        assert_eq!(s.level_of(team, cid), Some(5), "{}", s.context());
        assert_eq!(s.count(0, "grave", change), 1, "{}", s.context());
    }
}

/// Level Thunder: "Inflict damage to your opponent equal to the combined Levels of all face-up monsters you control x100."
/// Dark Blade (4) and Thunder Kid (2), and a Set Feral Imp that is not face-up: 600.
#[test]
fn level_thunder_burns_100_for_each_level_of_my_face_up_monsters() {
    let thunder = card("Level Thunder");
    let Some(mut s) = field(&[hand(0, thunder), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon_set(0, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(thunder)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 600), "{}", s.context());
    assert_eq!(s.count(0, "grave", thunder), 1, "{}", s.context());
}

/// Block Dugout: "If the combined Levels of all monsters your opponent controls is less than the combined Levels of all
/// monsters you control, your opponent's monsters cannot declare an attack." My Dark Blade (4) against Thunder Kid (3): it
/// cannot attack; against Warwolf (4), equal Levels, it can.
#[test]
fn block_dugout_stops_attacks_while_my_monsters_have_more_levels() {
    let dugout = card("Block Dugout");
    let Some(mut s) = field(&[hand(0, dugout), mon(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Activate(dugout)], &[Attack(THUNDER_KID)]]).expect_err("cannot declare an attack");
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let Some(mut s) = field(&[hand(0, dugout), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(dugout)], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "equal Levels: {}", s.context());
}

/// Battle Eater: "If you would take battle damage less than or equal to this card's ATK: You can send this card from your hand
/// to the Graveyard; negate the attack and end the Battle Phase. Then, take damage equal to this card's ATK." Warwolf's direct
/// attack (2000) is negated, the Battle Phase ends, and I take Battle Eater's 2500.
#[test]
fn battle_eater_negates_an_attack_ends_the_battle_phase_and_costs_its_atk() {
    let eater = card("Battle Eater");
    let Some(mut s) = field(&[hand(0, eater), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(eater)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", eater), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 2500, 8000), "{}", s.context());
    assert_eq!(s.prompt()["type"], yo3_server::ocg::msg::SELECT_IDLECMD, "the Battle Phase is over: {}", s.context());
}

/// Fuse Ray Blaster: "When another "Fuse Ray Blaster" is Summoned while this card is in Attack Position: Inflict 800 damage to
/// your opponent." A second Blaster Normal Summoned beside one in Attack Position costs team 1 800; beside one in Defense
/// Position nothing.
#[test]
fn fuse_ray_blaster_burns_800_when_another_is_summoned_beside_it_in_attack_position() {
    let blaster = card("Fuse Ray Blaster");
    let Some(mut s) = field(&[mon(0, blaster), hand(0, blaster)]) else { return };
    s.run(&[Summon(blaster)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", blaster), s.lp(1)), (2, 8000 - 800), "{}", s.context());
    let Some(mut s) = field(&[mon_def(0, blaster), hand(0, blaster)]) else { return };
    s.run(&[Summon(blaster)]).unwrap();
    assert_eq!((s.count(0, "mzone", blaster), s.lp(1)), (2, 8000), "{}", s.context());
}

/// Gate Blocker 2: "You can Special Summon this card (from your hand) by Tributing 1 "Gate Blocker"." The Gate Blocker (100/2000)
/// is Tributed and Gate Blocker 2 (Level 6, 100/3000) comes. Without a Gate Blocker it is not offered.
#[test]
fn gate_blocker_2_is_special_summoned_by_tributing_a_gate_blocker() {
    let [blocker, blocker2] = ["Gate Blocker", "Gate Blocker 2"].map(card);
    let Some(mut s) = field(&[hand(0, blocker2), mon(0, blocker)]) else { return };
    s.run(&[SpSummon(blocker2)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", blocker2), s.count(0, "grave", blocker)), (1, 1), "{}", s.context());
    assert_eq!((s.level_of(0, blocker2), s.attack_of(0, blocker2)), (Some(6), Some(100)), "{}", s.context());
    let Some(mut s) = field(&[hand(0, blocker2), mon(0, DARK_BLADE)]) else { return };
    let err = s.run(&[SpSummon(blocker2)]).expect_err("no Gate Blocker");
    assert!(err.contains("never became possible"), "{err}");
}

/// Drain Strike: "If the equipped monster attacks a Defense Position monster, inflict half piercing battle damage to your
/// opponent, and gain LP equal to the battle damage inflicted." Warwolf (2000) equipped attacks Dark Blade in Defense (1500):
/// 500 piercing, halved: 250 damage to team 1, 250 gained.
#[test]
fn drain_strike_pierces_for_half_and_gains_what_it_inflicts() {
    let drain = card("Drain Strike");
    let Some(mut s) = field(&[hand(0, drain), mon(0, WARWOLF), mon_def(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(drain), EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 + 250, 8000 - 250), "{}", s.context());
}

/// Reincarnation Ring: "Tribute 1 monster; Special Summon 1 monster from your Graveyard whose Level is equal to double the
/// Tributed monster's, and if you do, equip it with this card. When this card leaves the field, destroy the equipped
/// monster." Thunder Kid (2) is Tributed for Dark Blade (4), not for Summoned Skull (6) beside it in the Graveyard, and Dark
/// Blade is equipped with the Ring; team 1's Mystical Space Typhoon on the Ring destroys Dark Blade too.
#[test]
fn reincarnation_ring_revives_at_double_the_level_and_takes_the_monster_with_it() {
    let ring = card("Reincarnation Ring");
    let (skull, mst) = (4028, 4909);
    let spawns = [set(0, ring), mon(0, THUNDER_KID), grave(0, skull), grave(0, DARK_BLADE), hand(1, mst)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(ring)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(0, "grave", THUNDER_KID), s.count(0, "grave", skull)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "szone", ring), 1, "the Ring is equipped: {}", s.context());
    s.prefer(&[(0, ring)]);
    s.run_both([&[], &[Activate(mst)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", ring), s.count(0, "grave", DARK_BLADE), s.count(0, "mzone", DARK_BLADE)), (1, 1, 0), "{}", s.context());
}

/// Spirit Mask: "When this card is sent to the Graveyard: send 1 card from your hand to the Graveyard." Equipped to Dark Blade,
/// it goes to the Graveyard with it when team 1's Dark Hole destroys the monster, and one card of my hand follows.
#[test]
fn spirit_mask_sends_a_card_from_the_hand_to_the_graveyard_when_it_is_sent_there() {
    let mask = card("Spirit Mask");
    let Some(mut s) = field(&[hand(0, mask), mon(0, DARK_BLADE), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(mask)]).unwrap();
    let (hand_before, imps) = (hand_size(&s, 0), s.count(0, "grave", FILLER));
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", mask), s.count(0, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!((hand_size(&s, 0), s.count(0, "grave", FILLER)), (hand_before - 1, imps + 1), "{}", s.context());
}

/// Diva Adriana: a Normal Monster, Level 1, 300/1000.
#[test]
fn diva_adriana_is_a_vanilla_level_1() {
    let diva = card("Diva Adriana");
    let Some(mut s) = field(&[hand(0, diva)]) else { return };
    s.run(&[Summon(diva)]).unwrap();
    clean(&s);
    assert_eq!((s.level_of(0, diva), s.attack_of(0, diva)), (Some(1), Some(300)), "{}", s.context());
    assert_eq!(s.monster(0, diva).and_then(|m| m["defense"].as_i64()), Some(1000), "{}", s.context());
}
