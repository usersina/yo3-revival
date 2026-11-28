//! The placeholder CPU's answers (`yo3_ocg::placeholder`, shared with the engine's bot) and the repeat guard.
use super::*;
pub(super) use crate::ocg::placeholder::{free_zones, sum_bounds, sum_choice, sum_fits};

/// The placeholder CPU's answer to any prompt (`yo3_ocg::placeholder::respond`).
pub fn bot_answer(m: &Json) -> Json {
    bot_response(m).json()
}

/// [`bot_answer`], typed.
pub fn bot_response(m: &Json) -> Response {
    match Ask::lenient(m) {
        Ok(Some(ask)) => crate::ocg::placeholder::respond(&ask, m),
        Ok(None) => {
            tracing::warn!("rules engine: no answer for prompt {}", m["type"]);
            Response::Chain(None)
        }
        Err(e) => {
            tracing::warn!("rules engine: a prompt that doesn't fit its struct: {e}");
            Response::Chain(None)
        }
    }
}

/// Spots an engine that keeps asking the same prompt after it was answered (the core refuses an answer it can't use and
/// asks again; an answer the server can't give would then loop under the table's lock).
#[derive(Default)]
pub struct Repeats {
    pub(super) last: Option<Json>,
    pub(super) times: u32,
}

/// How many times the same prompt may come back in a row before the duel is given up.
pub(super) const MAX_REPEATS: u32 = 8;
/// The most picks of a card list kept (a list holds a few dozen entries).
pub(super) const MAX_PICKS: usize = 256;

impl Repeats {
    /// Whether `prompt` was just asked too many times in a row.
    pub fn stuck(&mut self, prompt: &Json) -> bool {
        if self.last.as_ref() == Some(prompt) {
            self.times += 1;
        } else {
            self.last = Some(prompt.clone());
            self.times = 0;
        }
        self.times >= MAX_REPEATS
    }
}

/// The CPU's answer, and the default for the client's prompts that have no window yet: the placeholder CPU
/// (`yo3_ocg::placeholder`, which the engine's bot falls back on too).
pub(super) fn cpu_answer(m: &Json) -> Json {
    bot_answer(m)
}
