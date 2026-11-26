//! Anime-only cards, group 2 (rows 5259-5296 of engine/added/cards.tsv): the Robot, Tank and Player Tokens, Aura Armor,
//! Burning Soul Sword, Celebration of Creation, Compensation Mediation, Hyper Refresh, Silver Dollar, Take One Chance,
//! Clone Dragon, Flat LV4, Power Balance, Thirst for Compensation, Aetonyx Flame, Air Fortress Ziggurat, Fire Whip,
//! Gorlag, Junk Dealer (Manga), KC-1 Crayton, Sky Union, Soldier Revolt, Tank Corps and Toy Robot Box. These cards exist
//! only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through
//! the real core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime
//! ones. The last tests keep the client's view equal to the engine's where three translator fixes were made: Bounce Spell
//! moving a Spell with counters, a former Synchro material placed on the field, and Take One Chance's "if able".
//! Not covered here (outside this group's sample of the batch): Attrition, Landstar Forces, Roll of Fate, Star Blaster,
//! Vow of Tribe, Card Guard (Anime), Charm of Lamentation, Command Silencer, Negative Energy Generator, Reflect Energy,
//! Spell Sanctuary, Sword of Soul, Wish of Final Effort and Contagion of Madness. Every Trap here is Set first (a Trap
//! cannot be activated from the hand); neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The cards of this group (client ids).
const ROBOT_TOKEN: u16 = 5469; // Machine/WIND/Level 1, 0/0
const TANK_TOKEN: u16 = 5470; // Machine/EARTH/Level 4, 500/1200
const PLAYER_TOKEN: u16 = 5471; // Warrior/EARTH/Level 4, ATK and DEF = its controller's LP
const AURA_ARMOR: u16 = 5495;
const BURNING_SOUL_SWORD: u16 = 5500;
const CELEBRATION_OF_CREATION: u16 = 5563;
const COMPENSATION_MEDIATION: u16 = 5565;
const HYPER_REFRESH: u16 = 5566;
const SILVER_DOLLAR: u16 = 5569;
const TAKE_ONE_CHANCE: u16 = 5571;
const CLONE_DRAGON: u16 = 5634;
const FLAT_LV4: u16 = 5639;
const POWER_BALANCE: u16 = 5689;
const THIRST_FOR_COMPENSATION: u16 = 5693;
const AETONYX_FLAME: u16 = 5712;
const ZIGGURAT: u16 = 5742; // Air Fortress Ziggurat
const FIRE_WHIP: u16 = 5745;
const GORLAG: u16 = 5746; // FIRE Fiend, Level 4, 1000/1000
const JUNK_DEALER: u16 = 5747; // Junk Dealer (Manga)
const CRAYTON: u16 = 5748; // KC-1 Crayton: Level 4 Machine, 1500/1200
const SKY_UNION: u16 = 5749;
const SOLDIER_REVOLT: u16 = 5750;
const TANK_CORPS: u16 = 5751;
const TOY_ROBOT_BOX: u16 = 5801;
// Anime cards of group 1 and the printed anime cards used here.
const BOUNCE_SPELL: u16 = 5187;
const FULL_MOON_MIRROR: u16 = 5158; // the anime Full Moon Mirror: a Full Moon Counter for each face-up monster destroyed
// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // WIND, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const BATTLE_STEER: u16 = 4020; // Level 5, 1800/1300, vanilla: one Tribute
const DARK_HOLE: u16 = 4342;
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const POT_OF_GREED: u16 = 4844; // draw 2
const MONSTER_REBORN: u16 = 4842;
const TRIBUTE_TO_THE_DOOMED: u16 = 4856; // discard 1: destroy 1 monster
const DOKUROYAIBA: u16 = 5150; // a FIRE Fiend, Level 3, 1000/400, vanilla
const POLYMERIZATION: u16 = 4837;
const GAIA: u16 = 4044; // Gaia The Fierce Knight, Level 7, 2300/2100
const CURSE_OF_DRAGON: u16 = 4045; // Level 5, 2000/1500
const GAIA_CHAMPION: u16 = 4043; // Gaia the Dragon Champion, the Fusion of those two
const SCIENCE_SOLDIER: u16 = 5148;
const CYBER_SOLDIER: u16 = 4303; // Cyber Soldier of Darkworld
const CIPHER_SOLDIER: u16 = 5034; // the anime's "Kinetic Soldier" (script 511001897 lists Cipher Soldier, 79853073)
const BATTLE_FUSION: u16 = 11162; // played as Battle Fusion (Anime)
const JUNK_SYNCHRON: u16 = 7687; // a Level 3 Tuner
const GIANT_SOLDIER: u16 = 4080; // Giant Soldier of Stone: Level 3, 1300/2000
const GAIA_KNIGHT: u16 = 7697; // Gaia Knight, the Force of Earth: Level 6 Synchro (Junk Synchron + a Level 3)

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: team 0's has `bottom` at its very
/// bottom (out of the opening hand and the first draws), for a card an effect fetches from the Deck.
fn duel(seed: u64, lp: u32, fillers: [u16; 2], bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    duel_with_extra(seed, lp, fillers, bottom, [&[], &[]], spawns)
}

