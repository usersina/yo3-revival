//! Anime-only cards, group 4 (rows 5334-5366 of engine/added/cards.tsv): Noah Kaiba's, Zigfried von Schroeder's, Solomon
//! Muto's and Vivian Wong's cards. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap here is Set first (a Trap cannot be
//! activated from the hand); neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! Covered: the Tokens (Stone Giant, made by Ancient Key; Spirit, made by Vessel of Illusion), the cards the per-card fuzz
//! flagged (Flaming Fist, Ground Breaking, Vessel of Illusion, Enchanted Sword Nothung, Magical Alms, Nibelung's Ring,
//! Wotan's Judgment, Ancient City, Ancient Gate, Ancient Key, Dragon Lady, Luminous Clouds, Snowfall Sword), the signature
//! cards (Next World, Last Tusk Mammoth, Nibelung's Treasure, Ancient Dragon, The Legendary Gambler) and a sample of the
//! rest (Ancient Giant, Fricka's Mediation, Giant Flood, Goddess Erda's Guidance), and two regressions of fixes made for
//! this batch: The Legendary Gambler against an opponent's die roll, and Take One Chance picking Dark Magic Retribution
//! outside an attack. Deepest Impact is in `infinite_lp.rs` (at ∞ LP). Nibelung's Ring has an override
//! (`engine/overrides/c511000271.lua`: it returns to the Deck when its monster is destroyed by battle).
//! Not covered here: Chaos Barrier Field, Ice Age Panic, Swan Maiden, Ancient Tome, Flying Dragon Whirl, Lightning Saber,
//! Mystic Eruption and Kung Fu Nyan Nyan's own effect (outside this group's sample of the batch).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const ASURA_PRIEST: u16 = 5381; // a Spirit monster, Level 4, 1700/1200
const LUSTER_DRAGON: u16 = 5644; // Dragon, Level 4, 1900/1600 vanilla
const BLUE_EYES: u16 = 4007; // Dragon, Level 8, 3000/2500 vanilla
const LEGENDARY_SWORD: u16 = 4307; // Equip only to a Warrior: +300 ATK/DEF
const MAGIC_CYLINDER: u16 = 5124;
const MONSTER_REBORN: u16 = 4842;

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
// Flaming Fist, Ground Breaking and Vessel of Illusion (Spirit monsters)
// ---------------------------------------------------------------------------------------------------------------

