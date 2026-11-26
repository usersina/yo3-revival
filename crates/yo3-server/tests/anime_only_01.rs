//! Anime-only cards, group 1 (rows 5220-5254 of engine/added/cards.tsv): Dark Magic Retribution, Backup Gardna (Anime),
//! the Guardian cards, Mirror Tablet, Bounce Spell, Magical Pigeon, Necro Wall, Awakening from Beyond, Dark Spear,
//! Summoning Clock, Limit Tribute, Nightmare Binding, Self Tribute and Counterbalance. These cards exist only in the
//! anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core
//! with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones.
//! Necro Cycle is not tested: it needs "Necro Mannequin", which has no card script and so cannot be held.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg};

// The cards of this group (client ids).
const DARK_MAGIC_RETRIBUTION: u16 = 5188;
const BACKUP_GARDNA: u16 = 5283; // Backup Gardna (Anime), Level 4, 500/2200
const GUARDIAN_FORMATION: u16 = 5294;
const GUARDIAN_SHIELD: u16 = 5297;
const MIRROR_TABLET: u16 = 5459;
const BOUNCE_SPELL: u16 = 5187;
const MAGICAL_PIGEON: u16 = 5215;
const PIGEON_TOKEN: u16 = 5184;
const NECRO_WALL: u16 = 5464;
const NECRO_WALL_TOKEN: u16 = 5183;
const AWAKENING: u16 = 5186; // Awakening from Beyond
const DARK_SPEAR: u16 = 5189;
const SUMMONING_CLOCK: u16 = 5240;
const LIMIT_TRIBUTE: u16 = 5364;
const NIGHTMARE_BINDING: u16 = 5368;
const SELF_TRIBUTE: u16 = 5454;
const COUNTERBALANCE: u16 = 5456;
// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // 700/600 vanilla
const THRONE_ROOM: u16 = 4519; // Guardian of the Throne Room: a "Guardian", 1650/1600, vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_MAGICIAN_GIRL: u16 = 4766; // Level 6 Spellcaster
const BATTLE_STEER: u16 = 4020; // Level 5, 1800/1300, vanilla: one Tribute
const AXE_OF_DESPAIR: u16 = 4310; // an Equip Spell, +1000 ATK
const DARK_HOLE: u16 = 4342;
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const MST: u16 = 4909; // Mystical Space Typhoon
const MYSTIK_WOK: u16 = 5978; // Tribute 1 monster, gain its ATK or DEF as LP
const ZOMBIE_MASTER: u16 = 7215; // a Zombie with an ignition effect
const PYRAMID_TURTLE: u16 = 5423; // a Level 4 Zombie

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }
fn banished(team: u32, cid: u16) -> Spawn { Spawn { team, place: Banished, cid } }

