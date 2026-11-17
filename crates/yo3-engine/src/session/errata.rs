//! The cards `engine/overrides/` plays by the text the client shows (its CARD_Desc, the 2010–2012 wording) where
//! today's text, and ProjectIgnis' script, say something else: each test sets out a position and plays it through
//! (`Play`), and fails with ProjectIgnis' script of the card.
use std::collections::HashMap;

use super::*;
use yo3_ocg::{msg};

const GIANT_ORC: u32 = 73698349; // a Level 4 DARK Warrior, 2200 ATK, no effect: every Deck's filler
const CHOSEN_BY_THE_WORLD_CHALICE: u32 = 22916281; // a Level 3 Psychic Normal Monster
const NEKOGAL_1: u32 = 1761063; // a Level 3 Beast Normal Monster
const SUMMONED_SKULL: u32 = 70781052; // Level 6: 1 Tribute
const BLUE_EYES_WHITE_DRAGON: u32 = 89631139; // Level 8: 2 Tributes
const JAR_OF_GREED: u32 = 83968380; // a Trap: draw 1
const MYSTICAL_SPACE_TYPHOON: u32 = 5318639; // a Spell
const MONSTER_REBORN: u32 = 83764718;
const UP: u32 = decode::POSITION_FACEUP_ATTACK;
const SET: u32 = 0xA; // POS_FACEDOWN, for a Set Spell or Trap
/// A location for `Play::new` only: the Field Zone (the Spell & Trap Zone's sixth).
const FIELD_ZONE: u32 = 0x100;


/// A duel a test plays through: the prompt it waits at (team 0's or team 1's idle or battle prompt; the others get
/// `default_answer`) and every message so far.
struct Play {
    session: Session,
    prompt: Json,
    seen: Vec<Json>,
    /// Cards whose optional effects are used whenever they are offered (other optional effects are passed).
    use_cards: Vec<u32>,
    /// Cards a default choice of cards (to Tribute, discard, target...) leaves alone while it has others to choose.
    keep: Vec<u32>,
}

impl Play {
    /// `cards` (team, code, location, position) set out, Monster and Spell & Trap Zones filled from the left, ten Giant
    /// Orcs in each Deck besides; run to the first idle prompt. None without the engine data (engine/fetch.sh not run).
    fn new(cards: &[(u8, u32, u32, u32)]) -> Option<Play> {
        Play::with_filler(cards, GIANT_ORC)
    }

    /// `new`, with ten `filler` in each Deck.
    fn with_filler(cards: &[(u8, u32, u32, u32)], filler: u32) -> Option<Play> {
        let vendor = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine/vendor");
        if !vendor.join("BabelCDB/cards.cdb").is_file() {
            return None;
        }
        let mut session = Session::new(Cards::open(&vendor).expect("the card data"));
        let mut duel = session.make_duel([1, 2, 3, 4], 8000, 0, session.log.clone()).unwrap();
        for team in 0..2 {
            for _ in 0..10 {
                duel.new_card(team, 0, filler, team, LOCATION_DECK, 0, POSITION_FACEDOWN_DEFENSE);
            }
        }
        let mut zones: HashMap<(u8, u32), u32> = HashMap::new();
        for &(team, code, location, position) in cards {
            let next = zones.entry((team, location)).or_default();
            let (location, sequence) = match location {
                FIELD_ZONE => (LOCATION_SZONE, 5),
                LOCATION_MZONE | LOCATION_SZONE => (location, *next),
                _ => (location, 0),
            };
            *next += 1;
            duel.new_card(team, 0, code, team, location, sequence, position);
        }
        duel.start();
        session.duel = Some(duel);
        let mut play = Play { session, prompt: Json::Null, seen: vec![], use_cards: vec![], keep: vec![] };
        play.run_on();
        Some(play)
    }

    /// Runs the duel, answering every prompt but the idle and battle ones with `default_answer`.
    fn run_on(&mut self) {
        loop {
            let (messages, status) = self.session.run().unwrap();
            assert!(messages.iter().all(|m| m["type"] != msg::RETRY), "the core refused the answer to {}", self.prompt);
            let log = std::mem::take(&mut self.session.log.borrow_mut().lines);
            assert_eq!(log, Vec::<String>::new(), "the core's log at {}", self.prompt);
            self.seen.extend(messages.iter().cloned());
            if status != "await" {
                self.prompt = Json::Null;
                return;
            }
            self.prompt = messages.last().cloned().unwrap();
            if matches!(self.prompt["type"].as_u64(), Some(10 | 11)) {
                return;
            }
            let answer = self.default_answer();
            self.respond_only(answer);
        }
    }

    fn respond_only(&mut self, answer: Json) {
        self.session.duel.as_mut().unwrap().respond(&crate::encode::response(&answer));
    }

    fn respond(&mut self, answer: Json) {
        self.respond_only(answer);
        self.run_on();
    }

