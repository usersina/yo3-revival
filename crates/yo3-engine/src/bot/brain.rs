//! The bot's brains: one interface for every way an NPC can decide. A brain gets a [`Decision`] (the prompt, what the
//! bot's player sees, the last selection hint) and the [`Thinker`] (the duel's copies, the style, the time left) and
//! gives the answer. Each brain is measured the same way, in the arena (docs/development.md, "The duel bot"); a style
//! names its brain (`"brain"`: `placeholder`, `rules` or `search`, the default).
//!
//! Brains stack: [`Search`] tries options in copies of the duel and plays each copy on with [`Rules`]; both read the
//! card knowledge on the [`Board`] (`knowledge.rs`: what the bot's deck needs to bring out its bosses).
use serde_json::Value as Json;

use super::Thinker;
use super::board::{Board, Hidden};

/// A decision at hand, as the bot's player sees it.
pub struct Decision<'a> {
    /// The core's prompt (decoded).
    pub prompt: &'a Json,
    /// The bot's engine player.
    pub me: u8,
    /// What the bot sees of the duel now.
    pub board: &'a Board,
    /// The last selection hint the bot was given (what will happen to the cards it picks).
    pub hint: Option<u64>,
    /// The opponent's monster zones the bot couldn't see into when it started thinking.
    pub hidden: &'a Hidden,
}

/// A way to decide.
pub trait Brain: Sync {
    /// The name a style gives it.
    fn name(&self) -> &'static str;
    /// The answer to `d` (`thinker` holds the duel's copies, the style and the time left).
    fn decide(&self, thinker: &Thinker, d: &Decision) -> Json;
}

/// The placeholder CPU: the first legal choices (`yo3_ocg::placeholder`). The baseline.
pub struct Placeholder;

impl Brain for Placeholder {
    fn name(&self) -> &'static str {
        "placeholder"
    }
    fn decide(&self, _: &Thinker, d: &Decision) -> Json {
        super::policy::simple(d.prompt)
    }
}

/// Rules of thumb alone (`policy`): no copies, answers at once.
pub struct Rules;

impl Brain for Rules {
    fn name(&self) -> &'static str {
        "rules"
    }
    fn decide(&self, _: &Thinker, d: &Decision) -> Json {
        super::policy::answer(d.prompt, d.me, d.board, d.hint)
    }
}

/// The look-ahead: each option that matters tried in a copy of the duel, played on by [`Rules`] and judged by
/// [`Board::value`]; the rules decide the rest.
pub struct Search;

impl Brain for Search {
    fn name(&self) -> &'static str {
        "search"
    }
    fn decide(&self, thinker: &Thinker, d: &Decision) -> Json {
        thinker.search(d)
    }
}

/// Every brain, the default first.
pub const BRAINS: &[&dyn Brain] = &[&Search, &Rules, &Placeholder];

/// The brain named `name`; the default ([`Search`]) for any other.
pub fn by_name(name: &str) -> &'static dyn Brain {
    BRAINS.iter().copied().find(|b| b.name() == name).unwrap_or(BRAINS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_style_names_its_brain() {
        assert_eq!(by_name("rules").name(), "rules");
        assert_eq!(by_name("placeholder").name(), "placeholder");
        assert_eq!(by_name("search").name(), "search");
        assert_eq!(by_name("").name(), "search", "the default");
        assert_eq!(by_name("oracle").name(), "search");
    }
}
