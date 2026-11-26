//! Copy Tokens (#75): Tokens whose Level, Type and Attribute their card's effect gives, copied from a monster. Their rows
//! (engine/added-cards.json, rows 5325-5329) stand in with Fiend, DARK, Level 1 (`tools/engine/added-cards.py`'s
//! `encode`); the engine's field report brings the real values as each is summoned, and both seats' client views are
//! checked against the engine after every step, Type and Attribute included. The copied monsters are of another Type and
//! Attribute than the stand-in (Giant Axe Mummy: Zombie, EARTH), so a view still showing the stand-in fails.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The added cards (the ids the client knows).
const DOPPELGANGER_TOKEN: u16 = 6014;
const ANIME_DOPPELGANGER_TOKEN: u16 = 6015;
const SELECTION_TOKEN: u16 = 6016;
const CELL_DIVISION_TOKEN: u16 = 6017;
const PARASITE_TOKEN: u16 = 6074;
const SPIRIT_ILLUSION: u16 = 6077;
const NATURAL_SELECTION: u16 = 6080;
const CELL_DIVISION: u16 = 6081;
const RECKLESS_PARASITE: u16 = 6091;
// The game's own.
const GIANT_AXE_MUMMY: u16 = 5420; // Zombie, EARTH, Level 5, 1700/2000
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100
const BASIC_INSECT: u16 = 4056; // Insect, EARTH, Level 2, 500/700
const PARASITE_PARACIDE: u16 = 4769;
const FILLER: u16 = 4012; // Feral Imp

const ZOMBIE: i64 = 0x10;
const INSECT: i64 = 0x800;
const EARTH: i64 = 0x1;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn start(anime: bool, opponent_deck: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    if anime && !dir.join("vendor/BabelCDB/cards-unofficial.cdb").is_file() {
        return None;
    }
    let theirs: Vec<u16> = opponent_deck.iter().copied().chain(std::iter::repeat(FILLER)).take(40).collect();
    let decks = [(&[FILLER; 40][..], &[][..]), (&theirs[..], &[][..])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// A monster's (ATK, DEF, Level, Type, Attribute) as the engine has it.
type Stats = (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>);

fn stats(s: &Scenario, team: usize, cid: u16) -> Stats {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} on the field: {}", s.context())).clone();
    let n = |k: &str| m[k].as_i64();
    (n("attack"), n("defense"), n("level"), n("race"), n("attribute"))
}

/// Spirit Illusion: "Special Summon 1 "Doppelganger Token" (Level 5/ATK ?/DEF 0) with the equipped monster's Type,
/// Attribute, and ATK." Equipped to Giant Axe Mummy: a Level 5 Zombie EARTH Token with 1700 ATK.
#[test]
fn spirit_illusion_makes_a_doppelganger_token_like_the_equipped_monster() {
    let Some(mut s) = start(false, &[], &[at(0, Monster, GIANT_AXE_MUMMY), at(0, Hand, SPIRIT_ILLUSION)]) else { return };
    s.prefer(&[(0, GIANT_AXE_MUMMY)]);
    s.choose_options(&[1]);
    s.run(&[Activate(SPIRIT_ILLUSION), Activate(SPIRIT_ILLUSION)]).unwrap();
    clean(&s);
    assert_eq!(stats(&s, 0, DOPPELGANGER_TOKEN), (Some(1700), Some(0), Some(5), Some(ZOMBIE), Some(EARTH)), "{}", s.context());
}

/// The anime Spirit Illusion (anime cards on): "The ATK, DEF, Level, Attribute, and Type of this Token are the same as the
/// monster equipped with "Spirit Illusion"." Its own Doppelganger Token, another row.
#[test]
fn anime_spirit_illusion_makes_its_doppelganger_token_like_the_equipped_monster() {
    let Some(mut s) = start(true, &[], &[at(0, Monster, GIANT_AXE_MUMMY), at(0, Hand, SPIRIT_ILLUSION)]) else { return };
    s.prefer(&[(0, GIANT_AXE_MUMMY)]);
    s.run(&[Activate(SPIRIT_ILLUSION), Activate(SPIRIT_ILLUSION)]).unwrap();
    clean(&s);
    let (atk, def, level, race, attribute) = stats(&s, 0, ANIME_DOPPELGANGER_TOKEN);
    assert_eq!((atk, def, level, race, attribute), (Some(1700), Some(2000), Some(5), Some(ZOMBIE), Some(EARTH)), "{}", s.context());
}

/// Natural Selection (anime): "During your opponent's turn: Destroy 1 face-up monster your opponent controls, and Special
/// Summon 1 "Selection Token" with the same Level, Type, Attribute, ATK, and DEF as the destroyed monster, but it is
/// destroyed during the End Phase." The opponent's Warwolf becomes a Level 4 2000/100 Token on this side, gone at the End
/// Phase.
#[test]
fn natural_selection_turns_an_opponents_monster_into_a_selection_token_for_the_turn() {
    let Some(mut s) = start(true, &[], &[at(0, SpellSet, NATURAL_SELECTION), at(1, Hand, WARWOLF)]) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn, Chain(NATURAL_SELECTION)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    let warwolf = s.state()[1]["grave"].clone();
    let (atk, def, level, _, _) = stats(&s, 0, SELECTION_TOKEN);
    assert_eq!((atk, def, level), (Some(2000), Some(100), Some(4)), "Warwolf's stats: {warwolf} {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", SELECTION_TOKEN), s.count(0, "grave", SELECTION_TOKEN)), (0, 0), "destroyed at the End Phase: {}", s.context());
}

/// Cell Division (anime): "Target 1 Level 3 or lower Insect-Type monster you control; Special Summon 1 "Cell Division
/// Token" that has the same original Level, Type, Attribute, ATK and DEF as that monster." Basic Insect: a Level 2 Insect
/// EARTH 500/700 Token.
#[test]
fn cell_division_copies_a_small_insect() {
    let Some(mut s) = start(true, &[], &[at(0, Monster, BASIC_INSECT), at(0, Hand, CELL_DIVISION)]) else { return };
    s.prefer(&[(0, BASIC_INSECT)]);
    s.run(&[Activate(CELL_DIVISION)]).unwrap();
    clean(&s);
    assert_eq!(stats(&s, 0, CELL_DIVISION_TOKEN), (Some(500), Some(700), Some(2), Some(INSECT), Some(EARTH)), "{}", s.context());
}

/// Reckless Parasite (anime): "If this card is activated: Special Summon 1 "Parasite Paracide" from your opponent's Deck to
/// their side of the field in Attack Position. Monsters currently on the field ... are equipped with a "Parasite Token".
/// While a "Parasite Paracide" and "Reckless Parasite" are on the field, monsters equipped with a "Parasite Token" are
/// treated as Insect-Type." Warwolf gets a Parasite Token as an equip card and becomes an Insect.
#[test]
fn reckless_parasite_equips_parasite_tokens() {
    let Some(mut s) = start(true, &[PARASITE_PARACIDE], &[at(0, Monster, WARWOLF), at(0, Hand, RECKLESS_PARASITE)]) else { return };
    s.run(&[Activate(RECKLESS_PARASITE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", PARASITE_PARACIDE), 1, "{}", s.context());
    assert!(s.count(0, "szone", PARASITE_TOKEN) >= 1, "a Parasite Token equipped: {}", s.context());
    assert_eq!(stats(&s, 0, WARWOLF).3, Some(INSECT), "{}", s.context());
}