    /// What a player who uses only `use_cards`' optional effects answers: the fewest cards, the first place.
    fn default_answer(&self) -> Json {
        let p = &self.prompt;
        let used = |s: &Json| s["code"].as_u64().is_some_and(|c| self.use_cards.contains(&(c as u32)));
        let kept = |s: &Json| s["code"].as_u64().is_some_and(|c| self.keep.contains(&(c as u32)));
        // The first `min` of `list`, the kept cards last.
        let pick = |list: &str| {
            let mut order: Vec<usize> = (0..p[list].as_array().unwrap().len()).collect();
            order.sort_by_key(|&i| kept(&p[list][i]));
            order.truncate(p["min"].as_u64().unwrap() as usize);
            order
        };
        match p["type"].as_u64().unwrap_or(0) {
            16 => match p["selects"].as_array().unwrap().iter().position(used) {
                Some(i) => json!({"type": 8, "index": i}),
                None if p["forced"] == true => json!({"type": 8, "index": 0}),
                None => json!({"type": 8, "index": null}),
            },
            12 => json!({"type": 2, "yes": used(p)}),
            13 => json!({"type": 3, "yes": true}),
            14 => json!({"type": 4, "index": 0}),
            15 => json!({"type": 5, "indicies": pick("selects")}),
            20 => json!({"type": 12, "indicies": pick("selects")}),
            26 if p["can_finish"] == true => json!({"type": 7, "index": null}),
            26 => json!({"type": 7, "index": p["select_cards"].as_array().unwrap().iter().position(|c| !kept(c)).unwrap_or(0)}),
            18 | 24 => {
                // A set bit of the mask is a zone that can't be chosen; the low half is the choosing player's.
                let player = p["player"].as_u64().unwrap();
                let mask = p["field_mask"].as_u64().unwrap();
                let places: Vec<Json> = (0..32)
                    .filter(|b| mask & 1 << b == 0)
                    .take(p["count"].as_u64().unwrap().max(1) as usize)
                    .map(|b| {
                        let location = if b % 16 < 8 { LOCATION_MZONE } else { LOCATION_SZONE };
                        json!({"player": if b < 16 { player } else { 1 - player }, "location": location, "sequence": b % 8})
                    })
                    .collect();
                json!({"type": if p["type"] == 18 { 10 } else { 9 }, "places": places})
            }
            19 => json!({"type": 11, "position": 1 << p["positions"].as_u64().unwrap().trailing_zeros()}),
            21 | 25 => json!({"type": 15, "order": null}),
            _ => panic!("no default answer to {p}"),
        }
    }

    fn idle(&self) -> &Json {
        assert_eq!(self.prompt["type"], 11, "an idle prompt");
        &self.prompt
    }

    /// The index in the idle prompt's `list` of the first entry for `code` with `description` (any, if None).
    fn find(&self, list: &str, code: u32, description: Option<u64>) -> Option<usize> {
        self.idle()[list].as_array().unwrap().iter().position(|a| a["code"] == code && description.is_none_or(|d| a["description"] == d))
    }

    fn can_activate(&self, code: u32) -> bool {
        self.find("activates", code, None).is_some()
    }

    /// Effect `n` of `code` (its `aux.Stringid(id, n)` description) can be activated.
    fn can_activate_effect(&self, code: u32, n: u64) -> bool {
        self.find("activates", code, Some((code as u64) << 20 | n)).is_some()
    }

    fn act(&mut self, list: &str, action: u32, code: u32, description: Option<u64>) {
        let index = self.find(list, code, description).unwrap_or_else(|| panic!("{code} in {list}: {}", self.prompt));
        self.respond(json!({"type": 1, "action": action, "index": index}));
    }

    fn activate(&mut self, code: u32) {
        self.act("activates", 5, code, None);
    }

    fn activate_effect(&mut self, code: u32, n: u64) {
        self.act("activates", 5, code, Some((code as u64) << 20 | n));
    }

    fn summon(&mut self, code: u32) {
        self.act("summons", 0, code, None);
    }

    fn set_spell(&mut self, code: u32) {
        self.act("spell_sets", 4, code, None);
    }

    /// To the End Phase from the idle or battle prompt, then on to the next turn's first idle prompt.
    fn end_turn(&mut self) {
        let answer = match self.prompt["type"].as_u64() {
            Some(11) => json!({"type": 1, "action": 7, "index": 0}),
            Some(10) => json!({"type": 0, "action": 3, "index": 0}),
            _ => panic!("not an idle or battle prompt: {}", self.prompt),
        };
        self.respond(answer);
    }

    /// The codes of the cards at `location` of `team`'s.
    fn cards(&mut self, team: u8, location: u32) -> Vec<u32> {
        let found = self.session.query_location(decode::QUERY_CODE, team, location).unwrap();
        found.into_iter().flatten().filter_map(|q| q.get(decode::QUERY_CODE)).collect()
    }

    fn count(&mut self, team: u8, location: u32, code: u32) -> usize {
        self.cards(team, location).into_iter().filter(|c| *c == code).count()
    }

    /// `team`'s LP, from the messages so far.
    fn lp(&self, team: u64) -> i64 {
        self.seen.iter().filter(|m| m["player"] == team).fold(8000, |lp, m| {
            let amount = m["amount"].as_i64().unwrap_or(0);
            match m["type"].as_u64() {
                Some(msg::DAMAGE | msg::PAY_LPCOST) => lp - amount,
                Some(msg::RECOVER) => lp + amount,
                Some(msg::LPUPDATE) => m["lp"].as_i64().unwrap(),
                _ => lp,
            }
        })
    }

    /// The prompts for team 0's `code` to use an effect (MSG_SELECT_EFFECTYN) so far.
    fn asked(&self, code: u32) -> usize {
        self.seen.iter().filter(|m| m["type"] == 12 && m["code"] == code).count()
    }

