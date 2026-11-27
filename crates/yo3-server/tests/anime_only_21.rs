//! Anime-only cards, group 21 (rows 5883-5927 of engine/added/cards.tsv): Yusei Fudo's cards. These cards exist only in the
//! anime, so every duel plays with anime cards on (`Scenario::start_rule`, anime cards on). Each effect goes through the real core
//! with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones. Nearly
//! every card here is a Trap, Set first (a Trap cannot be activated from the hand); neither player has a Battle Phase in
//! turn 1, so team 0's attacks wait for turn 3.
//! Covered: the cards the per-card fuzz flagged (Arbitrator, Blue Expense Falcon, Burst Synchro Summon, Cosmic Blast,
//! Crossline Counter, Crush Star, Dimension Voyage, Explosive Breakout, Ghoul Summoner, Hardship, Harmonia Mirror, Harmony
//! Crystal, Life Gardna, Meteor Stream, Meteor Wave, Monster Chain, Regen Warrior, Rising Rush, Scouting Warrior), Emerging
//! Awakening (whose fuzz duel failed on Yomi Kid of batch 26, not on its own script), the cards `--find` noted (Cosmic
//! Blast, Ghoul Summoner, Guard Master, Hardship, Revenge Return, and Iron Resolve at ∞ LP), the signature cards (Burst
//! Synchro Summon, Cosmic Blast, Iron Resolve, Return Marker, Card Defense, Half Shield) and a seeded sample of the rest
//! (Devoted Gardna, Double Delta Warrior, Fake Gardna, Panic Wave, Realize Defense, Revival Ticket and Shard of Hope).
//! Every duel plays by the server's default revival rule (`graveyard-is-graveyard`, see `started`), because a Synchro Monster
//! a test puts on the field was never Synchro Summoned and revival effects (Dimension Voyage, Ghoul Summoner, Harmony Crystal,
//! Harmonia Mirror's Monster Reborn) would otherwise never take it.
//! The batch makes no Token. Two cards have an override: Hardship (`engine/overrides/c511000620.lua`: its damage answer was
//! never offered once it was equipped, it answered a Trap's damage too, and its Graveyard effect stopped on a Lua error) and
//! Monster Chain (`engine/overrides/c511000226.lua`: it destroyed itself the moment it was activated, its Chain Counters never
//! placed).
//! Not coverable: Rubble King (30 cards in the Graveyard), Shield Coat, Salvagent, Big Eater (played only as Cosmic Blast's
//! Special Summon to forbid), Chain Arrow, Counterattack Beacon, Enduring Soul, Future Battle, Joint Future and Power Shift were
//! not asked for (no flag, note or sample).
use yo3_server::duelcheck::scenario::{Action, Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Revival, Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DARK_HOLE: u16 = 4342;
const RAIGEKI: u16 = 4343;
const MONSTER_REBORN: u16 = 4842;
const OOKAZI: u16 = 4352; // 800 damage
const JUST_DESSERTS: u16 = 4865; // Trap: 500 damage for each monster the opponent controls
const CANNON_SOLDIER: u16 = 4518; // Tribute 1 monster: inflict 500 damage to your opponent
const GOYO: u16 = 7736; // Goyo Guardian: a Level 6 Synchro Monster, 2800/2000
const JUNK_WARRIOR: u16 = 7696; // Level 5 Synchro Monster, 2300/1300
const GAIA_KNIGHT: u16 = 7697; // Gaia Knight, the Force of Earth: a Level 6 Synchro Monster without an effect, 2600/800
const RED_DRAGON_ARCHFIEND: u16 = 7735; // a Dragon-Type Synchro Monster, 3000/2000
const TUNE_WARRIOR: u16 = 7684; // a Level 3 Tuner without an effect, 1600/200
const ARMORY_ARM: u16 = 7987; // Level 4 Synchro Monster: 1 Tuner + 1+ non-Tuner, 1800/1200
const PETIT_DRAGON: u16 = 4206; // Level 2 Dragon, 600/700
const LUSTER_DRAGON: u16 = 5644; // Level 4 Dragon, 1900/1600
const BLUE_EYES: u16 = 4007; // Level 8 Dragon
const INFINITE: i64 = 19_999_998;
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn banished(team: u32, cid: u16) -> Spawn { Spawn { team, place: Banished, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel under the server's default revival rule (`graveyard-is-graveyard`: a Synchro Monster put on the field
/// or into the Graveyard by a test was never Synchro Summoned, and revival effects still take it, as they do for players).
fn started(dir: &std::path::Path, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Scenario {
    Scenario::start_rule(dir, Revival::GraveyardIsGraveyard, true, seed, lp, decks, spawns).unwrap()
}

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: team 0's has `bottom` at its very
/// bottom (out of the opening hand and the first draws), for a card an effect fetches from the Deck.
fn duel(seed: u64, lp: u32, fillers: [u16; 2], bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(fillers[0], 40 - bottom.len())).collect();
    let decks = [(&mine[..], &[][..]), (&vec![fillers[1]; 40][..], &[][..])];
    Some(started(&dir, seed, lp, decks, spawns))
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [FILLER, FILLER], bottom, spawns) }

/// A duel with the Extra Decks given: `extra[team]`.
fn field_extra(extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], extra[0]), (&[FILLER; 40][..], extra[1])];
    Some(started(&dir, 7, 8000, decks, spawns))
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

/// A monster's (ATK, DEF, Level) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64())
}

/// The planned side is at its Main Phase 1's idle prompt: the monsters that can attack, going into the Battle Phase (none when
/// there is no Battle Phase to go to).
fn attackers(s: &mut Scenario) -> Vec<serde_json::Value> {
    if s.prompt()["to_bp"] != true {
        return vec![];
    }
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
    for _ in 0..8 {
        if s.prompt()["type"] == msg::SELECT_BATTLECMD {
            break;
        }
        s.pass().unwrap(); // the chances to chain going into the Battle Phase
    }
    offered(s, "attacks")
}

/// From a battle prompt on to the Main Phase 2 of the same turn.
fn to_main_phase_2(s: &mut Scenario) {
    if s.prompt()["type"] == msg::SELECT_BATTLECMD {
        s.respond(serde_json::json!({"type": 1, "action": battle::TO_M2, "index": null})).unwrap();
    }
    for _ in 0..8 {
        if s.prompt()["type"] == msg::SELECT_IDLECMD {
            break;
        }
        s.pass().unwrap();
    }
}

/// Team 0 at ∞ LP (Mystik Wok on the anime Geh: 8000 and ∞ is ∞), with `spawns` in place; team 1 at 8000.
fn infinite(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(0, WOK), mon(0, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = started(&dir, 7, 8000, decks, &field);
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[]);
    Some(s)
}

// ---------------------------------------------------------------------------------------------------------------
// Arbitrator, Iron Resolve, Card Defense: ending or negating the Battle Phase
// ---------------------------------------------------------------------------------------------------------------

