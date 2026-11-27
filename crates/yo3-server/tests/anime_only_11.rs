//! Anime-only cards, group 11 (rows 6180-6216 and 6370 of engine/added/cards.tsv): Nightshroud's Darkness cards, Axel Brodie's
//! Volcanic and Pyro cards and Jesse Anderson's Crystal Beast cards. These cards exist only in the anime, so every duel plays
//! with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models
//! checked after every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and Quick-Play Spell answering
//! the opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3. A
//! Trap's `Chain` takes the first chance team 0 gets, which can be its own End Phase: cards that must be chained to an event
//! use `ChainOnAttack`, or a plan that waits for the event.
//! Covered: the Tokens (Copy Token, made by Copy Doll; Bomb Token, made by Volcanic Mine and shown to the client as its own
//! card), the cards the per-card fuzz flagged (Darkness Necroslime and Darkness Raincrow: the fuzz's decks never held a second
//! Darkness monster or a Bramble with the Raincrow; Infinity and Zero: the fuzz never put both face-up between Set cards, so
//! the "between" effect was never used; Anchor Bind: no Umi; Big Volcano: two Pyros in the opponent's Graveyard; Buster Trap:
//! a Special Summon to the opponent's side in the Battle Phase; Fire Trap: a destruction of a Spell/Trap by the opponent;
//! Volcanic Armor: a Pyro of mine destroyed (the fuzz saw the Trap resolve and no damage, here the 500 comes in a Dark Hole
//! and in a battle); Volcanic Force: no Tri-Blaze Accelerator; Crystal Flash and Crystal Fortress: Crystal Beasts on the field),
//! the cards with a script note (Darkness Bramble and Darkness Seed set LP: their End Phase effect is a comparison with 4000,
//! so ∞ LP is left alone, in `infinite_lp`'s way: Mystik Wok on the anime Geh), the signature cards (Darkness with Zero,
//! Infinity and Darkness 1-3 in every order of the first chain link, Darkness Bramble and Seed, Volcanic Mine, Crystal Flash,
//! E Force) and the seeded 30% of the rest (Damage Capture, Darkness Outsider, Darkness Slime, G Force, Mine Blast, Take Flight,
//! Trap Strap), and every other card of the batch too, since each was cheap. Trap Strap's text ("Set that card face-down") is
//! kept by the script: the Trap still resolves and goes back Set. The game has another "Copy Token" (6462), so the anime one is
//! given by its id. "Negate Attack" is the anime Quick-Play Spell here, not the Trap of the game: Trap Strap is tried on Mirror
//! Force. The prompts of Zero and Infinity ("Activate 1 Set card", the zones) are answered by `drive` below, since the default
//! answers pick a zone at random.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DIAN_KETO: u16 = 4348; // Normal Spell, gain 1000 LP
const DARK_HOLE: u16 = 4342;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
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

/// The usual duel: 8000 LP, both Decks of filler.
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

/// Whether the card's effect is offered at team 0's first idle prompt.
fn playable(s: &mut Scenario, cid: u16) -> bool {
    s.run(&[]).unwrap();
    offered(s, "activates").contains(&cid.into())
}

/// Passes team 1's turn on to team 0's next Main Phase (after a plan that ended team 0's turn).
fn own_turn(s: &mut Scenario) {
    for _ in 0..40 {
        if s.prompt()["type"] == msg::SELECT_IDLECMD && s.prompt()["player"] == 0 {
            return;
        }
        s.pass().unwrap();
    }
    panic!("team 0's turn never came: {}", s.context());
}

/// Blasting Vein: "Destroy 1 Set Spell/Trap you control; draw 2 cards." Dian Keto Set is destroyed and two cards come; a
/// face-up Spell is not a cost it can pay (the card is not offered).
#[test]
fn blasting_vein_destroys_a_set_card_and_draws_two() {
    let vein = card("Blasting Vein");
    let Some(mut s) = field(&[], &[hand(0, vein), set(0, DIAN_KETO)]) else { return };
    s.run(&[]).unwrap();
    let before = hand_size(&s, 0);
    let deck = deck_size(&s, 0);
    s.run(&[Activate(vein)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", vein), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before - 1 + 2, "Blasting Vein left and two came: {}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 2, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, vein), spell(0, DIAN_KETO)]) else { return };
    assert!(!playable(&mut s, vein), "a face-up card is no cost: {}", s.context());
}