    /// The position of the card in `team`'s Monster Zone `sequence`.
    fn position(&mut self, team: u8, sequence: u32) -> u32 {
        let card = self.session.query(decode::QUERY_POSITION, team, LOCATION_MZONE, sequence).unwrap().expect("a monster");
        card.get(decode::QUERY_POSITION).unwrap()
    }

    /// From the idle prompt to the Battle Phase's prompt.
    fn battle(&mut self) {
        assert_eq!(self.idle()["to_bp"], true, "the Battle Phase can be entered: {}", self.prompt);
        self.respond(json!({"type": 1, "action": 6, "index": 0}));
        assert_eq!(self.prompt["type"], 10, "the battle prompt");
    }
}

/// `Play::new`, or return from the test without the engine data.
macro_rules! play {
    ($cards:expr) => {
        match Play::new($cards) {
            Some(play) => play,
            None => return,
        }
    };
}

#[test]
fn sangan_and_witch_of_the_black_forest_search_every_time() {
    // engine/overrides/c26202165.lua, c78010363.lua: "When this card is sent from the field to the Graveyard, add 1
    // monster ..." with no once-per-turn limit; today's text adds one. Both copies Tributed at once: both search.
    for code in [26202165, 78010363] {
        let mut cards = vec![(0, code, LOCATION_MZONE, UP), (0, code, LOCATION_MZONE, UP), (0, BLUE_EYES_WHITE_DRAGON, LOCATION_HAND, UP)];
        cards.extend([(0, NEKOGAL_1, LOCATION_DECK, POSITION_FACEDOWN_DEFENSE); 3]); // 1100 ATK, 900 DEF
        let mut play = play!(&cards);
        let hand = play.cards(0, LOCATION_HAND).len();
        play.summon(BLUE_EYES_WHITE_DRAGON);
        assert_eq!(play.cards(0, LOCATION_HAND).len(), hand - 1 + 2, "{code}: each copy added a monster: {}", play.prompt);
    }
}

#[test]
fn sinister_serpents_both_come_back() {
    // engine/overrides/c8131171.lua: no once-per-turn limit in the client's text, and nothing banished later.
    const SINISTER_SERPENT: u32 = 8131171;
    let mut play = play!(&[(0, SINISTER_SERPENT, LOCATION_GRAVE, UP), (0, SINISTER_SERPENT, LOCATION_GRAVE, UP)]);
    play.use_cards = vec![SINISTER_SERPENT];
    play.end_turn();
    play.end_turn();
    assert_eq!(play.idle()["player"], 0, "team 0's second turn");
    assert_eq!(play.count(0, LOCATION_HAND, SINISTER_SERPENT), 2, "both returned in the Standby Phase");
}

#[test]
fn rescue_cat_can_be_used_again() {
    // engine/overrides/c14878871.lua: no once-per-turn limit (and no negation of the Beasts' effects).
    const RESCUE_CAT: u32 = 14878871;
    let mut cards = vec![(0, RESCUE_CAT, LOCATION_MZONE, UP), (0, RESCUE_CAT, LOCATION_MZONE, UP)];
    cards.extend([(0, NEKOGAL_1, LOCATION_DECK, POSITION_FACEDOWN_DEFENSE); 8]);
    let mut play = play!(&cards);
    play.activate(RESCUE_CAT);
    assert_eq!(play.count(0, LOCATION_MZONE, NEKOGAL_1), 2);
    assert!(play.can_activate(RESCUE_CAT), "the other Rescue Cat too: {}", play.prompt);
}

#[test]
fn dark_magician_of_chaos_adds_a_spell_when_summoned() {
    // engine/overrides/c40737112.lua: "When this card is Normal or Special Summoned"; today's text waits for the End
    // Phase.
    const DARK_MAGICIAN_OF_CHAOS: u32 = 40737112;
    let mut play = play!(&[
        (0, DARK_MAGICIAN_OF_CHAOS, LOCATION_HAND, UP),
        (0, GIANT_ORC, LOCATION_MZONE, UP),
        (0, GIANT_ORC, LOCATION_MZONE, UP),
        (0, MYSTICAL_SPACE_TYPHOON, LOCATION_GRAVE, UP),
    ]);
    play.use_cards = vec![DARK_MAGICIAN_OF_CHAOS];
    play.summon(DARK_MAGICIAN_OF_CHAOS);
    assert_eq!(play.count(0, LOCATION_HAND, MYSTICAL_SPACE_TYPHOON), 1, "in Main Phase 1: {}", play.prompt);
}

#[test]
fn brionac_returns_your_own_cards_too() {
    // engine/overrides/c50321796.lua: "return the same number of cards from the field"; today's text takes only the
    // opponent's (and adds a once-per-turn limit).
    const BRIONAC: u32 = 50321796;
    let mut play = play!(&[(0, BRIONAC, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_HAND, UP)]);
    play.keep = vec![BRIONAC];
    assert!(play.can_activate(BRIONAC), "the opponent has no cards: {}", play.prompt);
    play.activate(BRIONAC);
    assert_eq!(play.cards(0, LOCATION_GRAVE), [GIANT_ORC], "the discard");
    assert_eq!(play.cards(0, LOCATION_MZONE), [BRIONAC], "the Giant Orc on the field went back to the hand");
    assert!(play.can_activate(BRIONAC), "and again: {}", play.prompt);
}

