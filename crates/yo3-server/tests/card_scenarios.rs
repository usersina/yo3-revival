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