/// Blasting Vein's cost is any Set Spell/Trap, Fire Trap included: the Trap is destroyed face-down, so its own text draws 1
/// more ("If this Set card is destroyed and sent to the Graveyard: Draw 1 card").
#[test]
fn fire_trap_draws_one_when_it_is_destroyed_face_down() {
    let [vein, fire_trap] = ["Blasting Vein", "Fire Trap"].map(card);
    let Some(mut s) = field(&[], &[hand(0, vein), set(0, fire_trap)]) else { return };
    s.run(&[]).unwrap();
    let before = hand_size(&s, 0);
    s.run(&[Activate(vein)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", fire_trap), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before - 1 + 2 + 1, "2 from the Vein and 1 from the Trap: {}", s.context());
}

/// Fire Trap: "When your opponent activates a card or effect that would destroy a Spell(s)/Trap(s) you control: Negate its
/// activation, and if you do, destroy it." Mystical Space Typhoon aimed at my Set Dian Keto is negated and destroyed, and
/// Dian Keto stays Set; the Trap goes to the Graveyard (destroyed by nothing, so it draws nothing).
#[test]
fn fire_trap_negates_a_spell_trap_destruction_and_destroys_it() {
    let [fire_trap, mst] = ["Fire Trap", "Mystical Space Typhoon"].map(card);
    let Some(mut s) = field(&[], &[set(0, fire_trap), set(0, DIAN_KETO), hand(1, mst)]) else { return };
    s.prefer(&[(0, DIAN_KETO)]);
    s.run_both([&[Chain(fire_trap)], &[Activate(mst)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", mst), 1, "Mystical Space Typhoon is destroyed: {}", s.context());
    assert_eq!(s.count(0, "szone", DIAN_KETO), 1, "Dian Keto is safe: {}", s.context());
    assert_eq!(s.count(0, "grave", fire_trap), 1, "{}", s.context());
}

/// Fire Trap does not answer an effect that destroys nothing of mine: Mystical Space Typhoon aimed at the opponent's own
/// Spell is not chained (the Trap is not offered).
#[test]
fn fire_trap_ignores_destruction_of_the_opponents_own_cards() {
    let [fire_trap, mst] = ["Fire Trap", "Mystical Space Typhoon"].map(card);
    let Some(mut s) = field(&[], &[set(0, fire_trap), spell(1, DIAN_KETO), hand(1, mst)]) else { return };
    s.run_both([&[], &[Activate(mst)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", fire_trap), 1, "still Set: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Jesse Anderson's Crystal Beast spells and traps
// ---------------------------------------------------------------------------------------------------------------

/// Take Flight: "While you control a face-up "Crystal Beast Sapphire Pegasus": Return all other "Crystal Beast" monsters on
/// the field to their owner's hand(s)." My Amethyst Cat and the opponent's Topaz Tiger go to their owners' hands, Pegasus and
/// a monster that is not a Crystal Beast stay. Without a face-up Pegasus the Spell is not offered.
#[test]
fn take_flight_returns_every_other_crystal_beast_to_its_owners_hand() {
    let [flight, pegasus, cat, tiger] =
        ["Take Flight", "Crystal Beast Sapphire Pegasus", "Crystal Beast Amethyst Cat", "Crystal Beast Topaz Tiger"].map(card);
    let spawns = [hand(0, flight), mon(0, pegasus), mon(0, cat), mon(0, THUNDER_KID), mon(1, tiger), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(flight)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", cat), 1, "my Cat is back in my hand: {}", s.context());
    assert_eq!(s.count(1, "hand", tiger), 1, "their Tiger is back in their hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", pegasus), 1, "Pegasus stays: {}", s.context());
    assert_eq!((s.count(0, "mzone", THUNDER_KID), s.count(1, "mzone", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", flight), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, flight), mon(0, cat), mon(1, tiger)]) else { return };
    assert!(!playable(&mut s, flight), "no Pegasus: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, flight), mon(1, pegasus), mon(0, cat)]) else { return };
    assert!(!playable(&mut s, flight), "the Pegasus must be mine: {}", s.context());
}

/// Crystal Fortress: "All monsters your opponent controls, with an ATK equal to or less than 1000 x the number of "Crystal
/// Beast" monsters you currently control, cannot attack this turn." Two Crystal Beasts make 2000. The opponent's first Dark
/// Magician (2500) attacks and the Fortress is chained to that attack: from then on Warwolf (2000) and Thunder Kid (700)
/// cannot attack, and the second Dark Magician (2500) can.
#[test]
fn crystal_fortress_stops_the_attacks_of_the_weak_monsters() {
    let [fortress, cat, tiger] = ["Crystal Fortress", "Crystal Beast Amethyst Cat", "Crystal Beast Topaz Tiger"].map(card);
    let spawns = [set(0, fortress), mon(0, cat), mon(0, tiger), mon(1, DARK_MAGICIAN), mon(1, DARK_MAGICIAN), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    for (second, allowed) in [(WARWOLF, false), (THUNDER_KID, false), (DARK_MAGICIAN, true)] {
        let Some(mut s) = field(&[], &spawns) else { return };
        let result = s.run_both([&[EndTurn, ChainOnAttack(fortress)], &[Attack(DARK_MAGICIAN), Attack(second)]]);
        match (allowed, result) {
            (true, Ok(())) => {}
            (false, Err(e)) if e.contains("can't attack") => {}
            (allowed, r) => panic!("{second}: attack allowed {allowed}, got {r:?}: {}", s.context()),
        }
        clean(&s);
        assert_eq!(s.count(0, "grave", fortress), 1, "{}", s.context());
    }
}

/// Crystal Flash: "Target 1 "Crystal Beast" monster you control; place 1 "Crystal Beast" monster from your hand to your Spell
/// & Trap Zone as a Continuous Spell. Until the End Phase, the targeted monster cannot be destroyed by battle, and you take
/// no Battle Damage from battles involving that monster." Warwolf (2000) attacks Sapphire Pegasus (1800): the Flash is chained
/// to the attack, Amethyst Cat goes from the hand to the Spell & Trap Zone, Pegasus survives and no damage is dealt.
#[test]
fn crystal_flash_places_a_crystal_beast_and_protects_the_target() {
    let [flash, pegasus, cat] = ["Crystal Flash", "Crystal Beast Sapphire Pegasus", "Crystal Beast Amethyst Cat"].map(card);
    let Some(mut s) = field(&[], &[set(0, flash), mon(0, pegasus), hand(0, cat), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, ChainOnAttack(flash)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", cat), 1, "Amethyst Cat is in the Spell & Trap Zone: {}", s.context());
    assert_eq!(s.count(0, "hand", cat), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", pegasus), 1, "Pegasus was not destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", flash), 1, "{}", s.context());
    // Without a Crystal Beast in the hand it cannot be activated.
    let Some(mut s) = field(&[], &[set(0, flash), mon(0, pegasus), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, ChainOnAttack(flash)], &[Attack(WARWOLF)]]).expect_err("nothing to chain");
    assert_eq!(s.count(0, "szone", flash), 1, "not activated: {}", s.context());
}

/// E Force: "Target 1 "Crystal Beast" card in your Spell & Trap Zone; Special Summon it." Amethyst Cat, placed there face-up
/// as a Continuous Spell, comes to the field as a monster.
#[test]
fn e_force_summons_a_crystal_beast_from_the_spell_and_trap_zone() {
    let [force, cat] = ["E Force", "Crystal Beast Amethyst Cat"].map(card);
    let Some(mut s) = field(&[], &[hand(0, force), spell(0, cat)]) else { return };
    s.run(&[Activate(force)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", cat), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", cat), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", force), 1, "{}", s.context());
    // Nothing of the kind in the Spell & Trap Zone: not offered.
    let Some(mut s) = field(&[], &[hand(0, force), mon(0, cat)]) else { return };
    assert!(!playable(&mut s, force), "{}", s.context());
}

/// G Force: "When an opponent's monster declares an attack: Special Summon 1 "Crystal Beast" monster from your hand in Attack
/// Position. Then, the monster Special Summoned by this effect becomes the new attack target." Warwolf (2000) attacks directly;
/// the Cat (1200) from my hand takes the attack instead: 800 damage, not 2000.
#[test]
fn g_force_summons_a_crystal_beast_that_becomes_the_attack_target() {
    let [force, cat] = ["G Force", "Crystal Beast Amethyst Cat"].map(card);
    let Some(mut s) = field(&[], &[set(0, force), hand(0, cat), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, ChainOnAttack(force)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 800, "the Cat's 1200 against 2000: {}", s.context());
    assert_eq!(s.count(0, "hand", cat), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", cat), 0, "the Cat was destroyed in the battle: {}", s.context());
    assert_eq!(s.count(0, "grave", force), 1, "{}", s.context());
}

/// Crystal Spring: "When a "Crystal Beast" monster you control is returned from the field to the Deck: Place that card and 2
/// other "Crystal Beast" monsters from your Deck on top of the Deck in any order." The opponent's Phoenix Wing Wind Blast
/// puts my Cat on top of my Deck; Amber Mammoth and Topaz Tiger (at the bottom of the Deck) join it on top. My turn's draw and
/// Pot of Greed's two take exactly those three cards.
#[test]
fn crystal_spring_puts_three_crystal_beasts_on_top_of_the_deck() {
    let [spring, phoenix, pot, cat, mammoth, tiger] =
        ["Crystal Spring", "Phoenix Wing Wind Blast", "Pot of Greed", "Crystal Beast Amethyst Cat", "Crystal Beast Amber Mammoth", "Crystal Beast Topaz Tiger"].map(card);
    let spawns = [spell(0, spring), mon(0, cat), hand(0, pot), set(1, phoenix)];
    let Some(mut s) = field(&[mammoth, tiger], &spawns) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER), (0, cat)]);
    let crystals = |s: &Scenario| [cat, mammoth, tiger].iter().map(|&c| s.count(0, "hand", c)).sum::<usize>();
    s.run_both([&[EndTurn], &[Activate(phoenix)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", cat), 0, "the Cat left the field: {}", s.context());
    assert_eq!(crystals(&s), 0, "none drawn yet: {}", s.context());
    s.run(&[]).unwrap();
    s.run(&[Activate(pot)]).unwrap();
    clean(&s);
    assert_eq!(crystals(&s), 3, "the turn's draw and Pot of Greed took the three on top: {}", s.context());
}

/// Sapphire Revive: "If you control a "Crystal Beast Sapphire Pegasus": Each player can target 1 Level 4 or lower monster
/// from their Graveyard; Special Summon it." Pegasus on my side: my Warwolf and their Thunder Kid come back, each to its owner's
/// side. Without Pegasus the Trap is not offered.
#[test]
fn sapphire_revive_brings_back_a_monster_for_each_player() {
    let [revive, pegasus] = ["Sapphire Revive", "Crystal Beast Sapphire Pegasus"].map(card);
    let spawns = [set(0, revive), mon(0, pegasus), grave(0, WARWOLF), grave(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[EndTurn, Activate(revive)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "my Warwolf is back: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "their Thunder Kid is back: {}", s.context());
    assert_eq!(s.count(0, "grave", revive), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, revive), grave(0, WARWOLF)]) else { return };
    s.run(&[EndTurn]).unwrap();
    own_turn(&mut s);
    assert!(!offered(&s, "activates").contains(&revive.into()), "no Pegasus: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Axel Brodie's Volcanic and Pyro cards
// ---------------------------------------------------------------------------------------------------------------

/// The anime Copy Token's own card (the game has another Copy Token, 6462, so the name is ambiguous).
const COPY_TOKEN: u16 = 16120;
const FIREYAROU: u16 = 4297; // Pyro, FIRE, Level 4, 1300/1000 vanilla
const BLAZING_INPACHI: u16 = 5876; // Pyro, FIRE, Level 4, 1850/0 vanilla
const FLAME_VIPER: u16 = 4650; // Pyro, EARTH, Level 2, 400/450 vanilla

/// Anchor Bind: "While "Umi" is on the field: destroy all face-up Defense Position FIRE monsters your opponent controls."
/// Fireyarou (FIRE, Defense) is destroyed; Blazing Inpachi (FIRE, Attack) and Warwolf (EARTH, Defense) stay. Without Umi on
/// the field the Trap is not offered.
#[test]
fn anchor_bind_destroys_the_fire_monsters_in_defense_while_umi_is_on_the_field() {
    let [bind, umi] = ["Anchor Bind", "Umi"].map(card);
    let spawns = [set(0, bind), spell(0, umi), mon_def(1, FIREYAROU), mon(1, BLAZING_INPACHI), mon_def(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[EndTurn, Activate(bind)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", FIREYAROU), 1, "{}", s.context());
    assert_eq!((s.count(1, "mzone", BLAZING_INPACHI), s.count(1, "mzone", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", bind), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, bind), mon_def(1, FIREYAROU)]) else { return };
    s.run(&[EndTurn]).unwrap();
    own_turn(&mut s);
    assert!(!offered(&s, "activates").contains(&bind.into()), "no Umi: {}", s.context());
}

/// Big Volcano: "Send 1 Pyro-Type monster from your hand to the Graveyard, then target 2 Pyro-Type monsters in your
/// opponent's Graveyard; Special Summon them to your opponent's side of the field." Fireyarou is sent; the opponent's Flame
/// Viper and Blazing Inpachi come back on the opponent's own side. With only one Pyro in their Graveyard it is not offered.
#[test]
fn big_volcano_summons_two_pyros_of_the_opponents_graveyard_to_their_side() {
    let volcano = card("Big Volcano");
    let spawns = [hand(0, volcano), hand(0, FIREYAROU), grave(1, FLAME_VIPER), grave(1, BLAZING_INPACHI)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(volcano)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FIREYAROU), 1, "the cost: {}", s.context());
    assert_eq!((s.count(1, "mzone", FLAME_VIPER), s.count(1, "mzone", BLAZING_INPACHI)), (1, 1), "{}", s.context());
    assert_eq!((s.count(0, "mzone", FLAME_VIPER), s.count(0, "mzone", BLAZING_INPACHI)), (0, 0), "{}", s.context());
    assert_eq!(s.count(1, "grave", FLAME_VIPER) + s.count(1, "grave", BLAZING_INPACHI), 0, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, volcano), hand(0, FIREYAROU), grave(1, FLAME_VIPER)]) else { return };
    assert!(!playable(&mut s, volcano), "only one Pyro to give: {}", s.context());
}

/// Volcanic Force: "Send 1 face-up "Tri-Blaze Accelerator" you control to the Graveyard; Special Summon 1 "Volcanic
/// Doomfire" from your hand, Deck, or Graveyard, ignoring its Summoning conditions, even if it was not properly Special
/// Summoned first." The Accelerator goes to the Graveyard and Volcanic Doomfire comes from the Deck, then from the Graveyard.
#[test]
fn volcanic_force_summons_volcanic_doomfire_for_a_tri_blaze_accelerator() {
    let [force, accelerator, doomfire] = ["Volcanic Force", "Tri-Blaze Accelerator", "Volcanic Doomfire"].map(card);
    let Some(mut s) = field(&[doomfire], &[set(0, force), spell(0, accelerator)]) else { return };
    s.run(&[EndTurn, Activate(force)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", doomfire), 1, "Doomfire from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", accelerator), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", accelerator), 0, "{}", s.context());
    assert_eq!(s.position_of(0, doomfire), Some(1), "face-up Attack: {}", s.context());
    let Some(mut s) = field(&[], &[set(0, force), spell(0, accelerator), grave(0, doomfire)]) else { return };
    s.run(&[EndTurn, Activate(force)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", doomfire), 1, "Doomfire from the Graveyard: {}", s.context());
    let Some(mut s) = field(&[doomfire], &[set(0, force)]) else { return };
    s.run(&[EndTurn]).unwrap();
    own_turn(&mut s);
    assert!(!offered(&s, "activates").contains(&force.into()), "no Tri-Blaze Accelerator: {}", s.context());
}

/// Volcanic Armor: "If a Pyro-Type monster you control is destroyed and sent to the Graveyard: Inflict 500 damage to your
/// opponent." Dark Hole destroys my Fireyarou (Pyro): 500 damage. My Warwolf (not a Pyro) costs the opponent nothing, and
/// their own Pyro destroyed with it does not count.
#[test]
fn volcanic_armor_inflicts_500_when_my_pyro_is_destroyed() {
    let armor = card("Volcanic Armor");
    for (mine, theirs, damage) in [(FIREYAROU, WARWOLF, 500), (WARWOLF, FIREYAROU, 0)] {
        let spawns = [set(0, armor), mon(0, mine), mon(1, theirs), hand(0, DARK_HOLE)];
        let Some(mut s) = field(&[], &spawns) else { return };
        keep_hands(&mut s);
        s.run(&[EndTurn, Activate(armor), Activate(DARK_HOLE)]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "grave", mine), 1, "my {mine} was destroyed: {}", s.context());
        assert_eq!(s.lp(1), 8000 - damage, "my {mine}, their {theirs}: {}", s.context());
        assert_eq!(s.lp(0), 8000, "{}", s.context());
    }
}

/// Volcanic Armor with the Pyro destroyed in battle: Warwolf (2000) kills my Fireyarou (1300) in the opponent's turn; the
/// Armor, activated in my turn 1, inflicts its 500 on top of the 700 of the battle (the opponent's LP is the 500).
#[test]
fn volcanic_armor_inflicts_500_when_my_pyro_is_destroyed_in_battle() {
    let armor = card("Volcanic Armor");
    let Some(mut s) = field(&[], &[set(0, armor), mon(0, FIREYAROU), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(armor), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FIREYAROU), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "battle damage to me: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "the Armor's damage to them: {}", s.context());
}

/// Volcanic Mine: "Special Summon as many "Bomb Tokens" (Pyro-Type/FIRE/Level 1/ATK 1000/DEF 1000) as possible to your
/// opponent's side of the field in Defense Position. Until the End Phase, your opponent's monsters cannot change their battle
/// positions, and their effects are negated. During the End Phase, destroy all "Bomb Tokens"." The opponent's field has one
/// monster, so four Tokens come, in Defense Position, with the Bomb Token's own card for the client (Pyro, FIRE, Level 1,
/// 1000/1000); at the End Phase they are all destroyed, and the client's view keeps up (`clean`).
#[test]
fn volcanic_mine_makes_bomb_tokens_that_are_destroyed_at_the_end_phase() {
    let [mine, bomb] = ["Volcanic Mine", "Bomb Token"].map(card);
    let Some(mut s) = field(&[], &[set(0, mine), mon(1, WARWOLF)]) else { return };
    s.run(&[EndTurn, Activate(mine)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", bomb), 4, "four Tokens fill the opponent's other zones: {}", s.context());
    assert_eq!(s.count(0, "mzone", bomb), 0, "{}", s.context());
    let t = s.monster(1, bomb).unwrap();
    assert_eq!((t["attack"].as_i64(), t["defense"].as_i64(), t["level"].as_i64(), t["race"].as_i64(), t["attribute"].as_i64()),
        (Some(1000), Some(1000), Some(1), Some(0x80), Some(4)), "{t}");
    assert_eq!(s.position_of(1, bomb), Some(4), "Defense Position: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    // The End Phase destroys them (they leave no card in the Graveyard, as a Token does).
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", bomb), 0, "{}", s.context());
    assert_eq!(s.count(1, "grave", bomb), 0, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Buster Trap: "When a monster(s) is Special Summoned to your opponent's side of the field during the Battle Phase:
/// Destroy that monster(s) and end the Battle Phase." Volcanic Mine chained to Thunder Kid's attack declaration puts Bomb
/// Tokens on the opponent's side in the Battle Phase: Buster Trap destroys them all and the Battle Phase ends (the attack
/// with it: the opponent loses nothing).
#[test]
fn buster_trap_destroys_monsters_summoned_to_the_opponents_side_in_the_battle_phase() {
    let [buster, mine, bomb] = ["Buster Trap", "Volcanic Mine", "Bomb Token"].map(card);
    let spawns = [set(0, buster), set(0, mine), mon(0, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[EndTurn, Attack(THUNDER_KID), ChainOnAttack(mine), Chain(buster)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", buster), 1, "Buster Trap was used: {}", s.context());
    assert_eq!(s.count(1, "mzone", bomb), 0, "every Token is destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000, "the Battle Phase ended before the attack hit: {}", s.context());
}

/// Buster Trap works only in the Battle Phase: Big Volcano's Special Summon in the Main Phase leaves the Trap Set (it is not
/// offered).
#[test]
fn buster_trap_ignores_a_main_phase_summon_to_the_opponents_side() {
    let [buster, volcano] = ["Buster Trap", "Big Volcano"].map(card);
    let spawns = [set(0, buster), hand(0, volcano), hand(0, FIREYAROU), grave(1, FLAME_VIPER), grave(1, BLAZING_INPACHI)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(volcano)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", buster), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", FLAME_VIPER), 1, "{}", s.context());
}

/// Damage Capture: "Activate only when you take damage. Target 1 face-up monster you control; it gains ATK equal to the damage
/// you took, until the End Phase." The opponent's Ookazi (800 damage) hits me: Thunder Kid (700) gains 800 and the Trap is spent.
/// Damage the opponent takes (my own Ookazi) does not allow it.
#[test]
fn damage_capture_gives_a_monster_the_atk_of_the_damage_taken() {
    let [capture, ookazi] = ["Damage Capture", "Ookazi"].map(card);
    let Some(mut s) = field(&[], &[set(0, capture), mon(0, THUNDER_KID), hand(1, ookazi)]) else { return };
    s.run_both([&[EndTurn, Chain(capture)], &[Activate(ookazi)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(700 + 800), "ATK equal to the damage: {}", s.context());
    assert_eq!(s.count(0, "grave", capture), 1, "{}", s.context());
    // The damage must be mine: Ookazi played by me on the opponent leaves the Trap Set.
    let Some(mut s) = field(&[], &[set(0, capture), mon(0, THUNDER_KID), hand(0, ookazi)]) else { return };
    s.run(&[Activate(ookazi)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(700), "{}", s.context());
    assert_eq!(s.count(0, "szone", capture), 1, "{}", s.context());
}

/// Mine Blast: "Send the top 5 cards of your Deck to the Graveyard. You cannot draw during your next Draw Phase." Set in turn
/// 1 and activated in turn 3 (its Draw Phase is over): the Deck loses five cards and the Graveyard gets them; in turn 5 no
/// card is drawn. Against the same duel without the Trap played (a draw in turns 3 and 5), the hand is one card smaller and the
/// Deck four cards smaller (five sent and one draw, against two draws).
#[test]
fn mine_blast_sends_five_cards_and_skips_the_next_draw() {
    let blast = card("Mine Blast");
    let run = |play: bool| -> Option<Scenario> {
        let mut s = field(&[], &[set(0, blast)])?;
        keep_hands(&mut s);
        let plan: &[yo3_server::duelcheck::scenario::Action] =
            if play { &[EndTurn, Activate(blast), EndTurn, Summon(FILLER)] } else { &[EndTurn, EndTurn, Summon(FILLER)] };
        s.run(plan).unwrap();
        clean(&s);
        Some(s)
    };
    let (Some(with), Some(without)) = (run(true), run(false)) else { return };
    assert_eq!(with.count(0, "grave", blast), 1, "{}", with.context());
    assert_eq!(hand_size(&with, 0) + 1, hand_size(&without, 0), "the draw of turn 5 was skipped: {} / {}", with.context(), without.context());
    assert_eq!(deck_size(&with, 0) + 4, deck_size(&without, 0), "five sent and one draw against two draws: {} / {}", with.context(), without.context());
    assert!(with.count(0, "grave", FILLER) >= 5, "the five cards are in the Graveyard: {}", with.context());
}

/// The One to be Protected: "When an opponent's monster attacks another monster: Send 1 monster from your Deck with an ATK
/// higher than the attacking monster's to the Graveyard; during damage calculation, the attack target cannot be destroyed by
/// battle, and its controller take no battle damage. After damage calculation, you take damage equal to the ATK of the monster
/// you sent." Warwolf (2000) attacks Thunder Kid (700): Dark Magician (2500, at the bottom of my Deck) is sent; Thunder Kid
/// survives and the battle costs me nothing, and then I take 2500.
#[test]
fn the_one_to_be_protected_saves_the_target_for_the_atk_of_a_monster_of_the_deck() {
    let [protected, magician] = ["The One to be Protected", "Dark Magician"].map(|n| if n == "Dark Magician" { DARK_MAGICIAN } else { card(n) });
    let Some(mut s) = field(&[magician], &[set(0, protected), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Chain(protected)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", magician), 1, "Dark Magician was sent from the Deck: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid was not destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2500, "no battle damage, then the sent monster's ATK: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    assert_eq!(s.count(0, "grave", protected), 1, "{}", s.context());
}

/// The One to be Protected: the monster sent must have a higher ATK than the attacker: a Deck of Thunder Kids (700) against
/// Warwolf (2000) gives nothing to send, and the Trap is not offered.
#[test]
fn the_one_to_be_protected_needs_a_stronger_monster_in_the_deck() {
    let protected = card("The One to be Protected");
    let Some(mut s) = field(&[THUNDER_KID], &[set(0, protected), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Chain(protected)], &[Attack(WARWOLF)]]).expect_err("nothing to send");
    assert_eq!(s.count(0, "szone", protected), 1, "{}", s.context());
}

/// Copy Doll: "Special Summon 1 "Copy Token" (Warrior-Type/EARTH/Level 1/ATK 0/DEF 0), but it cannot be Tributed for a
/// Tribute Summon." The Token comes face-up in Attack Position with those stats (the client's Copy Token card). Dark Magician
/// (Level 7, 2 Tributes) cannot be Tribute Summoned with the Token and one Thunder Kid, and can with two Thunder Kids.
#[test]
fn copy_doll_makes_a_copy_token_that_cannot_be_tributed_for_a_summon() {
    let doll = card("Copy Doll");
    let token = COPY_TOKEN;
    let Some(mut s) = field(&[], &[hand(0, doll), hand(0, DARK_MAGICIAN), mon(0, THUNDER_KID)]) else { return };
    s.run(&[Activate(doll)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 1, "{}", s.context());
    let t = s.monster(0, token).unwrap();
    assert_eq!((t["attack"].as_i64(), t["defense"].as_i64(), t["level"].as_i64(), t["race"].as_i64(), t["attribute"].as_i64()),
        (Some(0), Some(0), Some(1), Some(1), Some(1)), "Warrior, EARTH, Level 1, 0/0: {t}");
    assert_eq!(s.position_of(0, token), Some(1), "{}", s.context());
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "the Token is no Tribute: {:?}", offered(&s, "summons"));
    let Some(mut s) = field(&[], &[hand(0, DARK_MAGICIAN), mon(0, THUNDER_KID), mon(0, THUNDER_KID)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "two Thunder Kids are: {:?}", offered(&s, "summons"));
}

/// Copy Doll: "When an opponent's monster attacks the controller of the "Copy Token" directly, you can Tribute that Token, if
/// you do, you take no battle damage from that attack." A monster that attacks directly past the Token (the Alchemy Beast
/// Aretos the Tin, 500 ATK, "can attack directly") hits the Token's controller: the Token is Tributed (and shattered: no card
/// in the Graveyard) and I lose nothing; the Token shielded no attack that was not direct (Warwolf attacking the Token itself
/// destroys it, and I take no damage there either, as a Token has 0 ATK/DEF: the damage is the 2000 ATK).
#[test]
fn a_copy_token_is_tributed_to_stop_the_damage_of_a_direct_attack() {
    let doll = card("Copy Doll");
    let token = COPY_TOKEN;
    let aretos = card("Alchemy Beast - Aretos the Tin");
    let Some(mut s) = field(&[], &[hand(0, doll), mon(1, aretos)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(doll), EndTurn], &[Attack(aretos)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 0, "the Token was Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", token), 0, "a Token leaves no card: {}", s.context());
    assert_eq!(s.count(1, "mzone", aretos), 1, "{}", s.context());
    // Without the Token the same attack costs 500.
    let Some(mut s) = field(&[], &[mon(1, aretos)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(aretos)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
}

/// Switch Off: "Target 1 face-up Continuous Spell/Trap your opponent controls; its effect(s) are negated until the End Phase,
/// then, change 1 Defense Position monster your opponent controls to Attack Position." Messenger of Peace (monsters with 1500
/// or more ATK cannot attack) is negated, so Warwolf (2000) attacks in turn 3; the opponent's Thunder Kid in Defense Position
/// turns to Attack.
#[test]
fn switch_off_negates_a_continuous_card_and_turns_a_monster_to_attack() {
    let [off, messenger] = ["Switch Off", "Messenger of Peace"].map(card);
    let spawns = [set(0, off), mon(0, WARWOLF), spell(1, messenger), mon_def(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[EndTurn, Attack(WARWOLF)]).expect_err("Messenger of Peace stops the attack");
    assert!(err.contains("can't attack"), "{err}");
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(off)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", off), 1, "{}", s.context());
    assert_eq!(s.position_of(1, THUNDER_KID), Some(1), "Thunder Kid is in Attack Position: {}", s.context());
    s.run(&[Attack(WARWOLF)]).unwrap_or_else(|e| panic!("Messenger of Peace is negated, Warwolf attacks: {e}"));
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
}

/// Flaming Barbed Wire: "Change monsters your opponent controls to Defense Position, up to the number of cards in your hand.
/// The original DEF of those monsters becomes 0 until the End Phase. Monsters you currently control cannot attack directly
/// this turn." Both of the opponent's monsters go to Defense with 0 DEF; after Raigeki clears their field, my Thunder Kid cannot
/// attack directly (without the Wire it can).
#[test]
fn flaming_barbed_wire_turns_monsters_to_defense_and_stops_direct_attacks() {
    let [wire, raigeki] = ["Flaming Barbed Wire", "Raigeki"].map(card);
    let spawns = [hand(0, wire), hand(0, raigeki), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF), (1, DARK_MAGICIAN)]);
    s.prefer_most(true);
    s.run(&[Activate(wire)]).unwrap();
    clean(&s);
    assert_eq!((s.position_of(1, WARWOLF), s.position_of(1, DARK_MAGICIAN)), (Some(4), Some(4)), "{}", s.context());
    assert_eq!((s.monster(1, WARWOLF).unwrap()["base_defense"].as_i64(), s.monster(1, DARK_MAGICIAN).unwrap()["base_defense"].as_i64()), (Some(0), Some(0)), "{}", s.context());
    // Without the Wire, the same Raigeki leaves a direct attack.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(raigeki), EndTurn, Attack(THUNDER_KID)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 700, "a direct attack: {}", s.context());
    // The Wire and Raigeki, then no attack with Thunder Kid in the same turn: it cannot attack directly.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, FILLER), (1, WARWOLF), (1, DARK_MAGICIAN)]);
    s.prefer_most(true);
    let err = s.run(&[EndTurn, Activate(wire), Activate(raigeki), Attack(THUNDER_KID)]).expect_err("no direct attack");
    assert!(err.contains("can't attack"), "{err}");
}

/// Draw Blast: "When your opponent draws a card(s): Inflict 1000 damage to your opponent." The opponent's draw for the turn
/// costs them 1000.
#[test]
fn draw_blast_inflicts_1000_when_the_opponent_draws() {
    let blast = card("Draw Blast");
    let Some(mut s) = field(&[], &[set(0, blast)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(blast)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(0, "grave", blast), 1, "{}", s.context());
}

/// Trap Strap: "When your opponent activates a Trap Card: when it resolves, Set that card face-down instead of sending it to
/// the Graveyard. Then, you can Special Summon 1 "Crystal Beast" monster from your hand or Deck." The opponent's Mirror Force
/// (against my Thunder Kid's attack) goes back to their Spell & Trap Zone face-down and still resolves; Amethyst Cat comes from
/// my Deck, in time to be destroyed by it too (a Crystal Beast destroyed goes to the Spell & Trap Zone: its own effect).
#[test]
fn trap_strap_sets_the_trap_again_and_summons_a_crystal_beast() {
    let [strap, cat, mirror] = ["Trap Strap", "Crystal Beast Amethyst Cat", "Mirror Force"].map(card);
    let Some(mut s) = field(&[cat], &[set(0, strap), mon(0, THUNDER_KID), set(1, mirror)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(THUNDER_KID), Chain(strap)], &[ChainOnAttack(mirror)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", mirror), 1, "Mirror Force is Set again: {}", s.context());
    assert_eq!(s.count(1, "grave", mirror), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Mirror Force still resolved: {}", s.context());
    assert_eq!(s.count(0, "szone", cat), 1, "Amethyst Cat came from the Deck and was destroyed with it: {}", s.context());
    assert_eq!(s.count(0, "grave", strap), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Nightshroud's Darkness monsters
// ---------------------------------------------------------------------------------------------------------------

/// Darkness Slime: "If the total ATK of all face-up monsters you control is 0, you can Special Summon this card (from your
/// hand) in Defense Position. If this card is Special Summoned this way: Target 1 face-up monster your opponent controls;
/// this card's ATK becomes equal to that monster's DEF, and its DEF becomes equal to that monster's ATK." Darkness Eye (0 ATK) on
/// my side and Warwolf (2000/100) on theirs: the Slime comes in Defense Position with 100 ATK and 2000 DEF. It is not offered
/// with an ATK on my side (Thunder Kid) or with no monster at all.
#[test]
fn darkness_slime_swaps_its_atk_and_def_with_a_monster_of_the_opponent() {
    let [slime, eye] = ["Darkness Slime", "Darkness Eye"].map(card);
    let Some(mut s) = field(&[], &[hand(0, slime), mon(0, eye), mon(1, WARWOLF)]) else { return };
    s.run(&[SpSummon(slime)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, slime), Some(4), "face-up Defense Position: {}", s.context());
    let m = s.monster(0, slime).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64()), (Some(100), Some(2000)), "ATK = their DEF, DEF = their ATK: {m}");
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    for spawns in [vec![hand(0, slime), mon(0, eye), mon(0, THUNDER_KID)], vec![hand(0, slime)]] {
        let Some(mut s) = field(&[], &spawns) else { return };
        s.run(&[]).unwrap();
        assert!(!offered(&s, "special_summons").contains(&slime.into()), "not offered: {}", s.context());
    }
}

/// Darkness Outsider: "You can discard 1 card; Special Summon 1 monster from your opponent's Deck to your field, then shuffle
/// this card into your opponent's Deck." The opponent's Deck is Dark Magicians: one comes to my field, the Outsider goes into
/// their Deck (which keeps its size: one out, one in), a card of my hand is discarded.
#[test]
fn darkness_outsider_takes_a_monster_of_the_opponents_deck_and_goes_into_it() {
    let outsider = card("Darkness Outsider");
    let Some(mut s) = duel(7, 8000, [FILLER, DARK_MAGICIAN], &[], &[mon(0, outsider)]) else { return };
    let (hand_before, deck_before, grave_before) = (hand_size(&s, 0), deck_size(&s, 1), s.state()[0]["grave"].as_array().unwrap().len());
    s.run(&[Activate(outsider)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "their Dark Magician is mine: {}", s.context());
    assert_eq!(s.count(0, "mzone", outsider), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", outsider) + s.count(1, "grave", outsider), 0, "it is in their Deck, not a Graveyard: {}", s.context());
    assert_eq!(deck_size(&s, 1), deck_before, "one card out, the Outsider in: {}", s.context());
    assert_eq!(hand_size(&s, 0), hand_before - 1, "one card discarded: {}", s.context());
    assert_eq!(s.state()[0]["grave"].as_array().unwrap().len(), grave_before + 1, "{}", s.context());
}

/// Darkness Necroslime: "You can send this card to the GY, then target 1 "Darkness" monster in your GY, except "Darkness
/// Necroslime"; Special Summon it." Darkness Bramble in my Graveyard comes to the field and the Necroslime takes its place in the
/// Graveyard; a Graveyard with a Warwolf and a second Necroslime offers nothing.
#[test]
fn darkness_necroslime_brings_back_another_darkness_monster() {
    let [necro, bramble] = ["Darkness Necroslime", "Darkness Bramble"].map(card);
    let Some(mut s) = field(&[], &[mon(0, necro), grave(0, bramble), grave(0, WARWOLF)]) else { return };
    s.run(&[Activate(necro)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", bramble), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", necro), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", necro), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[mon(0, necro), grave(0, necro), grave(0, WARWOLF)]) else { return };
    assert!(!playable(&mut s, necro), "no other Darkness monster: {}", s.context());
}

/// Darkness Raincrow: "When an opponent's monster declares an attack: You can send this card from your hand and 1 "Darkness
/// Bramble" you control to the Graveyard; Special Summon 1 "Darkness Neosphere" from your Deck." Warwolf attacks; the Raincrow
/// and Bramble go to the Graveyard and Darkness Neosphere comes from the Deck.
#[test]
fn darkness_raincrow_trades_with_a_bramble_for_a_neosphere() {
    let [raincrow, bramble, neosphere] = ["Darkness Raincrow", "Darkness Bramble", "Darkness Neosphere"].map(card);
    let Some(mut s) = field(&[neosphere], &[hand(0, raincrow), mon(0, bramble), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, ChainOnAttack(raincrow)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", raincrow), s.count(0, "grave", bramble)), (1, 1), "both are sent: {}", s.context());
    assert_eq!(s.count(0, "mzone", neosphere), 1, "{}", s.context());
}

/// Darkness Eye: "Once per turn, during your Standby Phase, if this card is in Attack Position: Once during this turn, you can
/// Normal Summon 1 monster without Tributing." In Attack Position its Standby Phase effect lets Dark Magician (Level 7) be
/// Normal Summoned with no Tribute in turn 3; in Defense Position the Eye gives nothing (Dark Magician is not offered).
#[test]
fn darkness_eye_lets_a_high_level_monster_be_summoned_without_tributes() {
    let eye = card("Darkness Eye");
    let Some(mut s) = field(&[], &[mon(0, eye), hand(0, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Chain(eye), Summon(DARK_MAGICIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "summoned with no Tribute: {}", s.context());
    assert_eq!(s.count(0, "mzone", eye), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[mon_def(0, eye), hand(0, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER), (0, DARK_MAGICIAN)]);
    s.run(&[EndTurn]).unwrap();
    own_turn(&mut s);
    assert!(!offered(&s, "summons").contains(&DARK_MAGICIAN.into()), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Darkness Bramble and Darkness Seed: 4000 LP, and at ∞ LP
// ---------------------------------------------------------------------------------------------------------------

const INFINITE: i64 = 19_999_998;
const KURIBOH: u16 = 4064;
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK

/// Team 0 at ∞ LP (Mystik Wok on the anime Geh: 8000 and ∞ is ∞, as `infinite_lp.rs` does), with `spawns` in place.
fn infinite(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(0, WOK), mon(0, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &field).expect("the duel starts");
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[]);
    Some(s)
}

/// Darkness Bramble: "Once per turn, during the End Phase, if you have less than 4000 LP: Your LP become 4000." With 2000 LP its
/// End Phase sets them to 4000; with 5000 they stay. "Cannot be destroyed by battle": Warwolf (2000) attacking it (2000 ATK) is
/// the one destroyed.
#[test]
fn darkness_bramble_sets_low_lp_to_4000_and_survives_battle() {
    let bramble = card("Darkness Bramble");
    for (lp, after) in [(2000, 4000), (3999, 4000), (4000, 4000), (5000, 5000)] {
        let Some(mut s) = duel(7, lp, [FILLER, FILLER], &[], &[mon(0, bramble)]) else { return };
        keep_hands(&mut s);
        s.run(&[EndTurn]).unwrap();
        clean(&s);
        assert_eq!(s.lp(0), after, "{lp} LP: {}", s.context());
    }
    let Some(mut s) = field(&[], &[mon(0, bramble), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", bramble), 1, "indestructible by battle: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Darkness Bramble at ∞ LP ("works an amount out from LP": less than 4000 is a comparison of the LP): ∞ is more than 4000,
/// so the End Phase leaves it as it is.
#[test]
fn darkness_bramble_leaves_infinite_lp_alone() {
    let bramble = card("Darkness Bramble");
    let Some(mut s) = infinite(&[mon(0, bramble)]) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    assert_eq!(s.count(0, "mzone", bramble), 1, "{}", s.context());
}

/// Darkness Seed: "If this card is sent to the Graveyard: you can activate this effect; Special Summon this card during your
/// second Standby Phase after activation. If this card is Special Summoned this way, it gains the following effects. ● This card
/// cannot be destroyed by battle. ● Once per turn, during the End Phase, if you have less than 4000 LP: Your LP become 4000."
/// Dark Hole destroys the Seed in turn 1 (2000 LP): it is not back after turn 3's Standby Phase, comes back in turn 5's, and
/// that End Phase sets the LP to 4000; the LP stays 2000 until then.
#[test]
fn darkness_seed_returns_in_the_second_standby_phase_and_sets_the_lp() {
    let seed = card("Darkness Seed");
    let Some(mut s) = duel(7, 2000, [FILLER, FILLER], &[], &[mon(0, seed), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE), Chain(seed)]).unwrap();
    assert_eq!(s.count(0, "grave", seed), 1, "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", seed), s.lp(0)), (0, 2000), "not yet, after the first Standby Phase: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", seed), 1, "back in the second Standby Phase: {}", s.context());
    assert_eq!(s.lp(0), 4000, "the End Phase of that turn: {}", s.context());
}

/// Darkness Seed at ∞ LP: the Seed Tributed to Mystik Wok (the Wok's LP gain on ∞ stays ∞) returns in turn 5 and its End Phase
/// leaves the LP at ∞, since ∞ is not less than 4000.
#[test]
fn darkness_seed_leaves_infinite_lp_alone() {
    let seed = card("Darkness Seed");
    let Some(mut s) = infinite(&[mon(0, seed), hand(0, WOK)]) else { return };
    s.prefer(&[(0, seed)]);
    s.run(&[Activate(WOK), Chain(seed)]).unwrap();
    assert_eq!((s.count(0, "grave", seed), s.lp(0)), (1, INFINITE), "{}", s.context());
    s.prefer(&[]);
    for _ in 0..3 {
        s.run(&[EndTurn]).unwrap();
    }
    clean(&s);
    assert_eq!(s.count(0, "mzone", seed), 1, "back in turn 5: {}", s.context());
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
}

/// Answers prompts until team 0's next idle prompt: in a chain prompt that is forced, takes `first` the first time; a place
/// prompt (a zone of the Spell & Trap Zone for "Activate 1 Set card") takes sequence `zone`.
fn drive(s: &mut Scenario, mut first: Option<u16>, zone: Option<u32>) {
    for _ in 0..60 {
        let p = s.prompt().clone();
        if p["type"] == msg::SELECT_IDLECMD && p["player"] == 0 {
            return;
        }
        if p["type"] == msg::SELECT_CHAIN
            && p["forced"] == true
            && let Some(c) = first
            && let Some(i) = p["selects"].as_array().and_then(|v| v.iter().position(|x| x["cid"] == c))
        {
            first = None;
            s.respond(yo3_server::ocg::Response::Chain(Some(i)).json()).unwrap();
            continue;
        }
        if p["type"] == msg::SELECT_PLACE {
            let seq = zone.expect("a zone was asked");
            s.respond(yo3_server::ocg::Response::Place(vec![serde_json::json!({"player": 0, "location": 8, "sequence": seq})]).json()).unwrap();
            continue;
        }
        s.pass().unwrap();
    }
    panic!("no idle prompt: {}", s.context());
}

/// The Spell & Trap Zone of a team as (cid, position) in zone order (the Field Zone last).
fn szone(s: &Scenario, team: usize) -> Vec<(u64, u64)> {
    s.state()[team]["szone"].as_array().unwrap().iter().filter(|c| !c.is_null()).map(|c| (c["cid"].as_u64().unwrap(), c["position"].as_u64().unwrap())).collect()
}

fn darkness_cards() -> [u16; 6] {
    ["Darkness", "Zero", "Infinity", "Darkness 1", "Darkness 2", "Darkness 3"].map(card)
}

/// Darkness: "When this card is activated: Destroy all cards in your Spell & Trap Zones, also, after that, Set 1 "Zero",
/// "Infinity", "Darkness 1", "Darkness 2", and "Darkness 3" at random from your hand or Deck." The five are at the bottom of the
/// Deck: my face-up Dian Keto and my Set Fire Trap are destroyed first, then the five come Set (face-down) in the five zones,
/// each once, with the Field Spell beside them. With one of them missing the Spell is not offered.
#[test]
fn darkness_destroys_my_spells_and_traps_and_sets_the_five_cards() {
    let [dark, zero, inf, d1, d2, d3] = darkness_cards();
    let fire_trap = card("Fire Trap");
    let Some(mut s) = field(&[zero, inf, d1, d2, d3], &[hand(0, dark), spell(0, DIAN_KETO), set(0, fire_trap)]) else { return };
    s.run(&[Activate(dark)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO) + s.count(0, "grave", fire_trap), 2, "both were destroyed: {}", s.context());
    let zones = szone(&s, 0);
    let mut cids: Vec<u64> = zones.iter().map(|z| z.0).collect();
    cids.sort_unstable();
    let mut want: Vec<u64> = [dark, zero, inf, d1, d2, d3].iter().map(|&c| c as u64).collect();
    want.sort_unstable();
    assert_eq!(cids, want, "the five and the Field Spell: {zones:?}");
    for (cid, position) in zones {
        let face_down = cid != dark as u64;
        assert_eq!(position == 10, face_down, "{cid}: Set but for the Field Spell: {}", s.context());
    }
    let Some(mut s) = field(&[zero, inf, d1, d2], &[hand(0, dark)]) else { return };
    assert!(!playable(&mut s, dark), "no Darkness 3: {}", s.context());
}

/// Darkness: "Once per turn, during the End Phase: Set all face-up Traps you control, and randomly rearrange all cards in your
/// Spell & Trap Card Zone." A face-up Zero is Set again by the End Phase and the Set cards end in different orders in different
/// duels.
#[test]
fn darkness_sets_the_face_up_traps_again_and_rearranges_at_the_end_phase() {
    let [dark, zero, _, d1, d2, d3] = darkness_cards();
    let mut orders = std::collections::HashSet::new();
    for seed in 1..=8 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[spell(0, dark), spell(0, zero), set(0, d1), set(0, d2), set(0, d3)]) else { return };
        keep_hands(&mut s);
        assert_eq!(szone(&s, 0).iter().find(|z| z.0 == zero as u64).map(|z| z.1), Some(1), "face-up to start with: {}", s.context());
        s.run(&[EndTurn]).unwrap();
        clean(&s);
        let zones = szone(&s, 0);
        assert!(zones.iter().all(|&(cid, position)| (position == 10) == (cid != dark as u64)), "every Trap is Set: {zones:?}");
        orders.insert(zones.iter().map(|z| z.0).collect::<Vec<_>>());
    }
    assert!(orders.len() > 1, "the order changes with the seed: {orders:?}");
}

/// Darkness: "If a Spell/Trap you control leaves the field, destroy all Spell/Traps you control." The opponent's Mystical Space
/// Typhoon destroys my Set Dian Keto: my Set Zero and Darkness itself are destroyed with it.
#[test]
fn darkness_destroys_all_my_spells_and_traps_when_one_leaves_the_field() {
    let [dark, zero, ..] = darkness_cards();
    let mst = card("Mystical Space Typhoon");
    let Some(mut s) = field(&[], &[spell(0, dark), set(0, DIAN_KETO), set(0, zero), hand(1, mst)]) else { return };
    s.prefer(&[(0, DIAN_KETO)]);
    s.run_both([&[EndTurn], &[Activate(mst)]]).unwrap();
    clean(&s);
    assert_eq!(szone(&s, 0), vec![], "nothing is left: {}", s.context());
    assert_eq!(s.count(0, "grave", zero) + s.count(0, "grave", dark) + s.count(0, "grave", DIAN_KETO), 3, "{}", s.context());
}

/// Zero and Infinity, one of them face-up: "If you control "Darkness": apply the appropriate effect. If you do not control the
/// other: Activate 1 Set card in your Spell & Trap Card Zone." Zero activates the Set Darkness 2 I pick by its zone, which comes
/// face-up; the other Set cards stay Set. Without Darkness the effect is not offered. Infinity does the same.
#[test]
fn zero_and_infinity_each_activate_one_set_card() {
    let [dark, zero, inf, d1, d2, d3] = darkness_cards();
    for lone in [zero, inf] {
        let spawns = [spell(0, dark), spell(0, lone), set(0, d1), set(0, d2), set(0, d3), mon(0, THUNDER_KID)];
        let Some(mut s) = field(&[], &spawns) else { return };
        s.run(&[]).unwrap();
        let place = |s: &Scenario, cid: u16| s.state()[0]["szone"].as_array().unwrap().iter().position(|c| c["cid"] == cid).unwrap() as u32;
        let (seq_d2, lone_position) = (place(&s, d2), place(&s, lone));
        let idx = s.prompt()["activates"].as_array().unwrap().iter().position(|c| c["cid"] == lone).expect("its effect is offered");
        s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::ACTIVATE, "index": idx})).unwrap();
        drive(&mut s, None, Some(seq_d2));
        clean(&s);
        let zones = szone(&s, 0);
        let position = |cid: u16| zones.iter().find(|z| z.0 == cid as u64).map(|z| z.1);
        assert_eq!(position(d2), Some(5), "Darkness 2 was activated: {zones:?}");
        assert_eq!((position(d1), position(d3)), (Some(10), Some(10)), "the others stay Set: {zones:?}");
        assert_eq!(lone_position as usize, place(&s, lone) as usize, "{}", s.context());
        assert_eq!(s.attack_of(0, THUNDER_KID), Some(700), "Darkness 2 alone does nothing: {}", s.context());
        // Without Darkness on the field the effect is not offered.
        let Some(mut s) = field(&[], &[spell(0, lone), set(0, d1)]) else { return };
        s.run(&[]).unwrap();
        assert!(!offered(&s, "activates").contains(&lone.into()), "{:?}", offered(&s, "activates"));
    }
}

/// Zero and Infinity both face-up with "Darkness": "Activate all Set Spell and Trap Cards you control between this card and
/// the other." Darkness 1, 2 and 3 lie between them and all three are activated; the first one activated is the first card of the
/// chain and takes the bonus of the others: "Destroy 1 card your opponent controls, then destroy 1 card for each "Darkness"
/// Continuous Trap activated after this card" (3 cards), "1 monster gains 1000 ATK, then 1 gains 1000 more for each" (+3000), or
/// "inflict 1000 damage, then 1000 for each" (3000). The two others do nothing but count.
#[test]
fn zero_and_infinity_activate_everything_between_them_and_the_first_card_takes_the_bonus() {
    let [dark, zero, inf, d1, d2, d3] = darkness_cards();
    for first in [d1, d2, d3] {
        let spawns = [spell(0, dark), spell(0, zero), set(0, d1), set(0, d2), set(0, d3), spell(0, inf), mon(0, THUNDER_KID),
            mon(1, WARWOLF), mon(1, DARK_MAGICIAN), mon(1, FIREYAROU), mon(1, FILLER)];
        let Some(mut s) = field(&[], &spawns) else { return };
        s.run(&[]).unwrap();
        let idx = s.prompt()["activates"].as_array().unwrap().iter().position(|c| c["cid"] == zero).unwrap();
        s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::ACTIVATE, "index": idx})).unwrap();
        drive(&mut s, Some(first), None);
        clean(&s);
        let zones = szone(&s, 0);
        assert!([d1, d2, d3].iter().all(|&d| zones.contains(&(d as u64, 5))), "all three were activated: {zones:?}");
        let theirs = s.state()[1]["mzone"].as_array().unwrap().iter().filter(|c| !c.is_null()).count();
        let (destroyed, atk, damage) = (4 - theirs, s.attack_of(0, THUNDER_KID).unwrap() - 700, 8000 - s.lp(1));
        let expected = |card: u16, bonus: i64| if card == first { bonus } else { 0 };
        assert_eq!(destroyed as i64, expected(d1, 3), "first {first}: {}", s.context());
        assert_eq!(atk, expected(d2, 3000), "first {first}: {}", s.context());
        assert_eq!(damage, expected(d3, 3000), "first {first}: {}", s.context());
    }
}