#[test]
fn dark_strike_fighter_can_be_used_again() {
    // engine/overrides/c32646477.lua: no once-per-turn limit (or Main Phase 1 condition) in the client's text.
    const DARK_STRIKE_FIGHTER: u32 = 32646477;
    let mut play = play!(&[(0, DARK_STRIKE_FIGHTER, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_MZONE, UP)]);
    play.keep = vec![DARK_STRIKE_FIGHTER];
    play.activate(DARK_STRIKE_FIGHTER);
    assert_eq!(play.lp(1), 8000 - 4 * 200);
    assert!(play.can_activate(DARK_STRIKE_FIGHTER), "again: {}", play.prompt);
}

#[test]
fn each_ancient_fairy_dragon_uses_its_effects_once_a_turn() {
    // engine/overrides/c25862681.lua: "Once per turn", each copy for itself; today's text allows each effect once per
    // turn by name.
    const ANCIENT_FAIRY_DRAGON: u32 = 25862681;
    let mut play = play!(&[
        (0, ANCIENT_FAIRY_DRAGON, LOCATION_MZONE, UP),
        (0, ANCIENT_FAIRY_DRAGON, LOCATION_MZONE, UP),
        (0, GIANT_ORC, LOCATION_HAND, UP),
        (0, GIANT_ORC, LOCATION_HAND, UP),
    ]);
    play.activate_effect(ANCIENT_FAIRY_DRAGON, 0);
    assert_eq!(play.count(0, LOCATION_MZONE, GIANT_ORC), 1);
    assert!(play.can_activate_effect(ANCIENT_FAIRY_DRAGON, 0), "the other copy: {}", play.prompt);
    play.activate_effect(ANCIENT_FAIRY_DRAGON, 0);
    assert!(!play.can_activate_effect(ANCIENT_FAIRY_DRAGON, 0), "each copy once");
}

#[test]
fn exchange_of_the_spirit_needs_15_cards_in_your_graveyard_only() {
    // engine/overrides/c17484499.lua: "Activate only while there are 15 or more cards in your Graveyard"; today's text
    // needs 15 in both (and allows it once per Duel).
    const EXCHANGE_OF_THE_SPIRIT: u32 = 17484499;
    let mut cards = vec![(0, EXCHANGE_OF_THE_SPIRIT, LOCATION_SZONE, SET)];
    cards.extend([(0, GIANT_ORC, LOCATION_GRAVE, UP); 15]);
    let mut play = play!(&cards);
    assert!(play.can_activate(EXCHANGE_OF_THE_SPIRIT), "{}", play.prompt);
    play.activate(EXCHANGE_OF_THE_SPIRIT);
    assert_eq!(play.cards(0, LOCATION_DECK).len(), 15, "the Graveyard became the Deck");
    assert_eq!(play.lp(0), 7000);
}

#[test]
fn disk_commander_draws_every_time_it_comes_back() {
    // engine/overrides/c56570271.lua: "When this card is Special Summoned from the Graveyard, draw 2 cards"; today's
    // text allows it once per Duel.
    const DISK_COMMANDER: u32 = 56570271;
    let mut play = play!(&[
        (0, DISK_COMMANDER, LOCATION_GRAVE, UP),
        (0, DISK_COMMANDER, LOCATION_GRAVE, UP),
        (0, MONSTER_REBORN, LOCATION_HAND, UP),
        (0, MONSTER_REBORN, LOCATION_HAND, UP),
    ]);
    play.use_cards = vec![DISK_COMMANDER];
    let hand = play.cards(0, LOCATION_HAND).len();
    play.activate(MONSTER_REBORN);
    assert_eq!(play.cards(0, LOCATION_HAND).len(), hand + 1, "a Monster Reborn used and two cards drawn");
    play.activate(MONSTER_REBORN);
    assert_eq!(play.cards(0, LOCATION_HAND).len(), hand + 2, "and again: {}", play.prompt);
}

#[test]
fn each_second_coin_toss_redoes_a_toss() {
    // engine/overrides/c36562627.lua: "once per turn" for each copy; today's text allows it once per turn by name.
    const SECOND_COIN_TOSS: u32 = 36562627;
    const GODDESS_OF_WHIM: u32 = 67959180;
    let mut play = play!(&[(0, SECOND_COIN_TOSS, LOCATION_SZONE, UP), (0, SECOND_COIN_TOSS, LOCATION_SZONE, UP), (0, GODDESS_OF_WHIM, LOCATION_MZONE, UP)]);
    play.use_cards = vec![SECOND_COIN_TOSS];
    play.activate(GODDESS_OF_WHIM);
    assert_eq!(play.asked(SECOND_COIN_TOSS), 2, "one redo from each copy");
}

#[test]
fn goddess_of_whim_tosses_again() {
    // engine/overrides/c67959180.lua: no once-per-turn limit in the client's text.
    const GODDESS_OF_WHIM: u32 = 67959180;
    let mut play = play!(&[(0, GODDESS_OF_WHIM, LOCATION_MZONE, UP)]);
    play.activate(GODDESS_OF_WHIM);
    assert!(play.can_activate(GODDESS_OF_WHIM), "{}", play.prompt);
}