/// [`duel`] with an Extra Deck for each team (the Fusion and Synchro Monsters the scenario summons).
fn duel_with_extra(seed: u64, lp: u32, fillers: [u16; 2], bottom: &[u16], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(fillers[0], 40 - bottom.len())).collect();
    let decks = [(&mine[..], extra[0]), (&vec![fillers[1]; 40][..], extra[1])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler. Neither player may attack in turn 1 (the first player has no Battle
/// Phase), so a plan that attacks with team 0 waits for turn 3 and each End Phase before it discards down to 6 cards: tests
/// that keep a card in hand across those turns name the fillers as the cards to discard (`keep_hands`).
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

/// How many cards a team's grave holds.
fn grave_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["grave"].as_array().map_or(0, Vec::len)
}

/// The Monster Zone cards of one id, as the engine's state shows them.
fn monsters(s: &Scenario, team: usize, cid: u16) -> Vec<&serde_json::Value> {
    s.state()[team]["mzone"].as_array().map_or(vec![], |z| z.iter().filter(|c| c["cid"] == cid).collect())
}

// ---------------------------------------------------------------------------------------------------------------
// Aura Armor and its Player Token
// ---------------------------------------------------------------------------------------------------------------

/// Aura Armor: "When an opponent's monster declares a direct attack while you have 2000 or less LP: Negate the attack, and
/// Special Summon 1 "Player Token" (Warrior-Type/EARTH/Level 4/ATK ?/DEF ?). When that "Player Token" is Special Summoned,
/// halve your LP. The ATK/DEF of the "Player Token" are equal to your LP." With 2000 LP, Warwolf's direct attack is
/// negated, the LP drop to 1000 and the Player Token has 1000/1000; Dian Keto's 1000 LP afterwards make it 2000/2000.
#[test]
fn aura_armor_negates_a_direct_attack_and_makes_a_player_token_equal_to_the_lp() {
    let spawns = [set(0, AURA_ARMOR), hand(0, DIAN_KETO), hand(1, WARWOLF)];
    let Some(mut s) = duel(7, 2000, [FILLER, FILLER], &[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(AURA_ARMOR)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", AURA_ARMOR), 1, "the Trap was used: {}", s.context());
    assert_eq!(s.lp(0), 1000, "halved when the Token was Special Summoned, the attack did no damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", PLAYER_TOKEN), 1, "the Player Token: {}", s.context());
    let token = s.monster(0, PLAYER_TOKEN).unwrap().clone();
    assert_eq!((token["attack"].as_i64(), token["defense"].as_i64()), (Some(1000), Some(1000)), "equal to the LP: {}", s.context());
    assert_eq!((s.level_of(0, PLAYER_TOKEN), s.position_of(0, PLAYER_TOKEN)), (Some(4), Some(1)), "Level 4, face-up Attack: {}", s.context());
    assert_eq!(s.lp(1), 2000, "{}", s.context());
    // The ATK and DEF follow the LP.
    s.run_both([&[Activate(DIAN_KETO)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 2000, "{}", s.context());
    assert_eq!(s.attack_of(0, PLAYER_TOKEN), Some(2000), "the Token follows the LP: {}", s.context());
}

/// Aura Armor: "while you have 2000 or less LP": with 8000 LP the direct attack is not answered and goes through.
#[test]
fn aura_armor_needs_2000_lp_or_less() {
    let spawns = [set(0, AURA_ARMOR), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[ChainOnAttack(AURA_ARMOR)], &[Summon(WARWOLF), Attack(WARWOLF)]]).expect_err("not chainable");
    assert!(err.contains("ChainOnAttack(5495) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.lp(0), 6000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", PLAYER_TOKEN), 0, "{}", s.context());
}

/// The Player Token is a Token: when it is destroyed it goes to no Graveyard. Dark Hole destroys it.
#[test]
fn the_player_token_leaves_no_card_behind() {
    let spawns = [set(0, AURA_ARMOR), hand(1, WARWOLF), hand(1, DARK_HOLE)];
    let Some(mut s) = duel(7, 2000, [FILLER, FILLER], &[], &spawns) else { return };
    s.run_both([&[ChainOnAttack(AURA_ARMOR)], &[Summon(WARWOLF), Attack(WARWOLF), Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PLAYER_TOKEN), 0, "Dark Hole destroyed the Token: {}", s.context());
    assert_eq!(s.count(0, "grave", PLAYER_TOKEN), 0, "and it is in no Graveyard: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Hyper Refresh, Power Balance
// ---------------------------------------------------------------------------------------------------------------

/// Hyper Refresh: "If the combined ATK of monsters your opponent controls is higher than your LP while you control no
/// monsters: Double your LP." With 1500 LP against Warwolf's 2000 and no monster: 3000.
#[test]
fn hyper_refresh_doubles_the_lp() {
    let spawns = [hand(0, HYPER_REFRESH), mon(1, WARWOLF)];
    let Some(mut s) = duel(7, 1500, [FILLER, FILLER], &[], &spawns) else { return };
    s.run(&[Activate(HYPER_REFRESH)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 3000, "doubled: {}", s.context());
    assert_eq!(s.lp(1), 1500, "{}", s.context());
    assert_eq!(s.count(0, "grave", HYPER_REFRESH), 1, "{}", s.context());
}

/// Hyper Refresh: "while you control no monsters": with a monster of your own it cannot be activated.
#[test]
fn hyper_refresh_needs_an_empty_field() {
    let spawns = [hand(0, HYPER_REFRESH), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = duel(7, 1500, [FILLER, FILLER], &[], &spawns) else { return };
    let err = s.run(&[Activate(HYPER_REFRESH)]).expect_err("a monster of mine");
    assert!(err.contains("Activate(5566) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.lp(0), 1500, "{}", s.context());
}

/// Hyper Refresh: "higher than your LP": the opponent's 2000 ATK against 2500 LP is not enough.
#[test]
fn hyper_refresh_needs_the_opponents_atk_above_the_lp() {
    let spawns = [hand(0, HYPER_REFRESH), mon(1, WARWOLF)];
    let Some(mut s) = duel(7, 2500, [FILLER, FILLER], &[], &spawns) else { return };
    let err = s.run(&[Activate(HYPER_REFRESH)]).expect_err("2000 is not above 2500");
    assert!(err.contains("Activate(5566) never became possible"), "{err}");
    clean(&s);
}

/// Power Balance: "While you have no cards in your hand: Your opponent discards half the cards in their hand, then, draw
/// cards equal to the number of cards your opponent discarded." Team 0's Deck is all Dian Keto: the six cards of its hand
/// are played, then the Set Power Balance empties the opponent's six cards to three, and team 0 draws three.
#[test]
fn power_balance_makes_the_opponent_discard_half_and_draws_that_many() {
    let spawns = [set(0, POWER_BALANCE), hand(1, FILLER)];
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &spawns) else { return };
    let plan = [Activate(DIAN_KETO); 6];
    s.run(&plan).unwrap();
    assert_eq!(hand_size(&s, 0), 0, "my hand is empty: {}", s.context());
    assert_eq!(hand_size(&s, 1), 6, "{}", s.context());
    s.run(&[Activate(POWER_BALANCE)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 1), 3, "the opponent discarded half: {}", s.context());
    assert_eq!(grave_size(&s, 1), 3, "{}", s.context());
    assert_eq!(hand_size(&s, 0), 3, "and I drew as many: {}", s.context());
}

/// Power Balance: "While you have no cards in your hand": with a card in hand it is not offered.
#[test]
fn power_balance_needs_an_empty_hand() {
    let spawns = [set(0, POWER_BALANCE), hand(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(POWER_BALANCE)]).expect_err("cards in hand");
    assert!(err.contains("Activate(5689) never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Sky Union, Air Fortress Ziggurat and the Robot Token
// ---------------------------------------------------------------------------------------------------------------

/// Sky Union: "Tribute 3 monsters you control; Special Summon 1 "Air Fortress Ziggurat" from your Deck." Three Thunder
/// Kids are Tributed and Ziggurat (Level 8, 2500/800) comes from the bottom of the Deck.
#[test]
fn sky_union_tributes_three_monsters_for_ziggurat_from_the_deck() {
    let spawns = [hand(0, SKY_UNION), mon(0, THUNDER_KID), mon(0, THUNDER_KID), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[ZIGGURAT], &spawns) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Activate(SKY_UNION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 3, "three Tributes: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 0, "{}", s.context());
    assert_eq!(s.attack_of(0, ZIGGURAT), Some(2500), "Ziggurat is on the field: {}", s.context());
    assert_eq!(s.level_of(0, ZIGGURAT), Some(8), "{}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 1, "from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", SKY_UNION), 1, "{}", s.context());
}

/// Sky Union: "Tribute 3 monsters": with two monsters it cannot be activated.
#[test]
fn sky_union_needs_three_monsters() {
    let spawns = [hand(0, SKY_UNION), mon(0, THUNDER_KID), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[ZIGGURAT], &spawns) else { return };
    let err = s.run(&[Activate(SKY_UNION)]).expect_err("two Tributes only");
    assert!(err.contains("Activate(5749) never became possible"), "{err}");
    clean(&s);
}

/// Air Fortress Ziggurat: "Once per turn, during your End Phase: Special Summon 1 "Robot Token" (Machine-Type/WIND/Level
/// 1/ATK 0/DEF 0) in Defense Position". Sky Union brings Ziggurat, the End Phase brings the Robot Token.
#[test]
fn ziggurat_makes_a_robot_token_in_the_end_phase() {
    let spawns = [hand(0, SKY_UNION), mon(0, THUNDER_KID), mon(0, THUNDER_KID), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[ZIGGURAT], &spawns) else { return };
    s.run(&[Activate(SKY_UNION)]).unwrap();
    assert_eq!(s.count(0, "mzone", ROBOT_TOKEN), 0, "none before the End Phase: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ROBOT_TOKEN), 1, "the End Phase's Robot Token: {}", s.context());
    assert_eq!(s.position_of(0, ROBOT_TOKEN), Some(4), "in Defense Position: {}", s.context());
    assert_eq!((s.level_of(0, ROBOT_TOKEN), s.attack_of(0, ROBOT_TOKEN)), (Some(1), Some(0)), "{}", s.context());
    assert_eq!(s.monster(0, ROBOT_TOKEN).unwrap()["defense"], 0, "{}", s.context());
}

/// Air Fortress Ziggurat: "Unaffected by the effects of your opponent's Spells/Traps." The opponent's Dark Hole destroys
/// the End Phase's Robot Token and leaves Ziggurat; their own Warwolf is destroyed.
#[test]
fn ziggurat_is_unaffected_by_the_opponents_spells() {
    let spawns = [hand(0, SKY_UNION), mon(0, THUNDER_KID), mon(0, THUNDER_KID), mon(0, THUNDER_KID), mon(1, WARWOLF), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[ZIGGURAT], &spawns) else { return };
    s.run_both([&[Activate(SKY_UNION), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZIGGURAT), 1, "Ziggurat is unaffected: {}", s.context());
    assert_eq!(s.count(0, "mzone", ROBOT_TOKEN), 0, "the Token is not: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Air Fortress Ziggurat: "While you control a "Robot Token", your opponent cannot target monsters you control for an
/// attack, except "Robot Tokens"." Warwolf is told to attack Ziggurat and cannot: it destroys the Robot Token, and
/// Ziggurat (and the LP) stay as they were.
#[test]
fn ziggurat_lets_only_the_robot_token_be_attacked() {
    let spawns = [hand(0, SKY_UNION), mon(0, THUNDER_KID), mon(0, THUNDER_KID), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[ZIGGURAT], &spawns) else { return };
    s.prefer(&[(0, ZIGGURAT)]);
    s.run_both([&[Activate(SKY_UNION), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ROBOT_TOKEN), 0, "the Robot Token took the attack: {}", s.context());
    assert_eq!(s.count(0, "mzone", ZIGGURAT), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage to a Defense Position Token of 0 DEF: {}", s.context());
}

/// Toy Robot Box: "Discard 1 card; Special Summon 3 "Robot Tokens" (Machine-Type/WIND/Level 1/ATK 0/DEF 0), but they cannot
/// declare an attack." Three Robot Tokens come, one hand card is discarded and the Box is spent.
#[test]
fn toy_robot_box_summons_three_robot_tokens() {
    let spawns = [hand(0, TOY_ROBOT_BOX)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let hand_before = hand_size(&s, 0);
    s.run(&[Activate(TOY_ROBOT_BOX)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ROBOT_TOKEN), 3, "three Robot Tokens: {}", s.context());
    assert_eq!((s.level_of(0, ROBOT_TOKEN), s.attack_of(0, ROBOT_TOKEN)), (Some(1), Some(0)), "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "one card discarded as the cost: {}", s.context());
    assert_eq!(s.count(0, "grave", TOY_ROBOT_BOX), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before - 2, "the Box and the discard left the hand: {}", s.context());
}

/// Toy Robot Box: "Discard 1 card": with nothing else in the hand it is not on offer (the Deck is all Dian Keto: the six
/// cards drawn are played, only the Box is left).
#[test]
fn toy_robot_box_needs_a_card_to_discard() {
    let Some(mut s) = duel(7, 8000, [DIAN_KETO, FILLER], &[], &[hand(0, TOY_ROBOT_BOX)]) else { return };
    s.run(&[Activate(DIAN_KETO); 6]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), 1, "only the Box is left: {}", s.context());
    assert_eq!(s.count(0, "hand", TOY_ROBOT_BOX), 1, "{}", s.context());
    let offered: Vec<_> = s.prompt()["activates"].as_array().unwrap().iter().map(|a| a["cid"].clone()).collect();
    assert!(!offered.contains(&TOY_ROBOT_BOX.into()), "the Box is not on offer: {offered:?}");
}

/// Toy Robot Box: "but they cannot declare an attack": in the next own turn Dark Magician can attack, a Robot Token cannot.
#[test]
fn robot_tokens_cannot_declare_an_attack() {
    let spawns = [hand(0, TOY_ROBOT_BOX), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(TOY_ROBOT_BOX), EndTurn, Attack(ROBOT_TOKEN)]).expect_err("the token cannot attack");
    assert!(err.contains("can't attack"), "{err}");
    assert_eq!(s.count(0, "mzone", ROBOT_TOKEN), 3, "the tokens are there: {}", s.context());
}

/// Toy Robot Box: "While you control a "Robot Token", your opponent cannot target another monster you control for an
/// attack, except "Robot Tokens"." Warwolf is told to attack Dark Magician and can attack only a Robot Token: it destroys
/// one (the Tokens stand in Attack Position with 0 ATK, so 2000 battle damage) and Dark Magician is untouched.
#[test]
fn robot_tokens_shield_the_other_monsters_from_attacks() {
    let spawns = [hand(0, TOY_ROBOT_BOX), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run_both([&[Activate(TOY_ROBOT_BOX), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ROBOT_TOKEN), 2, "one Robot Token was attacked and destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "Warwolf won: {}", s.context());
    assert_eq!(s.lp(0), 6000, "2000 battle damage from a 0 ATK Token in Attack Position: {}", s.context());
}

/// The shield is only up while a Robot Token stands: with none (no Box), Warwolf attacks Dark Magician as it likes.
#[test]
fn without_a_robot_token_the_monster_can_be_attacked() {
    let spawns = [mon(0, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf attacked Dark Magician and lost: {}", s.context());
    assert_eq!(s.lp(1), 7500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Tank Corps, KC-1 Crayton and the Tank Token
// ---------------------------------------------------------------------------------------------------------------

/// Tank Corps: "Activate only while "KC-1 Crayton" is on the field. Special Summon 3 "Tank Tokens" (Machine-Type/EARTH/
/// Level 4/ATK 500/DEF 1200) in Defense Position." KC-1 Crayton: "This card gains 500 ATK for each "Tank Token" you
/// control." Three Tank Tokens in Defense Position and Crayton at 1500 + 1500.
#[test]
fn tank_corps_summons_three_tank_tokens_and_crayton_gains_500_for_each() {
    let spawns = [set(0, TANK_CORPS), mon(0, CRAYTON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    assert_eq!(s.attack_of(0, CRAYTON), Some(1500), "{}", s.context());
    s.run(&[Activate(TANK_CORPS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TANK_TOKEN), 3, "three Tank Tokens: {}", s.context());
    assert_eq!(s.position_of(0, TANK_TOKEN), Some(4), "in Defense Position: {}", s.context());
    assert_eq!((s.level_of(0, TANK_TOKEN), s.attack_of(0, TANK_TOKEN)), (Some(4), Some(500)), "{}", s.context());
    assert_eq!(s.monster(0, TANK_TOKEN).unwrap()["defense"], 1200, "{}", s.context());
    assert_eq!(s.attack_of(0, CRAYTON), Some(3000), "+500 for each of the three: {}", s.context());
    assert_eq!(s.count(0, "grave", TANK_CORPS), 1, "{}", s.context());
}

/// The Tank Tokens are Tokens: the opponent's Dark Hole destroys them with Crayton, and they go to no Graveyard (only
/// Crayton does).
#[test]
fn tank_tokens_leave_no_card_behind_when_destroyed() {
    let spawns = [set(0, TANK_CORPS), mon(0, CRAYTON), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(TANK_CORPS)]).unwrap();
    assert_eq!(s.attack_of(0, CRAYTON), Some(3000), "{}", s.context());
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TANK_TOKEN) + s.count(0, "grave", TANK_TOKEN), 0, "no Token left anywhere: {}", s.context());
    assert_eq!(s.count(0, "grave", CRAYTON), 1, "Crayton died to Dark Hole too: {}", s.context());
}

/// Tank Corps: "Activate only while "KC-1 Crayton" is on the field": with no Crayton it is not offered.
#[test]
fn tank_corps_needs_crayton_on_the_field() {
    let Some(mut s) = field(&[], &[set(0, TANK_CORPS), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(TANK_CORPS)]).expect_err("no Crayton");
    assert!(err.contains("Activate(5751) never became possible"), "{err}");
    clean(&s);
}

/// Tank Corps: "They cannot be Tributed for a Tribute Summon." Crayton is the opponent's (either field counts), so the
/// three Tank Tokens are all I have: Battle Steer needs one Tribute and cannot be Summoned.
#[test]
fn tank_tokens_cannot_be_tributed_for_a_tribute_summon() {
    let spawns = [set(0, TANK_CORPS), hand(0, BATTLE_STEER), mon(1, CRAYTON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(TANK_CORPS), Summon(BATTLE_STEER)]).expect_err("nothing to Tribute");
    assert!(err.contains("Summon(4020) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", TANK_TOKEN), 3, "{}", s.context());
    assert_eq!(s.attack_of(1, CRAYTON), Some(1500), "the opponent's Crayton counts only its own Tokens: {}", s.context());
}

/// The control of the Tribute test: with a Warwolf of mine next to the Tokens, Battle Steer is Tribute Summoned on it.
#[test]
fn a_monster_next_to_the_tank_tokens_can_still_be_tributed() {
    let spawns = [set(0, TANK_CORPS), hand(0, BATTLE_STEER), mon(1, CRAYTON), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(TANK_CORPS), Summon(BATTLE_STEER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BATTLE_STEER), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was the Tribute: {}", s.context());
    assert_eq!(s.count(0, "mzone", TANK_TOKEN), 3, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Clone Dragon
// ---------------------------------------------------------------------------------------------------------------

/// Clone Dragon: "When you Normal Summoned exactly 1 monster: You can Special Summon this card from your hand, and if you
/// do, its name, original ATK, and DEF become the same as the Summoned monster's." Warwolf (2000/100) is Normal Summoned
/// and the Dragon becomes another Warwolf: 2000/100 (the name is changed by EFFECT_CHANGE_CODE, which the engine's state does not show).
#[test]
fn clone_dragon_becomes_a_copy_of_the_summoned_monster() {
    let spawns = [hand(0, CLONE_DRAGON), hand(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Summon(WARWOLF), Chain(CLONE_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", CLONE_DRAGON), 0, "it left the hand: {}", s.context());
    let clone = s.monster(0, CLONE_DRAGON).unwrap_or_else(|| panic!("Clone Dragon is on the field: {}", s.context())).clone();
    assert_eq!((clone["attack"].as_i64(), clone["defense"].as_i64()), (Some(2000), Some(100)), "Warwolf's stats: {}", s.context());
}

/// Clone Dragon: "This card cannot declare an attack unless you pay 1000 LP." Attacking with it costs 1000 LP.
#[test]
fn clone_dragon_pays_1000_lp_to_attack() {
    let Some(mut s) = field(&[], &[mon(0, CLONE_DRAGON)]) else { return };
    s.run(&[Attack(CLONE_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "1000 LP paid for the attack: {}", s.context());
}

/// Clone Dragon: with less than 1000 LP the cost cannot be paid and the attack is not on offer.
#[test]
fn clone_dragon_cannot_attack_without_1000_lp() {
    let Some(mut s) = duel(7, 900, [FILLER, FILLER], &[], &[mon(0, CLONE_DRAGON), mon(0, THUNDER_KID)]) else { return };
    let err = s.run(&[Attack(CLONE_DRAGON)]).expect_err("cannot pay");
    assert!(err.contains("can't attack"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Gorlag, Fire Whip, Aetonyx Flame
// ---------------------------------------------------------------------------------------------------------------

/// Gorlag: "Gains 500 ATK for each FIRE monster on the field." Gorlag (FIRE itself, 1000) with a Dokuroyaiba of each
/// side: three FIRE monsters, 2500.
#[test]
fn gorlag_gains_500_for_each_fire_monster_on_the_field() {
    let spawns = [mon(0, GORLAG), mon(0, DOKUROYAIBA), mon(1, DOKUROYAIBA)];
    let Some(s) = field(&[], &spawns) else { return };
    assert_eq!(s.attack_of(0, GORLAG), Some(2500), "{}", s.context());
}

/// Gorlag: "If this card destroys an opponent's monster by battle and sends it to the Graveyard: Special Summon that
/// monster to your side of the field in face-up Attack Position at the end of the Battle Phase, it is treated as a FIRE
/// monster, and its effects are negated. If this card leaves the field: Destroy all monsters Special Summoned by this
/// card's effect." Gorlag (1500) destroys Thunder Kid (700) in turn 3; it comes over as a FIRE monster at the end of the
/// Battle Phase (Gorlag then has 2000); when the opponent destroys Gorlag with Tribute to the Doomed Thunder Kid goes too.
#[test]
fn gorlag_takes_the_monster_it_destroyed_and_loses_it_with_gorlag() {
    let spawns = [mon(0, GORLAG), mon(1, THUNDER_KID), hand(1, TRIBUTE_TO_THE_DOOMED)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Attack(GORLAG)]).unwrap();
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "Thunder Kid lost the battle: {}", s.context());
    s.pass().unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 0, "it left the Graveyard: {}", s.context());
    let kid = s.monster(0, THUNDER_KID).unwrap_or_else(|| panic!("Thunder Kid is Gorlag's now: {}", s.context())).clone();
    assert_eq!(kid["attribute"], 4, "treated as a FIRE monster: {}", s.context());
    assert_eq!(kid["position"], 1, "in face-up Attack Position: {}", s.context());
    assert_eq!(s.attack_of(0, GORLAG), Some(2000), "two FIRE monsters: {}", s.context());
    // Gorlag leaves the field: the monster it took is destroyed.
    s.prefer(&[(0, GORLAG), (0, FILLER), (1, FILLER)]);
    s.run_both([&[], &[Activate(TRIBUTE_TO_THE_DOOMED)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GORLAG), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 0, "destroyed with Gorlag: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "back in its owner's Graveyard: {}", s.context());
}

/// Gorlag: "Cannot attack directly." With no monster on the other side Warwolf is offered a direct attack and Gorlag is not
/// (checked in turn 3, the first Battle Phase of team 0).
#[test]
fn gorlag_cannot_attack_directly() {
    let Some(mut s) = field(&[], &[mon(0, GORLAG), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Attack(GORLAG)]).expect_err("no direct attack");
    assert!(err.contains("can't attack"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Fire Whip: "Activate only while you control a face-up FIRE Fiend-Type monster. Special Summon all monsters that were
/// destroyed this turn to your side of the field. Those monsters are treated as FIRE monsters, also their effects are
/// negated." Dokuroyaiba (1000) destroys Thunder Kid (700) in battle (turn 3, the first Battle Phase of team 0); Fire Whip
/// brings Thunder Kid to my side as a FIRE monster.
#[test]
fn fire_whip_summons_the_monsters_destroyed_this_turn_to_my_side() {
    let spawns = [hand(0, FIRE_WHIP), mon(0, DOKUROYAIBA), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Attack(DOKUROYAIBA), Activate(FIRE_WHIP)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 0, "Thunder Kid left their Graveyard: {}", s.context());
    let kid = s.monster(0, THUNDER_KID).unwrap_or_else(|| panic!("Thunder Kid is on my side: {}", s.context())).clone();
    assert_eq!(kid["attribute"], 4, "treated as FIRE: {}", s.context());
    assert_eq!(s.count(0, "grave", FIRE_WHIP), 1, "{}", s.context());
}

/// Fire Whip brings back "all monsters destroyed this turn", mine too: Warwolf (2000) attacks Dark Magician (2500) and is
/// destroyed; Fire Whip returns it to my field as a FIRE monster.
#[test]
fn fire_whip_also_brings_back_my_own_destroyed_monsters() {
    let spawns = [hand(0, FIRE_WHIP), mon(0, DOKUROYAIBA), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, DARK_MAGICIAN), (0, FILLER), (1, FILLER)]);
    s.run(&[Attack(WARWOLF), Activate(FIRE_WHIP)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 0, "Warwolf is not in the Graveyard: {}", s.context());
    assert_eq!(s.monster(0, WARWOLF).map(|w| w["attribute"].clone()), Some(4.into()), "back as a FIRE monster: {}", s.context());
}

/// Fire Whip: "Activate only while you control a face-up FIRE Fiend-Type monster": a Warwolf alone gives no activation,
/// although Thunder Kid was destroyed this turn.
#[test]
fn fire_whip_needs_a_fire_fiend() {
    let spawns = [hand(0, FIRE_WHIP), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Attack(WARWOLF), Activate(FIRE_WHIP)]).expect_err("no FIRE Fiend");
    assert!(err.contains("Activate(5745) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "Thunder Kid was destroyed and stays there: {}", s.context());
}

/// Aetonyx Flame: "Destroy all monsters on the field other than FIRE Fiend-Type monsters. Then, you can Special Summon 1
/// FIRE Fiend-Type monster from your Graveyard. You cannot declare an attack during the turn you activate this card."
/// Team 1 plays it in turn 2: Warwolf and Thunder Kid are destroyed, Dokuroyaiba stays and a second one comes from the
/// Graveyard; the Battle Phase can be entered but neither Dokuroyaiba is offered an attack.
#[test]
fn aetonyx_flame_destroys_the_others_and_revives_a_fire_fiend() {
    let spawns = [hand(1, AETONYX_FLAME), mon(1, DOKUROYAIBA), mon(1, THUNDER_KID), mon(0, WARWOLF), grave(1, DOKUROYAIBA)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn], &[Activate(AETONYX_FLAME), Attack(DOKUROYAIBA)]]).expect_err("no attack this turn");
    assert!(err.contains("team 1: 5150 can't attack"), "{err}");
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "their own non-Fiend is destroyed too: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DOKUROYAIBA), 2, "the Fiend stayed and its twin came from the Graveyard: {}", s.context());
    assert_eq!(s.count(1, "grave", DOKUROYAIBA), 0, "{}", s.context());
}

/// Aetonyx Flame: the Fiend-Types stay, so with only FIRE Fiends on the field there is nothing to destroy and no activation.
#[test]
fn aetonyx_flame_needs_a_monster_to_destroy() {
    let Some(mut s) = field(&[], &[hand(0, AETONYX_FLAME), mon(0, DOKUROYAIBA), mon(1, DOKUROYAIBA)]) else { return };
    let err = s.run(&[Activate(AETONYX_FLAME)]).expect_err("nothing to destroy");
    assert!(err.contains("Activate(5712) never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Junk Dealer (Manga), Soldier Revolt
// ---------------------------------------------------------------------------------------------------------------

/// Junk Dealer (Manga): "When your opponent Fusion Summons a monster: All the Fusion Materials that were sent to the
/// Graveyard for that Fusion Summon are Special Summoned to your side of the field." The opponent fuses Gaia The Fierce
/// Knight and Curse of Dragon into Gaia the Dragon Champion; both Materials stand on my side.
#[test]
fn junk_dealer_takes_the_fusion_materials() {
    let spawns = [set(0, JUNK_DEALER), hand(1, POLYMERIZATION), mon(1, GAIA), mon(1, CURSE_OF_DRAGON)];
    let Some(mut s) = duel_with_extra(7, 8000, [FILLER, FILLER], &[], [&[], &[GAIA_CHAMPION]], &spawns) else { return };
    s.run_both([&[Chain(JUNK_DEALER)], &[Activate(POLYMERIZATION)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", GAIA_CHAMPION), 1, "the Fusion Monster came: {}", s.context());
    assert_eq!(s.count(0, "mzone", GAIA), 1, "Gaia The Fierce Knight is on my side: {}", s.context());
    assert_eq!(s.count(0, "mzone", CURSE_OF_DRAGON), 1, "and Curse of Dragon: {}", s.context());
    assert_eq!(s.count(1, "grave", GAIA) + s.count(1, "grave", CURSE_OF_DRAGON), 0, "not in their Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", JUNK_DEALER), 1, "{}", s.context());
}

/// Junk Dealer: only the opponent's Fusion Summon starts it: my own is no reason to chain.
#[test]
fn junk_dealer_ignores_my_own_fusion_summon() {
    let spawns = [set(0, JUNK_DEALER), hand(0, POLYMERIZATION), mon(0, GAIA), mon(0, CURSE_OF_DRAGON)];
    let Some(mut s) = duel_with_extra(7, 8000, [FILLER, FILLER], &[], [&[GAIA_CHAMPION], &[]], &spawns) else { return };
    let err = s.run(&[Activate(POLYMERIZATION), Chain(JUNK_DEALER)]).expect_err("not offered");
    assert!(err.contains("Chain(5747) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", GAIA_CHAMPION), 1, "{}", s.context());
}

/// Soldier Revolt: "Activate only while you control a face-up "Science Soldier", "Cyber Soldier of Darkworld" and "Kinetic
/// Soldier" during your Main Phase. Send your opponent's hand to the Graveyard and destroy all cards on your opponent's
/// side of the field. You cannot conduct your Battle Phase this turn." (The script's third name is Cipher Soldier, the
/// card "Kinetic Soldier" was called first.) Team 1 plays it in turn 2: team 0's six cards go, so do Warwolf and the Set
/// Dark Hole, and the Battle Phase is closed to team 1 although it has three monsters.
#[test]
fn soldier_revolt_empties_the_opponents_hand_and_field() {
    let spawns = [set(1, SOLDIER_REVOLT), mon(1, SCIENCE_SOLDIER), mon(1, CYBER_SOLDIER), mon(1, CIPHER_SOLDIER), mon(0, WARWOLF),
        set(0, DARK_HOLE), hand(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[]]).unwrap();
    let (hand_before, grave_before) = (hand_size(&s, 0), grave_size(&s, 0));
    assert_eq!(hand_before, 6, "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], true, "team 1 could attack: {}", s.context());
    s.run_both([&[], &[Activate(SOLDIER_REVOLT)]]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), 0, "their hand was sent to the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "the Set card is destroyed too: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "from the hand: {}", s.context());
    assert_eq!(grave_size(&s, 0), grave_before + hand_before + 2, "the hand and two cards of the field: {}", s.context());
    assert_eq!(s.count(1, "mzone", SCIENCE_SOLDIER), 1, "my side is untouched: {}", s.context());
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase this turn: {}", s.context());
}

/// Soldier Revolt: it needs all three Soldiers: with two of them it is not offered.
#[test]
fn soldier_revolt_needs_all_three_soldiers() {
    let spawns = [set(0, SOLDIER_REVOLT), mon(0, SCIENCE_SOLDIER), mon(0, CYBER_SOLDIER), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(SOLDIER_REVOLT)]).expect_err("only two Soldiers");
    assert!(err.contains("Activate(5750) never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Burning Soul Sword, Celebration of Creation, Silver Dollar, Flat LV4, Thirst for Compensation
// ---------------------------------------------------------------------------------------------------------------

/// Burning Soul Sword: "You can Tribute 1 monster; the equipped monster gains ATK equal to the Tributed monster's ATK,
/// until the end of this turn (even if this card leaves the field)." Warwolf (2000) wears it and Tributes Dark Magician
/// (2500): 4500 until the End Phase, then back to 2000.
#[test]
fn burning_soul_sword_adds_the_tributed_monsters_atk_for_the_turn() {
    let spawns = [hand(0, BURNING_SOUL_SWORD), mon(0, WARWOLF), mon(0, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, DARK_MAGICIAN)]);
    s.run(&[Activate(BURNING_SOUL_SWORD)]).unwrap();
    assert_eq!(s.count(0, "szone", BURNING_SOUL_SWORD), 1, "the Sword is equipped: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "{}", s.context());
    s.run(&[Activate(BURNING_SOUL_SWORD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "Dark Magician was Tributed: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(4500), "2000 + 2500: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "until the end of the turn: {}", s.context());
}

/// Celebration of Creation: "During your opponent's turn, when a monster(s) is Special Summoned by card effect, even
/// during the Damage Step: It becomes the End Phase of this turn." The opponent's Monster Reborn brings Warwolf back and
/// the Trap is answered: the opponent's next prompt is not another Main Phase prompt of its turn but the start of my turn
/// (the turn's remaining phases are skipped), and Warwolf has not attacked.
#[test]
fn celebration_of_creation_ends_the_opponents_turn_at_a_special_summon() {
    let spawns = [set(0, CELEBRATION_OF_CREATION), hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(CELEBRATION_OF_CREATION)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CELEBRATION_OF_CREATION), 1, "the Trap was used: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "Warwolf was revived: {}", s.context());
    assert_eq!(s.prompt()["player"], 0, "the turn is over, my turn 3 begins: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Celebration of Creation, the control: without the Trap the opponent goes on in its Main Phase after Monster Reborn and
/// its Warwolf attacks directly.
#[test]
fn without_celebration_of_creation_the_revived_monster_attacks() {
    let spawns = [hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(MONSTER_REBORN)]]).unwrap();
    assert_eq!(s.prompt()["player"], 1, "the Main Phase goes on: {}", s.context());
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6000, "{}", s.context());
}

/// Silver Dollar: "When an opponent's monster attacks a monster you control with 1000 or less ATK: Your monster cannot be
/// destroyed by this battle, and you take no battle damage from that battle." Warwolf (2000) attacks Thunder Kid (700):
/// Thunder Kid stays and no LP is lost.
#[test]
fn silver_dollar_saves_a_weak_monster_and_its_controllers_lp() {
    let spawns = [set(0, SILVER_DOLLAR), mon(0, THUNDER_KID), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Chain(SILVER_DOLLAR)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SILVER_DOLLAR), 1, "the Trap was used: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid was not destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000, "and no battle damage was taken: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Silver Dollar: "a monster you control with 1000 or less ATK": Dark Magician (2500) attacked gives no activation.
#[test]
fn silver_dollar_ignores_a_strong_monster() {
    let spawns = [set(0, SILVER_DOLLAR), mon(0, DARK_MAGICIAN), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[Chain(SILVER_DOLLAR)], &[Summon(WARWOLF), Attack(WARWOLF)]]).expect_err("not on offer");
    assert!(err.contains("Chain(5569) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", SILVER_DOLLAR), 1, "still Set: {}", s.context());
}

/// Flat LV4: "When a monster you control is destroyed: Both players can Special Summon 1 Level 4 monster from their Deck."
/// The opponent's Dark Hole destroys my Thunder Kid and both of us take a Feral Imp (Level 4) from our Decks.
#[test]
fn flat_lv4_lets_both_players_summon_a_level_4_from_the_deck() {
    let spawns = [set(0, FLAT_LV4), mon(0, THUNDER_KID), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Chain(FLAT_LV4)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "my Level 4 from my Deck: {}", s.context());
    assert_eq!(s.count(1, "mzone", FILLER), 1, "their Level 4 from their Deck: {}", s.context());
}

/// Flat LV4: "When a monster you control is destroyed": the opponent's monster destroyed alone is no reason to chain.
#[test]
fn flat_lv4_ignores_the_opponents_destroyed_monster() {
    let spawns = [set(0, FLAT_LV4), mon(1, THUNDER_KID), hand(0, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(DARK_HOLE), Chain(FLAT_LV4)]).expect_err("only their monster was destroyed");
    assert!(err.contains("Chain(5639) never became possible"), "{err}");
    clean(&s);
}

/// Thirst for Compensation: "When your opponent adds a card(s) to their hand by a card effect: Special Summon up to 2 Level
/// 4 or lower monsters from your hand in face-up Defense Position, also, those monsters cannot change their battle
/// positions or be Tributed for a Tribute Summon (or Set)." The opponent's Pot of Greed answers: two Feral Imps come in
/// Defense Position, and a Battle Steer of mine finds nothing to Tribute (the Imps cannot be).
#[test]
fn thirst_for_compensation_summons_two_monsters_that_cannot_be_tributed() {
    let spawns = [set(0, THIRST_FOR_COMPENSATION), hand(0, BATTLE_STEER), hand(1, POT_OF_GREED)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, FILLER)]);
    s.prefer_most(true);
    s.run_both([&[Chain(THIRST_FOR_COMPENSATION)], &[Activate(POT_OF_GREED)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", POT_OF_GREED), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 2, "two monsters from my hand: {}", s.context());
    for m in monsters(&s, 0, FILLER) {
        assert_eq!(m["position"], 4, "in face-up Defense Position: {}", s.context());
    }
    let err = s.run(&[Summon(BATTLE_STEER)]).expect_err("no Tribute allowed");
    assert!(err.contains("Summon(4020) never became possible"), "{err}");
}

/// Thirst for Compensation: "When your opponent adds a card(s) to their hand": a card I add myself is no reason to chain.
#[test]
fn thirst_for_compensation_ignores_my_own_draw() {
    let spawns = [set(0, THIRST_FOR_COMPENSATION), hand(0, POT_OF_GREED)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(POT_OF_GREED), Chain(THIRST_FOR_COMPENSATION)]).expect_err("my own draw");
    assert!(err.contains("Chain(5693) never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Take One Chance, Compensation Mediation
// ---------------------------------------------------------------------------------------------------------------

/// Take One Chance: "Choose 1 random card in your Graveyard, and activate that card immediately, if able." Dark Hole is
/// the only card in the Graveyard: it is activated and destroys every monster.
#[test]
fn take_one_chance_activates_the_card_in_the_graveyard() {
    let spawns = [set(0, TAKE_ONE_CHANCE), grave(0, DARK_HOLE), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(TAKE_ONE_CHANCE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Dark Hole's effect ran: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "Dark Hole went back to the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", TAKE_ONE_CHANCE), 1, "{}", s.context());
}

/// Take One Chance (regression, the override): "if able" is checked as it resolves too. Battle Fusion (Anime) and Dark Hole
/// are in the Graveyard; the Trap is on offer because of Dark Hole, and when the random pick is Battle Fusion (its condition
/// reads the attacker, there is none outside a battle) it stays in the Graveyard with no Lua error and nothing else
/// happens. Across seeds both picks occur: Dark Hole destroys Warwolf and Thunder Kid, Battle Fusion leaves them.
#[test]
fn take_one_chance_leaves_a_card_it_cannot_activate_in_the_graveyard() {
    let spawns = [set(0, TAKE_ONE_CHANCE), grave(0, BATTLE_FUSION), grave(0, DARK_HOLE), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let mut picks = std::collections::BTreeSet::new();
    for seed in 1..=10 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        s.run(&[Activate(TAKE_ONE_CHANCE)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "grave", TAKE_ONE_CHANCE), 1, "seed {seed}: {}", s.context());
        if s.count(0, "mzone", WARWOLF) == 1 {
            // Battle Fusion was drawn: nothing happened and it is still where it was.
            assert_eq!(s.count(0, "grave", BATTLE_FUSION), 1, "seed {seed}: {}", s.context());
            assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "seed {seed}: {}", s.context());
            assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "seed {seed}: {}", s.context());
            picks.insert("Battle Fusion");
        } else {
            assert_eq!(s.count(0, "grave", WARWOLF) + s.count(1, "grave", THUNDER_KID), 2, "seed {seed}: Dark Hole ran: {}", s.context());
            assert_eq!(s.count(0, "grave", BATTLE_FUSION), 1, "seed {seed}: {}", s.context());
            picks.insert("Dark Hole");
        }
    }
    assert_eq!(picks.len(), 2, "ten seeds pick both cards, got {picks:?}");
}

/// Take One Chance with Battle Fusion (Anime) as the only card in the Graveyard: the Trap is not on offer at all, because
/// the filter that lists activatable cards reads Battle Fusion's condition outside a chain (no attacker: false). The override's
/// check at resolution is reached only through the random pick above.
#[test]
fn take_one_chance_is_not_offered_for_battle_fusion_alone() {
    let Some(mut s) = field(&[], &[set(0, TAKE_ONE_CHANCE), grave(0, BATTLE_FUSION), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(TAKE_ONE_CHANCE)]).expect_err("Battle Fusion cannot be activated now");
    assert!(err.contains("Activate(5571) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "grave", BATTLE_FUSION), 1, "{}", s.context());
}

/// Take One Chance: with no Spell or Trap that it could activate in the Graveyard (only a monster) it is not offered.
#[test]
fn take_one_chance_needs_a_spell_or_trap_in_the_graveyard() {
    let Some(mut s) = field(&[], &[set(0, TAKE_ONE_CHANCE), grave(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(TAKE_ONE_CHANCE)]).expect_err("only a monster there");
    assert!(err.contains("Activate(5571) never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Compensation Mediation
// ---------------------------------------------------------------------------------------------------------------

/// Compensation Mediation is a Set Trap of team 0, chained to team 1's Battle Phase at its first chance (the Battle Start
/// step: the engine's Battle Phase window). Team 1 plays turn 2 up to the point after the Trap resolved and its prompt
/// comes back. With `synchro`, team 1 first Synchro Summons Gaia Knight, the Force of Earth from Junk Synchron and Giant
/// Soldier of Stone, so that its Graveyard holds exactly those two former Synchro Materials; otherwise Thunder Kid and
/// Dark Magician. Returns the duel at that prompt and whether the Battle Phase ended (an Idle prompt: the Main Phase 2)
/// or went on (a Battle prompt), with the Deck size team 1 started with.
fn mediation(seed: u64, synchro: bool) -> Option<(Scenario, bool, i64)> {
    use yo3_server::ocg::{Response, idle};
    let mut spawns = vec![set(0, COMPENSATION_MEDIATION), mon(1, WARWOLF)];
    let extra: [&[u16]; 2] = if synchro {
        spawns.extend([mon(1, JUNK_SYNCHRON), mon(1, GIANT_SOLDIER)]);
        [&[], &[GAIA_KNIGHT]]
    } else {
        spawns.extend([grave(1, THUNDER_KID), grave(1, DARK_MAGICIAN)]);
        [&[], &[]]
    };
    let mut s = duel_with_extra(seed, 8000, [FILLER, FILLER], &[], extra, &spawns)?;
    keep_hands(&mut s);
    let deck = deck_size(&s, 1);
    let (mut chained, mut summoned) = (false, !synchro);
    for _ in 0..120 {
        let p = s.prompt().clone();
        let (kind, player) = (p["type"].clone(), p["player"].clone());
        let trap = p["selects"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == COMPENSATION_MEDIATION));
        let in_hand = |list: &str, cid: u16| p[list].as_array().and_then(|v| v.iter().position(|c| c["cid"] == cid));
        if kind == msg::SELECT_CHAIN && player == 0 && trap.is_some() && !chained {
            chained = true;
            s.respond(Response::Chain(trap).json()).unwrap();
        } else if chained && player == 1 && (kind == msg::SELECT_IDLECMD || kind == msg::SELECT_BATTLECMD) {
            return Some((s, kind == msg::SELECT_IDLECMD, deck));
        } else if kind == msg::SELECT_IDLECMD && player == 1 && !summoned && in_hand("special_summons", GAIA_KNIGHT).is_some() {
            summoned = true;
            let index = in_hand("special_summons", GAIA_KNIGHT);
            s.respond(serde_json::json!({"type": 1, "action": idle::SPSUMMON, "index": index})).unwrap();
        } else if kind == msg::SELECT_IDLECMD && player == 1 && summoned && p["to_bp"] == true {
            s.respond(serde_json::json!({"type": 1, "action": idle::TO_BP, "index": null})).unwrap();
        } else {
            s.pass().unwrap();
        }
    }
    panic!("the Trap never resolved: {}", s.context());
}

/// Compensation Mediation: "Your opponent selects 2 cards from their Graveyard, then they shuffle them together with this
/// card. You then select 1 card from among those 3 cards. If that card is "Compensation Mediation", end the Battle Phase
/// for this turn, then your opponent places the remaining cards on top of their Deck in any order they choose." The pick
/// is random, so seeds are tried until it is the Trap: the attacker's Battle Phase is over (it is asked for its Main Phase
/// 2, not a battle command, and no attack was declared), its Graveyard is empty and both cards are on its Deck (its Deck
/// holds the two besides its turn's draw), the Trap in my Graveyard.
#[test]
fn compensation_mediation_ends_the_battle_phase_when_it_is_picked() {
    for seed in 1..=30 {
        let Some((s, ended, deck)) = mediation(seed, false) else { return };
        clean(&s);
        assert_eq!(s.count(0, "grave", COMPENSATION_MEDIATION), 1, "the Trap is always sent to my Graveyard: {}", s.context());
        if !ended {
            continue;
        }
        assert_eq!(grave_size(&s, 1), 0, "both cards left the Graveyard: {}", s.context());
        assert_eq!(deck_size(&s, 1), deck - 1 + 2, "on top of the Deck, after the turn's draw: {}", s.context());
        assert_eq!(s.prompt()["to_bp"], false, "no more Battle Phase this turn: {}", s.context());
        assert_eq!(s.lp(0), 8000, "Warwolf never attacked: {}", s.context());
        return;
    }
    panic!("no seed of 30 picked the Trap");
}

/// Compensation Mediation: "If that card is not "Compensation Mediation", the Battle Phase continues and you send the card
/// you picked and "Compensation Mediation" to the Graveyard. Your opponent places the remaining card on top of their
/// Deck." Seeds are tried until the pick is another card: the attacker gets its battle command again, one of the two cards
/// is in its Graveyard and the other on its Deck; the attack then goes through.
#[test]
fn compensation_mediation_lets_the_battle_phase_continue_when_another_card_is_picked() {
    for seed in 1..=30 {
        let Some((mut s, ended, deck)) = mediation(seed, false) else { return };
        clean(&s);
        assert_eq!(s.count(0, "grave", COMPENSATION_MEDIATION), 1, "the Trap is always sent to my Graveyard: {}", s.context());
        if ended {
            continue;
        }
        assert_eq!(grave_size(&s, 1), 1, "the picked card is in the Graveyard: {}", s.context());
        assert_eq!(deck_size(&s, 1), deck - 1 + 1, "the other one is on top of the Deck, after the turn's draw: {}", s.context());
        assert_eq!(s.lp(0), 8000, "{}", s.context());
        s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::battle::ATTACK, "index": 0})).unwrap();
        s.run(&[]).unwrap();
        clean(&s);
        assert_eq!(s.lp(0), 6000, "the Battle Phase went on: Warwolf attacked directly: {}", s.context());
        return;
    }
    panic!("no seed of 30 picked another card");
}

/// Compensation Mediation (regression, the translator): the Graveyard holds two cards that were Synchro Materials (their
/// reason keeps SYNCHRO | MATERIAL when Duel.MoveToField puts them on the field for the shuffle). The client's view must
/// not take that for a Synchro Summon's tuning: it stays equal to the engine's through the whole Trap, in both outcomes.
#[test]
fn compensation_mediation_moves_former_synchro_materials_without_a_phantom_synchro_summon() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in 1..=30 {
        let Some((s, ended, _)) = mediation(seed, true) else { return };
        clean(&s);
        assert_eq!(s.count(0, "grave", COMPENSATION_MEDIATION), 1, "seed {seed}: {}", s.context());
        assert_eq!(s.count(1, "mzone", GAIA_KNIGHT), 1, "seed {seed}: the Synchro Monster stayed: {}", s.context());
        // The Synchro Materials are in the Graveyard (one picked) or on top of the Deck, never on the field.
        assert_eq!(s.count(1, "mzone", JUNK_SYNCHRON) + s.count(1, "mzone", GIANT_SOLDIER), 0, "seed {seed}: {}", s.context());
        seen.insert(ended);
        if seen.len() == 2 {
            return;
        }
    }
    panic!("30 seeds did not reach both outcomes with Synchro Materials, got {seen:?}");
}

// ---------------------------------------------------------------------------------------------------------------
// Bounce Spell moving a Spell with counters (regression, the translator)
// ---------------------------------------------------------------------------------------------------------------

/// Bounce Spell (regression, the translator): "Target 1 face-up Spell your opponent controls; take control it and activate
/// the effect of it." Team 0's Full Moon Mirror holds a Full Moon Counter (Dark Hole destroyed Warwolf); team 1's Bounce
/// Spell takes it. The engine clears a card's counters silently when it moves a Spell/Trap to the other field: the client's
/// view drops them too (it stays equal to the engine's) and the Mirror now sits on team 1's side with none.
#[test]
fn bounce_spell_takes_a_spell_with_counters_and_the_client_drops_them() {
    let spawns = [hand(0, FULL_MOON_MIRROR), hand(0, DARK_HOLE), mon(1, WARWOLF), hand(1, BOUNCE_SPELL)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(FULL_MOON_MIRROR), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    let mirror = s.state()[0]["szone"].as_array().unwrap().iter().find(|c| c["cid"] == FULL_MOON_MIRROR).cloned().expect("the Mirror is on my field");
    assert_eq!(mirror["counters"].as_object().map(|c| c.values().filter_map(|n| n.as_i64()).sum::<i64>()), Some(1), "one Full Moon Counter: {}", s.context());
    s.run_both([&[EndTurn], &[Activate(BOUNCE_SPELL)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", FULL_MOON_MIRROR), 0, "the Mirror left my field: {}", s.context());
    let moved = s.state()[1]["szone"].as_array().unwrap().iter().find(|c| c["cid"] == FULL_MOON_MIRROR).cloned()
        .unwrap_or_else(|| panic!("the Mirror is on their side now: {}", s.context()));
    assert!(moved["counters"].as_object().is_none_or(|c| c.values().all(|n| n == 0)), "the engine cleared its counters: {moved}");
    assert_eq!(s.count(1, "grave", BOUNCE_SPELL), 1, "{}", s.context());
}
