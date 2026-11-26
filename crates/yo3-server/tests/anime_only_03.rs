//! Anime-only cards, group 3 (rows 5297-5324 of engine/added/cards.tsv): Mirage Ruler, Infestation, Parasite Worm,
//! Turn Jump (Anime), 100-Year Awakening, Curse of Thorns, Thorn Princess, Mimesis, Parasite Caterpillar and Poison
//! Butterfly, Seven Kid Goats, Little Red Riding Hood, Armored Centipede, Brave Attack, Lengard, Rainbow Blessing (DM) and
//! Spell Textbook. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap here is Set first (a Trap cannot be
//! activated from the hand); neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! Not covered here: Giant's Training (it needs "Tom Thumb" and "Globerman", which have a script but no row of the
//! client's card table, so no client id: they cannot be held in a duel), Aria From Beyond, Blocken, Cursebreaker, Ground
//! Erosion, Magician's Selection, Lev. 2 Power Boost, Forest Hunter, Forest Wolf, Pheromone Wasp and Spinning Wheel
//! Spindle (outside this group's sample of the batch; Cell Division and Reckless Parasite were held back).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, idle, msg};

// The cards of this group (client ids).
const LENGARD: u16 = 5862; // Rock, Level 3, 300/100
const MIRAGE_RULER: u16 = 5864;
const RAINBOW_BLESSING: u16 = 5865; // Rainbow Blessing (DM)
const SPELL_TEXTBOOK: u16 = 5866;
const TURN_JUMP: u16 = 5867; // Turn Jump (Anime)
const BRAVE_ATTACK: u16 = 5808;
const ARMORED_CENTIPEDE: u16 = 5870; // Insect, Level 5, 1600/1600
const INFESTATION: u16 = 5936;
const MIMESIS: u16 = 5938;
const PARASITE_CATERPILLAR: u16 = 5939; // Insect, Level 3, 800/800
const PARASITE_WORM: u16 = 5940;
const POISON_BUTTERFLY: u16 = 5942; // Insect, Level 7, 2700/1800
const AWAKENING: u16 = 5997; // 100-Year Awakening
const CURSE_OF_THORNS: u16 = 5998; // a Continuous Trap
const LITTLE_RED_RIDING_HOOD: u16 = 6009; // Level 3, 800/1200
const SEVEN_KID_GOATS: u16 = 6011; // Beast, Level 3, 700/700
const THORN_PRINCESS: u16 = 6013; // Spellcaster, Level 4, 400/1200
// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DOKUROYAIBA: u16 = 5150; // a FIRE Fiend, Level 3, 1000/400, vanilla
const BASIC_INSECT: u16 = 4056; // Insect, Level 2, 500/700, vanilla
const HERCULES_BEETLE: u16 = 4058; // Insect, Level 5, 1500/2000, vanilla
const GAZELLE: u16 = 4753; // Gazelle the King of Mythical Beasts: Beast, Level 4, 1500/1200, vanilla
const SANGAN: u16 = 4054; // when sent from the field to the Graveyard: add a monster of 1500 ATK or less from the Deck
const KURIBOH: u16 = 4064; // played as the anime Kuriboh
const DARK_HOLE: u16 = 4342;
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const MST: u16 = 4909; // Mystical Space Typhoon
const MONSTER_REBORN: u16 = 4842;
const TOY_ROBOT_BOX: u16 = 5801; // Special Summons three Robot Tokens at once

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }
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

/// The usual duel: 8000 LP, both Decks of filler. Neither player may attack in turn 1, so a plan that attacks with team 0
/// waits for turn 3, and each End Phase before it discards down to 6 cards: tests that keep a card in hand across those
/// turns name the fillers as the cards to discard (`keep_hands`).
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [FILLER, FILLER], bottom, spawns) }

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
// Mimesis
// ---------------------------------------------------------------------------------------------------------------