#[test]
fn makyura_lets_you_activate_any_number_of_traps_from_your_hand() {
    // engine/overrides/c21593977.lua: "the owner of this card can activate Trap Card(s) from his/her hand"; today's
    // text allows 1 Trap.
    const MAKYURA: u32 = 21593977;
    let mut play = play!(&[
        (0, MAKYURA, LOCATION_MZONE, UP),
        (0, SUMMONED_SKULL, LOCATION_HAND, UP),
        (0, JAR_OF_GREED, LOCATION_HAND, UP),
        (0, JAR_OF_GREED, LOCATION_HAND, UP),
    ]);
    play.use_cards = vec![MAKYURA];
    play.summon(SUMMONED_SKULL);
    play.activate(JAR_OF_GREED);
    assert!(play.can_activate(JAR_OF_GREED), "the second Jar of Greed: {}", play.prompt);
}

#[test]
fn makyura_sent_from_the_hand_counts_too() {
    // engine/overrides/c21593977.lua: "During the turn this card is sent to the Graveyard"; today's text wants it sent
    // from the Monster Zone. Brionac's cost discards it.
    const MAKYURA: u32 = 21593977;
    const BRIONAC: u32 = 50321796;
    let mut play =
        play!(&[(0, BRIONAC, LOCATION_MZONE, UP), (1, GIANT_ORC, LOCATION_MZONE, UP), (0, MAKYURA, LOCATION_HAND, UP), (0, JAR_OF_GREED, LOCATION_HAND, UP)]);
    play.use_cards = vec![MAKYURA];
    play.keep = vec![BRIONAC, JAR_OF_GREED];
    assert!(!play.can_activate(JAR_OF_GREED));
    play.activate(BRIONAC);
    assert_eq!(play.cards(0, LOCATION_GRAVE), [MAKYURA]);
    assert!(play.can_activate(JAR_OF_GREED), "{}", play.prompt);
}

#[test]
fn catapult_turtle_can_be_used_again() {
    // engine/overrides/c95727991.lua: no once-per-turn limit in the client's text.
    const CATAPULT_TURTLE: u32 = 95727991;
    let mut play = play!(&[(0, CATAPULT_TURTLE, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_MZONE, UP)]);
    play.keep = vec![CATAPULT_TURTLE];
    play.activate(CATAPULT_TURTLE);
    assert_eq!(play.lp(1), 8000 - 1100);
    assert!(play.can_activate(CATAPULT_TURTLE), "{}", play.prompt);
}

#[test]
fn chaos_emperor_dragon_counts_every_card_it_sends() {
    // engine/overrides/c82301904.lua: "300 damage ... for each card sent to the Graveyard by this effect"; today's text
    // counts only those sent to the opponent's Graveyard (and adds a once-per-turn limit and a lock on your other cards).
    const CHAOS_EMPEROR_DRAGON: u32 = 82301904;
    let mut play = play!(&[(0, CHAOS_EMPEROR_DRAGON, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_HAND, UP), (1, GIANT_ORC, LOCATION_MZONE, UP)]);
    let ours = play.cards(0, LOCATION_HAND).len() as i64 + 1;
    play.activate(CHAOS_EMPEROR_DRAGON);
    assert_eq!(play.lp(0), 7000);
    assert_eq!(play.lp(1), 8000 - (ours + 1) * 300, "Chaos Emperor Dragon, the hand and the opponent's Giant Orc");
}

#[test]
fn temple_of_the_kings_lets_every_trap_set_this_turn_be_activated() {
    // engine/overrides/c29762407.lua: "can activate Trap Cards the same turn they are Set"; today's text allows 1.
    const TEMPLE_OF_THE_KINGS: u32 = 29762407;
    let mut play = play!(&[(0, TEMPLE_OF_THE_KINGS, LOCATION_SZONE, UP), (0, JAR_OF_GREED, LOCATION_HAND, UP), (0, JAR_OF_GREED, LOCATION_HAND, UP)]);
    play.set_spell(JAR_OF_GREED);
    play.set_spell(JAR_OF_GREED);
    play.activate(JAR_OF_GREED);
    assert!(play.can_activate(JAR_OF_GREED), "the second: {}", play.prompt);
}

#[test]
fn temple_of_the_kings_summons_any_extra_deck_monster() {
    // engine/overrides/c29762407.lua: "Special Summon 1 monster from your hand, Deck, or Extra Deck"; today's text
    // wants a Fusion Monster from the Extra Deck. Spells for Decks, so that only the Extra Deck has a monster.
    const TEMPLE_OF_THE_KINGS: u32 = 29762407;
    const MYSTICAL_BEAST_OF_SERKET: u32 = 89194033;
    const GOYO_GUARDIAN: u32 = 7391448; // a Synchro Monster
    let Some(mut play) = Play::with_filler(
        &[
            (0, TEMPLE_OF_THE_KINGS, LOCATION_SZONE, UP),
            (0, MYSTICAL_BEAST_OF_SERKET, LOCATION_MZONE, UP),
            (0, GOYO_GUARDIAN, LOCATION_EXTRA, POSITION_FACEDOWN_DEFENSE),
        ],
        MYSTICAL_SPACE_TYPHOON,
    ) else {
        return;
    };
    assert!(play.can_activate_effect(TEMPLE_OF_THE_KINGS, 0), "{}", play.prompt);
    play.activate_effect(TEMPLE_OF_THE_KINGS, 0);
    assert_eq!(play.cards(0, LOCATION_MZONE), [GOYO_GUARDIAN]);
}