/// Flaming Fist: "Target 1 Spirit monster you control; it gains 200 ATK until the End Phase, also, if it inflicts battle
/// damage to your opponent this turn, gain 500 LP." Team 1's Asura Priest (a Spirit, 1700) gains 200 and attacks
/// directly in turn 2: 1900 damage, and its controller gains 500.
#[test]
fn flaming_fist_gives_a_spirit_200_atk_and_500_lp_for_battle_damage() {
    let fist = card("Flaming Fist");
    let Some(mut s) = field(&[], &[hand(1, fist), mon(1, ASURA_PRIEST)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(fist)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, ASURA_PRIEST), Some(1900), "+200 ATK: {}", s.context());
    s.run_both([&[], &[Attack(ASURA_PRIEST)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1900, "the attack with 1900 ATK: {}", s.context());
    assert_eq!(s.lp(1), 8500, "500 LP for the battle damage: {}", s.context());
    assert_eq!(s.count(1, "grave", fist), 1, "{}", s.context());
}

/// Flaming Fist: the 500 LP need battle damage to the opponent: Asura Priest (1900) attacking Warwolf (2000) is destroyed
/// and its controller takes 100, with no LP gained.
#[test]
fn flaming_fist_gives_no_lp_without_battle_damage_to_the_opponent() {
    let fist = card("Flaming Fist");
    let Some(mut s) = field(&[], &[hand(1, fist), mon(1, ASURA_PRIEST), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(fist), Attack(ASURA_PRIEST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", ASURA_PRIEST), 1, "Warwolf won the battle: {}", s.context());
    assert_eq!(s.lp(1), 7900, "100 damage and nothing gained: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Flaming Fist: "Target 1 Spirit monster": Warwolf is not one, so the card is not offered.
#[test]
fn flaming_fist_needs_a_spirit_monster() {
    let fist = card("Flaming Fist");
    let Some(mut s) = field(&[], &[hand(0, fist), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(fist)]).expect_err("no Spirit");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Ground Breaking: "Add 1 Spirit monster from your Graveyard to your hand." Asura Priest comes back, Warwolf (not a
/// Spirit) stays in the Graveyard.
#[test]
fn ground_breaking_adds_a_spirit_from_the_graveyard_to_the_hand() {
    let breaking = card("Ground Breaking");
    let Some(mut s) = field(&[], &[set(0, breaking), grave(0, ASURA_PRIEST), grave(0, WARWOLF)]) else { return };
    s.run(&[Activate(breaking)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", ASURA_PRIEST), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", ASURA_PRIEST), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", breaking), 1, "{}", s.context());
}

/// Ground Breaking: with no Spirit in the Graveyard (only Warwolf) the card is not offered.
#[test]
fn ground_breaking_needs_a_spirit_in_the_graveyard() {
    let breaking = card("Ground Breaking");
    let Some(mut s) = field(&[], &[set(0, breaking), grave(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(breaking)]).expect_err("no Spirit");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Vessel of Illusion: "When a Spirit monster you control is destroyed and sent to the Graveyard: Special Summon 1 "Spirit
/// Token" that has the same original Level, Type, Attribute, ATK, and DEF as the destroyed monster." Dark Hole destroys
/// Asura Priest (Level 4, Fairy, LIGHT, 1700/1200): the Token has those.
#[test]
fn vessel_of_illusion_summons_a_spirit_token_like_the_destroyed_spirit() {
    let (vessel, token) = (card("Vessel of Illusion"), card("Spirit Token"));
    let Some(mut s) = field(&[], &[set(0, vessel), mon(0, ASURA_PRIEST), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(vessel)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ASURA_PRIEST), 1, "Asura Priest was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 1, "the Spirit Token came: {}", s.context());
    let t = s.monster(0, token).unwrap();
    let priest = s.state()[0]["grave"].as_array().unwrap().iter().find(|c| c["cid"] == ASURA_PRIEST).unwrap().clone();
    assert_eq!((t["attack"].as_i64(), t["defense"].as_i64(), t["level"].as_i64()), (Some(1700), Some(1200), Some(4)), "{t}");
    assert_eq!(t["race"], priest["race"], "the same Type: {t} {priest}");
    assert_eq!(t["attribute"], priest["attribute"], "the same Attribute: {t} {priest}");
    assert_eq!(s.count(0, "grave", vessel), 1, "{}", s.context());
}

/// Vessel of Illusion: "a Spirit monster": Warwolf destroyed by Dark Hole gives no Token (and the Trap is not offered).
#[test]
fn vessel_of_illusion_ignores_a_monster_that_is_not_a_spirit() {
    let (vessel, token) = (card("Vessel of Illusion"), card("Spirit Token"));
    let Some(mut s) = field(&[], &[set(0, vessel), mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(vessel)], &[Activate(DARK_HOLE)]]).expect_err("no Spirit destroyed");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Ancient Key and Stone Giant Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Ancient Key: "When you activate this card: Special Summon 2 "Stone Giant Tokens" (Rock-Type/EARTH/Level 3/ATK 400/DEF
/// 2000) to your field. If an Attack Position "Stone Giant" does not attack, its controller takes 500 damage at the end of
/// their turn." Two Tokens, and 1000 damage at the end of turn 1 (they stay in Attack Position and never attack).
#[test]
fn ancient_key_summons_two_stone_giant_tokens_that_hurt_their_controller() {
    let (key, token) = (card("Ancient Key"), card("Stone Giant Token"));
    let Some(mut s) = field(&[], &[hand(0, key)]) else { return };
    s.run(&[Activate(key)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 2, "two Tokens: {}", s.context());
    assert_eq!(s.count(0, "szone", key), 1, "the Key stays on the field: {}", s.context());
    let t = s.monster(0, token).unwrap();
    assert_eq!((t["attack"].as_i64(), t["defense"].as_i64(), t["level"].as_i64()), (Some(400), Some(2000), Some(3)), "{t}");
    assert_eq!((t["race"].as_u64(), t["attribute"].as_u64()), (Some(0x100), Some(1)), "Rock and EARTH: {t}");
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000, "500 damage for each Token that did not attack: {}", s.context());
}

/// Stone Giant Token: "Cannot be Tributed for a Tribute Summon." With two Tokens and Dark Magician (Level 7, 2 Tributes)
/// in the hand it is not offered for a Summon; with two Thunder Kids instead it is.
#[test]
fn stone_giant_tokens_cannot_be_tributed_for_a_tribute_summon() {
    let (key, token) = (card("Ancient Key"), card("Stone Giant Token"));
    let Some(mut s) = field(&[], &[hand(0, key), hand(0, DARK_MAGICIAN)]) else { return };
    s.run(&[Activate(key)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 2, "{}", s.context());
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "{}", s.context());
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "no Tribute Summon: {:?}", offered(&s, "summons"));
    // The control: two ordinary monsters.
    let spawns = [hand(0, DARK_MAGICIAN), mon(0, THUNDER_KID), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "a Tribute Summon: {:?}", offered(&s, "summons"));
}

/// Stone Giant Token: a Token that attacked takes no part in that damage. Turn 1's End Phase costs both Tokens 500 each; in
/// turn 3 one Token attacks Thunder Kid (400 against 700: 300 damage) and only the one that stayed home costs 500.
#[test]
fn a_stone_giant_token_that_attacked_does_not_hurt_its_controller() {
    let (key, token) = (card("Ancient Key"), card("Stone Giant Token"));
    let Some(mut s) = field(&[], &[hand(0, key), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(key), EndTurn, Attack(token)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000 - 300, "turn 1's damage and the lost battle: {}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000 - 300 - 500, "500 for the Token that stayed home only: {}", s.context());
}

/// Ancient Key: "You can Tribute this card face-up and 2 "Stone Giants" you control whose battle positions have been
/// changed twice each; place 1 "Ancient Gate" from your hand, Deck or Graveyard face-up in your Spell & Trap Zone." The
/// Tokens cannot change position the turn they are Special Summoned (and cost 1000 at that End Phase): they go to Defense
/// in turn 3 and back to Attack in turn 5, and then the Key, the Tokens and the Gate (from the Deck) trade.
#[test]
fn ancient_key_places_ancient_gate_after_two_position_changes() {
    let (key, token, gate) = (card("Ancient Key"), card("Stone Giant Token"), card("Ancient Gate"));
    let Some(mut s) = field(&[gate], &[hand(0, key)]) else { return };
    keep_hands(&mut s);
    let plan = [Activate(key), EndTurn, Reposition(token), Reposition(token), EndTurn, Reposition(token), Reposition(token)];
    s.run_both([&plan, &[EndTurn, EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 2, "{}", s.context());
    assert_eq!(s.position_of(0, token), Some(1), "back in Attack Position: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000, "only turn 1's End Phase cost anything: {}", s.context());
    s.run(&[Activate(key)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", gate), 1, "the Gate is on the field: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 0, "the Tokens were Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", key), 1, "and the Key: {}", s.context());
}

/// Ancient Key: the Tokens' positions "changed twice each": after one change only, the effect is not offered.
#[test]
fn ancient_key_needs_two_position_changes_of_each_token() {
    let (key, token, gate) = (card("Ancient Key"), card("Stone Giant Token"), card("Ancient Gate"));
    let Some(mut s) = field(&[gate], &[hand(0, key)]) else { return };
    keep_hands(&mut s);
    let plan = [Activate(key), EndTurn, Reposition(token), Reposition(token), EndTurn, Activate(key)];
    let err = s.run_both([&plan, &[]]).expect_err("one change each");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", gate), 0, "{}", s.context());
}

/// Ancient Gate: "Cannot be placed on the field, except by "Ancient Key"." In the hand it is on offer neither to Set nor to
/// activate.
#[test]
fn ancient_gate_cannot_be_set_or_activated() {
    let gate = card("Ancient Gate");
    let Some(mut s) = field(&[], &[hand(0, gate)]) else { return };
    s.run(&[]).unwrap();
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "{}", s.context());
    assert!(!offered(&s, "spell_sets").contains(&gate.into()), "no Set: {:?}", offered(&s, "spell_sets"));
    assert!(!offered(&s, "activates").contains(&gate.into()), "no activation: {:?}", offered(&s, "activates"));
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Ancient Giant, Ancient Dragon and Ancient City
// ---------------------------------------------------------------------------------------------------------------

/// Ancient Giant: "Once per turn, during your End Phase, if this card did not attack an opponent's monster this turn:
/// Take 300 damage." It takes 300 at the end of turn 1 and nothing in the opponent's End Phase.
#[test]
fn ancient_giant_hurts_its_controller_in_their_end_phase() {
    let giant = card("Ancient Giant");
    let Some(mut s) = field(&[], &[mon(0, giant)]) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7700, "300 damage in my End Phase: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7700, "none in the opponent's End Phase: {}", s.context());
}

/// Ancient Giant: attacking an opponent's monster (Thunder Kid, 700) in turn 3 spares it the damage that turn (but not an
/// attack on the opponent directly: with no monster to attack the 300 damage is taken).
#[test]
fn ancient_giant_takes_no_damage_when_it_attacked_a_monster() {
    let giant = card("Ancient Giant");
    let Some(mut s) = field(&[], &[mon(0, giant), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(giant), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1500, "2200 against 700: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 300, "only turn 1's damage: {}", s.context());
    // Without a monster to attack: a direct attack is no attack on a monster.
    let Some(mut s) = field(&[], &[mon(0, giant)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(giant), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2200, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 300 - 300, "turn 1's and turn 3's: {}", s.context());
}

/// Ancient Dragon: "If this card destroys an opponent's monster by battle: Destroy all Defense Position monsters your
/// opponent controls." It (2800) destroys Warwolf (2000, Attack Position) in turn 3; Thunder Kid (face-up Defense) and a
/// Set Dark Magician go with it, and the Dragon survives.
#[test]
fn ancient_dragon_destroys_the_defense_position_monsters_after_a_battle_kill() {
    let dragon = card("Ancient Dragon (Anime)");
    let spawns = [mon(0, dragon), mon(1, WARWOLF), mon_set(1, DARK_MAGICIAN), mon_def(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(dragon)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "destroyed by the battle: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "the Set monster: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "the face-up Defense Position monster: {}", s.context());
    assert_eq!(s.count(0, "mzone", dragon), 1, "{}", s.context());
}

/// Ancient Dragon, the other half: a battle that destroys nothing leaves the Defense Position monsters alone. The Dragon
/// (2800) is attacked by Blue-Eyes (3000), which destroys it, so a Set Thunder Kid stays.
#[test]
fn ancient_dragon_leaves_the_defense_position_monsters_when_it_destroys_nothing() {
    let dragon = card("Ancient Dragon (Anime)");
    let spawns = [mon(0, dragon), mon(1, BLUE_EYES), mon_set(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    // The opponent's Blue-Eyes attacks the Dragon in turn 2 and destroys it: nothing of the Dragon's effect applies.
    s.run_both([&[], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", dragon), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "the Defense Position monster stays: {}", s.context());
}

/// Ancient Dragon: "destroys an opponent's monster by battle" applies to a Defense Position monster too: it (2800) attacks a
/// Set Dark Magician (2100 DEF) and destroys it, and the other Set monster goes as well.
#[test]
fn ancient_dragon_also_counts_a_defense_position_kill() {
    let dragon = card("Ancient Dragon (Anime)");
    let spawns = [mon(0, dragon), mon_set(1, DARK_MAGICIAN), mon_set(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(dragon)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN) + s.count(1, "grave", THUNDER_KID), 2, "both were destroyed: {}", s.context());
}

/// Ancient City: "If you control a "Ancient Gate", "Ancient Tome" and "Ancient Giant": You can Tribute them; Special Summon
/// 1 "Ancient Dragon" from your hand, Deck or Graveyard." The three are Tributed (a Spell and a monster too) and the Dragon
/// comes from the Deck.
#[test]
fn ancient_city_tributes_the_three_for_ancient_dragon() {
    let [city, gate, tome, giant, dragon] =
        ["Ancient City", "Ancient Gate", "Ancient Tome", "Ancient Giant", "Ancient Dragon (Anime)"].map(card);
    let spawns = [spell(0, city), spell(0, gate), spell(0, tome), mon(0, giant)];
    let Some(mut s) = field(&[dragon], &spawns) else { return };
    s.run(&[Activate(city)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dragon), 1, "Ancient Dragon is on the field: {}", s.context());
    assert_eq!(s.count(0, "grave", gate) + s.count(0, "grave", tome) + s.count(0, "grave", giant), 3, "{}", s.context());
    assert_eq!(s.count(0, "szone", city), 1, "the Field Spell stays: {}", s.context());
}

/// Ancient City: "If an "Ancient Dragon" you control is destroyed: Special Summon it during the End Phase." Dark Hole
/// destroys the Dragon in turn 2; at that End Phase it is back on team 0's field.
#[test]
fn ancient_city_brings_a_destroyed_ancient_dragon_back_in_the_end_phase() {
    let (city, dragon) = (card("Ancient City"), card("Ancient Dragon (Anime)"));
    let Some(mut s) = field(&[], &[spell(0, city), mon(0, dragon), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", dragon), 1, "destroyed: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", dragon), 1, "back in the End Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", dragon), 0, "{}", s.context());
}

/// Ancient City: "If you control" all three: with the Gate missing the effect is not offered.
#[test]
fn ancient_city_needs_the_gate_the_tome_and_the_giant() {
    let [city, tome, giant, dragon] = ["Ancient City", "Ancient Tome", "Ancient Giant", "Ancient Dragon (Anime)"].map(card);
    let Some(mut s) = field(&[dragon], &[spell(0, city), spell(0, tome), mon(0, giant)]) else { return };
    let err = s.run(&[Activate(city)]).expect_err("no Gate");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Nothung, Snowfall Sword, Wotan's Judgment, Fricka's Mediation
// ---------------------------------------------------------------------------------------------------------------

/// Enchanted Sword Nothung: "The equipped monster gains 400 ATK. When you equip this card, destroy and banish all Dragon-Type
/// monsters your opponent controls." Dark Blade (1800) gets 2200, Blue-Eyes and Luster Dragon are banished (not sent to
/// the Graveyard), Warwolf stays.
#[test]
fn nothung_banishes_the_opponents_dragons_when_equipped() {
    let nothung = card("Enchanted Sword Nothung");
    let spawns = [mon(0, DARK_BLADE), hand(0, nothung), mon(1, BLUE_EYES), mon(1, LUSTER_DRAGON), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(nothung)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(2200), "+400 ATK: {}", s.context());
    assert_eq!(s.count(1, "removed", BLUE_EYES) + s.count(1, "removed", LUSTER_DRAGON), 2, "banished: {}", s.context());
    assert_eq!(s.count(1, "grave", BLUE_EYES) + s.count(1, "grave", LUSTER_DRAGON), 0, "not in the Graveyard: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", nothung), 1, "{}", s.context());
}

/// Enchanted Sword Nothung: "Before damage calculation, if the equipped monster battles a Dragon-Type monster: Destroy that
/// monster." Dark Blade (2200 equipped) is attacked by a Luster Dragon Summoned after the Sword (1900): it is destroyed
/// before damage, so neither player takes damage (without the effect the Dragon would lose 300 to Dark Blade).
#[test]
fn nothung_destroys_a_dragon_that_attacks_the_equipped_monster() {
    let nothung = card("Enchanted Sword Nothung");
    let spawns = [mon(0, DARK_BLADE), hand(0, nothung), hand(1, LUSTER_DRAGON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(nothung)], &[Summon(LUSTER_DRAGON), Attack(LUSTER_DRAGON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", LUSTER_DRAGON), 1, "destroyed before damage: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
}

/// Enchanted Sword Nothung, the same on the equipped monster's own attack: Dark Blade attacks a Luster Dragon that came after
/// the Sword in turn 2 and survives in turn 3 with 0 damage dealt (the Dragon is destroyed before the damage step).
#[test]
fn nothung_destroys_a_dragon_the_equipped_monster_attacks() {
    let nothung = card("Enchanted Sword Nothung");
    let spawns = [mon(0, DARK_BLADE), hand(0, nothung), hand(1, LUSTER_DRAGON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(nothung), EndTurn, Attack(DARK_BLADE)], &[SetMonster(LUSTER_DRAGON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", LUSTER_DRAGON), 1, "destroyed at the start of the battle: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no damage: {}", s.context());
}

/// Snowfall Sword: "When your opponent activates a Spell/Trap Card while a Warrior-Type monster you control that is equipped
/// with an Equip Spell is attacking: Negate its activation, and if you do, destroy it." Team 1's Magic Cylinder is
/// negated and destroyed, so Dark Blade (equipped with Legendary Sword: 2100) attacks Warwolf and destroys it.
#[test]
fn snowfall_sword_negates_and_destroys_a_spell_or_trap_against_an_equipped_warrior() {
    let snowfall = card("Snowfall Sword");
    let spawns = [set(0, snowfall), mon(0, DARK_BLADE), hand(0, LEGENDARY_SWORD), set(1, MAGIC_CYLINDER), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(LEGENDARY_SWORD), Attack(DARK_BLADE), Chain(snowfall)], &[Chain(MAGIC_CYLINDER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MAGIC_CYLINDER), 1, "destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", snowfall), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "no 2100 damage from the Cylinder: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the attack went on: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 100, "2100 against 2000: {}", s.context());
}

/// Snowfall Sword: "equipped with an Equip Spell": the same attack by an unequipped Warrior gives no chance for it.
#[test]
fn snowfall_sword_needs_an_equipped_warrior() {
    let snowfall = card("Snowfall Sword");
    let spawns = [set(0, snowfall), mon(0, DARK_BLADE), set(1, MAGIC_CYLINDER), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s
        .run_both([&[Attack(DARK_BLADE), Chain(snowfall)], &[Chain(MAGIC_CYLINDER)]])
        .expect_err("no Equip Spell on the attacker");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", snowfall), 1, "still Set: {}", s.context());
    assert_eq!(s.count(1, "grave", MAGIC_CYLINDER), 1, "the Cylinder was activated and resolved: {}", s.context());
}

/// Wotan's Judgment: "When an opponent's monster declares an attack: Switch the top card of your Deck with a Spell in your
/// hand, then shuffle your Deck; negate the attack and end the Battle Phase." Dian Keto goes into the Deck, the top card
/// (a Feral Imp) into the hand; the attack of Warwolf is negated, and the opponent is in its Main Phase 2: Thunder Kid cannot attack.
#[test]
fn wotans_judgment_switches_a_spell_with_the_top_card_and_ends_the_battle_phase() {
    let wotan = card("Wotan's Judgment");
    let spawns = [set(0, wotan), hand(0, DIAN_KETO), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let deck_before = deck_size(&s, 0);
    s.run_both([&[ChainOnAttack(wotan)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(msg::SELECT_IDLECMD), Some(1)), "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over (Thunder Kid cannot attack): {}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", wotan), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", DIAN_KETO), 0, "Dian Keto went to the Deck: {}", s.context());
    assert_eq!(hand_size(&s, 0), 6, "six after the End Phase's discard, one card out and one in: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck_before, "{}", s.context());
}

/// Wotan's Judgment: "a Spell in your hand": without one the Trap is not offered and the attack lands.
#[test]
fn wotans_judgment_needs_a_spell_in_the_hand() {
    let wotan = card("Wotan's Judgment");
    let Some(mut s) = field(&[], &[set(0, wotan), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(wotan)], &[Attack(WARWOLF)]]).expect_err("no Spell to switch");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "the attack landed: {}", s.context());
}

/// Fricka's Mediation: "Banish 1 "Wotan's Judgment" from your Graveyard; you take no battle damage this turn." Chained to
/// Warwolf's attack on Thunder Kid (700): Thunder Kid dies and its controller takes no damage; Wotan's Judgment is banished.
#[test]
fn fricka_s_mediation_banishes_wotans_judgment_and_prevents_battle_damage() {
    let (fricka, wotan) = (card("Fricka's Mediation"), card("Wotan's Judgment"));
    let spawns = [set(0, fricka), grave(0, wotan), mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(fricka)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", wotan), 1, "banished as the cost: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the battle still destroyed it: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
}

/// Fricka's Mediation, the control: without the Trap Thunder Kid's loss costs 1300 LP.
#[test]
fn without_fricka_s_mediation_the_battle_damage_is_taken() {
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}

/// Fricka's Mediation: "Banish 1 "Wotan's Judgment" from your Graveyard": without one it is not offered.
#[test]
fn fricka_s_mediation_needs_wotans_judgment_in_the_graveyard() {
    let fricka = card("Fricka's Mediation");
    let Some(mut s) = field(&[], &[set(0, fricka), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[ChainOnAttack(fricka)], &[Attack(WARWOLF)]]).expect_err("no Wotan's Judgment");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Giant Flood, Goddess Erda's Guidance, Next World
// ---------------------------------------------------------------------------------------------------------------

/// Giant Flood: "During your Main Phase 1: Send all monsters on the field and from both players' hands to the Graveyard."
/// Both fields and both hands lose their monsters (Dian Keto, a Spell, stays in the hand).
#[test]
fn giant_flood_sends_every_monster_on_the_field_and_in_both_hands_to_the_graveyard() {
    let flood = card("Giant Flood");
    let spawns = [hand(0, flood), hand(0, DIAN_KETO), mon(0, WARWOLF), mon(1, THUNDER_KID), hand(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(flood)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF) + s.count(1, "grave", THUNDER_KID), 2, "the field's monsters: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF) + s.count(1, "mzone", THUNDER_KID), 0, "{}", s.context());
    assert_eq!((hand_size(&s, 0), hand_size(&s, 1)), (1, 0), "only Dian Keto is left in hand: {}", s.context());
    assert_eq!(s.count(0, "hand", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "the opponent's hand: {}", s.context());
    assert!(s.count(0, "grave", FILLER) >= 1, "my hand's Feral Imps too: {}", s.context());
}

/// Giant Flood: "During your Main Phase 1": in the Main Phase 2 it is not offered.
#[test]
fn giant_flood_is_for_main_phase_1_only() {
    let flood = card("Giant Flood");
    let Some(mut s) = field(&[], &[hand(0, flood), mon(1, THUNDER_KID)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "activates").contains(&flood.into()), "offered in Main Phase 1: {:?}", offered(&s, "activates"));
    // Through the Battle Phase (Warwolf attacks Thunder Kid) to the Main Phase 2.
    let Some(mut s) = field(&[], &[hand(0, flood), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    if s.prompt()["type"] == msg::SELECT_BATTLECMD {
        s.respond(serde_json::json!({"type": 1, "action": battle::TO_M2, "index": null})).unwrap();
    }
    for _ in 0..8 {
        if s.prompt()["type"] == msg::SELECT_IDLECMD {
            break;
        }
        s.pass().unwrap();
    }
    assert_eq!((s.prompt()["type"].as_u64(), s.prompt()["player"].as_u64()), (Some(msg::SELECT_IDLECMD), Some(0)), "my Main Phase 2: {}", s.context());
    assert_eq!(s.count(0, "hand", flood), 1, "still in my hand: {}", s.context());
    assert!(!offered(&s, "activates").contains(&flood.into()), "not in Main Phase 2: {:?}", offered(&s, "activates"));
    clean(&s);
}

/// Goddess Erda's Guidance: "Discard 1 Spell, then target 1 Trap in your Graveyard; Set it." Dian Keto is discarded and Magic
/// Cylinder (the only Trap in the Graveyard) is Set.
#[test]
fn goddess_erdas_guidance_discards_a_spell_and_sets_a_trap_from_the_graveyard() {
    let erda = card("Goddess Erda's Guidance");
    let spawns = [hand(0, erda), hand(0, DIAN_KETO), grave(0, MAGIC_CYLINDER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(erda)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "the Spell was discarded: {}", s.context());
    assert_eq!(s.count(0, "szone", MAGIC_CYLINDER), 1, "the Trap is Set: {}", s.context());
    assert_eq!(s.count(0, "grave", MAGIC_CYLINDER), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", erda), 1, "{}", s.context());
}

/// Goddess Erda's Guidance: "Discard 1 Spell": with no other Spell in the hand it is not offered.
#[test]
fn goddess_erdas_guidance_needs_a_spell_to_discard() {
    let erda = card("Goddess Erda's Guidance");
    let Some(mut s) = field(&[], &[hand(0, erda), grave(0, MAGIC_CYLINDER)]) else { return };
    let err = s.run(&[Activate(erda)]).expect_err("no Spell to discard");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Next World: "Target 1 monster your opponent controls; Normal Summon 1 monster from your hand with a Level equal to the
/// targeted monster's. You can also Normal Summon that monster without Tributing." The opponent's Dark Magician (Level 7)
/// lets me Summon mine with no Tribute (the script ignores the Normal Summon count: the turn's own Normal Summon stays free).
#[test]
fn next_world_normal_summons_a_monster_of_the_same_level_without_tribute() {
    let next = card("Next World");
    let spawns = [hand(0, next), hand(0, DARK_MAGICIAN), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(next)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "Dark Magician came with no Tribute: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 0, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.prompt()["type"], msg::SELECT_IDLECMD, "{}", s.context());
    // The script Summons with "ignore the Normal Summon count", so the turn's own Normal Summon is still free.
    assert!(offered(&s, "summons").contains(&FILLER.into()), "the Normal Summon is not used: {:?}", offered(&s, "summons"));
}

/// Next World: "a Level equal to the targeted monster's": Warwolf (Level 4) as the only target and Dark Magician (Level 7) as
/// the only monster in the hand: the card is not offered.
#[test]
fn next_world_needs_a_monster_of_the_same_level_in_the_hand() {
    let next = card("Next World");
    let spawns = [hand(0, next), hand(0, DARK_MAGICIAN), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    // Thunder Kid is Level 3 and the hand's monsters are Dark Magician (7) and the Feral Imps (4).
    let err = s.run(&[Activate(next)]).expect_err("no equal Level");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Last Tusk Mammoth
// ---------------------------------------------------------------------------------------------------------------

/// Last Tusk Mammoth (Anime): "Your opponent also takes any battle damage from battles involving this card." Warwolf (2000)
/// attacks the Mammoth (800, Attack Position): its controller takes 1200, and so does the attacker's.
#[test]
fn last_tusk_mammoth_hurts_both_players_when_attacked() {
    let mammoth = card("Last Tusk Mammoth (Anime)");
    let Some(mut s) = field(&[], &[mon(0, mammoth), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", mammoth), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1200, "the damage to the Mammoth's controller: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1200, "and the same to the opponent: {}", s.context());
}

/// Last Tusk Mammoth (Anime): its own attack. It (800) attacks Dark Blade (1800) in turn 3: both players take 1000.
#[test]
fn last_tusk_mammoth_hurts_both_players_when_it_attacks() {
    let mammoth = card("Last Tusk Mammoth (Anime)");
    let Some(mut s) = field(&[], &[mon(0, mammoth), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Attack(mammoth)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
}

/// Last Tusk Mammoth, the control: a monster without it (Thunder Kid) hurts only the loser.
#[test]
fn without_the_mammoth_only_the_loser_takes_battle_damage() {
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1300, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Nibelung's Ring and Nibelung's Treasure
// ---------------------------------------------------------------------------------------------------------------

/// Nibelung's Ring: "The equipped monster cannot attack, be Tributed, change its battle position, or activate its effect(s).
/// Draw 2 cards during the Draw Phase. If a Monster Card(s) is drawn this way, discard 1 of them to the Graveyard." Equipped
/// to my Warwolf in turn 1: turn 3's draw is 2 cards, and one of the two Feral Imps is discarded.
#[test]
fn nibelungs_ring_draws_two_and_discards_a_monster_drawn() {
    let ring = card("Nibelung's Ring");
    let Some(mut s) = field(&[], &[hand(0, ring), mon(0, WARWOLF)]) else { return };
    let (deck, hand_before) = (deck_size(&s, 0), hand_size(&s, 0));
    s.run_both([&[Activate(ring), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(deck_size(&s, 0), deck - 2, "two cards drawn in turn 3's Draw Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "one monster of the two was discarded: {}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before - 1 + 2 - 1, "the Ring left the hand, two cards came, one was discarded: {}", s.context());
    assert_eq!(s.count(0, "szone", ring), 1, "{}", s.context());
}

/// Nibelung's Ring: the equipped monster cannot attack (Warwolf waits turns and never can), and cannot change its position.
#[test]
fn nibelungs_ring_stops_the_equipped_monster_from_attacking_or_changing_position() {
    let ring = card("Nibelung's Ring");
    let Some(mut s) = field(&[], &[hand(0, ring), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(ring), Attack(WARWOLF)]).expect_err("cannot attack");
    assert!(err.contains("never became possible") || err.contains("can't attack"), "{err}");
    assert_eq!(s.count(0, "szone", ring), 1, "the Ring was equipped: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, ring), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(ring), Reposition(WARWOLF)]).expect_err("cannot change position");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "szone", ring), 1, "the Ring was equipped: {}", s.context());
    clean(&s);
}

/// Nibelung's Ring: "If the monster equipped with this card is destroyed by battle with a monster, return this card to the
/// Deck. Then shuffle the Deck." (engine/overrides/c511000271.lua: upstream never scripted it.) The Ring is on my Thunder
/// Kid; Warwolf destroys it in turn 2 and the Ring is in my Deck, not in the Graveyard.
#[test]
fn nibelungs_ring_returns_to_the_deck_when_its_monster_is_destroyed_by_battle() {
    let ring = card("Nibelung's Ring");
    let Some(mut s) = field(&[], &[hand(0, ring), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let deck = deck_size(&s, 0);
    s.run_both([&[Activate(ring), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "destroyed by battle: {}", s.context());
    assert_eq!(s.count(0, "grave", ring), 0, "the Ring is not in the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "szone", ring), 0, "{}", s.context());
    assert_eq!(deck_size(&s, 0), deck + 1, "it is in the Deck: {}", s.context());
}

/// Nibelung's Ring, the other half: a monster destroyed by an effect (Dark Hole) sends the Ring to the Graveyard.
#[test]
fn nibelungs_ring_goes_to_the_graveyard_when_its_monster_is_destroyed_by_an_effect() {
    let ring = card("Nibelung's Ring");
    let Some(mut s) = field(&[], &[hand(0, ring), mon(0, THUNDER_KID), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(ring), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ring), 1, "{}", s.context());
}

/// Nibelung's Treasure: "Activate 1 "Nibelung's Ring" from your Deck to your opponent's side of the field, equipping it to a
/// monster they control, then, draw 5 cards." The Ring (the Deck's last card) lands on team 1's side, equipped to Warwolf,
/// and I draw 5: so team 1's Warwolf cannot attack in turn 2.
#[test]
fn nibelungs_treasure_equips_a_ring_to_the_opponents_monster_and_draws_five() {
    let (treasure, ring) = (card("Nibelung's Treasure"), card("Nibelung's Ring"));
    let Some(mut s) = field(&[ring], &[hand(0, treasure), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let (hand_before, deck_before) = (hand_size(&s, 0), deck_size(&s, 0));
    s.run(&[Activate(treasure)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", ring), 1, "the Ring is on the opponent's side: {}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before - 1 + 5, "five cards drawn, the Treasure used: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck_before - 1 - 5, "the Ring and five cards left the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", treasure), 1, "{}", s.context());
    // Equipped to Warwolf: it cannot attack in turn 2.
    let err = s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).expect_err("the Ring stops the attack");
    assert!(err.contains("never became possible") || err.contains("can't attack"), "{err}");
}

/// Nibelung's Treasure: "Nibelung's Ring from your Deck": without one in the Deck it is not offered.
#[test]
fn nibelungs_treasure_needs_a_ring_in_the_deck() {
    let treasure = card("Nibelung's Treasure");
    let Some(mut s) = field(&[], &[hand(0, treasure), mon(1, WARWOLF)]) else { return };
    let err = s.run(&[Activate(treasure)]).expect_err("no Ring");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Magical Alms
// ---------------------------------------------------------------------------------------------------------------

/// Magical Alms: "When you activate this card: gain 1000 Life Points. During each player's turn: the turn player guesses how
/// many Spell Cards they will activate for the rest of that turn, then both players gain that amount of Life Points x500.
/// During each player's End Phase, if the turn player activated less Spell Cards than they predicted: they take damage
/// equal to the guessed number x1000." Activated in the Main Phase with a guess of 0: +1000 for me, nothing else, no damage.
#[test]
fn magical_alms_gains_1000_and_a_guess_of_zero_costs_nothing() {
    let alms = card("Magical Alms");
    let Some(mut s) = field(&[], &[hand(0, alms)]) else { return };
    s.declare_number(Some(0));
    s.run(&[Activate(alms), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 9000, "1000 for the activation, 0 x 500 for the guess, no damage: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Magical Alms: a guess of 2 gives both players 1000 LP; with fewer Spell Cards activated than guessed the turn player takes
/// the guess x 1000 in the End Phase. Only Magical Alms itself is activated in turn 1: if its own activation counted it would be 1, and
/// 2000 damage is still due.
#[test]
fn magical_alms_punishes_a_guess_that_was_too_high() {
    let alms = card("Magical Alms");
    let Some(mut s) = field(&[], &[hand(0, alms)]) else { return };
    s.declare_number(Some(2));
    s.run(&[Activate(alms)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 + 1000 + 1000, "the activation and the guess's 2 x 500: {}", s.context());
    assert_eq!(s.lp(1), 8000 + 1000, "the opponent gains the same from the guess: {}", s.context());
    // The End Phase takes 2 x 1000; the run then goes on into the opponent's Standby Phase, where the same Alms asks
    // them for their guess (the harness answers every number with 2): both players gain 1000 again.
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 + 1000 + 1000 - 2000 + 1000, "2 x 1000 damage, then the opponent's guess: {}", s.context());
    assert_eq!(s.lp(1), 8000 + 1000 + 1000, "{}", s.context());
}

/// Magical Alms: a guess met by the Spell Cards activated that turn (Dian Keto twice, besides the Alms itself) costs nothing.
#[test]
fn magical_alms_costs_nothing_when_the_guess_is_met() {
    let alms = card("Magical Alms");
    let Some(mut s) = field(&[], &[hand(0, alms), hand(0, DIAN_KETO), hand(0, DIAN_KETO)]) else { return };
    s.declare_number(Some(2));
    s.run(&[Activate(alms), Activate(DIAN_KETO), Activate(DIAN_KETO), EndTurn]).unwrap();
    clean(&s);
    // (The run ends in the opponent's turn, whose Standby Phase gave both players 1000 more for their guess of 2.)
    assert_eq!(s.lp(0), 8000 + 1000 + 1000 + 2000 + 1000, "the Alms, the guess and two Dian Keto, no damage: {}", s.context());
    assert_eq!(s.lp(1), 8000 + 1000 + 1000, "{}", s.context());
}

/// Magical Alms: does the Alms' own activation count among the Spell Cards the turn player "activated"? With a guess of 1 and
/// nothing else activated, the script counts it (its activation resolves after the counter is registered): no damage.
#[test]
fn magical_alms_counts_its_own_activation_in_the_turn_it_is_activated() {
    let alms = card("Magical Alms");
    let Some(mut s) = field(&[], &[hand(0, alms)]) else { return };
    s.declare_number(Some(1));
    s.run(&[Activate(alms), EndTurn]).unwrap();
    clean(&s);
    // 8000 + 1000 (activation) + 500 (guess) at once, + 500 from the opponent's Standby Phase guess of 1; no damage.
    assert_eq!(s.lp(0), 8000 + 1000 + 500 + 500, "{}", s.context());
    assert_eq!(s.lp(1), 8000 + 500 + 500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Luminous Clouds and Dragon Lady
// ---------------------------------------------------------------------------------------------------------------

/// Luminous Clouds: "Tribute 1 "Master Kyonshee" and 1 "Kung Fu Nyan Nyan"; Special Summon 1 "Dragon Lady" from your hand or
/// Deck." Both are Tributed and Dragon Lady (2500/2100) comes from the Deck.
#[test]
fn luminous_clouds_tributes_the_two_for_dragon_lady() {
    let [clouds, nyan, lady] = ["Luminous Clouds", "Kung Fu Nyan Nyan", "Dragon Lady"].map(card);
    let kyonshee = card("Master Kyonshee");
    let Some(mut s) = field(&[lady], &[hand(0, clouds), mon(0, kyonshee), mon(0, nyan)]) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Activate(clouds)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", lady), 1, "Dragon Lady is on the field: {}", s.context());
    assert_eq!(s.attack_of(0, lady), Some(2500), "{}", s.context());
    assert_eq!(s.count(0, "grave", kyonshee) + s.count(0, "grave", nyan), 2, "both were Tributed: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 1, "from the Deck: {}", s.context());
}

/// Luminous Clouds: both Tributes are needed: with only Master Kyonshee the card is not offered.
#[test]
fn luminous_clouds_needs_both_monsters() {
    let [clouds, lady, kyonshee] = ["Luminous Clouds", "Dragon Lady", "Master Kyonshee"].map(card);
    let Some(mut s) = field(&[lady], &[hand(0, clouds), mon(0, kyonshee), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(clouds)]).expect_err("no Kung Fu Nyan Nyan");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Dragon Lady: "You cannot Special Summon other monsters the turn this card is Summoned." After Luminous Clouds, Monster Reborn
/// (to bring back a Tributed monster) is not offered.
#[test]
fn dragon_lady_stops_other_special_summons_that_turn() {
    let [clouds, nyan, lady, kyonshee] = ["Luminous Clouds", "Kung Fu Nyan Nyan", "Dragon Lady", "Master Kyonshee"].map(card);
    let spawns = [hand(0, clouds), mon(0, kyonshee), mon(0, nyan), hand(0, MONSTER_REBORN)];
    let Some(mut s) = field(&[lady], &spawns) else { return };
    let err = s.run(&[Activate(clouds), Activate(MONSTER_REBORN)]).expect_err("no other Special Summon");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", lady), 1, "{}", s.context());
}

/// Dragon Lady: "Once per turn: You can target 1 Spell/Trap your opponent controls; destroy it. This card cannot declare an
/// attack the turn you activate this effect." The opponent's Set Magic Cylinder is destroyed; Dragon Lady then cannot attack.
#[test]
fn dragon_lady_destroys_a_spell_or_trap_and_then_cannot_attack() {
    let lady = card("Dragon Lady");
    let Some(mut s) = field(&[], &[mon(0, lady), set(1, MAGIC_CYLINDER), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(lady)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MAGIC_CYLINDER), 1, "destroyed: {}", s.context());
    // Turn 3: it destroys nothing more (no Spell/Trap), and may attack: 2500 against Thunder Kid.
    s.run_both([&[EndTurn, Attack(lady)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "attacked in a later turn: {}", s.context());
}

/// Dragon Lady: after the effect it cannot attack in the same turn (turn 3, with both a Trap to destroy and Thunder Kid to
/// attack).
#[test]
fn dragon_lady_cannot_attack_the_turn_it_destroys() {
    let lady = card("Dragon Lady");
    let Some(mut s) = field(&[], &[mon(0, lady), set(1, MAGIC_CYLINDER), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(lady), Attack(lady)], &[EndTurn]]).expect_err("cannot attack");
    assert!(err.contains("never became possible") || err.contains("can't attack"), "{err}");
    assert_eq!(s.count(1, "grave", MAGIC_CYLINDER), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Legendary Gambler
// ---------------------------------------------------------------------------------------------------------------

/// The Legendary Gambler: "When your opponent rolls a six-sided die to activate a card or effect, you can roll a six-sided
/// die. If your die result is equal to or higher than your opponent's, negate your opponent's card effect." Dark Blade
/// (1800) attacks directly in turn 3 and team 1's Skull Dice (the anime Trap: roll a die when it is activated; the attacker's
/// ATK becomes its ATK divided by the result) is answered once in every seed with no engine error (before the override, my
/// own roll triggered the card again: "recursive event trigger"). Some seeds negate it (the damage is the full 1800), some do
/// not (1800 divided by the roll, 900 or less).
#[test]
fn the_legendary_gambler_answers_an_opponents_die_roll_once() {
    let (gambler, skull) = (card("The Legendary Gambler"), card("Skull Dice"));
    let (mut negated, mut let_through) = (0, 0);
    for seed in 1..=24 {
        let spawns = [spell(0, gambler), mon(0, DARK_BLADE), set(1, skull)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        keep_hands(&mut s);
        s.run_both([&[Attack(DARK_BLADE)], &[ChainOnAttack(skull)]]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(1, "grave", skull), 1, "seed {seed}: the Trap was activated: {}", s.context());
        let asked = s.prompts.iter().filter(|&&(team, kind)| team == 0 && kind == msg::SELECT_YESNO).count();
        assert_eq!(asked, 1, "seed {seed}: the Gambler asks once for the one roll: {:?}", s.prompts);
        match 8000 - s.lp(1) {
            1800 => negated += 1,
            900 | 600 | 450 | 360 | 300 => let_through += 1,
            other => panic!("seed {seed}: damage {other}: {}", s.context()),
        }
    }
    assert!(negated > 0 && let_through > 0, "both outcomes in 24 seeds: negated {negated}, let through {let_through}");
}

/// The Legendary Gambler: "When your opponent rolls": my own roll gives it nothing to answer. Team 0's Skull Dice against
/// Warwolf's direct attack (2000) resolves with the roll's effect and no question: 2000 divided by the die is the damage.
#[test]
fn the_legendary_gambler_ignores_its_controllers_own_die_roll() {
    let (gambler, skull) = (card("The Legendary Gambler"), card("Skull Dice"));
    for seed in [1, 2, 3, 4, 5, 6] {
        let spawns = [spell(0, gambler), set(0, skull), mon(1, WARWOLF)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        keep_hands(&mut s);
        s.run_both([&[ChainOnAttack(skull)], &[Attack(WARWOLF)]]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        let damage = 8000 - s.lp(0);
        assert!([2000, 1000, 666, 500, 400, 333].contains(&damage), "seed {seed}: 2000 divided by a die, got {damage}: {}", s.context());
        assert_eq!(s.count(0, "grave", skull), 1, "seed {seed}: {}", s.context());
        assert!(!s.prompts.iter().any(|&(team, kind)| team == 0 && kind == msg::SELECT_YESNO), "seed {seed}: no question: {:?}", s.prompts);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Take One Chance (batch 2) with Dark Magic Retribution
// ---------------------------------------------------------------------------------------------------------------

/// Take One Chance (regression, engine/overrides/c511001499.lua): Dark Magic Retribution (a Trap that needs an attack) and Dian
/// Keto are in the Graveyard; Dian Keto makes the Trap on offer. When the random pick is Dark Magic Retribution, its
/// condition reads an attacker there is none of: it stays in the Graveyard with no Lua error and nothing else happens (LP
/// stay). When it is Dian Keto, 1000 LP are gained. Across seeds both occur.
#[test]
fn take_one_chance_leaves_dark_magic_retribution_in_the_graveyard_outside_an_attack() {
    let (chance, retribution) = (card("Take One Chance"), card("Dark Magic Retribution"));
    let spawns = [set(0, chance), grave(0, retribution), grave(0, DIAN_KETO)];
    let mut picks = std::collections::BTreeSet::new();
    for seed in 1..=12 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        s.run(&[Activate(chance)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        assert_eq!(s.count(0, "grave", chance), 1, "seed {seed}: {}", s.context());
        if s.lp(0) == 8000 {
            assert_eq!(s.count(0, "grave", retribution), 1, "seed {seed}: it stayed: {}", s.context());
            assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "seed {seed}: {}", s.context());
            assert_eq!(s.count(0, "szone", retribution), 0, "seed {seed}: {}", s.context());
            picks.insert("Dark Magic Retribution");
        } else {
            assert_eq!(s.lp(0), 9000, "seed {seed}: Dian Keto ran: {}", s.context());
            assert_eq!(s.count(0, "grave", retribution), 1, "seed {seed}: {}", s.context());
            picks.insert("Dian Keto");
        }
    }
    assert_eq!(picks.len(), 2, "twelve seeds pick both cards, got {picks:?}");
}