/// An anime-cards duel, both Decks of filler: team 0's has `bottom` at its very bottom (out of the opening hand and the
/// first draws), for a card an effect fetches from the Deck.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(FILLER, 40 - bottom.len())).collect();
    let decks = [(&mine[..], &[][..]), (&[FILLER; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// Dark Magic Retribution: "When an opponent's monster declares an attack: Target 1 Spellcaster-Type monster in your
/// Graveyard that was destroyed the previous turn; Special Summon it, and battle with that opponent's monster, also
/// destroy it at the end of the Battle Phase." Dark Magician is destroyed by Dark Hole in turn 1, then Warwolf attacks
/// directly in turn 2: Dark Magician comes back, battles Warwolf (2500 against 2000) and is destroyed at the end of the
/// Battle Phase.
#[test]
fn dark_magic_retribution_revives_a_spellcaster_destroyed_last_turn_and_battles() {
    let spawns = [set(0, DARK_MAGIC_RETRIBUTION), mon(0, DARK_MAGICIAN), hand(0, DARK_HOLE), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Activate(DARK_HOLE), EndTurn, ChainOnAttack(DARK_MAGIC_RETRIBUTION)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_MAGIC_RETRIBUTION), 1, "the Trap was used: {}", s.context());
    assert!(s.monster(0, DARK_MAGICIAN).is_some(), "Dark Magician is back on the field after the battle: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf lost the battle: {}", s.context());
    assert_eq!(s.lp(1), 7500, "2500 against 2000: 500 damage to the attacker's controller: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    // The end of the Battle Phase destroys it.
    s.pass().unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 0, "destroyed at the end of the Battle Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "{}", s.context());
}

/// Dark Magic Retribution: only a Spellcaster "destroyed the previous turn" can be targeted. One that was in the
/// Graveyard from the start (not destroyed last turn) gives no activation: the attack goes through.
#[test]
fn dark_magic_retribution_needs_a_monster_destroyed_the_turn_before() {
    let spawns = [set(0, DARK_MAGIC_RETRIBUTION), grave(0, DARK_MAGICIAN), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let r = s.run_both([&[ChainOnAttack(DARK_MAGIC_RETRIBUTION)], &[Summon(WARWOLF), Attack(WARWOLF)]]);
    let err = r.expect_err("the Trap could never be chained");
    assert!(err.contains("ChainOnAttack(5188) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.lp(0), 6000, "Warwolf's attack was a direct 2000: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 0, "{}", s.context());
}

/// Backup Gardna (Anime): "You can target an equipped Equip Card on the field; equip it to another appropriate target on
/// the field." Axe of Despair on Warwolf is moved to Backup Gardna.
#[test]
fn backup_gardna_moves_an_equip_card_to_another_monster() {
    let spawns = [hand(0, AXE_OF_DESPAIR), mon(0, BACKUP_GARDNA), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(AXE_OF_DESPAIR)]).unwrap();
    assert_eq!(s.attack_of(0, WARWOLF), Some(3000), "Axe of Despair is on Warwolf: {}", s.context());
    s.prefer(&[(0, AXE_OF_DESPAIR), (0, BACKUP_GARDNA)]);
    s.run(&[Activate(BACKUP_GARDNA)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "Warwolf lost the Equip Card: {}", s.context());
    assert_eq!(s.attack_of(0, BACKUP_GARDNA), Some(1500), "and Backup Gardna holds it: {}", s.context());
    assert_eq!(s.count(0, "szone", AXE_OF_DESPAIR), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", AXE_OF_DESPAIR), 0, "{}", s.context());
}

/// Backup Gardna (Anime): "another appropriate target": Dark Spear is equipped to Dark Magician only, so with nothing
/// else it can go to there is no effect to activate.
#[test]
fn backup_gardna_cannot_move_an_equip_card_to_an_inappropriate_monster() {
    let spawns = [hand(0, DARK_SPEAR), mon(0, DARK_MAGICIAN), mon(0, BACKUP_GARDNA), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run(&[Activate(DARK_SPEAR)]).unwrap();
    assert_eq!(s.count(0, "szone", DARK_SPEAR), 1, "Dark Spear is equipped: {}", s.context());
    let err = s.run(&[Activate(BACKUP_GARDNA)]).expect_err("no appropriate target");
    assert!(err.contains("Activate(5283) never became possible"), "{err}");
    clean(&s);
}

/// Guardian Formation: "When a "Guardian" monster is targeted for an attack: Move that monster to an unoccupied Monster
/// Card Zone, then, equip 1 Equip Spell from your Deck to an appropriate monster you control." Warwolf attacks Guardian
/// of the Throne Room (1650); Axe of Despair is equipped from the Deck (2650) and Warwolf is destroyed in the battle.
#[test]
fn guardian_formation_moves_the_attacked_guardian_and_equips_from_the_deck() {
    let spawns = [set(0, GUARDIAN_FORMATION), mon(0, THRONE_ROOM), hand(1, WARWOLF)];
    let Some(mut s) = field(&[AXE_OF_DESPAIR], &spawns) else { return };
    let deck = s.state()[0]["deck"].as_i64().unwrap();
    s.run_both([&[ChainOnAttack(GUARDIAN_FORMATION)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", AXE_OF_DESPAIR), 1, "Axe of Despair came from the Deck: {}", s.context());
    assert_eq!(s.state()[0]["deck"], deck - 1, "{}", s.context());
    assert_eq!(s.attack_of(0, THRONE_ROOM), Some(2650), "the Guardian wears it: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "2650 beats Warwolf's 2000: {}", s.context());
    assert_eq!(s.lp(1), 7350, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Guardian Formation: only a "Guardian" monster attacked starts it; Warwolf attacked is no reason to chain.
#[test]
fn guardian_formation_ignores_an_attack_on_another_monster() {
    let spawns = [set(0, GUARDIAN_FORMATION), mon(0, THUNDER_KID), hand(1, WARWOLF)];
    let Some(mut s) = field(&[AXE_OF_DESPAIR], &spawns) else { return };
    let err = s.run_both([&[ChainOnAttack(GUARDIAN_FORMATION)], &[Summon(WARWOLF), Attack(WARWOLF)]]).expect_err("not chainable");
    assert!(err.contains("ChainOnAttack(5294) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", AXE_OF_DESPAIR), 0, "{}", s.context());
}

/// Guardian Shield: "Equip only to a "Guardian" monster. It gains 300 DEF." On Guardian of the Throne Room (1600 DEF).
#[test]
fn guardian_shield_gives_a_guardian_300_def() {
    let spawns = [hand(0, GUARDIAN_SHIELD), mon(0, THRONE_ROOM), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(GUARDIAN_SHIELD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", GUARDIAN_SHIELD), 1, "{}", s.context());
    assert_eq!(s.monster(0, THRONE_ROOM).unwrap()["defense"], 1900, "{}", s.context());
    assert_eq!(s.monster(0, WARWOLF).unwrap()["defense"], 100, "Warwolf is no Guardian: {}", s.context());
}

/// Guardian Shield: "Equip only to a "Guardian" monster": with no Guardian on the field it cannot be activated.
#[test]
fn guardian_shield_cannot_equip_a_monster_that_is_no_guardian() {
    let Some(mut s) = field(&[], &[hand(0, GUARDIAN_SHIELD), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(GUARDIAN_SHIELD)]).expect_err("no Guardian to equip");
    assert!(err.contains("Activate(5297) never became possible"), "{err}");
    clean(&s);
}

/// Guardian Shield: "If a "Guardian" monster you control would be destroyed by battle, you can destroy this card
/// instead." Warwolf (2000) attacks the equipped Guardian (1650): the Shield is destroyed in its place.
#[test]
fn guardian_shield_is_destroyed_instead_of_the_guardian() {
    let spawns = [hand(0, GUARDIAN_SHIELD), mon(0, THRONE_ROOM), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Activate(GUARDIAN_SHIELD)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THRONE_ROOM), 1, "the Guardian stayed: {}", s.context());
    assert_eq!(s.count(0, "grave", THRONE_ROOM), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", GUARDIAN_SHIELD), 1, "the Shield took its place: {}", s.context());
    assert_eq!(s.lp(0), 7650, "the battle damage was still dealt: {}", s.context());
}

/// Mirror Tablet: "When a monster you control is destroyed by battle with an opponent's monster: You can make 1 face-up
/// Attack Position monster you control gain ATK equal to half of that opponent's monster's ATK until the end of the
/// Damage Step and battle that opponent's monster." Dark Magician (2500) destroys Thunder Kid; Warwolf gains 1250 (3250),
/// battles Dark Magician and destroys it, 750 damage.
#[test]
fn mirror_tablet_gives_another_monster_half_the_attackers_atk_and_battles_it() {
    let spawns = [set(0, MIRROR_TABLET), mon(0, THUNDER_KID), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[Chain(MIRROR_TABLET)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was destroyed by the attack: {}", s.context());
    assert_eq!(s.count(0, "grave", MIRROR_TABLET), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "Warwolf destroyed Dark Magician: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    // Team 0 lost 1800 (2500 - 700) in the first battle; the second cost team 1 750 (3250 - 2500).
    assert_eq!(s.lp(1), 7250, "3250 against 2500: {}", s.context());
    assert_eq!(s.lp(0), 6200, "700 against 2500: {}", s.context());
}

/// Bounce Spell: "Target 1 face-up Spell your opponent controls; take control it and activate the effect of it." The
/// opponent's face-up Dark Hole comes over and destroys every monster.
#[test]
fn bounce_spell_takes_a_spell_and_activates_its_effect() {
    let spawns = [hand(0, BOUNCE_SPELL), spell(1, DARK_HOLE), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(BOUNCE_SPELL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "Dark Hole's effect ran: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 0, "{}", s.context());
    assert_eq!(s.count(1, "szone", DARK_HOLE), 0, "Dark Hole no longer sits on their side: {}", s.context());
}

/// Bounce Spell resolves with nothing when its target is gone (the override's fix, the per-card fuzz's seed 59357): the
/// opponent answers with Mystical Space Typhoon on their own Dark Hole.
#[test]
fn bounce_spell_does_nothing_when_its_target_was_destroyed_in_response() {
    let spawns = [hand(0, BOUNCE_SPELL), spell(1, DARK_HOLE), set(1, MST), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, DARK_HOLE)]);
    // Typhoon is chained at the first chance after Bounce Spell is on the chain (an earlier window comes at the turn's start).
    let listed = |p: &serde_json::Value| p["selects"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == MST));
    loop {
        s.run_to(&[Activate(BOUNCE_SPELL)], |p| p["type"] == msg::SELECT_CHAIN && p["player"] == 1 && listed(p).is_some()).unwrap();
        if s.count(0, "szone", BOUNCE_SPELL) == 1 {
            let index = listed(s.prompt()).unwrap();
            s.respond(Response::Chain(Some(index)).json()).unwrap();
            break;
        }
        s.pass().unwrap();
    }
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "Typhoon destroyed it: {}", s.context());
    assert_eq!(s.count(0, "grave", BOUNCE_SPELL), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "nothing was destroyed on the field: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
}

/// Magical Pigeon: "Return 1 "Dark Magician Girl" you control to your hand; Special Summon 2 "Pigeon Tokens" ... in
/// Defense Position." Dark Magician Girl goes to the hand; two Pigeon Tokens stand in Defense Position.
#[test]
fn magical_pigeon_returns_the_girl_and_summons_two_tokens() {
    let Some(mut s) = field(&[], &[hand(0, MAGICAL_PIGEON), mon(0, DARK_MAGICIAN_GIRL)]) else { return };
    s.run(&[Activate(MAGICAL_PIGEON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN_GIRL), 1, "back in the hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN_GIRL), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", PIGEON_TOKEN), 2, "two Pigeon Tokens: {}", s.context());
    assert_eq!(s.position_of(0, PIGEON_TOKEN), Some(4), "in Defense Position: {}", s.context());
    assert_eq!(s.level_of(0, PIGEON_TOKEN), Some(1), "{}", s.context());
    assert_eq!(s.attack_of(0, PIGEON_TOKEN), Some(0), "{}", s.context());
}

/// Magical Pigeon: "also, during the End Phase of this turn, you can destroy all "Pigeon Tokens" you control and Special
/// Summon 1 "Dark Magician Girl" from your hand." At the End Phase the tokens go and Dark Magician Girl is back.
#[test]
fn magical_pigeon_swaps_the_tokens_for_the_girl_in_the_end_phase() {
    let Some(mut s) = field(&[], &[hand(0, MAGICAL_PIGEON), mon(0, DARK_MAGICIAN_GIRL)]) else { return };
    s.run(&[Activate(MAGICAL_PIGEON), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PIGEON_TOKEN), 0, "the tokens were destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN_GIRL), 1, "the Girl is Special Summoned from the hand: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN_GIRL), 0, "{}", s.context());
}

/// Necro Wall: "Once per turn, during your End Phase: You can Special Summon 1 "Necro Wall Token" ... for each Zombie-Type
/// monster on the field, except this card, but it cannot declare an attack." Zombie Master (mine) and Pyramid Turtle
/// (theirs) give two tokens.
#[test]
fn necro_wall_summons_a_token_for_each_other_zombie_on_the_field() {
    let spawns = [mon(0, NECRO_WALL), mon(0, ZOMBIE_MASTER), mon(1, PYRAMID_TURTLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[EndTurn, Chain(NECRO_WALL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", NECRO_WALL_TOKEN), 2, "one for each other Zombie, theirs included: {}", s.context());
    assert_eq!(s.level_of(0, NECRO_WALL_TOKEN), Some(1), "{}", s.context());
    assert_eq!(s.attack_of(0, NECRO_WALL_TOKEN), Some(0), "{}", s.context());
}

/// Necro Wall: with no other Zombie on the field the End Phase effect gives nothing.
#[test]
fn necro_wall_alone_summons_no_token() {
    let Some(mut s) = field(&[], &[mon(0, NECRO_WALL), mon(0, WARWOLF)]) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", NECRO_WALL_TOKEN), 0, "{}", s.context());
}

/// Necro Wall: the token "cannot declare an attack": in the next own turn a Zombie Master can attack, the token cannot.
#[test]
fn necro_wall_tokens_cannot_attack() {
    let spawns = [mon(0, NECRO_WALL), mon(0, ZOMBIE_MASTER), mon(1, PYRAMID_TURTLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[EndTurn, Chain(NECRO_WALL), Attack(NECRO_WALL_TOKEN)]).expect_err("the token cannot attack");
    assert!(err.contains("can't attack"), "{err}");
    assert!(s.count(0, "mzone", NECRO_WALL_TOKEN) >= 1, "the token exists: {}", s.context());
}

/// Awakening from Beyond: "Your opponent draws 2 cards; add 1 monster from your Graveyard to your hand."
#[test]
fn awakening_from_beyond_draws_two_for_the_opponent_and_recovers_a_monster() {
    let Some(mut s) = field(&[], &[hand(0, AWAKENING), grave(0, WARWOLF)]) else { return };
    let theirs = hand_size(&s, 1);
    s.run(&[Activate(AWAKENING)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 1), theirs + 2, "the opponent drew 2: {}", s.context());
    assert_eq!(s.count(0, "hand", WARWOLF), 1, "Warwolf came back from the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 0, "{}", s.context());
}

/// Dark Spear: "Equip only to "Dark Magician". If it attacks a Defense Position monster, inflict piercing battle damage
/// to your opponent." Dark Magician (2500) attacks Warwolf in Defense (100): 2400 pierces through.
#[test]
fn dark_spear_gives_dark_magician_piercing_damage() {
    let spawns = [hand(0, DARK_SPEAR), mon(0, DARK_MAGICIAN), mon_def(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(DARK_SPEAR), Attack(DARK_MAGICIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", DARK_SPEAR), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2400, "piercing damage 2500 - 100: {}", s.context());
}

/// Dark Spear: "Equip only to "Dark Magician"": a Warwolf cannot wear it.
#[test]
fn dark_spear_equips_only_to_dark_magician() {
    let Some(mut s) = field(&[], &[hand(0, DARK_SPEAR), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(DARK_SPEAR)]).expect_err("no Dark Magician");
    assert!(err.contains("Activate(5189) never became possible"), "{err}");
    clean(&s);
}

/// Summoning Clock: "Activate only when your opponent activates a Spell/Trap Card. Once per turn, during your Standby
/// Phase, place 1 Turn Counter on this card. You can Tribute this card and 1 monster; Special Summon monsters from your
/// hand equal to the number of Turn Counters that were on this card." Chained to Dian Keto in turn 2, it has one counter
/// at turn 3's Standby Phase; Tributed with Thunder Kid it Special Summons one monster from the hand.
#[test]
fn summoning_clock_special_summons_one_monster_per_turn_counter() {
    let spawns = [set(0, SUMMONING_CLOCK), mon(0, THUNDER_KID), hand(0, WARWOLF), hand(1, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    // Team 0 draws in turn 1 and discards down to 6 at its end: a filler, not Warwolf.
    s.prefer(&[(0, FILLER)]);
    s.run_both([&[Chain(SUMMONING_CLOCK)], &[Activate(DIAN_KETO)]]).unwrap();
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(SUMMONING_CLOCK)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 9000, "Dian Keto resolved: {}", s.context());
    assert_eq!(s.count(0, "grave", SUMMONING_CLOCK), 1, "the Clock was Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "with Thunder Kid: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "one counter, one monster from the hand: {}", s.context());
    assert_eq!(s.count(0, "hand", WARWOLF), 0, "{}", s.context());
}

/// Summoning Clock: "Activate only when your opponent activates a Spell/Trap Card": my own Spell does not offer it.
#[test]
fn summoning_clock_is_not_chained_to_the_owners_own_spell() {
    let spawns = [set(0, SUMMONING_CLOCK), hand(0, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(DIAN_KETO), Chain(SUMMONING_CLOCK)]).expect_err("not offered");
    assert!(err.contains("Chain(5240) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", SUMMONING_CLOCK), 1, "still Set: {}", s.context());
}

/// Limit Tribute: "Both players can only Tribute 1 monster per turn." After Battle Steer is Tribute Summoned on Thunder
/// Kid, Mystik Wok has nothing left to Tribute this turn.
#[test]
fn limit_tribute_blocks_a_second_tribute_in_the_same_turn() {
    let spawns = [set(0, LIMIT_TRIBUTE), hand(0, BATTLE_STEER), hand(0, MYSTIK_WOK), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(LIMIT_TRIBUTE), Summon(BATTLE_STEER), Activate(MYSTIK_WOK)]).expect_err("a second Tribute");
    assert!(err.contains("Activate(5978) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", BATTLE_STEER), 1, "the Tribute Summon was the one Tribute: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
}

/// Limit Tribute, the control: with no Limit Tribute the same turn's Mystik Wok Tributes Battle Steer for LP.
#[test]
fn without_limit_tribute_a_second_tribute_goes_through() {
    let spawns = [hand(0, BATTLE_STEER), hand(0, MYSTIK_WOK), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Summon(BATTLE_STEER), Activate(MYSTIK_WOK)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BATTLE_STEER), 1, "Battle Steer was Tributed to Mystik Wok: {}", s.context());
    assert!(s.lp(0) > 8000, "and its LP was gained: {}", s.context());
}

/// Limit Tribute: "Both players can only Tribute 1 monster per turn": a Level 7 Tribute Summon needs 2 Tributes at once,
/// which is more than 1 monster.
#[test]
fn limit_tribute_blocks_a_two_tribute_summon() {
    let spawns = [set(0, LIMIT_TRIBUTE), hand(0, DARK_MAGICIAN), mon(0, THUNDER_KID), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let r = s.run(&[Activate(LIMIT_TRIBUTE), Summon(DARK_MAGICIAN)]);
    assert!(r.is_err(), "Dark Magician needs 2 Tributes and was summoned: {}", s.context());
}

/// Nightmare Binding: "Target 1 monster your opponent controls; it loses 800 ATK, then, gain 800 LP ..."
#[test]
fn nightmare_binding_takes_800_atk_and_gives_800_lp() {
    let Some(mut s) = field(&[], &[hand(0, NIGHTMARE_BINDING), mon(1, WARWOLF)]) else { return };
    s.run(&[Activate(NIGHTMARE_BINDING)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, WARWOLF), Some(1200), "{}", s.context());
    assert_eq!(s.lp(0), 8800, "{}", s.context());
}

/// Nightmare Binding: "... and it cannot be Tributed." Battle Steer needs a Tribute and the bound Warwolf is the only one
/// the opponent has.
#[test]
fn nightmare_binding_makes_the_monster_untributable() {
    let spawns = [hand(0, NIGHTMARE_BINDING), mon(1, WARWOLF), hand(1, BATTLE_STEER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[Activate(NIGHTMARE_BINDING)], &[Summon(BATTLE_STEER)]]).expect_err("nothing to Tribute");
    assert!(err.contains("Summon(4020) never became possible"), "{err}");
    clean(&s);
}

/// Nightmare Binding, the control: Warwolf unbound is Tributed for Battle Steer.
#[test]
fn an_unbound_monster_is_tributed_for_a_tribute_summon() {
    let spawns = [mon(1, WARWOLF), hand(1, BATTLE_STEER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[], &[Summon(BATTLE_STEER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", BATTLE_STEER), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Nightmare Binding: "... its effects cannot be activated". Zombie Master's ignition effect is no longer on offer.
#[test]
fn nightmare_binding_stops_the_monsters_effects() {
    let spawns = [hand(0, NIGHTMARE_BINDING), mon(1, ZOMBIE_MASTER), grave(1, PYRAMID_TURTLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[Activate(NIGHTMARE_BINDING)], &[Activate(ZOMBIE_MASTER)]]).expect_err("no effect");
    assert!(err.contains("Activate(7215) never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(1, "grave", PYRAMID_TURTLE), 1, "{}", s.context());
}

/// Nightmare Binding, the control: Zombie Master unbound revives Pyramid Turtle.
#[test]
fn an_unbound_zombie_master_revives_a_zombie() {
    let spawns = [mon(1, ZOMBIE_MASTER), grave(1, PYRAMID_TURTLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, PYRAMID_TURTLE)]);
    s.run_both([&[], &[Activate(ZOMBIE_MASTER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", PYRAMID_TURTLE), 1, "{}", s.context());
}

/// Self Tribute: "If a monster(s) you control would be destroyed: Pay 1000 LP; that monster(s) cannot be destroyed this
/// turn." Warwolf (2000) attacks Thunder Kid (700): it survives, but the 1300 damage and the 1000 LP are paid.
#[test]
fn self_tribute_saves_a_monster_from_battle_destruction() {
    let spawns = [set(0, SELF_TRIBUTE), mon(0, THUNDER_KID), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Chain(SELF_TRIBUTE)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid was not destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 0, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000 - 1300, "1000 paid and 1300 battle damage: {}", s.context());
    assert_eq!(s.count(0, "grave", SELF_TRIBUTE), 1, "{}", s.context());
}

/// Self Tribute: it also answers an effect that would destroy: the opponent's Dark Hole destroys Warwolf, not Thunder Kid.
#[test]
fn self_tribute_saves_a_monster_from_a_destroying_effect() {
    let spawns = [set(0, SELF_TRIBUTE), mon(0, THUNDER_KID), mon(1, WARWOLF), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Chain(SELF_TRIBUTE)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid is indestructible this turn: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "their own monster was destroyed: {}", s.context());
    assert_eq!(s.lp(0), 7000, "{}", s.context());
}

/// Counterbalance: "Once per turn, during the End Phase: The turn player must send cards from the top of their Deck to
/// the Graveyard, equal to the total number of monsters that are banished and on the field." Warwolf, Thunder Kid and a
/// banished Warwolf are 3: team 0's own End Phase sends 3 cards.
#[test]
fn counterbalance_mills_the_turn_player_by_the_monsters_on_the_field_and_banished() {
    let spawns = [hand(0, COUNTERBALANCE), mon(0, WARWOLF), mon(1, THUNDER_KID), banished(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let deck = s.state()[0]["deck"].as_i64().unwrap();
    s.run(&[Activate(COUNTERBALANCE), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.state()[0]["deck"], deck - 3, "three cards off the top of the turn player's Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 3, "{}", s.context());
}

/// Counterbalance: "the turn player" includes the opponent: in their End Phase their Deck loses the same number.
#[test]
fn counterbalance_works_on_the_opponents_end_phase_too() {
    let spawns = [hand(0, COUNTERBALANCE), mon(0, WARWOLF), mon(1, THUNDER_KID), banished(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Activate(COUNTERBALANCE), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", FILLER), 3, "their End Phase milled them: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 3, "and mine did earlier: {}", s.context());
}