#[test]
fn ring_of_destruction_takes_any_face_up_monster_in_any_turn() {
    // engine/overrides/c83555666.lua: "Select and destroy 1 face-up monster, and inflict damage to both players equal
    // to its ATK"; today's text wants an opponent's monster, in their turn.
    const RING_OF_DESTRUCTION: u32 = 83555666;
    let mut play = play!(&[(0, RING_OF_DESTRUCTION, LOCATION_SZONE, SET), (0, GIANT_ORC, LOCATION_MZONE, UP)]);
    assert!(play.can_activate(RING_OF_DESTRUCTION), "{}", play.prompt);
    play.activate(RING_OF_DESTRUCTION);
    assert_eq!((play.lp(0), play.lp(1)), (8000 - 2200, 8000 - 2200));
}

#[test]
fn each_dewloren_returns_cards_once_a_turn() {
    // engine/overrides/c70583986.lua: "Once per turn", each copy for itself; today's text allows it once per turn by
    // name.
    const DEWLOREN: u32 = 70583986;
    let mut play = play!(&[(0, DEWLOREN, LOCATION_MZONE, UP), (0, DEWLOREN, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_MZONE, UP), (0, GIANT_ORC, LOCATION_MZONE, UP)]);
    play.keep = vec![DEWLOREN];
    play.activate(DEWLOREN);
    assert_eq!(play.count(0, LOCATION_MZONE, GIANT_ORC), 1, "one returned");
    assert!(play.can_activate(DEWLOREN), "the other copy: {}", play.prompt);
}

#[test]
fn mind_master_cant_tribute_a_mind_master() {
    // engine/overrides/c96782886.lua: "Tribute 1 Psychic-Type monster, except "Mind Master""; today's text drops the
    // exception (and adds a once-per-turn limit).
    const MIND_MASTER: u32 = 96782886; // a Level 1 Psychic
    let mut cards = vec![(0, MIND_MASTER, LOCATION_MZONE, UP)];
    cards.extend([(0, CHOSEN_BY_THE_WORLD_CHALICE, LOCATION_DECK, POSITION_FACEDOWN_DEFENSE); 3]);
    let play = play!(&cards);
    assert!(!play.can_activate(MIND_MASTER), "only itself to Tribute: {}", play.prompt);
    cards.extend([(0, CHOSEN_BY_THE_WORLD_CHALICE, LOCATION_MZONE, UP); 2]);
    let mut play = play!(&cards);
    play.activate(MIND_MASTER);
    assert!(play.can_activate(MIND_MASTER), "no once-per-turn limit: {}", play.prompt);
}

#[test]
fn mausoleum_of_the_emperor_is_part_of_the_normal_summon() {
    // engine/overrides/c80921533.lua: "Both players can Normal Summon or Set monsters without Tributing, by paying 1000
    // Life Points x the number of monsters needed"; today's text makes it an effect the turn player activates first.
    const MAUSOLEUM_OF_THE_EMPEROR: u32 = 80921533;
    let mut play = play!(&[(0, MAUSOLEUM_OF_THE_EMPEROR, FIELD_ZONE, UP), (0, SUMMONED_SKULL, LOCATION_HAND, UP), (0, BLUE_EYES_WHITE_DRAGON, LOCATION_HAND, UP), (1, SUMMONED_SKULL, LOCATION_HAND, UP)]);
    assert!(!play.can_activate(MAUSOLEUM_OF_THE_EMPEROR), "{}", play.prompt);
    play.summon(BLUE_EYES_WHITE_DRAGON);
    assert_eq!(play.cards(0, LOCATION_MZONE), [BLUE_EYES_WHITE_DRAGON]);
    assert_eq!(play.lp(0), 6000, "two Tributes' worth");
    play.end_turn();
    assert_eq!(play.idle()["player"], 1);
    assert!(play.find("summons", SUMMONED_SKULL, None).is_some(), "the opponent's too: {}", play.prompt);
    play.summon(SUMMONED_SKULL);
    assert_eq!(play.lp(1), 7000);
}

#[test]
fn crush_card_virus_checks_the_cards_the_opponent_draws() {
    // engine/overrides/c57728570.lua: "... your opponent's hand, and all cards they draw (until the end of your
    // opponent's 3rd turn after this card's activation)"; today's text checks the hand and field once.
    const CRUSH_CARD_VIRUS: u32 = 57728570;
    const SANGAN: u32 = 26202165; // a DARK monster with 1000 ATK
    let mut play = play!(&[(0, CRUSH_CARD_VIRUS, LOCATION_SZONE, SET), (0, SANGAN, LOCATION_MZONE, UP), (1, GIANT_ORC, LOCATION_MZONE, UP)]);
    play.activate(CRUSH_CARD_VIRUS);
    assert_eq!(play.count(1, LOCATION_GRAVE, GIANT_ORC), 1, "the Giant Orc on the field");
    play.end_turn();
    assert_eq!(play.idle()["player"], 1);
    assert_eq!(play.count(1, LOCATION_HAND, GIANT_ORC), 0, "the one drawn was destroyed: {}", play.prompt);
    assert_eq!(play.count(1, LOCATION_GRAVE, GIANT_ORC), 2);
}