/// Mimesis: "When an Insect-Type monster you control is targeted for an attack: Return that monster to the hand, and if
/// you do, Special Summon 1 Level 4 or lower Insect-Type monster from your hand, and it becomes the new attack target."
/// Warwolf (2000) attacks Hercules Beetle (1500): the Beetle goes to the hand, Basic Insect (Level 2, 500) is Summoned from
/// it and takes the attack: it is destroyed and the damage is 1500, not the 500 the Beetle would have cost.
#[test]
fn mimesis_returns_the_attacked_insect_and_puts_another_in_its_place() {
    let spawns = [set(0, MIMESIS), mon(0, HERCULES_BEETLE), hand(0, BASIC_INSECT), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(MIMESIS)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", MIMESIS), 1, "the Trap was used: {}", s.context());
    assert_eq!(s.count(0, "hand", HERCULES_BEETLE), 1, "the attacked Insect went to the hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", HERCULES_BEETLE), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", BASIC_INSECT), 1, "Basic Insect took the attack and was destroyed: {}", s.context());
    assert_eq!(s.lp(0), 6500, "2000 against 500 ATK in Attack Position: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Mimesis, the control: without the Trap the Beetle is destroyed and the damage is 500.
#[test]
fn without_mimesis_the_attacked_insect_is_destroyed() {
    let spawns = [mon(0, HERCULES_BEETLE), hand(0, BASIC_INSECT), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", HERCULES_BEETLE), 1, "{}", s.context());
    assert_eq!(s.lp(0), 7500, "{}", s.context());
}

/// Mimesis: "When an Insect-Type monster you control is targeted": Dark Magician (a Spellcaster) attacked gives no
/// activation, although Basic Insect waits in the hand.
#[test]
fn mimesis_ignores_an_attacked_monster_that_is_not_an_insect() {
    let spawns = [set(0, MIMESIS), mon(0, DARK_MAGICIAN), hand(0, BASIC_INSECT), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(MIMESIS)], &[Attack(WARWOLF)]]).expect_err("not on offer");
    assert!(err.contains("Chain(5938) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", MIMESIS), 1, "still Set: {}", s.context());
}

/// Mimesis: "Special Summon 1 Level 4 or lower Insect-Type monster from your hand": with no such monster in the hand (a
/// Level 5 Beetle is no help) the Trap is not offered.
#[test]
fn mimesis_needs_a_small_insect_in_the_hand() {
    let spawns = [set(0, MIMESIS), mon(0, HERCULES_BEETLE), hand(0, HERCULES_BEETLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(MIMESIS)], &[Attack(WARWOLF)]]).expect_err("no Level 4 or lower Insect in hand");
    assert!(err.contains("Chain(5938) never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Parasite Caterpillar and Poison Butterfly
// ---------------------------------------------------------------------------------------------------------------

/// Parasite Caterpillar: "After damage calculation, if this card is targeted for an attack: Equip this card to the attacking
/// monster. The equipped monster is treated as Insect-Type, and it cannot be Tributed. It cannot be targeted for an attack,
/// but does not prevent you from attacking directly. During your opponent's 3rd Standby Phase after this card was equipped
/// to that monster: Destroy the equipped monster and Special Summon 1 "Poison Butterfly" from your Deck to your
/// opponent's side of the field." Warwolf (2000) attacks the Caterpillar (800) in turn 2: it is equipped to Warwolf (an
/// Insect now) instead of going to the Graveyard. In turn 3 Thunder Kid cannot attack Warwolf and attacks directly. At
/// the opponent's 3rd Standby Phase (turn 8) Warwolf is destroyed and Poison Butterfly (from team 0's Deck) is on their side.
#[test]
fn parasite_caterpillar_equips_itself_to_the_attacker_and_later_destroys_it() {
    let spawns = [mon(0, PARASITE_CATERPILLAR), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[POISON_BUTTERFLY], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", PARASITE_CATERPILLAR), 1, "the Caterpillar is an Equip Card now: {}", s.context());
    assert_eq!(s.count(0, "grave", PARASITE_CATERPILLAR), 0, "and not in the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "mzone", PARASITE_CATERPILLAR), 0, "{}", s.context());
    assert_eq!(s.lp(0), 6800, "the attack was still 1200 damage: {}", s.context());
    assert_eq!(s.monster(1, WARWOLF).unwrap()["race"], 2048, "treated as an Insect: {}", s.context());
    // Turn 3: Warwolf cannot be targeted for an attack, and Thunder Kid attacks directly.
    s.run_both([&[Attack(THUNDER_KID)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7300, "a direct attack of 700: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    // Turns 4 and 6 (the 1st and 2nd Standby Phases): Warwolf is still there.
    s.run_both([&[EndTurn, EndTurn], &[EndTurn, EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "Warwolf survives two Standby Phases: {}", s.context());
    assert_eq!(s.count(1, "mzone", POISON_BUTTERFLY), 0, "{}", s.context());
    // Turn 8: the 3rd.
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf is destroyed at the 3rd Standby Phase: {}", s.context());
    assert_eq!(s.count(1, "mzone", POISON_BUTTERFLY), 1, "Poison Butterfly is on their side: {}", s.context());
    assert_eq!(s.attack_of(1, POISON_BUTTERFLY), Some(2700), "{}", s.context());
}

/// Poison Butterfly: "Once per turn, during your End Phase: Take 500 damage." Its controller takes 500 at the end of its own
/// turn, and nothing at the end of the opponent's.
#[test]
fn poison_butterfly_hurts_its_controller_in_their_end_phase() {
    let Some(mut s) = field(&[], &[mon(0, POISON_BUTTERFLY)]) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "500 damage in my End Phase: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "none in the opponent's End Phase: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Curse of Thorns, 100-Year Awakening and Thorn Princess
// ---------------------------------------------------------------------------------------------------------------

/// Curse of Thorns: "Negate the effects of cards that activate in the Graveyard." Sangan is destroyed by Dark Hole and its
/// mandatory trigger ("add a monster of 1500 ATK or less from your Deck") is negated: the hand only loses Dark Hole.
#[test]
fn curse_of_thorns_negates_an_effect_that_activates_in_the_graveyard() {
    let spawns = [spell(0, CURSE_OF_THORNS), mon(0, SANGAN), hand(0, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SANGAN), 1, "Sangan was destroyed: {}", s.context());
    assert_eq!(hand_size(&s, 0), before - 1, "its search was negated: {}", s.context());
}

/// Curse of Thorns, the control: without it Sangan's search adds a card to the hand.
#[test]
fn without_curse_of_thorns_sangan_searches() {
    let spawns = [mon(0, SANGAN), hand(0, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), before, "Dark Hole left, a monster came: {}", s.context());
}

/// Curse of Thorns: "When this card is destroyed: Inflict 500 damage to the player that destroyed it." The opponent's
/// Mystical Space Typhoon destroys it: 500 damage to the opponent, none to me.
#[test]
fn curse_of_thorns_damages_the_opponent_who_destroys_it() {
    let spawns = [spell(0, CURSE_OF_THORNS), hand(1, MST)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, CURSE_OF_THORNS)]);
    s.run_both([&[], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CURSE_OF_THORNS), 1, "the Curse was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 7500, "500 damage to the player that destroyed it: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Curse of Thorns: "the player that destroyed it" includes its own controller: my Mystical Space Typhoon costs me 500.
#[test]
fn curse_of_thorns_damages_its_own_controller_who_destroys_it() {
    let spawns = [spell(0, CURSE_OF_THORNS), hand(0, MST)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, CURSE_OF_THORNS)]);
    s.run(&[Activate(MST)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CURSE_OF_THORNS), 1, "{}", s.context());
    assert_eq!(s.lp(0), 7500, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Curse of Thorns is a Trap card: Set, it is activated in the Main Phase and stays face-up on the field.
#[test]
fn curse_of_thorns_is_activated_from_the_set_position_and_stays() {
    let Some(mut s) = field(&[], &[set(0, CURSE_OF_THORNS)]) else { return };
    s.run(&[Activate(CURSE_OF_THORNS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", CURSE_OF_THORNS), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", CURSE_OF_THORNS), 0, "{}", s.context());
}

/// 100-Year Awakening: "Tribute "Curse of Thorns" on the field; Special Summon 1 "Thorn Princess" from your Deck."
/// Thorn Princess (Level 4, 400/1200) is the Deck's last card. The Curse is the Tribute.
#[test]
fn awakening_tributes_curse_of_thorns_for_thorn_princess() {
    let spawns = [spell(0, CURSE_OF_THORNS), hand(0, AWAKENING)];
    let Some(mut s) = field(&[THORN_PRINCESS], &spawns) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Activate(AWAKENING)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CURSE_OF_THORNS), 1, "the Curse was Tributed: {}", s.context());
    assert_eq!(s.count(0, "szone", CURSE_OF_THORNS), 0, "{}", s.context());
    assert_eq!((s.level_of(0, THORN_PRINCESS), s.attack_of(0, THORN_PRINCESS)), (Some(4), Some(400)), "Thorn Princess: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 1, "from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", AWAKENING), 1, "{}", s.context());
    // The Tribute is not a destruction: no damage from the Curse.
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// 100-Year Awakening: "Tribute "Curse of Thorns" on the field": without one it is not offered.
#[test]
fn awakening_needs_curse_of_thorns_on_the_field() {
    let Some(mut s) = field(&[THORN_PRINCESS], &[hand(0, AWAKENING)]) else { return };
    let err = s.run(&[Activate(AWAKENING)]).expect_err("no Curse");
    assert!(err.contains("Activate(5997) never became possible"), "{err}");
    clean(&s);
}

/// Thorn Princess: "If this card is Special Summoned: Target 1 monster your opponent controls; take control of it."
/// 100-Year Awakening brings her and Warwolf changes sides.
#[test]
fn thorn_princess_takes_control_of_an_opponents_monster() {
    let spawns = [spell(0, CURSE_OF_THORNS), hand(0, AWAKENING), mon(1, WARWOLF)];
    let Some(mut s) = field(&[THORN_PRINCESS], &spawns) else { return };
    s.run(&[Activate(AWAKENING)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THORN_PRINCESS), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf is mine now: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
}

/// Thorn Princess: "Cannot be Normal Summoned/Set." With her in the hand she is on offer neither to Summon nor to Set.
#[test]
fn thorn_princess_cannot_be_normal_summoned_or_set() {
    let Some(mut s) = field(&[], &[hand(0, THORN_PRINCESS)]) else { return };
    s.run(&[]).unwrap();
    assert_eq!(s.count(0, "hand", THORN_PRINCESS), 1, "{}", s.context());
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "{}", s.context());
    assert!(!offered(&s, "summons").contains(&THORN_PRINCESS.into()), "no Normal Summon: {:?}", offered(&s, "summons"));
    assert!(!offered(&s, "monster_sets").contains(&THORN_PRINCESS.into()), "no Set: {:?}", offered(&s, "monster_sets"));
    assert!(offered(&s, "summons").contains(&FILLER.into()), "the other cards can be Summoned: {:?}", offered(&s, "summons"));
    s.pass().unwrap();
    clean(&s);
}

/// Thorn Princess: "Must be Special Summoned with "100-Year Awakening", and cannot be Special Summoned by other ways." Monster
/// Reborn has only her in the Graveyard to choose and is not offered.
#[test]
fn thorn_princess_cannot_be_special_summoned_by_other_cards() {
    let Some(mut s) = field(&[], &[hand(0, MONSTER_REBORN), grave(0, THORN_PRINCESS)]) else { return };
    let err = s.run(&[Activate(MONSTER_REBORN)]).expect_err("no legal target");
    assert!(err.contains("Activate(4842) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "grave", THORN_PRINCESS), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Seven Kid Goats and Little Red Riding Hood
// ---------------------------------------------------------------------------------------------------------------

/// Seven Kid Goats: "When this card is Normal or Flip Summoned: Destroy 1 face-up Beast-Type monster your opponent controls,
/// and if you do, inflict damage to your opponent equal to the ATK of the destroyed monster." Gazelle (1500) is destroyed
/// and the opponent takes 1500.
#[test]
fn seven_kid_goats_destroys_a_beast_and_inflicts_its_atk() {
    let spawns = [hand(0, SEVEN_KID_GOATS), mon(1, GAZELLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Summon(SEVEN_KID_GOATS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", GAZELLE), 1, "Gazelle was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 6500, "and its 1500 ATK was inflicted: {}", s.context());
    assert_eq!(s.count(0, "mzone", SEVEN_KID_GOATS), 1, "{}", s.context());
}

/// Seven Kid Goats: "Beast-Type": a monster of another Type (Warwolf) is left alone and no damage is inflicted.
#[test]
fn seven_kid_goats_leaves_a_monster_that_is_not_a_beast() {
    let spawns = [hand(0, SEVEN_KID_GOATS), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Summon(SEVEN_KID_GOATS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Seven Kid Goats: "face-up": a Set Beast is not destroyed and no damage is inflicted.
#[test]
fn seven_kid_goats_leaves_a_face_down_beast() {
    let spawns = [hand(0, SEVEN_KID_GOATS), mon_set(1, GAZELLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Summon(SEVEN_KID_GOATS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", GAZELLE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Little Red Riding Hood: "When this card is destroyed by battle and sent to the Graveyard: you can Special Summon 1 Level
/// 4 or lower Warrior-Type monster from your Deck." Warwolf destroys her in turn 2; Dark Blade (Level 4 Warrior, the
/// Deck's last card) comes.
#[test]
fn little_red_riding_hood_summons_a_small_warrior_when_destroyed_by_battle() {
    let spawns = [mon(0, LITTLE_RED_RIDING_HOOD), mon(1, WARWOLF)];
    let Some(mut s) = field(&[DARK_BLADE], &spawns) else { return };
    keep_hands(&mut s);
    let deck = deck_size(&s, 0);
    s.run_both([&[Chain(LITTLE_RED_RIDING_HOOD)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LITTLE_RED_RIDING_HOOD), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade came from the Deck: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 1, "{}", s.context());
}

/// Little Red Riding Hood: "destroyed by battle": Dark Hole destroying her gives no effect, Dark Blade stays in the Deck.
#[test]
fn little_red_riding_hood_needs_a_battle_destruction() {
    let spawns = [mon(0, LITTLE_RED_RIDING_HOOD), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[DARK_BLADE], &spawns) else { return };
    let deck = deck_size(&s, 0);
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LITTLE_RED_RIDING_HOOD), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 0, "{}", s.context());
    assert_eq!(deck_size(&s, 0), deck, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Parasite Worm, Infestation
// ---------------------------------------------------------------------------------------------------------------

/// Parasite Worm: "When your opponent Summons exactly 1 monster: Destroy it and inflict damage to your opponent equal to its
/// ATK." The opponent's Normal Summon of Warwolf is destroyed and costs them 2000.
#[test]
fn parasite_worm_destroys_a_normal_summoned_monster_and_inflicts_its_atk() {
    let spawns = [set(0, PARASITE_WORM), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Chain(PARASITE_WORM)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", PARASITE_WORM), 1, "the Trap was used: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.lp(1), 6000, "its 2000 ATK as damage: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Parasite Worm answers a Special Summon too: Monster Reborn brings Warwolf back and it is destroyed with 2000 damage.
#[test]
fn parasite_worm_answers_a_special_summon() {
    let spawns = [set(0, PARASITE_WORM), hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(PARASITE_WORM)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", PARASITE_WORM), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "destroyed again: {}", s.context());
    assert_eq!(s.lp(1), 6000, "{}", s.context());
}

/// Parasite Worm: "exactly 1 monster": Toy Robot Box Special Summons three Robot Tokens at once and the Trap is not offered.
#[test]
fn parasite_worm_ignores_several_monsters_summoned_at_once() {
    let spawns = [set(0, PARASITE_WORM), hand(1, TOY_ROBOT_BOX)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(PARASITE_WORM)], &[Activate(TOY_ROBOT_BOX)]]).expect_err("three Tokens at once");
    assert!(err.contains("Chain(5940) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", PARASITE_WORM), 1, "still Set: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Parasite Worm: "When your opponent Summons": my own Summon is no reason to chain.
#[test]
fn parasite_worm_ignores_my_own_summon() {
    let spawns = [set(0, PARASITE_WORM), hand(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Summon(WARWOLF), Chain(PARASITE_WORM)]).expect_err("my own monster");
    assert!(err.contains("Chain(5940) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Infestation: "All face-up monsters lose 100 ATK. Then, each player takes 100 damage for each monster they control."
/// Team 0 has Warwolf and Thunder Kid, team 1 a face-up Dark Magician and a Set Gazelle: the face-up ones lose 100 ATK, I
/// take 200 and they take 200 (the Set monster counts, though it loses nothing).
#[test]
fn infestation_weakens_every_face_up_monster_and_hurts_each_player_per_monster() {
    let spawns = [set(0, INFESTATION), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, DARK_MAGICIAN), mon_set(1, GAZELLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(INFESTATION)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(1900), "{}", s.context());
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(600), "{}", s.context());
    assert_eq!(s.attack_of(1, DARK_MAGICIAN), Some(2400), "{}", s.context());
    assert_eq!(s.lp(0), 7800, "100 for each of my two monsters: {}", s.context());
    assert_eq!(s.lp(1), 7800, "100 for each of theirs: {}", s.context());
    assert_eq!(s.count(0, "grave", INFESTATION), 1, "{}", s.context());
}

/// Infestation: "face-up monsters": with no face-up monster on the field it is not offered, a Set one alone does not count.
#[test]
fn infestation_needs_a_face_up_monster() {
    let Some(mut s) = field(&[], &[set(0, INFESTATION), mon_set(1, GAZELLE)]) else { return };
    let err = s.run(&[Activate(INFESTATION)]).expect_err("nothing face-up");
    assert!(err.contains("Activate(5936) never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Mirage Ruler
// ---------------------------------------------------------------------------------------------------------------

/// Mirage Ruler is a Set Trap of team 0. Team 1's Warwolf attacks Thunder Kid in turn 2; the Trap is activated at the attack
/// and, once the Kid is destroyed (the only monster of mine), chained again from the field as a quick effect. Returns the
/// duel after that second chain resolved, or None when it was never offered.
fn ruler(extra_monster: Option<u16>) -> Option<Scenario> {
    let mut spawns = vec![set(0, MIRAGE_RULER), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    spawns.extend(extra_monster.map(|c| mon(0, c)));
    let mut s = field(&[], &spawns)?;
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (1, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[ChainOnAttack(MIRAGE_RULER), Chain(MIRAGE_RULER)], &[Attack(WARWOLF)]]).unwrap();
    Some(s)
}

/// Mirage Ruler: "Activate only when an opponent's monster declares an attack. During your opponent's Battle Phase, if all
/// monsters you control are destroyed: You can Tribute this card; return all monsters you controlled during this turn's
/// Battle Phase to the field, also, your LP become the amount you had at the start of this Battle Phase, then pay 1000 LP."
/// Warwolf destroys Thunder Kid (1300 damage: 6700 LP); the Ruler is Tributed, Thunder Kid is back and the LP are the
/// 8000 of the Battle Phase's start less 1000.
#[test]
fn mirage_ruler_returns_the_destroyed_monsters_and_resets_the_lp() {
    let Some(s) = ruler(None) else { return };
    clean(&s);
    assert_eq!(s.count(0, "grave", MIRAGE_RULER), 1, "the Ruler was Tributed: {}", s.context());
    assert_eq!(s.count(0, "szone", MIRAGE_RULER), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid is back: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 0, "{}", s.context());
    assert_eq!(s.lp(0), 7000, "the 8000 of the start of the Battle Phase, then 1000 paid: {}", s.context());
}

/// Mirage Ruler: "if all monsters you control are destroyed": with Dark Magician surviving next to Thunder Kid, the second
/// effect is not offered; the Ruler stays on the field and the Kid in the Graveyard.
#[test]
fn mirage_ruler_needs_all_my_monsters_destroyed() {
    let spawns = [set(0, MIRAGE_RULER), mon(0, THUNDER_KID), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (1, WARWOLF), (0, FILLER), (1, FILLER)]);
    let err = s.run_both([&[ChainOnAttack(MIRAGE_RULER), Chain(MIRAGE_RULER)], &[Attack(WARWOLF)]]).expect_err("Dark Magician survives");
    assert!(err.contains("Chain(5864) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", MIRAGE_RULER), 1, "the Ruler is on the field: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
}

/// Mirage Ruler: "Activate only when an opponent's monster declares an attack": my own attack gives no activation.
#[test]
fn mirage_ruler_answers_only_an_opponents_attack() {
    let spawns = [set(0, MIRAGE_RULER), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Attack(WARWOLF), ChainOnAttack(MIRAGE_RULER)]).expect_err("my own attack");
    assert!(err.contains("ChainOnAttack(5864) never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", MIRAGE_RULER), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Armored Centipede, Brave Attack, Lengard
// ---------------------------------------------------------------------------------------------------------------

/// Armored Centipede: "If this card destroys an Insect-Type monster by battle, it gains 500 ATK." It (1600) destroys Basic
/// Insect (500) in turn 3 and has 2100.
#[test]
fn armored_centipede_gains_500_atk_for_destroying_an_insect() {
    let spawns = [mon(0, ARMORED_CENTIPEDE), mon(1, BASIC_INSECT)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    assert_eq!(s.attack_of(0, ARMORED_CENTIPEDE), Some(1600), "{}", s.context());
    s.run(&[Attack(ARMORED_CENTIPEDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BASIC_INSECT), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, ARMORED_CENTIPEDE), Some(2100), "+500: {}", s.context());
}

/// Armored Centipede: "Insect-Type": destroying a Thunder Kid by battle gives nothing.
#[test]
fn armored_centipede_gains_nothing_for_another_type() {
    let spawns = [mon(0, ARMORED_CENTIPEDE), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Attack(ARMORED_CENTIPEDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, ARMORED_CENTIPEDE), Some(1600), "{}", s.context());
}

/// Brave Attack: "Target 1 Attack Position monster you control; it gains the ATK of all Attack Position monsters you control,
/// and attacks 1 of monster your opponent controls. During the End Phase, destroy all face-up Attack Position monsters you
/// control. You cannot conduct your Battle Phase the turn you activate this effect." In turn 3 Warwolf (2000) is the target
/// and gains the 700 and 1000 of Thunder Kid and Dokuroyaiba: 3700 against Dark Magician (2500): it is destroyed and the
/// opponent takes 1200. The Battle Phase is closed afterwards, and the End Phase destroys my three monsters.
#[test]
fn brave_attack_adds_the_atk_of_the_others_attacks_and_destroys_my_monsters_at_the_end() {
    let spawns = [hand(0, BRAVE_ATTACK), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(0, DOKUROYAIBA), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (1, DARK_MAGICIAN), (0, FILLER), (1, FILLER)]);
    s.run(&[EndTurn, Activate(BRAVE_ATTACK)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "Dark Magician was destroyed in the battle: {}", s.context());
    assert_eq!(s.lp(1), 6800, "3700 against 2500: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(3700), "{}", s.context());
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase this turn: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    for cid in [WARWOLF, THUNDER_KID, DOKUROYAIBA] {
        assert_eq!(s.count(0, "grave", cid), 1, "{cid} was destroyed in the End Phase: {}", s.context());
    }
}

/// Brave Attack: "Target 1 Attack Position monster you control" and "all other Attack Position monsters": with one monster
/// only there is nothing to add and it is not offered.
#[test]
fn brave_attack_needs_two_attack_position_monsters() {
    let Some(mut s) = field(&[], &[hand(0, BRAVE_ATTACK), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    let err = s.run(&[Activate(BRAVE_ATTACK)]).expect_err("one monster only");
    assert!(err.contains("Activate(5808) never became possible"), "{err}");
    clean(&s);
}

/// Lengard: "If you would take battle damage from a battle involving a monster you control, you can destroy this face-up
/// card instead." Warwolf (2000) attacks Thunder Kid (700) in turn 2: the 1300 damage is paid by Lengard, who is destroyed
/// (the Kid is destroyed by the battle as well).
#[test]
fn lengard_takes_the_battle_damage_in_my_place_by_being_destroyed() {
    let spawns = [mon(0, LENGARD), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (0, FILLER), (1, FILLER)]);
    s.run_both([&[Chain(LENGARD)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LENGARD), 1, "Lengard was destroyed instead: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the battle still destroyed the Kid: {}", s.context());
    assert_eq!(s.lp(0), 8000, "and no damage was taken: {}", s.context());
}

/// Lengard, the control: without Lengard the same battle costs 1300 LP.
#[test]
fn without_lengard_the_battle_damage_is_taken() {
    let spawns = [mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6700, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Rainbow Blessing (DM) and Spell Textbook
// ---------------------------------------------------------------------------------------------------------------

/// Rainbow Blessing (DM): "Equip only to "Kuriboh". You can Tribute the equipped monster; monsters you control can attack
/// directly this turn." It is equipped to Kuriboh in turn 1; in turn 3 Kuriboh is Tributed and Warwolf attacks directly
/// although Dark Magician stands on the other side.
#[test]
fn rainbow_blessing_lets_a_monster_attack_directly_after_tributing_kuriboh() {
    let spawns = [hand(0, RAINBOW_BLESSING), mon(0, KURIBOH), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, KURIBOH), (0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(RAINBOW_BLESSING)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", RAINBOW_BLESSING), 1, "equipped to Kuriboh: {}", s.context());
    s.run(&[EndTurn, Activate(RAINBOW_BLESSING)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "the equipped Kuriboh was Tributed: {}", s.context());
    s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD && p["to_bp"] == true).unwrap();
    s.respond(serde_json::json!({"type": 1, "action": idle::TO_BP, "index": null})).unwrap();
    while s.prompt()["type"] != msg::SELECT_BATTLECMD {
        s.pass().unwrap();
    }
    let attack = s.prompt()["attacks"].as_array().and_then(|a| a.iter().find(|a| a["cid"] == WARWOLF)).cloned();
    assert_eq!(attack.as_ref().map(|a| a["can_direct"].clone()), Some(true.into()), "Warwolf may attack directly: {}", s.context());
}

/// Rainbow Blessing (DM): "Equip only to "Kuriboh"": with only a Warwolf to carry it, it cannot be activated.
#[test]
fn rainbow_blessing_equips_only_to_kuriboh() {
    let Some(mut s) = field(&[], &[hand(0, RAINBOW_BLESSING), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(RAINBOW_BLESSING)]).expect_err("no Kuriboh");
    assert!(err.contains("Activate(5865) never became possible"), "{err}");
    clean(&s);
}

/// Spell Textbook: "Discard all cards in your hand (if any); Shuffle your Deck, then draw 1 card and reveal it. If it is a
/// Spell, activate it." A Deck of Dian Keto only: the hand is discarded, the card drawn is a Spell and is activated: 1000 LP.
#[test]
fn spell_textbook_activates_a_spell_it_draws() {
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &[hand(0, SPELL_TEXTBOOK)]) else { return };
    let before = hand_size(&s, 0);
    assert!(before > 1, "{}", s.context());
    s.run(&[Activate(SPELL_TEXTBOOK)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 9000, "the drawn Dian Keto was activated: {}", s.context());
    assert_eq!(hand_size(&s, 0), 0, "the hand was discarded and the drawn card played: {}", s.context());
    assert_eq!(s.count(0, "grave", DIAN_KETO), before, "{before} cards: the discards and the drawn one: {}", s.context());
    assert_eq!(s.count(0, "grave", SPELL_TEXTBOOK), 1, "{}", s.context());
}

/// Spell Textbook: "If it is a Spell": a monster is drawn and stays in the hand.
#[test]
fn spell_textbook_keeps_a_monster_it_draws() {
    let Some(mut s) = field(&[], &[hand(0, SPELL_TEXTBOOK), hand(0, DIAN_KETO)]) else { return };
    s.run(&[Activate(SPELL_TEXTBOOK)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "Dian Keto was discarded with the rest: {}", s.context());
    assert_eq!(hand_size(&s, 0), 1, "only the card drawn is left: {}", s.context());
    assert_eq!(s.count(0, "hand", FILLER), 1, "a monster, not played: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Turn Jump (Anime)
// ---------------------------------------------------------------------------------------------------------------

/// Turn Jump (Anime): "During the Battle Phase: Make each player end their next respective 3 turns and resume the Duel at
/// that player's Battle Phase." Team 0 chains it at the start of its turn 3's Battle Phase: six turns go by at once (the
/// opponent's first, then mine, three each), with no draw in them, and the duel is back at my Battle Phase, with Warwolf's
/// attack on offer and nothing lost. The turn after that goes on as usual: the opponent draws and has a Main Phase.
#[test]
fn turn_jump_skips_three_turns_of_each_player_and_resumes_at_the_battle_phase() {
    let spawns = [hand(0, TURN_JUMP), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert!(!offered(&s, "activates").contains(&TURN_JUMP.into()), "not in the Main Phase: {:?}", offered(&s, "activates"));
    s.respond(serde_json::json!({"type": 1, "action": idle::TO_BP, "index": null})).unwrap();
    let (hands, decks) = ([hand_size(&s, 0), hand_size(&s, 1)], [deck_size(&s, 0), deck_size(&s, 1)]);
    // Chain it at the start of the Battle Phase, then let everything else pass.
    let (mut turns, mut draws) = (vec![], 0);
    let mut chained = false;
    for _ in 0..40 {
        let p = s.prompt().clone();
        let at = p["selects"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == TURN_JUMP));
        if p["type"] == msg::SELECT_CHAIN && p["player"] == 0 && at.is_some() && !chained {
            chained = true;
            s.respond(yo3_server::ocg::Response::Chain(at).json()).unwrap();
        } else if p["type"] == msg::SELECT_BATTLECMD && chained {
            break;
        } else {
            s.pass().unwrap();
        }
        for m in s.messages() {
            match m["type"].as_u64() {
                Some(40) => turns.push(m["player"].as_u64().unwrap()),
                Some(90) => draws += 1,
                _ => {}
            }
        }
    }
    clean(&s);
    assert!(chained, "Turn Jump was on offer in the Battle Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", TURN_JUMP), 1, "{}", s.context());
    assert_eq!(turns, [1, 0, 1, 0, 1, 0], "three turns of each player, the opponent's first: {}", s.context());
    assert_eq!(draws, 0, "no draw in the skipped turns: {}", s.context());
    assert_eq!(([hand_size(&s, 0), hand_size(&s, 1)], [deck_size(&s, 0), deck_size(&s, 1)]), ([hands[0] - 1, hands[1]], decks), "{}", s.context());
    assert_eq!((s.prompt()["type"].clone(), s.prompt()["player"].clone()), (msg::SELECT_BATTLECMD.into(), 0.into()), "back at my Battle Phase: {}", s.context());
    assert_eq!(offered(&s, "attacks"), [WARWOLF], "Warwolf can attack: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    // The Battle Phase is played out and the next turn is the opponent's, a normal one: it draws and reaches its Main Phase.
    s.respond(serde_json::json!({"type": 1, "action": battle::ATTACK, "index": 0})).unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "the attack went through: {}", s.context());
    assert_eq!(s.lp(1), 6700, "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.prompt()["player"], 1, "the opponent's turn: {}", s.context());
    assert_eq!(hand_size(&s, 1), hands[1] + 1, "which draws as usual: {}", s.context());
}

/// Turn Jump (Anime): "During the Battle Phase": in the Main Phase it is not offered.
#[test]
fn turn_jump_is_not_offered_outside_the_battle_phase() {
    let Some(mut s) = field(&[], &[hand(0, TURN_JUMP), mon(0, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert_eq!(s.count(0, "hand", TURN_JUMP), 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&TURN_JUMP.into()), "{:?}", offered(&s, "activates"));
}
