//! Anime-only cards, group 14 (rows 5658-5692 of engine/added/cards.tsv): Alice's Doll cards, Atticus Rhodes', Syrus
//! Truesdale's and Bastion Misawa's. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play Spell answering the
//! opponent is Set first (a Set card is ready in turn 2, for the other side's turn 3 effects), and neither player has a
//! Battle Phase in turn 1, so team 0's attacks wait for turn 3. A Trap is `type 0x4` in the data: Cyber Repairer, Dark
//! Computer Virus, Life Force, Mirror Damage, Metal Plus, Magnet Force Plus and Minus, Ring of Life and One Hundred Thousand
//! Gauss are Traps here, Remove Bomb and Magnet Conductor Plus Spells. The duels' Decks are shuffled by the seed, so
//! a card is put in a Deck only to be fetched, never at a place in it (Remove Bomb takes whatever the top five are).
//! Covered: the cards the per-card fuzz flagged (Alice the Wandering Doll, Doll Chimera, Marionette Burial, Necro Dollmeister,
//! Released Curse, Bronze Warrior, Dragon Heart, Miracle Moment, Path to Destiny, Dark Computer Virus, Amorphous Barrier,
//! Magnet Conductor Plus, One Hundred Thousand Gauss), the cards `--find` noted (Dragon Heart, Last Magnet, Magnet Force
//! Minus and Plus: no Lua error can be reached on any of them, for a Dragon Heart whose target is destroyed in the chain,
//! a Last Magnet with both monsters destroyed or a Magnet Force whose monster dies; Magnet Force Plus has an override,
//! below), the signature cards (Alice, Doll Chimera, Marionette Burial, Cursed Dollhouse, Eshila, Door to Mirrorland, Training
//! Wheels and Training Field, the Magnet Forces, Spotlight, Ultimate Stage Costume) and a seeded sample (Cursed Ring, Cyber
//! Repairer, Doll Part Pink, Metal Plus, Ring of Life), with Life Force, Mirror Damage and Remove Bomb on the way. The batch
//! makes no Token.
//! Two overrides: Magnet Force Plus (`engine/overrides/c170000122.lua`: upstream's made its monster a Minus, so two Minus
//! monsters negated each other's attacks) and Spotlight (`c511000446.lua`: its 300 ATK goes to whichever monster it
//! targets, so Dark Computer Virus moves it along with the target).
//! Not coverable: Power Off. It sends a Plus Minus monster that was Special Summoned with materials; the only one the
//! client has is Conduction Warrior Linear Magnum Plus Minus, whose materials (Magnet Warrior Sigma Plus and Omega Minus,
//! passcodes 170000111-2) are no card the client has, so no duel can bring it onto the field with any: what is tested is
//! that it is not offered. Magnet Conductor Plus and One Hundred Thousand Gauss use that Conduction Warrior as their Plus
//! (and Minus) monster, put on the field or in the Graveyard by the test.
use yo3_server::duelcheck::scenario::{Action, Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg};