#[test]
fn brain_control_takes_any_face_up_monster() {
    // engine/overrides/c87910978.lua: "Select 1 face-up monster your opponent controls"; today's text wants one that
    // can be Normal Summoned/Set, so not a Synchro Monster.
    const BRAIN_CONTROL: u32 = 87910978;
    const GOYO_GUARDIAN: u32 = 7391448;
    let mut play = play!(&[(0, BRAIN_CONTROL, LOCATION_HAND, UP), (1, GOYO_GUARDIAN, LOCATION_MZONE, UP)]);
    assert!(play.can_activate(BRAIN_CONTROL), "{}", play.prompt);
    play.activate(BRAIN_CONTROL);
    assert_eq!(play.cards(0, LOCATION_MZONE), [GOYO_GUARDIAN]);
}

#[test]
fn necrovalley_lets_a_graveyard_card_use_its_own_effect() {
    // engine/overrides/c47355498.lua: "Cards in either player's Graveyard cannot be affected by card effects, except
    // for their own effects"; today's text negates a card's own effect that moves it out of the Graveyard.
    const NECROVALLEY: u32 = 47355498;
    const SINISTER_SERPENT: u32 = 8131171;
    let mut play = play!(&[(0, NECROVALLEY, FIELD_ZONE, UP), (0, SINISTER_SERPENT, LOCATION_GRAVE, UP)]);
    play.use_cards = vec![SINISTER_SERPENT];
    play.end_turn();
    play.end_turn();
    assert_eq!(play.count(0, LOCATION_HAND, SINISTER_SERPENT), 1, "it returned itself");
}

#[test]
fn darkness_approaches_keeps_the_battle_position() {
    // engine/overrides/c80168720.lua: "flip it face-down, but do not change its battle position"; today's text makes it
    // face-down Defense Position.
    const DARKNESS_APPROACHES: u32 = 80168720;
    let mut play =
        play!(&[(0, DARKNESS_APPROACHES, LOCATION_HAND, UP), (0, GIANT_ORC, LOCATION_HAND, UP), (0, GIANT_ORC, LOCATION_HAND, UP), (1, GIANT_ORC, LOCATION_MZONE, UP)]);
    play.activate(DARKNESS_APPROACHES);
    assert_eq!(play.position(1, 0), 0x2, "face-down Attack Position");
}

#[test]
fn imperial_order_is_paid_for_in_your_own_standby_phase() {
    // engine/overrides/c61740673.lua: "Pay 700 Life Points during each of your Standby Phases"; today's text pays in
    // every Standby Phase.
    const IMPERIAL_ORDER: u32 = 61740673;
    let mut play = play!(&[(0, IMPERIAL_ORDER, LOCATION_SZONE, UP)]);
    assert_eq!(play.lp(0), 7300, "700 in your first Standby Phase");
    play.end_turn();
    assert_eq!(play.lp(0), 7300, "nothing in the opponent's");
    play.end_turn();
    assert_eq!(play.lp(0), 6600, "700 in your next");
}

#[test]
fn future_fusion_sends_the_materials_when_activated() {
    // engine/overrides/c77565204.lua: the materials go to the Graveyard on activation; today's text waits for your 1st
    // Standby Phase. The Fusion Monster still comes in the 2nd.
    const FUTURE_FUSION: u32 = 77565204;
    const BLUE_EYES_ULTIMATE_DRAGON: u32 = 23995346;
    let mut cards = vec![(0, FUTURE_FUSION, LOCATION_SZONE, SET), (0, BLUE_EYES_ULTIMATE_DRAGON, LOCATION_EXTRA, POSITION_FACEDOWN_DEFENSE)];
    cards.extend([(0, BLUE_EYES_WHITE_DRAGON, LOCATION_DECK, POSITION_FACEDOWN_DEFENSE); 6]); // three left after the turns' draws
    let mut play = play!(&cards);
    play.activate(FUTURE_FUSION);
    assert_eq!(play.count(0, LOCATION_GRAVE, BLUE_EYES_WHITE_DRAGON), 3, "{}", play.prompt);
    for _ in 0..4 {
        play.end_turn();
    }
    assert_eq!(play.count(0, LOCATION_MZONE, BLUE_EYES_ULTIMATE_DRAGON), 1, "in the 2nd Standby Phase");
}

#[test]
fn ultimate_tyranno_must_attack() {
    // engine/overrides/c15894048.lua: "it must attack first, and attack all monsters your opponent controls once
    // each"; today's text lets it attack them all but doesn't make it.
    const ULTIMATE_TYRANNO: u32 = 15894048;
    let mut play = play!(&[(0, ULTIMATE_TYRANNO, LOCATION_MZONE, UP), (1, NEKOGAL_1, LOCATION_MZONE, UP), (1, NEKOGAL_1, LOCATION_MZONE, UP)]);
    play.end_turn();
    play.end_turn();
    play.battle();
    assert_eq!((play.prompt["to_m2"].clone(), play.prompt["to_ep"].clone()), (json!(false), json!(false)), "{}", play.prompt);
}

