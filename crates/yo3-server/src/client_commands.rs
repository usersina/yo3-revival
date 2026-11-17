//! Every duel command the client understands, the way `ENGINE_MESSAGES` (translate.rs) lists the rules engine's
//! messages: whether the server sends it, why not, or that it is still undecided.
//!
//! A command is the data of an engine command (duelRequestName 0x11 {id, param1..param4}). Two sources make the list:
//! - which of the client's duel components handles each id, read from their dispatch tables (docs/protocol.md,
//!   "Components"): DuelScene `0x514290`, DuelCardManager `0x4FF000`, DuelCockpit `0x502EB0`, DuelFieldManager
//!   `0x50A690`, DuelSummon `0x5165A0`, DuelChain `0x501560`, DuelCutin `0x505A70`, DuelBattle `0x4F8A90`, the pop
//!   `0x522A50` (the data commands, 1000 and up), and the rock-paper-scissors scene (1501-1503);
//! - how often Konami's own server sent each one in the 228 duel logs we have (`konami`).
//!
//! The card info panel's level and stats (1018-1020) went unsent for years because nothing set "Konami sent it 1,322
//! times" next to "we never do"; the docs called them bookkeeping. Here each one the server doesn't send needs a reason
//! with evidence, or stays [`Use::Undecided`] in plain sight. `tests/client_commands.rs` checks the table against the
//! client's handlers and, with the logs present, against what Konami sent; the duel check fails on any command the
//! server sends that the table doesn't mark [`Use::Sent`] (`duelcheck::coverage::native`).
use crate::duelcheck::coverage::Coverage;

/// What the server does with a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    /// Sent; what the duel check's client model makes of it.
    Sent(Coverage),
    /// Not sent, and why, with the evidence.
    NotSent(&'static str),
    /// No duel component's dispatch table has a case for it: whatever Konami's server meant by it, the client ignores
    /// it.
    NoHandler,
    /// Handled by the client, not sent, and not yet known whether players miss it.
    Undecided(&'static str),
}

/// One command: its id, its name (Konami's view vocabulary where it is known, see docs/protocol.md "Command names"),
/// the components that handle it, how often Konami's server sent it in the 228 logs, and what the server does.
#[derive(Clone, Copy, Debug)]
pub struct ClientCommand {
    pub id: u32,
    pub name: &'static str,
    pub handlers: &'static [&'static str],
    pub konami: u32,
    pub usage: Use,
}

const SCENE: &str = "DuelScene";
const CARDS: &str = "DuelCardManager";
const COCKPIT: &str = "DuelCockpit";
const FIELD: &str = "DuelFieldManager";
const SUMMON: &str = "DuelSummon";
const CHAIN: &str = "DuelChain";
const CUTIN: &str = "DuelCutin";
const BATTLE: &str = "DuelBattle";
const POP: &str = "the pop";
const JANKEN: &str = "the rock-paper-scissors scene";

use Coverage::{Interaction, Presentation, State};
use Use::{NoHandler, NotSent, Sent, Undecided};

