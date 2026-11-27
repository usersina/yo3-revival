//! Scenario tests for cards the duel check's self-play never (or never visibly) exercised (`tools/engine/coverage.py`,
//! #67): each plays the situation the card is for through the real core, both seats' client models checked after every
//! step (`duelcheck::scenario`). Card texts: the official database (db.yugioh-card.com, by the Konami id).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};

const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const SPIRIT_REAPER: u16 = 5526;
const GOYO_GUARDIAN: u16 = 7736;
const JUNK_SYNCHRON: u16 = 7687;
const ZUBABA_KNIGHT: u16 = 9563;
const GAGAGA_MAGICIAN: u16 = 9618;
const MIRROR_FORCE: u16 = 4887;

const FACE_UP_DEFENSE: u64 = 0x4;

fn start(decks: [(&[u16], &[u16]); 2]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(Scenario::start(&dir, 7, 8000, decks).expect("the duel starts"))
}

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

/// Spirit Reaper (5526): "Cannot be destroyed by battle." Attacked by a 2000 ATK monster, it stays and its controller
/// takes the 1700 difference.
#[test]
fn spirit_reaper_survives_a_battle() {
    let Some(mut s) = start([(&[SPIRIT_REAPER; 40], &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(SPIRIT_REAPER)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(0, SPIRIT_REAPER).is_some(), "Spirit Reaper was destroyed: {}", s.state());
    assert_eq!(s.lp(0), 8000 - 1700);
    clean(&s);
}

/// Goyo Guardian (7736): "When this card destroys an opponent's monster by battle and sends it to the Graveyard: You can
/// Special Summon that monster to your side of the field in face-up Defense Position." Synchro Summoned from Junk
/// Synchron and Zubaba Knight (Level 3 + 3), it attacks Gene-Warped Warwolf and takes it.
#[test]
fn goyo_guardian_takes_the_monster_it_destroys() {
    let deck: Vec<u16> = [ZUBABA_KNIGHT, JUNK_SYNCHRON].iter().copied().cycle().take(40).collect();
    let Some(mut s) = start([(&deck, &[GOYO_GUARDIAN]), (&[WARWOLF; 40], &[])]) else { return };
    let plan = [Summon(ZUBABA_KNIGHT), Summon(JUNK_SYNCHRON), SpSummon(GOYO_GUARDIAN), Attack(GOYO_GUARDIAN)];
    s.run_both([&plan, &[Summon(WARWOLF)]]).unwrap();
    assert!(s.monster(0, GOYO_GUARDIAN).is_some());
    assert_eq!(s.position_of(0, WARWOLF), Some(FACE_UP_DEFENSE), "Warwolf should be on Goyo's side: {}", s.state());
    assert!(s.monster(1, WARWOLF).is_none());
    clean(&s);
}

/// Gagaga Magician (9618): "Once per turn: You can declare a Level from 1 to 8; this card becomes that Level until the
/// End Phase."
#[test]
fn gagaga_magician_becomes_the_declared_level() {
    let Some(mut s) = start([(&[GAGAGA_MAGICIAN; 40], &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(GAGAGA_MAGICIAN), Activate(GAGAGA_MAGICIAN)]).unwrap();
    let level = s.level_of(0, GAGAGA_MAGICIAN).expect("Gagaga Magician is on the field");
    assert!((1..=8).contains(&level) && level != 4, "Level {level}: {}", s.state());
    clean(&s);
}

/// Mirror Force (4887): "When an opponent's monster declares an attack: Destroy all your opponent's Attack Position
/// monsters." Set, then activated at Gene-Warped Warwolf's attack.
#[test]
fn mirror_force_destroys_the_attacker() {
    let deck: Vec<u16> = [WARWOLF, MIRROR_FORCE].iter().copied().cycle().take(40).collect();
    let Some(mut s) = start([(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[SetSpell(MIRROR_FORCE), Chain(MIRROR_FORCE)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(1, WARWOLF).is_none(), "the attacker survived: {}", s.state());
    assert_eq!(s.count(1, "grave", WARWOLF), 1);
    assert_eq!(s.count(0, "grave", MIRROR_FORCE), 1);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000));
    clean(&s);
}

// The preset decks' effect cards that the self-play never exercised (`tools/engine/coverage.py --decks`): Yusei's first,
// then the other characters'. Texts are the pinned card data's (`tools/engine/cardtext.py`), which can be worded
// differently from the client's 2010-2012 card text; where they differ the test says which one it follows.
const QUICKDRAW_SYNCHRON: u16 = 8439;
const TURRET_WARRIOR: u16 = 8004;
const ZERO_GARDNA: u16 = 8441;
const BRIONAC: u16 = 7548;
const MIST_VALLEY_SOLDIER: u16 = 7532;
const EMERGENCY_TELEPORT: u16 = 7747;
const PSYCHIC_COMMANDER: u16 = 7718;
const ASSAULT_BEAST: u16 = 8022;
const ASSAULT_MODE_ACTIVATE: u16 = 8058;
const DEBRIS_DRAGON: u16 = 8005;
const JINZO: u16 = 4758;
const MARSHMALLON: u16 = 6000;
const CYBER_DRAGON: u16 = 6390;
const COMMAND_KNIGHT: u16 = 5017;

/// A 40-card main deck whose opening hand is `hand` (the last five cards), the rest Gene-Warped Warwolves: the draws
/// after the opening hand are Warwolves too.
fn deck_with(hand: &[u16]) -> Vec<u16> {
    let mut deck = vec![WARWOLF; 40 - hand.len()];
    deck.extend_from_slice(hand);
    deck
}

/// Quickdraw Synchron (8439): "You can Special Summon this card (from your hand) by sending 1 monster from your hand to
/// the GY." Its own procedure, no Normal Summon used.
#[test]
fn quickdraw_synchron_special_summons_itself_for_a_discard() {
    let Some(mut s) = start([(&deck_with(&[WARWOLF, QUICKDRAW_SYNCHRON]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[SpSummon(QUICKDRAW_SYNCHRON)]).unwrap();
    assert!(s.monster(0, QUICKDRAW_SYNCHRON).is_some(), "{}", s.state());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "one monster went to the Graveyard for it");
    clean(&s);
}

/// Turret Warrior (8004): "You can Special Summon this card (from your hand) by Tributing 1 Warrior-Type monster, and if
/// you do, it gains ATK equal to the Tributed monster's original ATK." Command Knight's 400 ATK bonus to Warriors goes
/// with it, so the gain is its original 1200 only: 1200 + 1200.
#[test]
fn turret_warrior_gains_the_tributed_warriors_attack() {
    let Some(mut s) = start([(&deck_with(&[COMMAND_KNIGHT, TURRET_WARRIOR]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(COMMAND_KNIGHT), SpSummon(TURRET_WARRIOR)]).unwrap();
    assert_eq!(s.attack_of(0, TURRET_WARRIOR), Some(2400), "{}", s.state());
    assert_eq!(s.count(0, "grave", COMMAND_KNIGHT), 1);
    clean(&s);
}

/// Command Knight (5017): "All Warrior monsters you control gain 400 ATK." It is a Warrior itself: 1200 + 400.
#[test]
fn command_knight_gives_warriors_400_attack_itself_included() {
    let Some(mut s) = start([(&deck_with(&[COMMAND_KNIGHT]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(COMMAND_KNIGHT)]).unwrap();
    assert_eq!(s.attack_of(0, COMMAND_KNIGHT), Some(1600), "{}", s.state());
    clean(&s);
}

/// Zero Gardna (8441): "You can Tribute this card to activate its effect. If you do, monsters you control cannot be
/// destroyed by battle, and you take no Battle Damage during this turn. This effect can be activated during either
/// player's turn." Chained to Gene-Warped Warwolf's attack on it, it is the Tribute and takes no damage.
#[test]
fn zero_gardna_saves_its_controller_from_battle_damage() {
    let Some(mut s) = start([(&deck_with(&[ZERO_GARDNA]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(ZERO_GARDNA), ChainOnAttack(ZERO_GARDNA)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000, "{}", s.state());
    assert_eq!(s.count(0, "grave", ZERO_GARDNA), 1, "it was the Tribute");
    clean(&s);
}

/// Mist Valley Soldier (7532): "At the end of the Damage Step, if this card battled an opponent's monster that was not
/// destroyed: Return that opponent's monster to the hand." Spirit Reaper cannot be destroyed by battle. The effect is
/// not optional in the card's text: it resolves without a question to the player.
#[test]
fn mist_valley_soldier_returns_the_monster_that_survived() {
    let Some(mut s) = start([(&deck_with(&[MIST_VALLEY_SOLDIER]), &[]), (&deck_with(&[SPIRIT_REAPER]), &[])]) else { return };
    s.run_both([&[Summon(MIST_VALLEY_SOLDIER), Attack(MIST_VALLEY_SOLDIER)], &[Summon(SPIRIT_REAPER)]]).unwrap();
    assert!(s.monster(1, SPIRIT_REAPER).is_none(), "{}", s.state());
    assert_eq!(s.count(1, "hand", SPIRIT_REAPER), 1);
    clean(&s);
}

/// Brionac, Dragon of the Ice Barrier (7548): "You can discard any number of cards to the GY, then target the same
/// number of cards your opponent controls; return those cards to the hand." Synchro Summoned from Junk Synchron and
/// Zubaba Knight, it discards one card and returns Gene-Warped Warwolf.
#[test]
fn brionac_returns_a_card_for_each_card_discarded() {
    let hand = [WARWOLF, ZUBABA_KNIGHT, JUNK_SYNCHRON];
    let Some(mut s) = start([(&deck_with(&hand), &[BRIONAC]), (&[WARWOLF; 40], &[])]) else { return };
    // The client's text lets it return any player's cards from the field, so the target is chosen: not Brionac itself.
    s.prefer(&[(1, WARWOLF)]);
    let plan = [Summon(ZUBABA_KNIGHT), Summon(JUNK_SYNCHRON), SpSummon(BRIONAC), Activate(BRIONAC)];
    s.run_both([&plan, &[Summon(WARWOLF)]]).unwrap();
    assert!(s.monster(0, BRIONAC).is_some());
    assert!(s.monster(1, WARWOLF).is_none(), "{}", s.state());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the discard");
    clean(&s);
}

/// Emergency Teleport (7747): "Special Summon 1 Level 3 or lower Psychic monster from your hand or Deck, but banish it
/// during the End Phase of this turn." Psychic Commander comes and goes at the End Phase.
#[test]
fn emergency_teleport_summons_a_psychic_and_banishes_it_at_the_end_phase() {
    let Some(mut s) = start([(&deck_with(&[EMERGENCY_TELEPORT, PSYCHIC_COMMANDER]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Activate(EMERGENCY_TELEPORT)]).unwrap();
    assert!(s.monster(0, PSYCHIC_COMMANDER).is_some(), "{}", s.state());
    s.run(&[EndTurn]).unwrap();
    assert!(s.monster(0, PSYCHIC_COMMANDER).is_none(), "{}", s.state());
    assert_eq!(s.count(0, "removed", PSYCHIC_COMMANDER), 1);
    clean(&s);
}

/// Assault Beast (8022): "You can discard this card to the GY; add 1 "Assault Mode Activate" from your Deck to your hand."
#[test]
fn assault_beast_fetches_assault_mode_activate() {
    let mut deck = deck_with(&[ASSAULT_BEAST]);
    deck[0] = ASSAULT_MODE_ACTIVATE;
    let Some(mut s) = start([(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Activate(ASSAULT_BEAST)]).unwrap();
    assert_eq!(s.count(0, "hand", ASSAULT_MODE_ACTIVATE), 1, "{}", s.state());
    assert_eq!(s.count(0, "grave", ASSAULT_BEAST), 1);
    clean(&s);
}

/// Jinzo (4758): "Trap Cards, and their effects on the field, cannot be activated. Negate all Trap effects on the
/// field." The opponent's Set Mirror Force can't answer its attack, so the 2400 goes through.
#[test]
fn jinzo_stops_a_set_trap() {
    let Some(mut s) = start([(&deck_with(&[WARWOLF, JINZO]), &[]), (&deck_with(&[MIRROR_FORCE]), &[])]) else { return };
    s.run_both([&[Summon(WARWOLF), Summon(JINZO), Attack(JINZO)], &[SetSpell(MIRROR_FORCE)]]).unwrap();
    assert_eq!(s.lp(1), 8000 - 2400, "{}", s.state());
    assert_eq!(s.count(1, "szone", MIRROR_FORCE), 1, "Mirror Force stayed Set");
    assert!(s.monster(0, JINZO).is_some());
    clean(&s);
}

/// Marshmallon (6000): "Cannot be destroyed by battle. After damage calculation, if this card was attacked, and was
/// face-down at the start of the Damage Step: The attacking player takes 1000 damage."
#[test]
fn marshmallon_hurts_the_attacker_of_a_face_down_card() {
    let Some(mut s) = start([(&deck_with(&[MARSHMALLON]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[SetMonster(MARSHMALLON)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(0, MARSHMALLON).is_some(), "{}", s.state());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7000));
    clean(&s);
}

/// Cyber Dragon (6390): "If only your opponent controls a monster, you can Special Summon this card (from your hand)."
#[test]
fn cyber_dragon_comes_when_only_the_opponent_has_a_monster() {
    let Some(mut s) = start([(&deck_with(&[CYBER_DRAGON]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[SpSummon(CYBER_DRAGON)], &[Summon(WARWOLF)]]).unwrap();
    assert_eq!(s.attack_of(0, CYBER_DRAGON), Some(2100), "{}", s.state());
    clean(&s);
}

/// Debris Dragon (8005): "When this card is Normal Summoned: You can target 1 monster with 500 or less ATK in your GY;
/// Special Summon that target in Attack Position, but negate its effects." Spirit Reaper (300 ATK) is in the Graveyard
/// as Quickdraw Synchron's discard.
#[test]
fn debris_dragon_brings_back_a_weak_monster() {
    let hand = [SPIRIT_REAPER, SPIRIT_REAPER, SPIRIT_REAPER, DEBRIS_DRAGON, QUICKDRAW_SYNCHRON];
    let Some(mut s) = start([(&deck_with(&hand), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.prefer(&[(0, SPIRIT_REAPER)]);
    s.run(&[SpSummon(QUICKDRAW_SYNCHRON), Summon(DEBRIS_DRAGON)]).unwrap();
    assert!(s.monster(0, SPIRIT_REAPER).is_some(), "{}", s.state());
    assert_eq!(s.position_of(0, SPIRIT_REAPER), Some(1), "in Attack Position");
    clean(&s);
}

const KURIBOH: u16 = 4064;
const SCRAP_IRON_SCARECROW: u16 = 7694;

/// Kuriboh (4064): "During damage calculation, if your opponent's monster attacks (Quick Effect): You can discard this
/// card; you take no battle damage from that battle." Command Knight (1200) attacked by Gene-Warped Warwolf (2000) is
/// destroyed, but its controller loses no Life Points.
#[test]
fn kuriboh_takes_the_battle_damage_away() {
    let Some(mut s) = start([(&deck_with(&[COMMAND_KNIGHT, KURIBOH]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(COMMAND_KNIGHT), ChainOnAttack(KURIBOH)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "discarded for its effect");
    assert_eq!(s.count(0, "grave", COMMAND_KNIGHT), 1, "the battle still destroyed it");
    clean(&s);
}

/// Scrap-Iron Scarecrow (7694): "When an opponent's monster declares an attack: Target the attacking monster; negate the
/// attack, also, after that, Set this card face-down instead of sending it to the GY." Set, it stops Warwolf's direct
/// attack and is Set again.
#[test]
fn scrap_iron_scarecrow_negates_an_attack_and_sets_itself_again() {
    let Some(mut s) = start([(&deck_with(&[SCRAP_IRON_SCARECROW]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[SetSpell(SCRAP_IRON_SCARECROW), ChainOnAttack(SCRAP_IRON_SCARECROW)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(0, "szone", SCRAP_IRON_SCARECROW), 1, "Set again: {}", s.context());
    clean(&s);
}

const DOUBLE_COSTON: u16 = 6040;
const DARK_MAGICIAN: u16 = 4041;

/// Double Coston (6040): "This card can be treated as 2 Tributes for the Tribute Summon of a DARK monster." It alone is
/// the Tribute for Dark Magician (Level 7, DARK). The engine's Tribute prompt asks for 2 and offers one card with
/// `release_param` 2, which the translator now reads (it stopped the duel before).
#[test]
fn double_coston_is_the_tribute_for_a_level_7_dark_monster() {
    let Some(mut s) = start([(&deck_with(&[DOUBLE_COSTON, DARK_MAGICIAN]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(DOUBLE_COSTON), Summon(DARK_MAGICIAN)]).unwrap();
    assert!(s.monster(0, DARK_MAGICIAN).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", DOUBLE_COSTON), 1);
    clean(&s);
}

const FAMILIAR_POSSESSED_WYNN: u16 = 6510;
const WYNN_THE_WIND_CHARMER: u16 = 6338;
const HUNTER_OWL: u16 = 7038;

/// Familiar-Possessed - Wynn (6510): "You can Special Summon this card (from your hand or Deck) by sending 1 face-up
/// "Wynn the Wind Charmer" and 1 face-up WIND monster from your Monster Zone to the GY." Here from the Deck (the engine
/// lists it at location 1, which the translator now offers on the Deck pile: `translate_contract.rs`).
#[test]
fn familiar_possessed_wynn_comes_from_the_deck() {
    let mut deck = deck_with(&[WYNN_THE_WIND_CHARMER, HUNTER_OWL]);
    deck[0] = FAMILIAR_POSSESSED_WYNN;
    let Some(mut s) = start([(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(WYNN_THE_WIND_CHARMER), Summon(HUNTER_OWL), SpSummon(FAMILIAR_POSSESSED_WYNN)]).unwrap();
    assert!(s.monster(0, FAMILIAR_POSSESSED_WYNN).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", WYNN_THE_WIND_CHARMER), 1, "its materials were sent to the Graveyard");
    assert_eq!(s.count(0, "hand", FAMILIAR_POSSESSED_WYNN), 0, "it was not in the hand");
    clean(&s);
}

const SYNCHRO_STRIKE: u16 = 7921;

/// Synchro Strike (7921), a Normal Trap (Set first, activated from the next turn on; the database row's type is Trap, not
/// the Quick-Play Spell it was first taken for): "Select 1 face-up Synchro Monster on the field. It gains 500 ATK for each
/// material used for its Synchro Summon until the end of the turn." Goyo Guardian (2800) Synchro Summoned from two
/// materials gains 1000.
#[test]
fn synchro_strike_adds_500_attack_per_material() {
    let deck: Vec<u16> = [ZUBABA_KNIGHT, JUNK_SYNCHRON, SYNCHRO_STRIKE].iter().copied().cycle().take(40).collect();
    let Some(mut s) = start([(&deck, &[GOYO_GUARDIAN]), (&[WARWOLF; 40], &[])]) else { return };
    let plan = [
        SetSpell(SYNCHRO_STRIKE),
        Summon(ZUBABA_KNIGHT),
        Summon(JUNK_SYNCHRON),
        SpSummon(GOYO_GUARDIAN),
        EndTurn,
        Activate(SYNCHRO_STRIKE),
    ];
    s.run(&plan).unwrap();
    assert_eq!(s.attack_of(0, GOYO_GUARDIAN), Some(2800 + 1000), "{}", s.state());
    clean(&s);
}

const MAX_WARRIOR: u16 = 8438;
const FORTRESS_WARRIOR: u16 = 7985;
const SYNCHRO_DEFLECTOR: u16 = 7758;
const MIST_WURM: u16 = 7549;

/// Max Warrior (8438): "If this card attacks an opponent's monster, this card gains 400 ATK during the Damage Step only.
/// If this card destroys an opponent's monster by battle, until your next Standby Phase: this card becomes Level 2 and its
/// original ATK and DEF are halved." 1800 + 400 beats Gene-Warped Warwolf's 2000, then it is Level 2 with 900 ATK.
#[test]
fn max_warrior_shrinks_after_destroying_a_monster() {
    let Some(mut s) = start([(&deck_with(&[MAX_WARRIOR]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(MAX_WARRIOR), Attack(MAX_WARRIOR)], &[Summon(WARWOLF)]]).unwrap();
    assert!(s.monster(1, WARWOLF).is_none(), "Warwolf should be destroyed: {}", s.state());
    assert_eq!(s.lp(1), 8000 - 200, "{}", s.state());
    assert_eq!(s.level_of(0, MAX_WARRIOR), Some(2), "{}", s.state());
    assert_eq!(s.attack_of(0, MAX_WARRIOR), Some(900), "{}", s.state());
    clean(&s);
}

/// Fortress Warrior (7985): "You take no battle damage from attacks involving this card. Once per turn, this card cannot
/// be destroyed by battle." Warwolf (2000) attacks its 600 ATK: no damage, and it stays.
#[test]
fn fortress_warrior_takes_no_battle_damage_and_survives_once() {
    let Some(mut s) = start([(&deck_with(&[FORTRESS_WARRIOR]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(FORTRESS_WARRIOR)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(0, FORTRESS_WARRIOR).is_some(), "{}", s.state());
    assert_eq!(s.lp(0), 8000, "{}", s.state());
    clean(&s);
}

/// Synchro Deflector (7758): "Activate only when a face-up Synchro Monster you control is selected as an attack target.
/// Negate the attack and destroy 1 monster your opponent controls." Goyo Guardian (2800) would beat the attacking
/// Warwolf and hurt its controller by 800; with the Trap the attack is negated and Warwolf destroyed: no damage.
#[test]
fn synchro_deflector_negates_the_attack_and_destroys_the_attacker() {
    let deck: Vec<u16> = [ZUBABA_KNIGHT, JUNK_SYNCHRON, SYNCHRO_DEFLECTOR].iter().copied().cycle().take(40).collect();
    let Some(mut s) = start([(&deck, &[GOYO_GUARDIAN]), (&[WARWOLF; 40], &[])]) else { return };
    let mine = [
        SetSpell(SYNCHRO_DEFLECTOR),
        Summon(ZUBABA_KNIGHT),
        Summon(JUNK_SYNCHRON),
        SpSummon(GOYO_GUARDIAN),
        ChainOnAttack(SYNCHRO_DEFLECTOR),
    ];
    // Warwolf waits a turn, so it attacks only once Goyo Guardian is there to be selected.
    s.run_both([&mine, &[Summon(WARWOLF), EndTurn, Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(1, WARWOLF).is_none(), "{}", s.state());
    assert_eq!(s.lp(1), 8000, "the attack was negated, no battle damage: {}", s.state());
    assert_eq!(s.count(0, "grave", SYNCHRO_DEFLECTOR), 1, "{}", s.state());
    clean(&s);
}

/// Mist Wurm (7549): "If this card is Synchro Summoned: Target up to 3 cards your opponent controls; return those
/// targets to the hand." Synchro Summoned from Junk Synchron and two Zubaba Knights (3 + 3 + 3), it returns Warwolf.
#[test]
fn mist_wurm_returns_the_opponents_monster() {
    let deck: Vec<u16> = [ZUBABA_KNIGHT, JUNK_SYNCHRON].iter().copied().cycle().take(40).collect();
    let Some(mut s) = start([(&deck, &[MIST_WURM]), (&[WARWOLF; 40], &[])]) else { return };
    let mine = [Summon(ZUBABA_KNIGHT), Summon(ZUBABA_KNIGHT), Summon(JUNK_SYNCHRON), SpSummon(MIST_WURM)];
    s.run_both([&mine, &[Summon(WARWOLF)]]).unwrap();
    assert!(s.monster(0, MIST_WURM).is_some(), "{}", s.state());
    assert!(s.monster(1, WARWOLF).is_none(), "Warwolf should be back in the hand: {}", s.state());
    clean(&s);
}

const WALL_OF_REVEALING_LIGHT: u16 = 5990;

/// Wall of Revealing Light (5990): "Activate by paying any multiple of 1000 Life Points." The client's cards and its
/// windows leave no number past the number dialog (#16): the client shows at most 99,999 LP (five figures), the core's
/// `ANNOUNCE_NUMBER` counts its options in a byte (300 options reached the server as the first 44), and no card
/// declares a minimum or step above 15 units. So the steps `translate_contract.rs` covers have no real card; this plays
/// the number dialog through the translator's windows against the real engine: 8 numbers of 1000, the 5000 picked is
/// what the engine takes.
#[test]
fn wall_of_revealing_light_pays_the_number_picked_in_the_dialog() {
    let Some(mut s) = start([(&deck_with(&[WALL_OF_REVEALING_LIGHT]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.declare_number(Some(5_000));
    s.run(&[SetSpell(WALL_OF_REVEALING_LIGHT), Activate(WALL_OF_REVEALING_LIGHT)]).unwrap();
    assert_eq!(s.lp(0), 8000 - 5000, "{}", s.context());
    assert_eq!(s.dialogs, 1, "8 numbers of 1000 are one number dialog");
    clean(&s);
}

// The NPC decks' cards the preset-deck coverage (`task check:decks`, 2026-11-20) listed as never activated, in the most
// decks first. Cards whose effect never starts a chain (Jinzo, Marshmallon, Cyber Dragon above) are listed too; those
// with a test here are ones the NPCs play in several decks.
const NEGATE_ATTACK: u16 = 4804;
const DIMENSION_WALL: u16 = 6448;
const WABOKU: u16 = 4886;
const DD_ASSAILANT: u16 = 5932;
const BLADEDGE: u16 = 6477;

/// Negate Attack (4804): "When an opponent's monster declares an attack: Target the attacking monster; negate the
/// attack, then end the Battle Phase." Command Knight (1600 with its own bonus) is attacked by Gene-Warped Warwolf.
#[test]
fn negate_attack_stops_the_attack() {
    let Some(mut s) = start([(&deck_with(&[COMMAND_KNIGHT, NEGATE_ATTACK]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(COMMAND_KNIGHT), SetSpell(NEGATE_ATTACK), ChainOnAttack(NEGATE_ATTACK)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(0, COMMAND_KNIGHT).is_some(), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000));
    assert_eq!(s.count(0, "grave", NEGATE_ATTACK), 1);
    clean(&s);
}

/// Dimension Wall (6448): "Activate only when an opponent's monster declares an attack. Instead of you, your opponent
/// takes the Battle Damage you would have taken from this battle." Command Knight still falls to Warwolf's 2000; the
/// 400 goes to the attacker's controller.
#[test]
fn dimension_wall_sends_the_battle_damage_back() {
    let Some(mut s) = start([(&deck_with(&[COMMAND_KNIGHT, DIMENSION_WALL]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(COMMAND_KNIGHT), SetSpell(DIMENSION_WALL), ChainOnAttack(DIMENSION_WALL)], &[Summon(WARWOLF), Attack(WARWOLF)]])
        .unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 400), "{}", s.context());
    assert_eq!(s.count(0, "grave", COMMAND_KNIGHT), 1);
    clean(&s);
}

/// Waboku (4886): "You take no battle damage this turn. Your monsters cannot be destroyed by battle this turn."
#[test]
fn waboku_saves_the_monster_and_the_life_points() {
    let Some(mut s) = start([(&deck_with(&[COMMAND_KNIGHT, WABOKU]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(COMMAND_KNIGHT), SetSpell(WABOKU), ChainOnAttack(WABOKU)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(0, COMMAND_KNIGHT).is_some(), "{}", s.context());
    assert_eq!(s.lp(0), 8000);
    assert_eq!(s.count(0, "grave", WABOKU), 1);
    clean(&s);
}

/// D.D. Assailant (5932): "After damage calculation, when this card is destroyed by battle with an opponent's monster:
/// Banish that monster, also banish this card." 1700 against Warwolf's 2000: 300 damage, then both are banished.
#[test]
fn dd_assailant_banishes_the_monster_that_destroyed_it() {
    let Some(mut s) = start([(&deck_with(&[DD_ASSAILANT]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[Summon(DD_ASSAILANT)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 300, "{}", s.context());
    assert_eq!(s.count(0, "removed", DD_ASSAILANT), 1, "{}", s.state());
    assert_eq!(s.count(1, "removed", WARWOLF), 1);
    assert!(s.monster(1, WARWOLF).is_none());
    clean(&s);
}

/// Elemental HERO Bladedge (6477): "If this card attacks a Defense Position monster, inflict piercing battle damage."
/// Tribute Summoned from two Warwolves, it attacks a Set Spirit Reaper (DEF 200, which can't be destroyed by battle):
/// 2600 - 200 = 2400.
#[test]
fn bladedge_pierces_a_defense_position_monster() {
    let Some(mut s) = start([(&deck_with(&[BLADEDGE]), &[]), (&deck_with(&[SPIRIT_REAPER]), &[])]) else { return };
    let plan = [Summon(WARWOLF), EndTurn, Summon(WARWOLF), EndTurn, Summon(BLADEDGE), Attack(BLADEDGE)];
    s.run_both([&plan, &[SetMonster(SPIRIT_REAPER)]]).unwrap();
    assert!(s.monster(0, BLADEDGE).is_some(), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2400, "{}", s.state());
    clean(&s);
}

const GRAVITY_BIND: u16 = 5134;

/// Gravity Bind (5134): "Level 4 or higher monsters cannot attack." Its hold on the monsters it stops is drawn as the
/// client's hover marker type 5 (#16, a guess from Konami's logs: continuous cards with the monsters they change), from
/// its Spell/Trap zone to each Level 4 monster, both ways, on both seats' screens (each from its own side).
#[test]
fn gravity_bind_marks_the_monsters_it_holds_back() {
    let Some(mut s) = start([(&deck_with(&[GRAVITY_BIND]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[SetSpell(GRAVITY_BIND), Chain(GRAVITY_BIND)], &[Summon(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "szone", GRAVITY_BIND), 1, "{}", s.context());
    let held: Vec<_> = s.markers(0).iter().filter(|m| m.2 == 5).collect();
    assert!(!held.is_empty(), "no marker from Gravity Bind: {:?} {}", s.markers(0), s.state());
    // From the planned side's Spell/Trap zone (5-9) to the opponent's monster (player 1, zone 0-4), and back.
    assert!(held.iter().any(|&&(a, b, _)| a & 0xFF == 0 && (5..=9).contains(&(a >> 8)) && b & 0xFF == 1 && b >> 8 <= 4), "{held:?}");
    assert!(held.iter().any(|&&(a, b, _)| a & 0xFF == 1 && b & 0xFF == 0), "both ways: {held:?}");
    assert!(s.markers(1).iter().any(|m| m.2 == 5), "the opponent's screen too: {:?}", s.markers(1));
    clean(&s);
}

const SOGEN: u16 = 4339;

/// Sogen (4339): "All Warrior and Beast-Warrior monsters on the field gain 200 ATK/DEF." Command Knight (a Warrior,
/// 1600 with its own bonus) gets 1800, and the Field Spell's hold on it is the hover marker type 3 (#16, a guess from
/// Konami's logs: a monster and the Field Spell that affects it), from the Field Spell zone (10).
#[test]
fn sogen_marks_the_warriors_it_powers_up() {
    let Some(mut s) = start([(&deck_with(&[COMMAND_KNIGHT, SOGEN]), &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Activate(SOGEN), Summon(COMMAND_KNIGHT)]).unwrap();
    assert_eq!(s.attack_of(0, COMMAND_KNIGHT), Some(1800), "{}", s.state());
    let field: Vec<_> = s.markers(0).iter().filter(|m| m.2 == 3).collect();
    assert!(field.iter().any(|&&(a, b, _)| a == 10 << 8 && b & 0xFF == 0 && b >> 8 <= 4), "{:?}", s.markers(0));
    clean(&s);
}

// Doppelwarrior (a player's report, 2026-11-22: "I Synchro Summon Bulb using Junk Synchron's effect but it didn't let me
// trigger its effect"): the engine offered it, but at the level the game's default Activation Message answers No to. Its 2008 text: "When a monster(s) is Special Summoned from your GY: You can Special Summon this
// card from your hand. If this card is sent to the GY as Synchro Material: You can Special Summon 2 Doppel Tokens".
const DOPPELWARRIOR: u16 = 9293;
const GLOW_UP_BULB: u16 = 9095;
const JUNK_WARRIOR: u16 = 7696;

/// Glow-Up Bulb in the Graveyard, Junk Synchron and Doppelwarrior in the hand.
fn doppelwarrior_board(extra: &[u16]) -> Option<Scenario> {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let dir = engine_dir()?;
    let field = [Spawn { team: 0, place: Grave, cid: GLOW_UP_BULB }, Spawn { team: 0, place: Hand, cid: JUNK_SYNCHRON },
        Spawn { team: 0, place: Hand, cid: DOPPELWARRIOR }];
    let deck: Vec<u16> = [WARWOLF; 37].into_iter().chain([GLOW_UP_BULB, JUNK_SYNCHRON, DOPPELWARRIOR]).collect();
    Some(Scenario::start_field(&dir, 7, 8000, [(&deck, extra), (&[WARWOLF; 40], &[])], &field).unwrap())
}

#[test]
fn doppelwarrior_comes_from_the_hand_when_junk_synchron_brings_a_monster_back_from_the_graveyard() {
    let Some(mut s) = doppelwarrior_board(&[]) else { return };
    s.prefer(&[(0, GLOW_UP_BULB)]);
    s.run(&[Summon(JUNK_SYNCHRON), Chain(DOPPELWARRIOR)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.count(0, "mzone", GLOW_UP_BULB), 1, "Junk Synchron brought Glow-Up Bulb back");
    assert_eq!(s.count(0, "mzone", DOPPELWARRIOR), 1, "Doppelwarrior came from the hand");
}

#[test]
fn the_player_is_asked_about_doppelwarrior_in_the_hand_and_yes_summons_it() {
    // The same moment through the game's windows: the effect question shows Doppelwarrior, and Yes reaches the engine.
    let Some(mut s) = doppelwarrior_board(&[]) else { return };
    s.prefer(&[(0, GLOW_UP_BULB)]);
    let offers = |p: &serde_json::Value| p["type"] == 16 && p["selects"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == DOPPELWARRIOR));
    s.run_to(&[Summon(JUNK_SYNCHRON)], offers).unwrap();
    let out = s.windows(0).expect("the player is asked");
    let id = |m: &yo3_proto::Message, k: &str| m.get(k).and_then(|v| v.as_u32());
    assert!(out.iter().any(|m| id(m, "id") == Some(0x3F2)), "the question shows the card: {out:?}");
    // The effect dialog's level (param3): 3 shows whatever the player's Activation Message option; at 1 the default
    // option answered No by itself, which is what the player saw.
    let dialog = out.iter().find(|m| id(m, "id") == Some(0x36)).expect("the effect dialog");
    assert_eq!(id(dialog, "param3"), Some(3), "{dialog:?}");
    s.answer_in_windows(0, &[], 1).unwrap();
    s.run(&[]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.count(0, "mzone", DOPPELWARRIOR), 1, "Yes summoned it");
}

#[test]
fn doppelwarrior_as_synchro_material_brings_two_doppel_tokens() {
    let Some(mut s) = doppelwarrior_board(&[JUNK_WARRIOR]) else { return };
    s.prefer(&[(0, GLOW_UP_BULB), (0, DOPPELWARRIOR)]);
    s.run(&[Summon(JUNK_SYNCHRON), Chain(DOPPELWARRIOR), SpSummon(JUNK_WARRIOR), Chain(DOPPELWARRIOR)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "Junk Warrior from Junk Synchron and Doppelwarrior");
    let monsters = s.state()[0]["mzone"].as_array().map_or(0, |m| m.iter().filter(|c| !c.is_null()).count());
    assert_eq!(monsters, 4, "Junk Warrior, Glow-Up Bulb and two Doppel Tokens: {:#?}", s.state()[0]["mzone"]);
}

// Black Brutdrago (#87): "1 Tuner + 1 or more non-Tuner Gemini monsters". A player's game closed when they Synchro
// Summoned it. Materials in the client: Blackwing - Gale the Whirlwind (a Level 3 Tuner) and King Pyron (a Level 5
// Gemini): 3 + 5 = 8.
const BLACK_BRUTDRAGO: u16 = 8478;
const GALE: u16 = 8011;
const KING_PYRON: u16 = 7203;

#[test]
fn black_brutdrago_is_synchro_summoned_from_a_tuner_and_a_gemini_issue_87() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: GALE }, Spawn { team: 0, place: Monster, cid: KING_PYRON }];
    let mut s = Scenario::start_field(&dir, 7, 8000, [(&[WARWOLF; 40], &[BLACK_BRUTDRAGO]), (&[WARWOLF; 40], &[])], &field).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    s.run(&[SpSummon(BLACK_BRUTDRAGO)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, BLACK_BRUTDRAGO), Some(3000), "Black Brutdrago is on the field");
}

// The reporter's own materials (#87): Tuned Magician (a Level 4 Gemini, a Tuner only while it has its effect) and
// Evocator Chevalier (a Level 4 Gemini), both given their effects by Unleash Your Power!, then Black Brutdrago.
// Tuned Magician is a Tuner by an effect, which the client's card data doesn't say.
#[test]
fn black_brutdrago_from_a_tuned_magician_unleash_your_power_made_a_tuner_issue_87() {
    use yo3_server::duelcheck::scenario::card;
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let (tuned, chevalier, unleash) = (card("Tuned Magician"), card("Evocator Chevalier"), card("Unleash Your Power!"));
    let field = [Spawn { team: 0, place: Monster, cid: tuned }, Spawn { team: 0, place: Monster, cid: chevalier },
        Spawn { team: 0, place: Hand, cid: unleash }];
    let mut s = Scenario::start_field(&dir, 7, 8000, [(&[WARWOLF; 40], &[BLACK_BRUTDRAGO]), (&[WARWOLF; 40], &[])], &field).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    s.run(&[Activate(unleash), SpSummon(BLACK_BRUTDRAGO)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, BLACK_BRUTDRAGO), Some(3000), "Black Brutdrago is on the field");
}

// The same going second (engine team 1), as in the live NPC duel where Kaiba went first.
#[test]
fn black_brutdrago_from_a_tuned_magician_going_second_issue_87() {
    use yo3_server::duelcheck::scenario::card;
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let (tuned, chevalier, unleash) = (card("Tuned Magician"), card("Evocator Chevalier"), card("Unleash Your Power!"));
    let field = [Spawn { team: 1, place: Monster, cid: tuned }, Spawn { team: 1, place: Monster, cid: chevalier },
        Spawn { team: 1, place: Hand, cid: unleash }];
    let mut s = Scenario::start_field(&dir, 7, 8000, [(&[WARWOLF; 40], &[]), (&[WARWOLF; 40], &[BLACK_BRUTDRAGO])], &field).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    s.run_both([&[EndTurn], &[Activate(unleash), SpSummon(BLACK_BRUTDRAGO)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(1, BLACK_BRUTDRAGO), Some(3000), "Black Brutdrago is on the field");
}

// Sillva, Warlord of Dark World (#88): discarded from the hand by a card effect, it Special Summons itself. The Tricky
// and Raigeki Break discard as a cost (no effect: it stays in the Graveyard, as the rules say); Morphing Jar's and Card
// Destruction's discards are effects. (A Morphing Jar flipped by an attack discards it in the Damage Step, where the
// engine raises no trigger for it: open, docs/roadmap.md.)
#[test]
fn sillva_comes_back_when_card_destruction_discards_it_issue_88() {
    use yo3_server::duelcheck::scenario::card;
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let (sillva, destruction) = (card("Sillva, Warlord of Dark World"), card("Card Destruction"));
    let field = [Spawn { team: 0, place: Hand, cid: sillva }, Spawn { team: 0, place: Hand, cid: destruction }];
    let mut s = Scenario::start_field(&dir, 7, 8000, [(&[WARWOLF; 40], &[]), (&[WARWOLF; 40], &[])], &field).unwrap();
    s.run(&[Activate(destruction)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.count(0, "mzone", sillva), 1, "Sillva came back: {:#?}", s.state()[0]);
}

#[test]
fn sillva_comes_back_when_a_flip_summoned_morphing_jar_discards_it_issue_88() {
    use yo3_server::duelcheck::scenario::card;
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let (sillva, jar) = (card("Sillva, Warlord of Dark World"), card("Morphing Jar"));
    let field = [Spawn { team: 0, place: MonsterSet, cid: jar }, Spawn { team: 0, place: Hand, cid: sillva }];
    let mut s = Scenario::start_field(&dir, 7, 8000, [(&[WARWOLF; 40], &[]), (&[WARWOLF; 40], &[])], &field).unwrap();
    s.run(&[Reposition(jar)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.count(0, "mzone", sillva), 1, "Sillva came back: {:#?}", s.state()[0]["grave"]);
}