#[test]
fn each_red_eyes_darkness_metal_dragon_summons_once_a_turn() {
    // engine/overrides/c88264978.lua: "Once per turn", each copy for itself, and no limit on its own Special Summon;
    // today's text allows each once per turn by name.
    const RED_EYES_DARKNESS_METAL_DRAGON: u32 = 88264978;
    let mut play = play!(&[
        (0, BLUE_EYES_WHITE_DRAGON, LOCATION_MZONE, UP),
        (0, BLUE_EYES_WHITE_DRAGON, LOCATION_MZONE, UP),
        (0, RED_EYES_DARKNESS_METAL_DRAGON, LOCATION_HAND, UP),
        (0, RED_EYES_DARKNESS_METAL_DRAGON, LOCATION_HAND, UP),
        (0, BLUE_EYES_WHITE_DRAGON, LOCATION_HAND, UP),
        (0, BLUE_EYES_WHITE_DRAGON, LOCATION_HAND, UP),
    ]);
    play.keep = vec![RED_EYES_DARKNESS_METAL_DRAGON];
    play.act("special_summons", 1, RED_EYES_DARKNESS_METAL_DRAGON, None);
    play.act("special_summons", 1, RED_EYES_DARKNESS_METAL_DRAGON, None);
    assert_eq!(play.count(0, LOCATION_MZONE, RED_EYES_DARKNESS_METAL_DRAGON), 2);
    play.activate_effect(RED_EYES_DARKNESS_METAL_DRAGON, 0);
    assert!(play.can_activate_effect(RED_EYES_DARKNESS_METAL_DRAGON, 0), "the other copy: {}", play.prompt);
}

#[test]
fn y_z_and_w_unions_protect_only_from_battle() {
    // engine/overrides/c65622692.lua, c64500000.lua, c96300057.lua: "If the equipped monster would be destroyed by
    // battle, destroy this card instead"; today's text protects from card effects too. Ring of Destruction destroys
    // the equipped monster.
    const RING_OF_DESTRUCTION: u32 = 83555666;
    const X_HEAD_CANNON: u32 = 62651957;
    const V_TIGER_JET: u32 = 51638941;
    for (union, host) in [(65622692, X_HEAD_CANNON), (64500000, X_HEAD_CANNON), (96300057, V_TIGER_JET)] {
        let mut play = play!(&[(0, host, LOCATION_MZONE, UP), (0, union, LOCATION_MZONE, UP), (0, RING_OF_DESTRUCTION, LOCATION_SZONE, SET)]);
        play.activate(union);
        assert_eq!(play.cards(0, LOCATION_MZONE), [host], "{union} equipped");
        play.activate(RING_OF_DESTRUCTION);
        assert_eq!(play.count(0, LOCATION_MZONE, host), 0, "{union} didn't take its place: {}", play.prompt);
    }
}

#[test]
fn old_rule_unions_come_back_in_attack_position() {
    // engine/overrides/c23265594.lua, c67159705.lua, c42940404.lua, c78349103.lua: "unequip it to Special Summon this
    // card in face-up Attack Position"; today's text lets you choose the position.
    const X_HEAD_CANNON: u32 = 62651957; // a Machine
    const CYBER_DRAGON: u32 = 70095154;
    for (union, host) in [(23265594, X_HEAD_CANNON), (67159705, CYBER_DRAGON), (42940404, X_HEAD_CANNON), (78349103, X_HEAD_CANNON)] {
        let mut play = play!(&[(0, host, LOCATION_MZONE, UP), (0, union, LOCATION_MZONE, UP)]);
        play.activate(union);
        play.end_turn();
        play.end_turn();
        let before = play.seen.len();
        play.activate(union);
        assert_eq!(play.count(0, LOCATION_MZONE, union), 1, "{union} unequipped");
        assert!(play.seen[before..].iter().all(|m| m["type"] != 19), "{union}: no position to choose");
    }
}

#[test]
fn twin_headed_behemoth_comes_back_by_itself() {
    // engine/overrides/c43586926.lua: "Special Summon it during the End Phase of this turn"; today's text says "You
    // can".
    const TWIN_HEADED_BEHEMOTH: u32 = 43586926;
    const RING_OF_DESTRUCTION: u32 = 83555666;
    let mut play = play!(&[(0, TWIN_HEADED_BEHEMOTH, LOCATION_MZONE, UP), (0, RING_OF_DESTRUCTION, LOCATION_SZONE, SET)]);
    play.activate(RING_OF_DESTRUCTION);
    assert_eq!(play.count(0, LOCATION_GRAVE, TWIN_HEADED_BEHEMOTH), 1);
    play.end_turn();
    assert_eq!(play.count(0, LOCATION_MZONE, TWIN_HEADED_BEHEMOTH), 1, "without being asked: {}", play.prompt);
}

#[test]
fn night_assailant_returns_another_night_assailant() {
    // engine/overrides/c16226786.lua: "select 1 Flip Effect Monster in your Graveyard, except this card"; today's text
    // excludes every "Night Assailant". Brionac's cost discards it.
    const NIGHT_ASSAILANT: u32 = 16226786;
    const BRIONAC: u32 = 50321796;
    let mut play = play!(&[
        (0, BRIONAC, LOCATION_MZONE, UP),
        (1, GIANT_ORC, LOCATION_MZONE, UP),
        (0, NIGHT_ASSAILANT, LOCATION_HAND, UP),
        (0, NIGHT_ASSAILANT, LOCATION_GRAVE, UP),
    ]);
    play.keep = vec![BRIONAC];
    let drawn = play.cards(0, LOCATION_HAND).len() - 1;
    play.activate(BRIONAC);
    assert_eq!(play.count(0, LOCATION_HAND, NIGHT_ASSAILANT), 1, "the other one: {}", play.prompt);
    assert_eq!(play.cards(0, LOCATION_HAND).len(), drawn + 1);
}