/// Arbitrator: "When a card(s) you control is destroyed during the Battle Phase: you can discard this card; end the Battle
/// Phase." Warwolf destroys my Thunder Kid (1300 damage); Arbitrator is discarded from my hand and the Battle Phase is over,
/// so Dark Blade cannot attack. Without Arbitrator it attacks next and takes 1800 more.
#[test]
fn arbitrator_ends_the_battle_phase_when_a_card_of_its_controller_is_destroyed() {
    let arbitrator = card("Arbitrator");
    let Some(mut s) = field(&[], &[hand(0, arbitrator), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(arbitrator)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", arbitrator), s.count(0, "grave", THUNDER_KID)), (1, 1), "discarded, and the Kid died: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(msg::SELECT_IDLECMD), Some(1)), "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over: {}", s.context());
    // The control: no Arbitrator in the hand, and Dark Blade attacks after Warwolf.
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1300 - 1800, "{}", s.context());
}

/// Iron Resolve: "During the Battle Phase: Pay half your LP; You take no battle damage this turn, then, end the Battle
/// Phase." Warwolf attacks directly; Iron Resolve costs 4000 of the 8000 LP, no damage is taken and Dark Blade does not get
/// to attack. Without it both attacks land (2000 + 1800).
#[test]
fn iron_resolve_pays_half_the_lp_to_take_no_battle_damage_and_end_the_battle_phase() {
    let resolve = card("Iron Resolve");
    let Some(mut s) = field(&[], &[set(0, resolve), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(resolve)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", resolve), 1, "{}", s.context());
    assert_eq!(s.lp(0), 4000, "half of 8000 was paid and no battle damage was taken: {}", s.context());
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(msg::SELECT_IDLECMD), Some(1)), "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over: {}", s.context());
    let Some(mut s) = field(&[], &[mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 2000 - 1800, "{}", s.context());
}

/// Iron Resolve at ∞ LP: half of ∞ is ∞, so the LP stay ∞ (guards.lua), the damage is still not taken and the Battle Phase
/// still ends: Dark Blade cannot attack after Warwolf.
#[test]
fn iron_resolve_at_infinite_lp_pays_infinity_and_still_ends_the_battle_phase() {
    let resolve = card("Iron Resolve");
    let Some(mut s) = infinite(&[set(0, resolve), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.run_both([&[ChainOnAttack(resolve)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", resolve), 1, "Iron Resolve was activated: {}", s.context());
    assert_eq!(s.lp(0), INFINITE, "half of ∞ is paid and ∞ is left: {}", s.context());
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(msg::SELECT_IDLECMD), Some(1)), "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over: {}", s.context());
}

/// Card Defense: "When an opponent's monster declares a direct attack: Send 1 card from your hand to the Graveyard, then
/// target the attacking monster; negate that attack, then draw 1 card." Warwolf attacks directly; one Feral Imp is sent to
/// the Graveyard, one card is drawn (the hand is as large as before), and no damage is taken. Dark Blade's attack is a direct
/// one too and is not negated (Card Defense is used up): 1800 damage.
#[test]
fn card_defense_negates_a_direct_attack_and_draws_a_card() {
    let defense = card("Card Defense");
    let Some(mut s) = field(&[], &[set(0, defense), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let (hand_before, imps) = (hand_size(&s, 0), s.count(0, "grave", FILLER));
    s.run_both([&[ChainOnAttack(defense)], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", defense), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), imps + 1, "one card was sent from the hand: {}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before, "and one drawn: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1800, "Warwolf's attack was negated, Dark Blade's was not: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Blue Expense Falcon
// ---------------------------------------------------------------------------------------------------------------

/// Blue Expense Falcon: "When this card is Normal Summoned: You can Special Summon 1 Level 4 or lower Dragon-Type monster
/// from your hand." Luster Dragon (Level 4) comes to the field in face-up Attack Position; Blue-Eyes (Level 8) and Dark
/// Blade (not a Dragon) stay in the hand.
#[test]
fn blue_expense_falcon_summons_a_level_4_or_lower_dragon_from_the_hand() {
    let falcon = card("Blue Expense Falcon");
    let spawns = [hand(0, falcon), hand(0, LUSTER_DRAGON), hand(0, BLUE_EYES), hand(0, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Summon(falcon), Chain(falcon)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", falcon), s.count(0, "mzone", LUSTER_DRAGON)), (1, 1), "{}", s.context());
    assert_eq!(s.position_of(0, LUSTER_DRAGON), Some(1), "face-up Attack: {}", s.context());
    assert_eq!((s.count(0, "hand", BLUE_EYES), s.count(0, "hand", DARK_BLADE)), (1, 1), "{}", s.context());
}

/// Blue Expense Falcon: "Cannot be used as a Synchro Material, except for the Synchro Summon of a WIND monster." Falcon (a
/// Level 2 Tuner) and Summoned Skull (Level 6) make the WIND Level 8 Stardust Dragon, but Falcon and Petit Dragon (Level 2)
/// are not offered the LIGHT Level 4 Armory Arm. With Nitro Synchron (also a Level 2 Tuner) in Falcon's place both are offered.
#[test]
fn blue_expense_falcon_is_synchro_material_only_for_a_wind_monster() {
    let (falcon, skull, nitro, stardust) = (card("Blue Expense Falcon"), card("Summoned Skull"), card("Nitro Synchron"), card("Stardust Dragon"));
    let spawns = [mon(0, falcon), mon(0, PETIT_DRAGON), mon(0, skull)];
    let Some(mut s) = field_extra([&[ARMORY_ARM, stardust], &[]], &spawns) else { return };
    s.run(&[]).unwrap();
    let now = offered(&s, "special_summons");
    assert!(now.contains(&stardust.into()), "WIND: {now:?}");
    assert!(!now.contains(&ARMORY_ARM.into()), "LIGHT: {now:?}");
    let spawns = [mon(0, nitro), mon(0, PETIT_DRAGON), mon(0, skull)];
    let Some(mut s) = field_extra([&[ARMORY_ARM, stardust], &[]], &spawns) else { return };
    s.run(&[]).unwrap();
    let now = offered(&s, "special_summons");
    assert!(now.contains(&stardust.into()) && now.contains(&ARMORY_ARM.into()), "the control: {now:?}");
}

// ---------------------------------------------------------------------------------------------------------------
// Burst Synchro Summon, Cosmic Blast, Crush Star, Harmony Crystal, Harmonia Mirror
// ---------------------------------------------------------------------------------------------------------------

/// Burst Synchro Summon: "When you would take damage from an opponent's card effect: You take no damage, also, Special
/// Summon 1 Synchro Monster from your Extra Deck with ATK equal to the damage you would have taken, ignoring its Summoning
/// conditions, but its effects are negated." Team 1's Just Desserts would deal 500 for each of my 4 monsters, 2000: no
/// damage, and Dark Diviner (ATK 2000) comes from the Extra Deck (Goyo Guardian, 2800, stays there). Without it 2000 is taken.
#[test]
fn burst_synchro_summon_takes_no_damage_and_summons_a_synchro_monster_of_that_atk() {
    let (burst, diviner) = (card("Burst Synchro Summon"), card("Dark Diviner"));
    let spawns = [set(0, burst), mon(0, THUNDER_KID), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(0, FILLER), set(1, JUST_DESSERTS)];
    let Some(mut s) = field_extra([&[diviner, GOYO], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(burst)], &[Activate(JUST_DESSERTS)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", diviner), 1, "Dark Diviner (2000 ATK) was Special Summoned: {}", s.context());
    assert_eq!(s.count(0, "mzone", GOYO), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", burst), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", JUST_DESSERTS), 1, "{}", s.context());
    // The control.
    let spawns = [mon(0, THUNDER_KID), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(0, FILLER), set(1, JUST_DESSERTS)];
    let Some(mut s) = field_extra([&[diviner, GOYO], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(JUST_DESSERTS)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
}

/// Cosmic Blast: "When a Dragon-Type Synchro Monster you control leaves the field: Inflict damage to your opponent equal to
/// its ATK. You cannot Special Summon the turn you activate this effect." Team 1's Dark Hole destroys my Red Dragon
/// Archfiend (3000): team 1 takes 3000.
#[test]
fn cosmic_blast_inflicts_the_atk_of_a_dragon_synchro_monster_that_left_the_field() {
    let cosmic = card("Cosmic Blast");
    let Some(mut s) = field(&[], &[set(0, cosmic), mon(0, RED_DRAGON_ARCHFIEND), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(cosmic)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 3000), "{}", s.context());
    assert_eq!((s.count(0, "grave", cosmic), s.count(0, "grave", RED_DRAGON_ARCHFIEND)), (1, 1), "{}", s.context());
    // A Dragon that is no Synchro Monster (Blue-Eyes) does not start it.
    let Some(mut s) = field(&[], &[set(0, cosmic), mon(0, BLUE_EYES), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(cosmic)], &[Activate(DARK_HOLE)]]).expect_err("no Synchro Monster");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Cosmic Blast, the oath: once it is activated in my own turn I cannot Special Summon that turn (Big Eater, Special Summoned
/// by sending 2 cards from the hand, is offered before Dark Hole and not after, where Cosmic Blast was activated), and I cannot
/// activate it in a turn I already Special Summoned in.
#[test]
fn cosmic_blast_forbids_special_summons_for_the_turn_and_needs_none_before() {
    let [cosmic, big_eater] = ["Cosmic Blast", "Big Eater"].map(card);
    let spawns = [set(0, cosmic), mon(0, RED_DRAGON_ARCHFIEND), hand(0, DARK_HOLE), hand(0, big_eater)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[]).unwrap();
    assert!(offered(&s, "special_summons").contains(&big_eater.into()), "before: {:?}", offered(&s, "special_summons"));
    s.run(&[Activate(DARK_HOLE), Chain(cosmic)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 3000, "{}", s.context());
    assert!(!offered(&s, "special_summons").contains(&big_eater.into()), "after: {:?}", offered(&s, "special_summons"));
    // Already Special Summoned this turn: Cosmic Blast cannot be activated.
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[SpSummon(big_eater), Activate(DARK_HOLE), Chain(cosmic)]).expect_err("a Special Summon came first");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Crush Star: "When a Synchro Monster on the field is destroyed: Target 2 monsters with 1000 or less ATK (1 in your hand
/// and 1 in your Graveyard); Special Summon them in face-up Defense Position." Dark Hole destroys my Goyo Guardian; Thunder
/// Kid (700) comes from the hand and Petit Dragon (600) from the Graveyard, both in Defense Position, and the Feral Imps
/// (1300) stay in the hand.
#[test]
fn crush_star_summons_two_monsters_of_1000_atk_or_less_in_defense_position() {
    let crush = card("Crush Star");
    let spawns = [set(0, crush), mon(0, GOYO), hand(0, DARK_HOLE), hand(0, THUNDER_KID), grave(0, PETIT_DRAGON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE), Chain(crush)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", THUNDER_KID), s.count(0, "mzone", PETIT_DRAGON)), (1, 1), "{}", s.context());
    assert_eq!((s.position_of(0, THUNDER_KID), s.position_of(0, PETIT_DRAGON)), (Some(4), Some(4)), "face-up Defense: {}", s.context());
    assert_eq!(s.count(0, "grave", GOYO), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", crush), 1, "{}", s.context());
}

/// Harmony Crystal: "Banish 2 monsters from your Graveyard, including 1 Synchro Monster that was destroyed and sent to the
/// Graveyard this turn. Target 1 Synchro Monster in your Graveyard and Special Summon it. Then, inflict damage to your
/// opponent equal to half of that monster's ATK." Dark Hole destroys my Goyo; Goyo and Warwolf are banished and Junk Warrior
/// (2300) is Special Summoned: team 1 takes 1150.
#[test]
fn harmony_crystal_banishes_two_monsters_to_revive_a_synchro_monster_and_burn_half_its_atk() {
    let crystal = card("Harmony Crystal");
    let spawns = [set(0, crystal), mon(0, GOYO), hand(0, DARK_HOLE), grave(0, WARWOLF), grave(0, JUNK_WARRIOR)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE), Activate(crystal)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "{}", s.context());
    assert_eq!((s.count(0, "removed", GOYO), s.count(0, "removed", WARWOLF)), (1, 1), "both banished as the cost: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1150, "half of 2300: {}", s.context());
    assert_eq!(s.count(0, "grave", crystal), 1, "{}", s.context());
}

/// Harmony Crystal: the Synchro Monster must have been destroyed this turn. Junk Warrior (put in the Graveyard without being
/// destroyed) and Warwolf are there, and the Crystal is not offered.
#[test]
fn harmony_crystal_needs_a_synchro_monster_destroyed_this_turn() {
    let crystal = card("Harmony Crystal");
    let Some(mut s) = field(&[], &[set(0, crystal), grave(0, WARWOLF), grave(0, JUNK_WARRIOR), grave(0, GOYO)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(crystal)]).expect_err("nothing was destroyed this turn");
    assert!(err.contains("never became possible"), "{err}");
}

/// Harmonia Mirror: "When your opponent Special Summons a Synchro Monster except by Synchro Summon: Take control of that
/// monster." Team 1 revives Goyo Guardian with Monster Reborn and I take control of it. A monster that is no Synchro Monster
/// (Warwolf) does not start it.
#[test]
fn harmonia_mirror_takes_a_synchro_monster_special_summoned_by_other_means() {
    let mirror = card("Harmonia Mirror");
    let spawns = [set(0, mirror), hand(1, MONSTER_REBORN), grave(1, GOYO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(mirror)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", GOYO), s.count(1, "mzone", GOYO)), (1, 0), "Goyo Guardian is mine: {}", s.context());
    assert_eq!(s.count(0, "grave", mirror), 1, "{}", s.context());
    let spawns = [set(0, mirror), hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(mirror)], &[Activate(MONSTER_REBORN)]]).expect_err("no Synchro Monster");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Crossline Counter, Explosive Breakout, Meteor Wave, Emerging Awakening
// ---------------------------------------------------------------------------------------------------------------

/// Crossline Counter: "When you take battle damage during your opponent's turn: Target 1 monster you control; it gains ATK
/// equal to twice the battle damage you took. The opponent's monster that attacked must attack the monster targeted by this
/// effect." Warwolf (2000) attacks my Dark Blade (200 damage); Dark Magician gains 400 (2900) and Warwolf must attack it:
/// Warwolf is destroyed and team 1 takes 900.
#[test]
fn crossline_counter_adds_twice_the_battle_damage_and_forces_the_attacker_onto_the_target() {
    let crossline = card("Crossline Counter");
    let spawns = [set(0, crossline), mon(0, DARK_BLADE), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE), (0, DARK_MAGICIAN), (1, FILLER)]);
    s.run_both([&[Chain(crossline)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_MAGICIAN), Some(2900), "2500 + 2 x 200: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf then had to attack Dark Magician: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 200, 8000 - 900), "{}", s.context());
}

/// Explosive Breakout: "When the only monster you control destroyed an opponent's Level 8 or higher monster by battle, and
/// sends it to the Graveyard: Your monster loses 800 ATK and can attack once again in a row." Dark Magician (2500) destroys
/// Javelin Beetle (Level 8, 2450): 50 damage; Dark Magician (1700) attacks again, directly: 1700.
#[test]
fn explosive_breakout_lets_the_only_monster_attack_again_for_800_less_atk() {
    let breakout = card("Explosive Breakout");
    let beetle = card("Javelin Beetle");
    let Some(mut s) = field(&[], &[set(0, breakout), mon(0, DARK_MAGICIAN), mon(1, beetle)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(DARK_MAGICIAN), Chain(breakout), Attack(DARK_MAGICIAN)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", beetle), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, DARK_MAGICIAN), Some(1700), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 50 - 1700, "{}", s.context());
    assert_eq!(s.count(0, "grave", breakout), 1, "{}", s.context());
}

/// Explosive Breakout: "the only monster you control" and "Level 8 or higher": with a second monster of mine, or against a
/// Level 4 monster (Warwolf), it is not offered.
#[test]
fn explosive_breakout_needs_the_only_monster_and_a_level_8_victim() {
    let breakout = card("Explosive Breakout");
    let beetle = card("Javelin Beetle");
    let Some(mut s) = field(&[], &[set(0, breakout), mon(0, DARK_MAGICIAN), mon(0, THUNDER_KID), mon(1, beetle)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(DARK_MAGICIAN), Chain(breakout)], &[EndTurn]]).expect_err("two monsters");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = field(&[], &[set(0, breakout), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(DARK_MAGICIAN), Chain(breakout)], &[EndTurn]]).expect_err("a Level 4 victim");
    assert!(err.contains("never became possible"), "{err}");
}

/// Meteor Wave: "Target 1 Synchro Monster you control; until the End Phase, it gains 300 ATK, and if it attacks a Defense
/// Position monster, inflict piercing battle damage." Gaia Knight (2900) attacks Warwolf in Defense Position (DEF 100): 2800
/// piercing damage. The 300 end with the turn.
#[test]
fn meteor_wave_adds_300_atk_and_pierces_for_the_turn() {
    let wave = card("Meteor Wave");
    let Some(mut s) = field(&[], &[set(0, wave), mon(0, GAIA_KNIGHT), mon_def(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(wave)], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, GAIA_KNIGHT), Some(2900), "{}", s.context());
    s.run(&[Attack(GAIA_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2800, "2900 - 100 pierced through: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, GAIA_KNIGHT), Some(2600), "until the End Phase: {}", s.context());
}

/// Meteor Wave: "Target 1 Synchro Monster you control": without one (Dark Blade) it is not offered.
#[test]
fn meteor_wave_needs_a_synchro_monster() {
    let wave = card("Meteor Wave");
    let Some(mut s) = field(&[], &[set(0, wave), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(wave)], &[EndTurn]]).expect_err("no Synchro Monster");
    assert!(err.contains("never became possible"), "{err}");
}

/// Emerging Awakening: "When you Normal Summon a monster: Banish 1 other monster you control, and if you do, inflict damage
/// to your opponent equal to the banished monster's Level x100. During your next Standby Phase, Special Summon the banished
/// monster and it gains ATK equal to its Level x100." I Normal Summon Thunder Kid and banish Warwolf (Level 4): 400 damage;
/// in my next Standby Phase Warwolf comes back with 2400 ATK. (The per-card fuzz's failing duel was Yomi Kid's, batch 26.)
#[test]
fn emerging_awakening_banishes_a_monster_for_damage_and_returns_it_stronger() {
    let awakening = card("Emerging Awakening");
    let Some(mut s) = field(&[], &[set(0, awakening), mon(0, WARWOLF), hand(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Summon(THUNDER_KID), Chain(awakening)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "Warwolf is banished: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 400, "Level 4 x 100: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "back in the Standby Phase: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2400), "2000 + 400: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dimension Voyage, Ghoul Summoner, Life Gardna
// ---------------------------------------------------------------------------------------------------------------

/// Dimension Voyage: "Special Summon as many of your Synchro Monsters that are banished as possible. Effect Monsters
/// Special Summoned by this effect cannot attack, and their effects are negated. During the End Phase, banish all monsters
/// Special Summoned by this effect." Goyo Guardian (an Effect Monster) and Gaia Knight (without an effect) come back; Goyo
/// cannot attack and Gaia Knight can (2600 directly); both are banished again in the End Phase.
#[test]
fn dimension_voyage_summons_every_banished_synchro_monster_and_banishes_them_at_the_end_phase() {
    let voyage = card("Dimension Voyage");
    let Some(mut s) = field(&[], &[set(0, voyage), banished(0, GOYO), banished(0, GAIA_KNIGHT)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(voyage)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", GOYO), s.count(0, "mzone", GAIA_KNIGHT)), (1, 1), "both are back: {}", s.context());
    assert_eq!(s.count(0, "grave", voyage), 1, "{}", s.context());
    let err = s.run(&[Attack(GOYO)]).expect_err("Goyo Guardian cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{GAIA_KNIGHT}")) && !err.contains(&format!("\"cid\":{GOYO}")), "{err}");
    s.run(&[Attack(GAIA_KNIGHT)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 2600, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", GOYO), s.count(0, "mzone", GAIA_KNIGHT)), (0, 0), "banished again: {}", s.context());
    assert_eq!((s.count(0, "removed", GOYO), s.count(0, "removed", GAIA_KNIGHT)), (1, 1), "{}", s.context());
}

/// Ghoul Summoner: "Activate this card by targeting 1 Synchro Monster in the Graveyard that was destroyed and sent there this
/// turn; Special Summon it and equip it with this card. When the equipped monster leaves the field, destroy this card. You
/// can discard 1 card, then target 1 Synchro Monster in your Graveyard; Special Summon it, and if you do, take damage equal to
/// its ATK." Dark Hole destroys my Goyo Guardian and the Ghoul Summoner brings it back, equipped. Then one Feral Imp is
/// discarded for Junk Warrior (2300 damage to me). Team 1's Raigeki destroys both monsters and the Ghoul Summoner follows.
#[test]
fn ghoul_summoner_revives_a_destroyed_synchro_monster_and_another_for_damage() {
    let ghoul = card("Ghoul Summoner");
    let spawns = [set(0, ghoul), mon(0, GOYO), hand(0, DARK_HOLE), grave(0, JUNK_WARRIOR), hand(1, RAIGEKI)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, GOYO), (0, FILLER)]);
    s.run(&[Activate(DARK_HOLE), Activate(ghoul)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GOYO), 1, "Goyo Guardian was revived: {}", s.context());
    assert_eq!(s.count(0, "szone", ghoul), 1, "equipped with the Ghoul Summoner: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let (imps, hand_before) = (s.count(0, "grave", FILLER), hand_size(&s, 0));
    s.prefer(&[(0, JUNK_WARRIOR), (0, FILLER)]);
    s.run(&[Activate(ghoul)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2300, "damage equal to its ATK: {}", s.context());
    assert_eq!((s.count(0, "grave", FILLER), hand_size(&s, 0)), (imps + 1, hand_before - 1), "one card was discarded: {}", s.context());
    // Raigeki destroys both; the Ghoul Summoner goes with its equipped monster.
    s.run_both([&[EndTurn], &[Activate(RAIGEKI)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", GOYO), s.count(0, "mzone", JUNK_WARRIOR)), (0, 0), "{}", s.context());
    assert_eq!((s.count(0, "szone", ghoul), s.count(0, "grave", ghoul)), (0, 1), "{}", s.context());
}

/// Ghoul Summoner: its first effect needs a Synchro Monster destroyed this turn: one that merely sits in the Graveyard (put
/// there without being destroyed) is no target, and the effect is not offered.
#[test]
fn ghoul_summoner_needs_a_synchro_monster_destroyed_this_turn() {
    let ghoul = card("Ghoul Summoner");
    let Some(mut s) = field(&[], &[set(0, ghoul), grave(0, GOYO)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(ghoul)]).expect_err("nothing was destroyed this turn");
    assert!(err.contains("never became possible"), "{err}");
}

/// Life Gardna: "If this card is sent to the Graveyard for a Synchro Summon: Gain 800 LP." Life Gardna (Level 1) and Tune
/// Warrior (Level 3) make Armory Arm (Level 4): 8800 LP. Dark Hole destroying it gives nothing.
#[test]
fn life_gardna_gains_800_lp_when_it_is_a_synchro_material() {
    let gardna = card("Life Gardna");
    let Some(mut s) = field_extra([&[ARMORY_ARM], &[]], &[mon(0, gardna), mon(0, TUNE_WARRIOR)]) else { return };
    s.run(&[SpSummon(ARMORY_ARM)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ARMORY_ARM), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", gardna), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8800, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, gardna), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    assert_eq!((s.count(0, "grave", gardna), s.lp(0)), (1, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Guard Master, Devoted Gardna, Fake Gardna
// ---------------------------------------------------------------------------------------------------------------

/// Guard Master: "When a face-up Attack Position monster you control is targeted for an attack while this card is in your
/// Graveyard: you can banish this card; Change the attacked monster to Defense Position. During this turn, that monster
/// cannot be destroyed by battle." Dark Magician (2500) attacks my Warwolf (2000/100): Warwolf is switched to Defense, is
/// not destroyed and I take no damage. Without Guard Master Warwolf is destroyed for 500.
#[test]
fn guard_master_turns_the_attacked_monster_to_defense_and_keeps_it_alive() {
    let master = card("Guard Master");
    let Some(mut s) = field(&[], &[grave(0, master), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(master)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", master), 1, "banished as the cost: {}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.position_of(0, WARWOLF)), (1, Some(4)), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(DARK_MAGICIAN)]]).unwrap();
    assert_eq!((s.count(0, "grave", WARWOLF), s.lp(0)), (1, 8000 - 500), "{}", s.context());
}

/// Devoted Gardna: "When a monster you control with 1000 or less ATK is targeted for an attack: You can send this card from
/// your hand to the Graveyard; you take no battle damage from this battle." Warwolf attacks my Thunder Kid (700): the Kid is
/// destroyed and I take nothing. A monster with more than 1000 ATK (Dark Blade) is not covered.
#[test]
fn devoted_gardna_saves_the_battle_damage_for_a_monster_of_1000_atk_or_less() {
    let gardna = card("Devoted Gardna");
    let Some(mut s) = field(&[], &[hand(0, gardna), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(gardna)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", gardna), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the Kid still dies: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, gardna), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(gardna)], &[Attack(DARK_MAGICIAN)]]).expect_err("1800 ATK");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000 - 700, "{}", s.context());
}

/// Fake Gardna: "When your opponent's monster declares a direct attack, you can Special Summon this card from your Graveyard
/// in face-up Defense Position. If this card is Special Summoned with this effect, banish it when it is destroyed." Dark
/// Magician (2500) attacks directly and Fake Gardna (DEF 2000) comes from the Graveyard.
#[test]
fn fake_gardna_comes_from_the_graveyard_at_a_direct_attack_and_is_banished_when_destroyed() {
    let fake = card("Fake Gardna");
    let Some(mut s) = field(&[], &[grave(0, fake), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(fake)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", fake), 0, "{}", s.context());
    // The attack was redirected to it (the attacker's controller picks the new target): destroyed, it is banished.
    assert_eq!((s.count(0, "removed", fake), s.count(0, "mzone", fake)), (1, 0), "destroyed, so banished: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Hardship (engine/overrides/c511000620.lua)
// ---------------------------------------------------------------------------------------------------------------

/// The planned side's plan runs until `team` is asked to chain to a card the plan has just activated (not at a phase's chance
/// to chain), and `cid`'s options in that question come back as (index, option): a card with two effects (Hardship's damage
/// answer and its Graveyard effect) shows both.
fn chain_options(s: &mut Scenario, plan: &[Action], team: u64, cid: u16) -> Vec<(usize, serde_json::Value)> {
    for _ in 0..12 {
        s.run_to(plan, |p| p["type"] == msg::SELECT_CHAIN && p["player"] == team).unwrap();
        if s.messages().iter().any(|m| m["type"] == msg::CHAINING || m["type"] == 71) {
            let selects = s.prompt()["selects"].as_array().cloned().unwrap_or_default();
            return selects.into_iter().enumerate().filter(|(_, c)| c["cid"] == cid).collect();
        }
        s.pass().unwrap();
    }
    panic!("no chain to answer: {}", s.context());
}

/// Hardship: "Equip this card to a monster you control. If you would take damage by an opponent's monster's effect: You can
/// reduce the ATK/DEF of the equipped monster to 0; you take no damage from that monster's effect." Team 1 equips Hardship to
/// Warwolf; my Cannon Soldier (Tribute 1 monster: 500 damage) is answered: Warwolf's ATK and DEF are 0 and team 1 takes
/// nothing, where the Cannon Soldier's cost is paid. Without Hardship the 500 are taken. (The answer was never offered before
/// the override: an equipped Continuous Trap's effects need `EFFECT_FLAG2_CONTINUOUS_EQUIP`.)
#[test]
fn hardship_reduces_the_equipped_monster_to_0_to_take_no_damage_from_a_monster_effect() {
    let hardship = card("Hardship");
    let spawns = [set(1, hardship), mon(1, WARWOLF), mon(0, CANNON_SOLDIER), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.prefer_most(true);
    s.run_both([&[EndTurn], &[Activate(hardship)]]).unwrap();
    assert_eq!(s.count(1, "szone", hardship), 1, "equipped: {}", s.context());
    let options = chain_options(&mut s, &[Activate(CANNON_SOLDIER)], 1, hardship);
    assert_eq!(options.len(), 2, "the damage answer and the Graveyard effect: {options:?}");
    let answer = options.iter().find(|(_, o)| o["description_text"] != "Destroy all Equip Cards").expect("the damage answer").0;
    s.respond(yo3_server::ocg::Response::Chain(Some(answer)).json()).unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000, "no damage: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the cost was paid: {}", s.context());
    assert_eq!((s.attack_of(1, WARWOLF), stats(&s, 1, WARWOLF).1), (Some(0), Some(0)), "{}", s.context());
    // The control: no Hardship, 500 damage.
    let Some(mut s) = field(&[], &[mon(1, WARWOLF), mon(0, CANNON_SOLDIER), mon(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.prefer_most(true);
    s.run(&[Activate(CANNON_SOLDIER)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

/// Hardship: "an opponent's monster's effect": the damage of a Trap Card (Just Desserts, 500 per monster) is not covered:
/// only the Graveyard effect is offered (the script answered any damage to me before the override).
#[test]
fn hardship_does_not_answer_the_damage_of_a_trap_card() {
    let hardship = card("Hardship");
    let spawns = [set(1, hardship), mon(1, WARWOLF), set(0, JUST_DESSERTS)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(hardship)]]).unwrap();
    let options = chain_options(&mut s, &[Activate(JUST_DESSERTS)], 1, hardship);
    assert_eq!(options.len(), 1, "no damage answer: {options:?}");
    assert_eq!(options[0].1["description_text"], "Destroy all Equip Cards", "{options:?}");
}

/// Hardship: "You can send this face-up card to the Graveyard; destroy the equipped monster, then destroy all Equip cards on
/// the field and send them to the Graveyard." Sword of Dark Destruction is on a Feral Imp and Hardship on Warwolf: Warwolf, the
/// Sword and Hardship all go to the Graveyard, and the Imp (not the equipped monster) lives, back to 1300 ATK.
#[test]
fn hardship_destroys_its_monster_and_every_equip_card() {
    let (hardship, sword) = (card("Hardship"), card("Sword of Dark Destruction"));
    let spawns = [hand(0, sword), set(0, hardship), mon(0, WARWOLF), mon(0, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, FILLER)]);
    s.run(&[Activate(sword)]).unwrap();
    assert_eq!(s.attack_of(0, FILLER), Some(1700), "{}", s.context());
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(hardship)]).unwrap();
    assert_eq!(s.count(0, "szone", hardship), 1, "{}", s.context());
    s.run(&[Activate(hardship)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", hardship), s.count(0, "grave", sword)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.count(0, "mzone", FILLER), s.attack_of(0, FILLER)), (1, Some(1300)), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Half Shield and Return Marker
// ---------------------------------------------------------------------------------------------------------------

/// Half Shield: "Halve the battle and effect damage from 1 opponent's monster." Warwolf (2000) destroys my Thunder Kid (700)
/// and the 1300 battle damage is halved: 650. Team 1's Cannon Soldier would deal 500 by its effect: 250.
#[test]
fn half_shield_halves_battle_damage_and_the_damage_of_a_monster_effect() {
    let half = card("Half Shield");
    let Some(mut s) = field(&[], &[set(0, half), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(half)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 650, "{}", s.context());
    assert_eq!(s.count(0, "grave", half), 1, "{}", s.context());
    let spawns = [set(0, half), mon(1, CANNON_SOLDIER), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, THUNDER_KID)]);
    s.prefer_most(true);
    s.run_both([&[Chain(half)], &[Activate(CANNON_SOLDIER)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 250, "{}", s.context());
    assert_eq!(s.count(0, "grave", half), 1, "{}", s.context());
}

/// Return Marker: "When your opponent activates an Effect Monster's effect that would destroy a card or inflict damage:
/// Negate the effect, and if you do, inflict damage to your opponent equal to that monster's ATK." Team 1's Cannon Soldier
/// (1400 ATK) tries 500 damage: it is negated (its Tribute is paid) and team 1 takes 1400.
#[test]
fn return_marker_negates_a_monster_effect_and_inflicts_the_monsters_atk() {
    let marker = card("Return Marker");
    let spawns = [set(0, marker), mon(1, CANNON_SOLDIER), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, THUNDER_KID)]);
    s.prefer_most(true);
    s.run_both([&[Chain(marker)], &[Activate(CANNON_SOLDIER)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1400), "negated, and the ATK inflicted: {}", s.context());
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(0, "grave", marker)), (1, 1), "{}", s.context());
}

/// Return Marker: "an Effect Monster's effect": the damage of a Trap (Just Desserts) is not answered.
#[test]
fn return_marker_does_not_answer_a_trap() {
    let marker = card("Return Marker");
    let Some(mut s) = field(&[], &[set(0, marker), mon(0, WARWOLF), set(1, JUST_DESSERTS)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(marker)], &[Activate(JUST_DESSERTS)]]).expect_err("a Trap");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Monster Chain, Panic Wave, Realize Defense
// ---------------------------------------------------------------------------------------------------------------

/// Monster Chain: "When this card is activated: Place Chain Counters on it equal to the number of monsters you control. Once
/// per turn, during your opponent's End Phase: Remove 1 Chain Counter from this card. When there are no Chain Counters on
/// this card, destroy it. While this card is on the field, neither player can declare an attack." Two monsters, so two
/// Counters: in turn 3 and in turn 4 (when team 0's) nobody can attack; Monster Chain is destroyed in team 1's End Phase of
/// turn 4, and Warwolf attacks in turn 5.
#[test]
fn monster_chain_stops_all_attacks_until_its_chain_counters_run_out() {
    let chain = card("Monster Chain");
    let spawns = [set(0, chain), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(chain)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", chain), 1, "{}", s.context());
    // Turn 2 (team 1's End Phase: 1 Counter left), then turn 3: no attacks, and Monster Chain stays.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "szone", chain), 1, "one Counter left: {}", s.context());
    assert!(attackers(&mut s).is_empty(), "no attack in turn 3: {}", s.context());
    to_main_phase_2(&mut s);
    // Turn 4 (team 1's End Phase removes the last Counter and destroys it), then turn 5: Warwolf attacks.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.count(0, "szone", chain), s.count(0, "grave", chain)), (0, 1), "no Counter left: {}", s.context());
    s.run(&[Attack(WARWOLF)]).unwrap();
    clean(&s);
}

/// Panic Wave: "Target 1 card you control, and destroy it. During this turn, negate the effects of all face-up Continuous
/// Spell Cards, Continuous Trap Cards, and Effect Monsters on the field." I destroy Thunder Kid, and my Cannon Soldier (an
/// Effect Monster) is negated for the rest of the turn: its effect can still be activated, as under Skill Drain, and its
/// Tribute (Warwolf) is paid, but it deals no damage. The turn after it deals its 500.
#[test]
fn panic_wave_destroys_one_card_and_negates_every_effect_monster_for_the_turn() {
    let wave = card("Panic Wave");
    let spawns = [set(0, wave), mon(0, CANNON_SOLDIER), mon(0, THUNDER_KID), mon(0, WARWOLF), mon(0, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(wave)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "mzone", FILLER)), (1, 1), "only the target was destroyed: {}", s.context());
    s.prefer(&[(0, WARWOLF)]);
    s.prefer_most(true);
    s.run(&[Activate(CANNON_SOLDIER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the Tribute was paid: {}", s.context());
    assert_eq!(s.lp(1), 8000, "but the effect was negated: {}", s.context());
    // The turn after, it works again.
    s.prefer(&[(0, FILLER)]);
    s.run_both([&[EndTurn, Activate(CANNON_SOLDIER)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

/// Realize Defense: "Target 1 face-up Attack Position monster you control with a higher original DEF than its original ATK;
/// Change it to Defense Position." Feral Imp (1300/1400) is switched to Defense Position; Warwolf (2000/100) is not a target.
/// With only Warwolf it is not offered.
#[test]
fn realize_defense_changes_a_monster_with_more_def_than_atk_to_defense_position() {
    let realize = card("Realize Defense");
    let Some(mut s) = field(&[], &[set(0, realize), mon(0, FILLER), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(realize)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.position_of(0, FILLER), s.position_of(0, WARWOLF)), (Some(4), Some(1)), "{}", s.context());
    assert_eq!(s.count(0, "grave", realize), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, realize), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(realize)], &[EndTurn]]).expect_err("Warwolf has more ATK than DEF");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Meteor Stream, Regen Warrior, Rising Rush, Scouting Warrior
// ---------------------------------------------------------------------------------------------------------------

/// Meteor Stream: "Activate only when a monster is Special Summoned during the turn it was sent from the field to the
/// Graveyard as a Tribute: Inflict 1000 damage to your opponent." I Tribute Summon Summoned Skull with Thunder Kid and
/// revive the Kid with Monster Reborn: 1000 damage. Reviving Warwolf instead (never Tributed) does not start it.
#[test]
fn meteor_stream_burns_1000_when_a_tributed_monster_is_special_summoned_the_same_turn() {
    let (stream, skull) = (card("Meteor Stream"), card("Summoned Skull"));
    let spawns = [set(0, stream), hand(0, skull), mon(0, THUNDER_KID), hand(0, MONSTER_REBORN), grave(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Summon(skull), Activate(MONSTER_REBORN), Chain(stream)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "the Tribute is back: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    assert_eq!(s.count(0, "grave", stream), 1, "{}", s.context());
    // The control.
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (0, WARWOLF)]);
    s.run(&[Summon(skull)]).unwrap();
    s.prefer(&[(0, WARWOLF)]);
    let err = s.run(&[Activate(MONSTER_REBORN), Chain(stream)]).expect_err("Warwolf was not Tributed");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Regen Warrior: "When this card is sent from your hand to the Graveyard: You can Special Summon it." The End Phase's
/// hand-size discard sends it from the hand and it comes back to the field.
#[test]
fn regen_warrior_returns_from_the_graveyard_when_discarded_from_the_hand() {
    let regen = card("Regen Warrior");
    let spawns = [hand(0, regen), hand(0, DARK_BLADE), hand(0, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, regen), (0, DARK_BLADE)]);
    s.run(&[EndTurn, Chain(regen)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", regen), s.count(0, "grave", regen)), (1, 0), "{}", s.context());
    assert_eq!(s.position_of(0, regen), Some(1), "{}", s.context());
}

/// Regen Warrior: "sent from your hand": destroyed on the field by Dark Hole it stays in the Graveyard.
#[test]
fn regen_warrior_stays_when_it_is_destroyed_on_the_field() {
    let regen = card("Regen Warrior");
    let Some(mut s) = field(&[], &[mon(0, regen), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", regen), s.count(0, "grave", regen)), (0, 1), "{}", s.context());
}

/// Rising Rush: "When your opponent takes damage from a card effect: Special Summon 1 Tuner from your hand." My Ookazi
/// (800) is answered with Tune Warrior from the hand (Warwolf, no Tuner, stays there).
#[test]
fn rising_rush_summons_a_tuner_from_the_hand_when_the_opponent_takes_effect_damage() {
    let rush = card("Rising Rush");
    let spawns = [set(0, rush), hand(0, OOKAZI), hand(0, TUNE_WARRIOR), hand(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(OOKAZI), Chain(rush)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
    assert_eq!((s.count(0, "mzone", TUNE_WARRIOR), s.count(0, "hand", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", rush), 1, "{}", s.context());
}

/// Rising Rush: battle damage is not "from a card effect".
#[test]
fn rising_rush_ignores_battle_damage() {
    let rush = card("Rising Rush");
    let Some(mut s) = field(&[], &[set(0, rush), hand(0, TUNE_WARRIOR), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(WARWOLF), Chain(rush)], &[EndTurn]]).expect_err("battle damage");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
}

/// Scouting Warrior: "When this card is added from your Deck to your hand by a card effect: you can Special Summon it."
/// Reinforcement of the Army fetches it from the bottom of my Deck and it is Special Summoned at once.
#[test]
fn scouting_warrior_is_summoned_when_a_card_effect_adds_it_from_the_deck() {
    let scouting = card("Scouting Warrior");
    let reinforcement = card("Reinforcement of the Army");
    let Some(mut s) = field(&[scouting], &[hand(0, reinforcement)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, scouting)]);
    s.run(&[Activate(reinforcement), Chain(scouting)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", scouting), s.count(0, "hand", scouting)), (1, 0), "{}", s.context());
    assert_eq!(s.position_of(0, scouting), Some(1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Revenge Return, Revival Ticket, Shard of Hope, Double Delta Warrior
// ---------------------------------------------------------------------------------------------------------------

/// Revenge Return: "If a monster you control is destroyed by battle: Draw 1 card." Warwolf destroys my Thunder Kid and I
/// draw 1 card.
#[test]
fn revenge_return_draws_a_card_when_a_monster_is_destroyed_by_battle() {
    let revenge = card("Revenge Return");
    let Some(mut s) = field(&[], &[set(0, revenge), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let before = hand_size(&s, 0);
    s.run_both([&[Chain(revenge)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "one card drawn: {}", s.context());
    assert_eq!(s.count(0, "grave", revenge), 1, "{}", s.context());
}

/// Revenge Return, from the Graveyard: "You can banish this card from your Graveyard, then target 1 monster in your Graveyard
/// that was destroyed by a card effect this turn; Special Summon it." Dark Hole destroys my Warwolf and the banished Revenge
/// Return brings it back (Dark Blade, put in the Graveyard without being destroyed, is no target).
#[test]
fn revenge_return_revives_a_monster_destroyed_by_a_card_effect_this_turn() {
    let revenge = card("Revenge Return");
    let spawns = [grave(0, revenge), mon(0, WARWOLF), hand(0, DARK_HOLE), grave(0, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE), Activate(revenge)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", revenge), 1, "banished from the Graveyard: {}", s.context());
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(0, "mzone", DARK_BLADE)), (1, 0), "{}", s.context());
}

/// Revenge Return: with no monster destroyed by a card effect this turn, the Graveyard effect is not offered.
#[test]
fn revenge_return_needs_a_monster_destroyed_by_a_card_effect_this_turn() {
    let revenge = card("Revenge Return");
    let Some(mut s) = field(&[], &[grave(0, revenge), grave(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(revenge)]).expect_err("nothing destroyed");
    assert!(err.contains("never became possible"), "{err}");
}

/// Revival Ticket: "Activate only when a monster you control is destroyed by battle. Destroy 1 card you control for each of
/// those monsters, and Special Summon those monsters destroyed by battle this turn from the Graveyard to your field, then,
/// gain LP equal to all battle damage you took this turn." Warwolf destroys my Thunder Kid (1300 damage); I destroy Dark
/// Blade, Thunder Kid returns and the 1300 LP come back.
#[test]
fn revival_ticket_revives_the_monster_destroyed_by_battle_for_a_card_and_regains_the_damage() {
    let ticket = card("Revival Ticket");
    let Some(mut s) = field(&[], &[set(0, ticket), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (0, DARK_BLADE)]);
    s.run_both([&[Chain(ticket)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", THUNDER_KID), s.count(0, "grave", DARK_BLADE)), (1, 1), "the Kid is back, Dark Blade paid: {}", s.context());
    assert_eq!(s.lp(0), 8000, "1300 damage and 1300 regained: {}", s.context());
    assert_eq!(s.count(0, "grave", ticket), 1, "{}", s.context());
}

/// Revival Ticket: "Destroy 1 card you control": with nothing else on my field to destroy it is not offered.
#[test]
fn revival_ticket_needs_another_card_to_destroy() {
    let ticket = card("Revival Ticket");
    let Some(mut s) = field(&[], &[set(0, ticket), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(ticket)], &[Attack(WARWOLF)]]).expect_err("nothing to destroy");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
}

/// Shard of Hope: "If a monster you control inflicts battle damage to your opponent: Draw 1 card, and if it is a Trap,
/// destroy this card and activate that card from your hand. If the activation requirement(s) are incorrect or it isn't a
/// Trap, destroy it and send it to the Graveyard." Warwolf attacks directly for 2000: I draw a Feral Imp and it goes to the
/// Graveyard (hand size as before, Shard of Hope stays).
#[test]
fn shard_of_hope_draws_a_card_and_sends_it_to_the_graveyard_when_it_is_no_trap() {
    let shard = card("Shard of Hope");
    let Some(mut s) = field(&[], &[set(0, shard), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(shard)]).unwrap();
    let (before, imps, deck) = (hand_size(&s, 0), s.count(0, "grave", FILLER), s.state()[0]["deck"].as_i64().unwrap());
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "szone", shard), 1, "Shard of Hope stays: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), imps + 1, "the drawn Imp is in the Graveyard: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_i64().unwrap(), deck - 1 - 1, "a card was drawn for the turn and one by Shard of Hope: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "the turn's draw only: {}", s.context());
}

/// Shard of Hope, the Trap case: with a Deck of Gravity Bind the drawn card is a Trap and is activated from the hand: Gravity
/// Bind lies face-up in my Spell & Trap Zone, and Shard of Hope is destroyed.
#[test]
fn shard_of_hope_activates_a_drawn_trap() {
    let (shard, bind) = (card("Shard of Hope"), card("Gravity Bind"));
    let Some(dir) = engine_dir() else { return };
    let spawns = [set(0, shard), mon(0, WARWOLF)];
    let decks = [(&[bind; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = started(&dir, 7, 8000, decks, &spawns);
    s.run(&[Activate(shard)]).unwrap();
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "szone", bind), 1, "the drawn Gravity Bind was activated: {}", s.context());
    assert_eq!(s.count(0, "grave", shard), 1, "Shard of Hope was destroyed: {}", s.context());
}

/// Double Delta Warrior: "If only your opponent controls a monster, you can Special Summon this card (from your hand)."
/// Team 1 has Warwolf and I have none: it comes to my field (Level 6, 0/2100). With a monster of mine, or none of team 1's, it
/// is not offered.
#[test]
fn double_delta_warrior_is_summoned_when_only_the_opponent_controls_a_monster() {
    let ddw = card("Double Delta Warrior");
    let Some(mut s) = field(&[], &[hand(0, ddw), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(ddw)]).unwrap();
    clean(&s);
    assert_eq!(stats(&s, 0, ddw), (Some(0), Some(2100), Some(6)), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, ddw), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&ddw.into()), "I control a monster: {:?}", offered(&s, "special_summons"));
    let Some(mut s) = field(&[], &[hand(0, ddw)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&ddw.into()), "nobody does: {:?}", offered(&s, "special_summons"));
}

/// Double Delta Warrior: "If this card is used as a Synchro Material, monsters you control cannot attack this turn." Nitro
/// Synchron (Level 2 Tuner) and it make Stardust Dragon (Level 8) in turn 3, and nothing of mine can attack: not Stardust
/// Dragon, not another monster. Without it a Synchro Summon leaves the attack alone.
#[test]
fn double_delta_warrior_as_synchro_material_stops_my_attacks_for_the_turn() {
    let (ddw, nitro, stardust) = (card("Double Delta Warrior"), card("Nitro Synchron"), card("Stardust Dragon"));
    let Some(mut s) = field_extra([&[stardust], &[]], &[hand(0, ddw), hand(0, nitro), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, SpSummon(ddw), Summon(nitro), SpSummon(stardust)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", stardust), 1, "{}", s.context());
    assert!(attackers(&mut s).is_empty(), "no attack this turn: {}", s.context());
    // The control: Life Gardna and Tune Warrior make Armory Arm, and it can attack.
    let gardna = card("Life Gardna");
    let Some(mut s) = field_extra([&[ARMORY_ARM], &[]], &[mon(0, gardna), mon(0, TUNE_WARRIOR), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, SpSummon(ARMORY_ARM)], &[EndTurn]]).unwrap();
    assert!(attackers(&mut s).iter().any(|c| *c == ARMORY_ARM), "{}", s.context());
}