const fn c(id: u32, name: &'static str, handlers: &'static [&'static str], konami: u32, usage: Use) -> ClientCommand {
    ClientCommand { id, name, handlers, konami, usage }
}

const NEVER: &str = "Konami's server never sent it in 228 duels";
const RESET: &str = "DuelSummon's Reset clears the marks its family's Set left; when Konami's server sent it is unread";
const UNREAD: &str = "the client handles it; what it shows and when Konami's server sent it are unread";

pub const CLIENT_COMMANDS: &[ClientCommand] = &[
    c(1, "DuelStart", &[SCENE], 261, Sent(Presentation)),
    c(2, "DuelEnd", &[SCENE, POP], 261, Sent(Presentation)),
    c(3, "?", &[], 313, NoHandler),
    c(4, "WaitInput", &[SCENE, FIELD, BATTLE, POP], 0, Sent(Interaction)),
    c(5, "PhaseChange", &[SCENE, COCKPIT], 11010, Sent(State)),
    c(6, "TurnChange", &[SCENE, CUTIN, POP], 2046, Sent(State)),
    c(7, "the field's background", &[SCENE], 74, Sent(Presentation)),
    c(8, "?", &[], 2503, NoHandler),
    // docs/protocol.md "Presentation from Konami's logs": after every summon cut-in and finished LP change; the client
    // changes the music by itself (DuelCockpit 0x5040F0).
    c(9, "BgmUpdate", &[], 4400, NoHandler),
    c(0xA, "BattleInit", &[BATTLE], 2514, Sent(State)),
    c(0xB, "BattleSelect", &[BATTLE], 718, Undecided("battle state 2 (0x4F8B95), no parameters: while a field card is \
         hovered, DuelBattle's frame (0x4F8CD9) draws effect 0xCB from the attacker an 0xC {attacker, -1} named to it, \
         behind a flag at the scene's +0x98 +0x44 (0x800) not yet traced; Konami sent 0xC {attacker, -1}, 0xB, then \
         0xC {attacker, target} every time. Live, with a command map giving the targets bit 0, a click on one answered (#51)")),
    c(0xC, "BattleAttack", &[BATTLE], 3162, Sent(State)),
    c(0xD, "BattleRun", &[BATTLE], 1419, Sent(State)),
    c(0xE, "BattleEnd", &[BATTLE], 959, Sent(State)),
    c(0xF, "LifeSet", &[COCKPIT], 645, Sent(State)),
    c(0x10, "LifeDamage", &[COCKPIT], 2964, Sent(State)),
    c(0x11, "?", &[], 2, NoHandler),
    c(0x12, "HandShuffle", &[CARDS], 883, Sent(State)),
    c(0x13, "HandShow", &[CARDS], 525, Sent(Presentation)),
    c(0x14, "HandOpen", &[CARDS], 225, Undecided(UNREAD)),
    c(0x15, "DeckShuffle", &[SCENE], 943, Undecided(UNREAD)),
    c(0x16, "DeckReset", &[CARDS], 2484, Sent(State)),
    c(0x17, "DeckFlipTop", &[CARDS], 659, Sent(Presentation)),
    c(0x18, "GraveTop", &[CARDS], 737, NotSent("its separate face-up object went stale when a card left from under the top (live); \
         the card table's face flag shows the top instead (docs/protocol.md, the Graveyard's top card)")),
    c(0x19, "CardLockon", &[CARDS], 1837, Sent(Presentation)),
    c(0x1A, "CardMove", &[CARDS, POP], 17207, Sent(State)),
    c(0x1B, "CardSwap", &[CARDS], 2, Sent(State)),
    c(0x1C, "CardFlipTurn", &[CARDS], 1243, Sent(State)),
    c(0x1D, "CardCheat", &[CARDS], 10366, Sent(Presentation)),
    c(0x1E, "CardSet", &[SCENE], 1792, Sent(Presentation)),
    c(0x1F, "CardVanish", &[CARDS], 1055, Sent(State)),
    c(0x20, "CardBreak", &[CARDS], 578, NotSent("only drops a monster zone's label after a battle (0x4FF000 case 0x20); the shatter \
         comes from CardVanish, as BattleRun's destroy bit can't wait for a protection (docs/protocol.md)")),
    c(0x21, "?", &[], 499, NoHandler),
    c(0x22, "a card's activation effect", &[SCENE], 3829, Sent(Presentation)),
    c(0x23, "?", &[SCENE], 227, Undecided(UNREAD)),
    c(0x24, "?", &[SCENE], 97, Undecided(UNREAD)),
    c(0x25, "CardEquip", &[SCENE, CARDS], 84, Undecided("we show equips with relation markers (1017); what CardEquip adds is unread")),
    c(0x26, "CardIncTurn (the relation markers' redraw)", &[CARDS, FIELD], 37597, Sent(Presentation)),
    c(0x27, "CardUpdate (counters)", &[CARDS], 117, Sent(State)),
    c(0x28, "MonstDeathTurn", &[CARDS], 0, NotSent(NEVER)),
    c(0x29, "MonstShuffle", &[CARDS], 1, Undecided(UNREAD)),
    c(0x2A, "TributeSet", &[SUMMON], 390, Sent(Presentation)),
    c(0x2B, "TributeReset", &[SUMMON], 19, Undecided(RESET)),
    c(0x2C, "TributeRun", &[SUMMON], 326, Sent(Presentation)),
    c(0x2D, "MaterialSet", &[SUMMON], 80, Sent(Presentation)),
    c(0x2E, "MaterialReset", &[SUMMON], 0, NotSent(NEVER)),
    c(0x2F, "MaterialRun", &[SUMMON], 58, Sent(Presentation)),
    c(0x30, "TuningSet", &[SUMMON], 1213, Sent(Presentation)),
    c(0x31, "TuningReset", &[SUMMON], 10, Undecided(RESET)),
    c(0x32, "TuningRun", &[SUMMON], 605, Sent(Presentation)),
    c(0x33, "ChainSet", &[CHAIN], 3767, Sent(Presentation)),
    c(0x34, "ChainRun", &[CHAIN], 3762, Sent(Presentation)),
    c(0x35, "RunSurrender", &[], 106, NoHandler),
    c(0x36, "RunDialog", &[SCENE, POP], 172, Sent(Interaction)),
    c(0x37, "RunList", &[SCENE, POP], 0, Sent(Interaction)),
    c(0x38, "RunSummon", &[SCENE], 1160, Sent(Presentation)),
    c(0x39, "RunSpSummon", &[SCENE], 1497, Sent(Presentation)),
    c(0x3A, "RunFusion", &[SCENE], 361, Sent(Presentation)),
    c(0x3C, "RunCoin", &[SCENE], 4, Sent(Presentation)),
    c(0x3D, "RunDice", &[SCENE], 0, Sent(Presentation)),
    c(0x3F, "?", &[SCENE], 0, NotSent(NEVER)),
    c(0x43, "a cut-in: the draw", &[CUTIN], 2776, Sent(Presentation)),
    c(0x44, "a cut-in: a summon", &[CUTIN], 2918, Sent(Presentation)),
    c(0x45, "?", &[], 361, NoHandler),
    c(0x46, "a cut-in: a reveal", &[CUTIN], 302, Sent(Presentation)),
    c(0x47, "a cut-in: an activation", &[CUTIN], 5508, Sent(Presentation)),
    c(0x48, "a cut-in: a card played", &[CUTIN], 3779, Sent(Presentation)),
    c(0x49, "?", &[], 94, NoHandler),
    c(0x4A, "?", &[], 81, NoHandler),
    c(0x4B, "?", &[], 993, NoHandler),
    c(0x4C, "a cut-in: the turn's end", &[CUTIN], 2046, Sent(Interaction)),
    c(0x4D, "a cut-in: damage", &[CUTIN], 1482, Sent(Presentation)),
    c(0x4E, "?", &[], 1517, NoHandler),
    c(0x50, "?", &[], 32, NoHandler),
    c(1000, "a card table entry", &[POP], 53586, Sent(State)),
    c(1001, "a monster record (begin)", &[POP], 10737, Sent(State)),
    c(1002, "a monster record (more)", &[POP], 10737, Sent(State)),
    c(1003, "a monster record (commit)", &[POP], 10737, Sent(State)),
    c(1004, "a message's length", &[POP], 172, Sent(Interaction)),
    c(1005, "a message's entries", &[POP], 172, Sent(Interaction)),
    c(1006, "the usable zones", &[POP], 282, Sent(Presentation)),
    c(1007, "a list's start", &[POP], 31333, Sent(Interaction)),
    c(1008, "a list's values", &[POP], 7012, Sent(Interaction)),
    c(1009, "a list card's place", &[POP], 7012, NotSent("Konami's value is the card's place (0 field, 0xB hand, 0xC extra, \
         0xD deck, 0xE graveyard, 0xF banished; +0x80 the opponent's), the same as 1011's marks; the pop keeps it as a u16 at \
         +0x760 (0x522D14), and its only reader, the pick sender 0x521CC0, keeps the low byte and puts the pick order above it")),
    c(1010, "a list's CARD_Prop rows", &[POP], 7012, Sent(Interaction)),
    c(1011, "a list's marks", &[POP], 7012, Sent(Interaction)),
    c(1013, "a place's typed counter (0x523F50)", &[POP], 6280, Undecided("a per-place 3-value map the card info panel reads; Konami sent \
         one for every place at the start; we send counters with CardUpdate (0x27)")),
    c(1014, "?", &[POP], 3495, Undecided("the pop keeps it (0x522DDB); its reader is unread")),
    c(1015, "a summon's material list", &[POP], 361, Sent(Presentation)),
    c(1016, "a summon's material list (more)", &[POP], 6, Sent(Presentation)),
    c(1017, "a relation marker", &[POP], 5307, Sent(State)),
    c(1018, "a hand monster's level (the card info panel)", &[POP], 5956, Sent(Presentation)),
    c(1019, "a Graveyard monster's stats (staged)", &[POP], 3160, Sent(Presentation)),
    c(1020, "a Graveyard monster's stats (stored)", &[POP], 3160, Sent(Presentation)),
    c(1021, "?", &[], 883, NoHandler),
    c(1022, "an acknowledgement request", &[POP], 57170, NotSent("the client answers it with duelRequestName 8 {player} \
         (0x521980); the connection DLL's settle marker (0x5F1) paces the batches instead (#11)")),
    c(1023, "a pile's count", &[POP], 2484, NotSent("Konami's opening put only each pile's top card in the table and set the \
         counts with it; ours puts every card in the table, and the moves keep the counts (docs/protocol.md)")),
    c(1024, "?", &[POP], 895, Undecided("the pop keeps it (0x522F88); its reader is unread")),
    c(1025, "?", &[POP], 386, Undecided("the pop keeps it (0x522E95); its reader is unread")),
    c(1027, "?", &[POP], 5, Undecided("the pop keeps it (0x522FA0); its reader is unread")),
    c(1500, "the pre-duel scene's start", &[POP], 351, Sent(Presentation)),
    c(1501, "rock-paper-scissors' result", &[JANKEN], 351, Sent(Presentation)),
    c(1502, "who chooses the turn order", &[JANKEN], 261, Sent(Presentation)),
    c(1503, "the turn order", &[JANKEN], 261, Sent(Presentation)),
    c(1504, "?", &[POP], 34, Sent(Presentation)),
    c(1505, "?", &[SCENE], 227, Sent(Presentation)),
    c(1506, "?", &[POP], 0, NotSent(NEVER)),
];

/// The table's entry for an id.
pub fn command(id: u32) -> Option<&'static ClientCommand> {
    CLIENT_COMMANDS.iter().find(|c| c.id == id)
}