// MSG_CHAINED: a card or effect was just put on the chain.
const CHAINED: u64 = 71;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: team 0's has `bottom` at its very
/// bottom (out of the opening hand and the first draws), for a card an effect fetches from the Deck.
fn duel(seed: u64, lp: u32, bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(FILLER, 40 - bottom.len())).collect();
    let decks = [(&mine[..], &[][..]), (&[FILLER; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, bottom, spawns) }

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

/// Plays `plan` for team 0 and, at the first chance `team` gets with `cid` on offer to answer a card that was just
/// activated (a chain link was started in the last step), chains `cid`. Chains offered before that (a Set card is
/// offered at every chance) are passed.
fn chain_to_activation(s: &mut Scenario, plan: &[Action], team: u64, cid: u16) {
    let asks = move |p: &serde_json::Value| p["type"] == msg::SELECT_CHAIN && p["player"] == team;
    s.run_to(plan, asks).unwrap();
    for _ in 0..40 {
        let started = s.messages().iter().any(|m| m["type"] == CHAINED);
        let at = s.prompt()["selects"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == cid));
        if let (true, Some(at)) = (started, at) {
            s.respond(Response::Chain(Some(at)).json()).unwrap();
            return;
        }
        s.pass().unwrap();
        s.run_to(plan, asks).unwrap();
    }
    panic!("{cid} was never offered against an activation: {}", s.context());
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn deck_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["deck"].as_u64().map_or(0, |n| n as usize)
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

// ---------------------------------------------------------------------------------------------------------------
// Alice the Wandering Doll
// ---------------------------------------------------------------------------------------------------------------

/// Alice the Wandering Doll: "Cannot be destroyed by battle. When this card is targeted for an attack: Inflict 500 damage
/// to your opponent, then switch control of this card at the end of the Damage Step." Warwolf (2000) attacks Alice (300 ATK):
/// Alice's controller takes 1700 (the attack is a battle all the same), the attacker's takes 500, Alice is not destroyed
/// and ends up on the attacker's side.
#[test]
fn alice_hurts_the_attacker_and_changes_sides_after_the_battle() {
    let alice = card("Alice the Wandering Doll");
    let Some(mut s) = field(&[], &[mon(0, alice), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (6300, 7500), "{}", s.context());
    assert_eq!(s.count(0, "mzone", alice), 0, "{}", s.context());
    assert_eq!(s.count(1, "mzone", alice), 1, "Alice switched sides: {}", s.context());
    assert_eq!(s.count(0, "grave", alice), 0, "{}", s.context());
}

/// Alice with nowhere to go: team 1 has five monsters, so control of Alice can't change and she is destroyed instead ("if
/// control can't change, destroy"). Released Curse, Set by Alice's owner: "When an "Alice the Wandering Doll" is destroyed:
/// Destroy all cards on the field, and if you do, inflict 300 damage to their owners for each card destroyed." The five
/// monsters and the Spell Set by team 0 go (Released Curse itself stays): 1500 to team 1 on top of Alice's 500, 300 to team 0.
#[test]
fn released_curse_destroys_every_card_and_hurts_their_owners_when_alice_is_destroyed() {
    let [alice, curse] = ["Alice the Wandering Doll", "Released Curse"].map(card);
    let spawns = [mon(0, alice), set(0, curse), set(0, DIAN_KETO), mon(1, WARWOLF), mon(1, DARK_BLADE), mon(1, DARK_BLADE), mon(1, DARK_BLADE), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[Chain(curse)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", alice), 1, "Alice was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", curse), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "{}", s.context());
    for (cid, n) in [(WARWOLF, 1), (DARK_BLADE, 4)] {
        assert_eq!(s.count(1, "grave", cid), n, "{}", s.context());
    }
    // Warwolf's 2000 against Alice's 300 hurt team 0 by 1700; Alice's 500, then 300 for each of five cards, hurt team 1.
    assert_eq!(s.lp(0), 8000 - 1700 - 300, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500 - 5 * 300, "{}", s.context());
}

/// Released Curse needs an Alice: Dark Hole destroying only a Warwolf leaves it unanswered (it is not even offered).
#[test]
fn released_curse_waits_for_alice() {
    let curse = card("Released Curse");
    let Some(mut s) = field(&[], &[set(0, curse), mon(1, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!(s.count(0, "szone", curse), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Doll Chimera
// ---------------------------------------------------------------------------------------------------------------

/// Doll Chimera: "Gains 400 ATK for each "Doll Part" monster in your Graveyard." Two Parts in its controller's Graveyard
/// make 800, a third in the opponent's none.
#[test]
fn doll_chimera_gains_400_for_each_doll_part_in_its_controllers_graveyard() {
    let [chimera, gold, blue, pink] = ["Doll Chimera", "Doll Part Gold", "Doll Part Blue", "Doll Part Pink"].map(card);
    let Some(s) = field(&[], &[mon(0, chimera), grave(0, gold), grave(0, blue), grave(1, pink)]) else { return };
    clean(&s);
    assert_eq!(s.attack_of(0, chimera), Some(800), "{}", s.context());
}

/// Doll Chimera: "If this card is destroyed and sent to the Graveyard: You can send 2 "Doll Part" monsters from your Deck
/// to the Graveyard; Special Summon this card." Dark Hole destroys it; Blue and Red leave the Deck and it comes back with
/// the 800 they give it. Marionette Burial's "cannot be Special Summoned by other ways" holds: Monster Reborn is not
/// offered for a Chimera in the Graveyard.
#[test]
fn doll_chimera_comes_back_when_destroyed_and_cannot_be_reborn() {
    let [chimera, blue, red] = ["Doll Chimera", "Doll Part Blue", "Doll Part Red"].map(card);
    let Some(mut s) = field(&[blue, red], &[mon(0, chimera), hand(0, DARK_HOLE)]) else { return };
    let deck = deck_size(&s, 0);
    s.run(&[Activate(DARK_HOLE), Chain(chimera)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", chimera), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", blue), s.count(0, "grave", red)), (1, 1), "{}", s.context());
    assert_eq!(deck_size(&s, 0), deck - 2, "{}", s.context());
    assert_eq!(s.attack_of(0, chimera), Some(800), "{}", s.context());
    // Monster Reborn does not take a Chimera out of the Graveyard.
    let Some(mut s) = field(&[], &[grave(0, chimera), hand(0, MONSTER_REBORN)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&MONSTER_REBORN.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Marionette Burial
// ---------------------------------------------------------------------------------------------------------------

/// Marionette Burial: "Activate only while you control a face-up "Alice the Wandering Doll": Send 1 face-up "Doll Part
/// Blue", "Doll Part Red", "Doll Part Gold" and "Doll Part Pink" you control to the Graveyard. Special Summon 1 "Doll
/// Chimera" from your Deck." Alice stays; the four Parts go to the Graveyard; the Chimera arrives with 1600 ATK.
#[test]
fn marionette_burial_turns_the_four_doll_parts_into_a_chimera() {
    let [alice, burial, chimera, gold, blue, pink, red] =
        ["Alice the Wandering Doll", "Marionette Burial", "Doll Chimera", "Doll Part Gold", "Doll Part Blue", "Doll Part Pink", "Doll Part Red"].map(card);
    let spawns = [mon(0, alice), mon(0, gold), mon(0, blue), mon(0, pink), mon(0, red), hand(0, burial)];
    let Some(mut s) = field(&[chimera], &spawns) else { return };
    s.run(&[Activate(burial)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", chimera), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", alice), 1, "{}", s.context());
    for part in [gold, blue, pink, red] {
        assert_eq!((s.count(0, "grave", part), s.count(0, "mzone", part)), (1, 0), "{}", s.context());
    }
    assert_eq!(s.count(0, "grave", burial), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, chimera), Some(1600), "{}", s.context());
    // Without Alice the card can't be activated, with every Part on the field.
    let spawns = [mon(0, gold), mon(0, blue), mon(0, pink), mon(0, red), hand(0, burial)];
    let Some(mut s) = field(&[chimera], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&burial.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Necro Dollmeister
// ---------------------------------------------------------------------------------------------------------------

/// Necro Dollmeister: "When a "Doll Part" monster you control is destroyed by battle: Special Summon 2 "Doll Part"
/// monsters from your Deck in face-up Attack Position." Warwolf destroys Doll Part Gold (0 ATK); Blue and Pink arrive.
#[test]
fn necro_dollmeister_brings_two_doll_parts_when_one_is_destroyed_by_battle() {
    let [dollmeister, gold, blue, pink] = ["Necro Dollmeister", "Doll Part Gold", "Doll Part Blue", "Doll Part Pink"].map(card);
    let Some(mut s) = field(&[blue, pink], &[mon(0, gold), set(0, dollmeister), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Chain(dollmeister)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", gold), 1, "{}", s.context());
    for part in [blue, pink] {
        assert_eq!(s.count(0, "mzone", part), 1, "{}", s.context());
        assert_eq!(s.position_of(0, part), Some(1), "face-up Attack: {}", s.context());
    }
    assert_eq!(s.count(0, "grave", dollmeister), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Cursed Dollhouse
// ---------------------------------------------------------------------------------------------------------------

/// Cursed Dollhouse: "When your opponent takes control of an "Alice the Wandering Doll" you control: You can Special Summon
/// 1 "Doll Part" monster from your Deck." Team 1's Warwolf attacks Alice, who changes sides: Doll Part Pink comes.
#[test]
fn cursed_dollhouse_brings_a_doll_part_when_the_opponent_takes_alice() {
    let [alice, house, pink] = ["Alice the Wandering Doll", "Cursed Dollhouse", "Doll Part Pink"].map(card);
    let Some(mut s) = field(&[pink], &[mon(0, alice), hand(0, house), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Activate(house), EndTurn, Chain(house)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", alice), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", pink), 1, "{}", s.context());
}

/// Cursed Dollhouse: "You also choose the targets for your opponent's attacks." With Alice and Dark Magician on team 0's
/// side, Warwolf's attack target is picked by team 0 (Dark Magician: Warwolf is destroyed); without the Dollhouse team 1
/// picks and team 0 is never asked.
#[test]
fn cursed_dollhouse_lets_its_controller_choose_the_targets_of_attacks() {
    let [alice, house] = ["Alice the Wandering Doll", "Cursed Dollhouse"].map(card);
    // Whether team 0 answers a card choice right after team 1's battle command (the attack target), not the End Phase's discard.
    let asked_team_0 = |s: &Scenario| {
        let kinds = &s.prompts;
        kinds.windows(2).any(|w| w[0] == (1, yo3_server::ocg::msg::SELECT_BATTLECMD) && w[1] == (0, yo3_server::ocg::msg::SELECT_CARD))
    };
    let Some(mut s) = field(&[], &[mon(0, alice), mon(0, DARK_MAGICIAN), hand(0, house), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run_both([&[Activate(house), EndTurn], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(asked_team_0(&s), "team 0 chose the target: {:?}", s.prompts);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf attacked Dark Magician: {}", s.context());
    assert_eq!(s.count(0, "mzone", alice), 1, "Alice was not attacked: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, alice), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run_both([&[EndTurn], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    assert!(!asked_team_0(&s), "without the Dollhouse team 1 chooses: {:?}", s.prompts);
}

// ---------------------------------------------------------------------------------------------------------------
// Eshila the Lovely Bisque Doll, Door to Mirrorland and the Doll Parts
// ---------------------------------------------------------------------------------------------------------------

/// Eshila the Lovely Bisque Doll: "This card gains 1000 ATK for every 4 "Doll Part" monsters in your Graveyard with
/// different names." Three different Parts and a second Gold: still 1000 (no set of four); Red makes four names, 2000;
/// two of each of the four names make two sets, 3000; the opponent's Parts count for nothing.
#[test]
fn eshila_gains_1000_for_every_four_different_doll_parts_in_the_graveyard() {
    let [eshila, gold, blue, pink, red] = ["Eshila the Lovely Bisque Doll", "Doll Part Gold", "Doll Part Blue", "Doll Part Pink", "Doll Part Red"].map(card);
    let Some(s) = field(&[], &[mon(0, eshila), grave(0, gold), grave(0, gold), grave(0, blue), grave(0, pink)]) else { return };
    clean(&s);
    assert_eq!(s.attack_of(0, eshila), Some(1000), "three names, no four: {}", s.context());
    let Some(s) = field(&[], &[mon(0, eshila), grave(0, gold), grave(0, gold), grave(0, blue), grave(0, pink), grave(0, red)]) else { return };
    assert_eq!(s.attack_of(0, eshila), Some(2000), "four names: {}", s.context());
    let eight = [mon(0, eshila), grave(0, gold), grave(0, gold), grave(0, blue), grave(0, blue), grave(0, pink), grave(0, pink), grave(0, red), grave(0, red)];
    let Some(s) = field(&[], &eight) else { return };
    assert_eq!(s.attack_of(0, eshila), Some(3000), "two sets of four: {}", s.context());
    let theirs = [mon(0, eshila), grave(1, gold), grave(1, blue), grave(1, pink), grave(1, red)];
    let Some(s) = field(&[], &theirs) else { return };
    assert_eq!(s.attack_of(0, eshila), Some(1000), "the opponent's Parts don't count: {}", s.context());
}

/// Door to Mirrorland: "Activate only if "Alice the Wandering Doll" is in your Graveyard: Special Summon 1 "Eshila the
/// Lovely Bisque Doll" from your hand or Deck." Eshila comes from the Deck; with Alice not in the Graveyard the card can't
/// be activated.
#[test]
fn door_to_mirrorland_brings_eshila_when_alice_is_in_the_graveyard() {
    let [alice, door, eshila] = ["Alice the Wandering Doll", "Door to Mirrorland", "Eshila the Lovely Bisque Doll"].map(card);
    let Some(mut s) = field(&[eshila], &[grave(0, alice), hand(0, door)]) else { return };
    s.run(&[Activate(door)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", eshila), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, eshila), Some(1000), "{}", s.context());
    assert_eq!(s.count(0, "grave", door), 1, "{}", s.context());
    let Some(mut s) = field(&[eshila], &[mon(0, alice), hand(0, door)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&door.into()), "{:?}", offered(&s, "activates"));
}

/// Doll Part Pink ("Broken doll body", a seeded sample): a Level 1 Spellcaster with no ATK or DEF, Normal Summoned; in the
/// Graveyard it is a "Doll Part" for Doll Chimera (400 ATK), on the field it is none (the Chimera stays at 0).
#[test]
fn doll_part_pink_is_a_level_1_spellcaster_with_no_stats_and_counts_as_a_doll_part() {
    let [pink, chimera] = ["Doll Part Pink", "Doll Chimera"].map(card);
    let Some(mut s) = field(&[], &[hand(0, pink), mon(0, chimera), mon(1, DARK_MAGICIAN)]) else { return };
    s.run(&[Summon(pink)]).unwrap();
    clean(&s);
    let m = s.monster(0, pink).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64()), (Some(0), Some(0), Some(1)), "{}", s.context());
    let spellcaster = s.monster(1, DARK_MAGICIAN).unwrap()["race"].clone();
    assert_eq!(m["race"], spellcaster, "Spellcaster: {}", s.context());
    assert_eq!(s.attack_of(0, chimera), Some(0), "{}", s.context());
    let Some(s) = field(&[], &[mon(0, chimera), grave(0, pink)]) else { return };
    assert_eq!(s.attack_of(0, chimera), Some(400), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Bronze Warrior, Path to Destiny and Miracle Moment
// ---------------------------------------------------------------------------------------------------------------

/// Bronze Warrior: "Once per turn: You can add 1 Beast-Warrior-Type monster from your Deck to your hand. While this
/// face-up card is in Attack Position, all Beast-Warrior-Type monsters gain 400 ATK." Panther Warrior (2000) is added once
/// (the second use is not offered); with Bronze Warrior in Attack Position both players' Panther Warriors have 2400, in
/// Defense Position 2000.
#[test]
fn bronze_warrior_searches_a_beast_warrior_once_and_powers_all_of_them_in_attack_position() {
    let [bronze, panther] = ["Bronze Warrior", "Panther Warrior"].map(card);
    let Some(mut s) = field(&[panther], &[mon(0, bronze), mon(0, panther), mon(1, panther)]) else { return };
    assert_eq!((s.attack_of(0, panther), s.attack_of(1, panther)), (Some(2400), Some(2400)), "{}", s.context());
    assert_eq!(s.attack_of(0, bronze), Some(500), "{}", s.context());
    s.run(&[Activate(bronze)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", panther), 1, "Panther Warrior was added: {}", s.context());
    assert!(!offered(&s, "activates").contains(&bronze.into()), "once per turn: {:?}", offered(&s, "activates"));
    s.run(&[Reposition(bronze)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, bronze), Some(4), "{}", s.context());
    assert_eq!((s.attack_of(0, panther), s.attack_of(1, panther)), (Some(2000), Some(2000)), "{}", s.context());
}

/// Path to Destiny: "Equip only to a Beast-Warrior-Type monster. It can attack directly." Panther Warrior (which attacks
/// by Tributing another monster, Dark Blade) can attack directly with it while Dark Magician stands in front of it; without
/// it the battle prompt offers no direct attack.
#[test]
fn path_to_destiny_lets_a_beast_warrior_attack_directly() {
    let [path, panther] = ["Path to Destiny", "Panther Warrior"].map(card);
    let can_direct = |s: &Scenario| s.prompt()["attacks"].as_array().unwrap().iter().any(|a| a["cid"] == panther && a["can_direct"] == true);
    let battle = |p: &serde_json::Value| p["type"] == yo3_server::ocg::msg::SELECT_BATTLECMD;
    let Some(mut s) = field(&[], &[mon(0, panther), mon(0, DARK_BLADE), hand(0, path), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_to(&[Activate(path), EndTurn, Attack(panther)], battle).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", path), 1, "equipped: {}", s.context());
    assert!(can_direct(&s), "{}", s.prompt());
    let Some(mut s) = field(&[], &[mon(0, panther), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_to(&[EndTurn, Attack(panther)], battle).unwrap();
    assert!(!can_direct(&s), "{}", s.prompt());
}

/// Path to Destiny goes only on a Beast-Warrior: with Dark Blade as the only monster it can't be activated.
#[test]
fn path_to_destiny_needs_a_beast_warrior() {
    let path = card("Path to Destiny");
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), hand(0, path)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&path.into()), "{:?}", offered(&s, "activates"));
}

/// Miracle Moment: "When a "Panther Warrior" you control equipped with "Path to Destiny" is destroyed by battle: Special
/// Summon 1 "Bronze Warrior" from your hand or Deck." Dark Magician (2500) destroys the Panther Warrior (2000) carrying
/// the Path: Bronze Warrior comes from the Deck.
#[test]
fn miracle_moment_brings_bronze_warrior_when_the_panther_with_the_path_is_destroyed() {
    let [path, panther, moment, bronze] = ["Path to Destiny", "Panther Warrior", "Miracle Moment", "Bronze Warrior"].map(card);
    let Some(mut s) = field(&[bronze], &[mon(0, panther), hand(0, path), set(0, moment), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[Activate(path), EndTurn, Chain(moment)], &[EndTurn, Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", panther), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", bronze), 1, "Bronze Warrior came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", moment), 1, "{}", s.context());
}

/// Miracle Moment answers only a Panther Warrior with the Path: a bare one destroyed by the same attack leaves it Set.
#[test]
fn miracle_moment_ignores_a_panther_without_the_path() {
    let [panther, moment, bronze] = ["Panther Warrior", "Miracle Moment", "Bronze Warrior"].map(card);
    let Some(mut s) = field(&[bronze], &[mon(0, panther), set(0, moment), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn, Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", panther), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", bronze), 0, "{}", s.context());
    assert_eq!(s.count(0, "szone", moment), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Spotlight, Ultimate Stage Costume and Dark Computer Virus
// ---------------------------------------------------------------------------------------------------------------

/// Spotlight: "Activate this card by targeting 1 monster you control; it gains 300 ATK. If the targeted monster leaves the
/// field: Target 1 monster you control, otherwise, destroy this card." Warwolf (2300 with it) loses to Dark Magician's
/// attack (200 damage) and the Spotlight moves to the Dark Blade: 2100.
#[test]
fn spotlight_gives_300_and_moves_to_another_monster_when_the_target_leaves() {
    let spot = card("Spotlight");
    let Some(mut s) = field(&[], &[hand(0, spot), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(spot)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, DARK_BLADE)), (Some(2300), Some(1800)), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn, Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", spot), 1, "Spotlight stays: {}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(2100), "Dark Blade got the 300: {}", s.context());
    assert_eq!(s.lp(0), 7800, "{}", s.context());
}

/// Spotlight: with no monster of its own left to target it is destroyed. Dark Hole takes both sides' monsters.
#[test]
fn spotlight_is_destroyed_when_its_controller_has_no_monster_left() {
    let spot = card("Spotlight");
    let Some(mut s) = field(&[], &[hand(0, spot), mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    s.run_both([&[Activate(spot), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", spot), 1, "{}", s.context());
}

/// Ultimate Stage Costume: "Activate this card by targeting 1 monster you control; it gains 3000 ATK/DEF. If that monster
/// attacks, its ATK/DEF becomes its previous ATK/DEF when it was targeted during the Damage Step only." Warwolf has 5000/3100;
/// attacking Dark Magician (2500) it has 2000 in the Damage Step and is destroyed (500 damage), where 5000 would have won.
#[test]
fn ultimate_stage_costume_gives_3000_but_not_in_the_damage_step_of_the_attack() {
    let costume = card("Ultimate Stage Costume");
    let Some(mut s) = field(&[], &[hand(0, costume), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    s.run(&[Activate(costume)]).unwrap();
    clean(&s);
    let m = s.monster(0, WARWOLF).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64()), (Some(5000), Some(3100)), "{}", s.context());
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf fought with 2000: {}", s.context());
    assert_eq!(s.lp(0), 7500, "{}", s.context());
}

/// Ultimate Stage Costume: "If that monster leaves the field: Target 1 monster you control; apply this card's effects to
/// it." Warwolf is destroyed attacking Dark Magician (with its own 2000) and the Dark Blade takes the 3000 (4800 ATK).
#[test]
fn ultimate_stage_costume_moves_to_another_monster_when_its_target_leaves() {
    let costume = card("Ultimate Stage Costume");
    let Some(mut s) = field(&[], &[hand(0, costume), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Activate(costume), EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(4800), "{}", s.context());
    assert_eq!(s.monster(0, DARK_BLADE).unwrap()["defense"].as_i64(), Some(4500), "{}", s.context());
}

/// Dark Computer Virus: "Tribute 1 DARK Machine-Type monster; Change the target of 1 Continuous Spell to another
/// appropriate target." Mechanicalchaser (DARK Machine) is Tributed; Spotlight, which targeted Warwolf, now targets the
/// Dark Blade: the client's hover marker moves from one to the other and so does the 300 ATK (an override of Spotlight:
/// upstream's stayed on Warwolf).
#[test]
fn dark_computer_virus_changes_the_target_of_a_continuous_spell() {
    let [spot, virus, chaser] = ["Spotlight", "Dark Computer Virus", "Mechanicalchaser"].map(card);
    let spawns = [hand(0, spot), set(0, virus), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(0, chaser)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, chaser)]);
    s.run(&[Activate(spot)]).unwrap();
    // The Spotlight's Spell & Trap zone (client zone 7) to the first Monster Zone, both ways, marker type 8.
    assert!(s.markers(0).contains(&(7 << 8, 0, 8)), "{:?}", s.markers(0));
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, DARK_BLADE)), (Some(2300), Some(1800)), "{}", s.context());
    s.prefer(&[(0, chaser), (0, DARK_BLADE)]);
    s.run_both([&[EndTurn, Activate(virus)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", chaser), 1, "Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", virus), 1, "{}", s.context());
    assert!(s.markers(0).contains(&(7 << 8, 1 << 8, 8)) && !s.markers(0).contains(&(7 << 8, 0, 8)), "{:?}", s.markers(0));
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, DARK_BLADE)), (Some(2000), Some(2100)), "the 300 went along: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Cursed Ring, Dragon Heart and Metal Plus (the Equip and Spell cards of Atticus' and Bastion's decks)
// ---------------------------------------------------------------------------------------------------------------

/// Cursed Ring (a seeded sample): "When your opponent declares an attack: Equip this card to the attacking monster.
/// Monsters it battles cannot be destroyed by battle." Dark Magician (2500) attacks Dark Blade (1800): the Ring goes on
/// Dark Magician, Dark Blade is not destroyed (700 damage all the same). Without the Ring it is destroyed.
#[test]
fn cursed_ring_equips_the_attacker_and_protects_the_monster_it_battles() {
    let ring = card("Cursed Ring");
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), set(0, ring), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[ChainOnAttack(ring)], &[EndTurn, Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", ring), 1, "the Ring stays on the field: {}", s.context());
    assert_eq!(s.lp(0), 7300, "{}", s.context());
}

/// The control for Cursed Ring: with no Ring Dark Blade is destroyed by the same attack (700 damage).
#[test]
fn cursed_ring_control_dark_blade_is_destroyed_without_it() {
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn, Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(0), 7300, "{}", s.context());
}

/// Dragon Heart: "Send 3 Dragon-Type monsters from your Deck to the Graveyard, then target 1 Dragon-Type monster you
/// control; it gains 1000 ATK until the End Phase. You cannot Normal or Special Summon the turn you activate this card."
/// Three Blue-Eyes go from the Deck, the Blue-Eyes on the field has 4000, Dark Blade can't be Normal Summoned, and the
/// extra ATK is gone the next turn.
#[test]
fn dragon_heart_sends_three_dragons_and_gives_1000_atk_until_the_end_phase() {
    let [heart, blue_eyes] = ["Dragon Heart", "Blue-Eyes White Dragon"].map(card);
    let Some(mut s) = field(&[blue_eyes; 3], &[hand(0, DARK_BLADE), hand(0, heart), mon(0, blue_eyes)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(heart)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", blue_eyes), 3, "{}", s.context());
    assert_eq!(s.attack_of(0, blue_eyes), Some(4000), "{}", s.context());
    assert!(!offered(&s, "summons").contains(&DARK_BLADE.into()), "no Normal Summon: {:?}", offered(&s, "summons"));
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, blue_eyes), Some(3000), "{}", s.context());
    // Without 3 Dragons in the Deck it can't be activated; after a Normal Summon neither.
    let Some(mut s) = field(&[blue_eyes; 2], &[hand(0, heart), mon(0, blue_eyes)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&heart.into()), "{:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[blue_eyes; 3], &[hand(0, DARK_BLADE), hand(0, heart), mon(0, blue_eyes)]) else { return };
    s.run(&[Summon(DARK_BLADE)]).unwrap();
    assert!(!offered(&s, "activates").contains(&heart.into()), "{:?}", offered(&s, "activates"));
}

/// Dragon Heart answered by Ring of Destruction (the script reads its target after its cost): the Blue-Eyes it targets is
/// destroyed in the chain, nothing happens to Dragon Heart's effect (no error), and the three Dragons stay in the
/// Graveyard.
#[test]
fn dragon_heart_whose_target_is_destroyed_in_the_chain_does_nothing() {
    let [heart, blue_eyes, ring] = ["Dragon Heart", "Blue-Eyes White Dragon", "Ring of Destruction"].map(card);
    let Some(mut s) = field(&[blue_eyes; 3], &[hand(0, heart), mon(0, blue_eyes), set(1, ring)]) else { return };
    s.prefer(&[(0, blue_eyes)]);
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap(); // the Ring was Set this turn: it is ready in turn 3
    chain_to_activation(&mut s, &[Activate(heart)], 1, ring);
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", blue_eyes), 4, "{}", s.context());
    assert_eq!(s.count(0, "grave", heart), 1, "{}", s.context());
}

/// Metal Plus ("Magic-Reflection Armor - Metal Plus"): "Equip this card to a monster you control. Negate the activations
/// of Spell Cards that target the equipped monster, and destroy those cards." Team 1's Book of Moon aimed at Warwolf is
/// negated and destroyed, and Warwolf stays face-up.
#[test]
fn metal_plus_negates_and_destroys_a_spell_that_targets_the_equipped_monster() {
    let [metal, book] = ["Metal Plus", "Book of Moon"].map(card);
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(0, DARK_BLADE), set(0, metal), hand(1, book)]) else { return };
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(metal)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", metal), 1, "equipped: {}", s.context());
    s.run_both([&[EndTurn], &[Activate(book)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", book), 1, "Book of Moon was destroyed: {}", s.context());
    assert_eq!(s.position_of(0, WARWOLF), Some(1), "Warwolf stayed face-up: {}", s.context());
    // The control: with no Metal Plus the same Book turns Warwolf face-down.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), hand(1, book)]) else { return };
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn], &[Activate(book)]]).unwrap();
    assert_eq!(s.position_of(0, WARWOLF), Some(8), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Syrus' Cycroid cards: Training Wheels and Training Field
// ---------------------------------------------------------------------------------------------------------------

/// Training Wheels: "Equip only to a "Cycroid". It can attack directly. If the equipped monster inflicts battle damage to
/// your opponent by a direct attack: Banish this card and the equipped monster, until your next Standby Phase." Cycroid
/// (800) attacks directly: 800 damage, both are banished, and in the turn after next (team 0's next Standby Phase) Cycroid
/// is back with the Wheels on it. Only a Cycroid takes them (Warwolf alone can't be targeted).
#[test]
fn training_wheels_let_a_cycroid_attack_directly_and_banish_it_until_the_next_standby_phase() {
    let [wheels, cycroid] = ["Training Wheels", "Cycroid"].map(card);
    let Some(mut s) = field(&[], &[mon(0, cycroid), hand(0, wheels)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(wheels), EndTurn, Attack(cycroid)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7200, "800 damage: {}", s.context());
    assert_eq!((s.count(0, "removed", cycroid), s.count(0, "removed", wheels)), (1, 1), "both banished: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", cycroid), 1, "Cycroid is back: {}", s.context());
    assert_eq!(s.count(0, "szone", wheels), 1, "with the Wheels: {}", s.context());
    // Only a Cycroid: with Warwolf as the only monster the Wheels can't be activated.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), hand(0, wheels)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&wheels.into()), "{:?}", offered(&s, "activates"));
}

/// Training Wheels: the equipped Cycroid may attack directly although Dark Magician stands in front of it.
#[test]
fn a_cycroid_with_training_wheels_can_attack_directly_past_a_monster() {
    let [wheels, cycroid] = ["Training Wheels", "Cycroid"].map(card);
    let Some(mut s) = field(&[], &[mon(0, cycroid), hand(0, wheels), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_to(&[Activate(wheels), EndTurn, Attack(cycroid)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    clean(&s);
    let attacks = s.prompt()["attacks"].as_array().unwrap().clone();
    assert!(attacks.iter().any(|a| a["cid"] == cycroid && a["can_direct"] == true), "{}", s.prompt());
}

/// Training Field: "Players can target their own monsters for attacks. When they do, the attacking monster gains 300 ATK
/// and the battling monsters are not destroyed by that battle." Warwolf (2000) attacks Dark Blade (1800), its own
/// monster: neither is destroyed, Warwolf has 2300 and the 500 difference is battle damage to its controller.
#[test]
fn training_field_lets_a_monster_attack_its_own_side_without_destroying_either() {
    let field_spell = card("Training Field");
    let Some(mut s) = field(&[], &[hand(0, field_spell), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[Activate(field_spell), EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "mzone", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2300), "{}", s.context());
    assert_eq!(s.lp(0), 7500, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The sample's Spells and Traps: Cyber Repairer, Life Force, Mirror Damage, Remove Bomb, Ring of Life
// ---------------------------------------------------------------------------------------------------------------

/// Cyber Repairer (a seeded sample): "Your opponent draws 1 card. During this turn, Machine-Type monsters you control
/// cannot be destroyed by battle." Warwolf (2000) attacks Cycroid (a Machine, 800): team 1 draws a card, Cycroid stays
/// (1200 damage); the same attack without it destroys Cycroid.
#[test]
fn cyber_repairer_makes_the_opponent_draw_and_protects_machines_from_battle() {
    let [repairer, cycroid] = ["Cyber Repairer", "Cycroid"].map(card);
    let Some(mut s) = field(&[], &[mon(0, cycroid), set(0, repairer), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(repairer)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", cycroid), 1, "Cycroid survived: {}", s.context());
    assert_eq!(s.count(0, "grave", repairer), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1200, "{}", s.context());
    let with_it = hand_size(&s, 1);
    let Some(mut s) = field(&[], &[mon(0, cycroid), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "grave", cycroid), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 1) + 1, with_it, "team 1 drew one card: {}", s.context());
}

/// Life Force: "If you would take battle damage, you can pay 400 LP instead." Warwolf (2000) destroys Feral Imp (1300)
/// and would deal 700: team 0 pays 400 and takes nothing else (7600). Without the card the damage is taken (7300).
#[test]
fn life_force_pays_400_lp_instead_of_battle_damage() {
    let life_force = card("Life Force");
    let spawns = [mon(0, FILLER), Spawn { team: 0, place: Spell, cid: life_force }, mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run_both([&[Chain(life_force)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7600, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 0, "the Imp was destroyed all the same: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, FILLER), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 7300, "{}", s.context());
}

/// Mirror Damage: "Equip this card to a monster you control. If you would take damage equal to the equipped monster's
/// original ATK by a Spell effect: Your opponent takes that effect damage instead. When this card leaves the field, destroy
/// the equipped monster." Bronze Warrior (500 ATK) wears it in answer to Hinotama (500 damage): team 0 takes nothing and
/// team 1 takes the 500 (7500).
#[test]
fn mirror_damage_turns_a_spells_damage_of_the_equipped_monsters_atk_back() {
    let [mirror, bronze] = ["Mirror Damage", "Bronze Warrior"].map(card);
    let hinotama = card("Hinotama");
    let Some(mut s) = field(&[], &[mon(0, bronze), set(0, mirror), hand(1, hinotama)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(mirror)], &[Activate(hinotama)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", mirror), 1, "equipped: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7500), "{}", s.context());
    // The control: a Spell dealing another amount (Final Flame, 600) is not turned back.
    let flame = card("Final Flame");
    let Some(mut s) = field(&[], &[mon(0, bronze), set(0, mirror), hand(1, flame)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(mirror)], &[Activate(flame)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (7400, 8000), "{}", s.context());
}

/// Mirror Damage: "When this card leaves the field, destroy the equipped monster." Mystical Space Typhoon destroys it:
/// Bronze Warrior goes with it.
#[test]
fn mirror_damage_leaving_the_field_destroys_the_equipped_monster() {
    let [mirror, bronze] = ["Mirror Damage", "Bronze Warrior"].map(card);
    let mst = card("Mystical Space Typhoon");
    let Some(mut s) = field(&[], &[mon(0, bronze), set(0, mirror), hand(1, mst)]) else { return };
    s.prefer(&[(0, mirror), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(mirror)], &[EndTurn]]).unwrap();
    s.run_both([&[EndTurn], &[Activate(mst)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", mirror), s.count(0, "grave", bronze)), (1, 1), "{}", s.context());
}

/// Remove Bomb: "Banish the top 5 cards of your Deck; Inflict 300 damage to your opponent for each monster banished this
/// way." The Deck (shuffled by the duel's seed) is half Dian Ketos, half Feral Imps: five cards are banished, and the
/// opponent takes 300 for each of the Imps among them (some, not all).
#[test]
fn remove_bomb_banishes_five_cards_and_burns_300_for_each_monster() {
    let bomb = card("Remove Bomb");
    let Some(dir) = engine_dir() else { return };
    let main: Vec<u16> = (0..40).map(|i| if i % 2 == 0 { DIAN_KETO } else { FILLER }).collect();
    let decks = [(&main[..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &[hand(0, bomb)]).unwrap();
    s.run(&[Activate(bomb)]).unwrap();
    clean(&s);
    let (spells, imps) = (s.count(0, "removed", DIAN_KETO), s.count(0, "removed", FILLER));
    assert_eq!(spells + imps, 5, "{}", s.context());
    assert!(imps > 0 && spells > 0, "a mixed top: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 300 * imps as i64, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Ring of Life (a Trap, Set first): "Destroy 1 face-up monster, and if you do, both players gain LP equal to its DEF."
/// Dark Magician (DEF 2100) is destroyed and both players gain 2100.
#[test]
fn ring_of_life_destroys_a_monster_and_both_players_gain_its_def() {
    let ring = card("Ring of Life");
    let Some(mut s) = field(&[], &[set(0, ring), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(1, DARK_MAGICIAN), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(ring)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (10100, 10100), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Amorphous Barrier and Last Magnet
// ---------------------------------------------------------------------------------------------------------------

/// Amorphous Barrier: "Activate only while you control 3 or more monsters. Negate the attack of 1 opponent's monster and
/// end the Battle Phase. When this card is sent to the Graveyard, you can Tribute 1 monster to return this card to the top
/// of the Deck." Warwolf's attack is negated and team 1's Dark Blade, which could have attacked after it, doesn't; the
/// Barrier then goes to the top of the Deck for a Feral Imp's Tribute and is drawn in the next turn.
#[test]
fn amorphous_barrier_negates_an_attack_ends_the_battle_phase_and_returns_to_the_deck() {
    let barrier = card("Amorphous Barrier");
    let spawns = [mon(0, FILLER), mon(0, FILLER), mon(0, DARK_BLADE), set(0, barrier), mon(1, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(barrier), Chain(barrier)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.prompt()["to_bp"], false, "the Battle Phase is over, Dark Magician cannot attack: {}", s.prompt());
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "a Feral Imp was Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", barrier), 0, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "hand", barrier), 1, "drawn from the top: {}", s.context());
}

/// Amorphous Barrier needs 3 monsters: with two it is not offered against an attack.
#[test]
fn amorphous_barrier_needs_three_monsters() {
    let barrier = card("Amorphous Barrier");
    let Some(mut s) = field(&[], &[mon(0, FILLER), mon(0, FILLER), set(0, barrier), mon(1, WARWOLF)]) else { return };
    s.run_both([&[ChainOnAttack(barrier)], &[EndTurn, Attack(WARWOLF)]]).expect_err("never offered");
}

/// Last Magnet: "Activate only when an opponent's monster destroys a monster you control by battle: Equip this card to that
/// monster. It loses 800 ATK." Dark Magician destroys Dark Blade: the Magnet goes on it and its ATK is 1700.
#[test]
fn last_magnet_equips_the_monster_that_destroyed_yours_and_takes_800_atk() {
    let magnet = card("Last Magnet");
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), set(0, magnet), mon(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[Chain(magnet)], &[EndTurn, Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", magnet), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, DARK_MAGICIAN), Some(1700), "{}", s.context());
}

/// Last Magnet answers a single destroyer: when both monsters destroy each other (Warwolf against Warwolf, 2000 each) two
/// monsters "destroy by battle" at once and it is not offered, so it stays Set.
#[test]
fn last_magnet_is_not_offered_when_both_monsters_destroy_each_other() {
    let magnet = card("Last Magnet");
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), set(0, magnet), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(1, "grave", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "szone", magnet), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Bastion's Magnet cards
// ---------------------------------------------------------------------------------------------------------------

/// Magnet Force Plus and Magnet Force Minus: "Target 1 face-up monster on the field; equip this card to that target. The
/// equipped monster is treated as a Plus (Minus) monster. During the Battle Step, while an attack involving the equipped
/// monster and a Plus (Minus) monster is occurring, negate the attack. If the opponent of the equipped monster's controller
/// controls a Minus (Plus) monster, change the equipped monster to Attack Position, and it must battle a Minus (Plus)
/// monster, if able." Warwolf wears the Plus (turn 3), Dark Blade the Minus (turn 4): opposite poles, so the Dark Blade
/// must attack Warwolf, not Dark Magician (a plain monster), and the attack is not negated: 200 damage to its controller
/// where the attack on Dark Magician would hurt 700. Upstream's Force Plus made its monster a Minus (a copy of the Minus
/// card's line), so the two Minus monsters negated each other's attack: the override.
#[test]
fn magnet_force_plus_and_minus_attract_each_other() {
    let [plus, minus] = ["Magnet Force Plus", "Magnet Force Minus"].map(card);
    let spawns = [mon(0, WARWOLF), mon(0, DARK_MAGICIAN), set(0, plus), mon(1, DARK_BLADE), set(1, minus)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (1, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(plus)], &[EndTurn, Activate(minus), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", plus), s.count(1, "szone", minus)), (1, 1), "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "Dark Blade battled Warwolf (2000): {}", s.context());
    assert_eq!(s.lp(1), 8000 - 200, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// The same pair the other way round: Plus and Minus on monsters that are not opposite poles negate each other's attack. Two
/// Minus monsters (Magnet Force Minus on Warwolf and on Dark Blade, one copy for each side): Dark Blade's attack on Warwolf
/// is negated and nothing is destroyed.
#[test]
fn magnet_force_minus_on_both_monsters_negates_their_attack() {
    let minus = card("Magnet Force Minus");
    let spawns = [mon(0, WARWOLF), set(0, minus), mon(1, DARK_BLADE), set(1, minus)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (1, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(minus)], &[EndTurn, Activate(minus), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", minus), s.count(1, "szone", minus)), (1, 1), "{}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", DARK_BLADE)), (1, 1), "nothing was destroyed: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "the attack was negated: {}", s.context());
}

/// A Magnet Force on a monster that is destroyed (Dark Hole) goes to the Graveyard with it, with no error from its
/// continuous effects (they read the equipped monster, which is gone).
#[test]
fn a_magnet_force_whose_monster_is_destroyed_goes_to_the_graveyard() {
    let [plus, minus] = ["Magnet Force Plus", "Magnet Force Minus"].map(card);
    let spawns = [mon(0, WARWOLF), mon(0, DARK_BLADE), set(0, plus), set(0, minus), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, DARK_BLADE), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(plus), Activate(minus)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", plus), s.count(0, "szone", minus)), (1, 1), "{}", s.context());
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", plus), s.count(0, "grave", minus)), (1, 1), "{}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
}

/// Magnet Conductor Plus: "Target 1 Plus monster in your Graveyard; add it to your hand." The only card of the game that is
/// a Plus (and a Minus) monster is the anime's Conduction Warrior Linear Magnum Plus Minus: it is added, a Warwolf next to
/// it is not a target.
#[test]
fn magnet_conductor_plus_adds_a_plus_monster_from_the_graveyard() {
    let [conductor, conduction] = ["Magnet Conductor Plus", "Conduction Warrior Linear Magnum Plus Minus"].map(card);
    let Some(mut s) = field(&[], &[hand(0, conductor), grave(0, conduction), grave(0, WARWOLF)]) else { return };
    s.run(&[Activate(conductor)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", conduction), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    // Without a Plus monster in the Graveyard it can't be activated.
    let Some(mut s) = field(&[], &[hand(0, conductor), grave(0, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&conductor.into()), "{:?}", offered(&s, "activates"));
}

/// One Hundred Thousand Gauss (a Trap): "While you control a Plus monster and a Minus monster: Change 1 of your opponent's
/// face-up Defense Position monsters to Attack Position, and if you do, it loses 800 ATK." Conduction Warrior (both) lets
/// Dark Magician in Defense Position switch to Attack with 1700; with no Plus and Minus monster the card is not offered.
#[test]
fn one_hundred_thousand_gauss_switches_a_defense_monster_to_attack_with_800_less() {
    let [gauss, conduction] = ["One Hundred Thousand Gauss", "Conduction Warrior Linear Magnum Plus Minus"].map(card);
    let Some(mut s) = field(&[], &[set(0, gauss), mon(0, conduction), mon_def(1, DARK_MAGICIAN)]) else { return };
    assert_eq!(s.position_of(1, DARK_MAGICIAN), Some(4), "{}", s.context());
    s.run_both([&[EndTurn, Activate(gauss)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, DARK_MAGICIAN), Some(1), "{}", s.context());
    assert_eq!(s.attack_of(1, DARK_MAGICIAN), Some(1700), "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, gauss), mon(0, WARWOLF), mon_def(1, DARK_MAGICIAN)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert!(!offered(&s, "activates").contains(&gauss.into()), "{:?}", offered(&s, "activates"));
}

/// Power Off: "Send 1 face-up Plus Minus monster you control that was Special Summoned by its effect to the Graveyard. Then,
/// Special Summon the monsters used to Special Summon it from your Graveyard." The only Plus Minus monster the client has,
/// Conduction Warrior, is Special Summoned with Magnet Warriors of the anime (Sigma Plus and Omega Minus, passcodes
/// 170000111-2) that no card of the client's is, so the card can't be played from start to end here: what is checked is
/// that a Conduction Warrior put on the field with no materials gives it nothing to send, so it is not offered.
#[test]
fn power_off_needs_a_plus_minus_monster_that_was_summoned_with_materials() {
    let [power_off, conduction] = ["Power Off", "Conduction Warrior Linear Magnum Plus Minus"].map(card);
    let Some(mut s) = field(&[], &[hand(0, power_off), mon(0, conduction)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&power_off.into()), "{:?}", offered(&s, "activates"));
}
