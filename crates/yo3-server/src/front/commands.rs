//! Chat commands: lines the server answers to the sender alone, in their chat window (#63, #58, #55, #44).
//!
//! The game handles its own `/` commands (`/h` lists them) and refuses an unknown one ("That command is not
//! available.") before anything is sent, but `/p NAME MESSAGE` goes to the server as a whisper (`chat_type` 0, `to_name`
//! NAME, `chat_str` MESSAGE; live). So the server is a player named [`SERVER`]: `/p Server banlist` is the command
//! `banlist`, answered privately (the game keeps `/p Server ` in the input, so the next command is just typed). A
//! line in a chat room that starts with `!` and a command's name does the same. Nothing of either is sent on.
use yo3_proto::Message;

use super::{FrontSession, chat_note};
use crate::lock::Locked;

/// The name a whisper goes to for the server's commands.
pub const SERVER: &str = "Server";

/// The commands, by the word that starts them.
const COMMANDS: [&str; 7] = ["help", "banlist", "draw", "draw5", "where", "report", "h"];

/// The longest reason a command's help line and answers may run: the game's chat window wraps longer lines.
const LINE: usize = 110;

/// Splits a line into its command word and the rest, if it is a command: any line whispered to [`SERVER`], or a line in
/// a chat room that starts with `!` and names a command.
pub fn parse(text: &str, whisper_to: Option<&str>) -> Option<(String, String)> {
    let text = text.trim();
    let to_server = whisper_to.is_some_and(|n| n.trim().eq_ignore_ascii_case(SERVER));
    let body = match text.strip_prefix('!') {
        Some(rest) => rest,
        None if to_server => text.strip_prefix('/').unwrap_or(text),
        None => return None,
    };
    let (word, rest) = body.trim_start().split_once(char::is_whitespace).unwrap_or((body.trim_start(), ""));
    let word = word.to_lowercase();
    (to_server || COMMANDS.contains(&word.as_str())).then(|| (word, rest.trim().to_string()))
}

/// `items` joined into chat lines of at most [`LINE`] characters.
fn wrap(head: &str, items: &[String], separator: &str) -> Vec<String> {
    let mut lines = vec![];
    let mut line = head.to_string();
    for (i, item) in items.iter().enumerate() {
        let piece = if i + 1 < items.len() { format!("{item}{separator}") } else { item.clone() };
        if line.chars().count() + piece.chars().count() > LINE && !line.trim().is_empty() && line != head {
            lines.push(std::mem::take(&mut line));
            line = "  ".to_string();
        }
        line.push_str(&piece);
    }
    lines.push(line);
    lines
}

impl FrontSession {
    /// A chat line that is a command: what to answer the sender, as chat lines. None: it is ordinary chat.
    pub(super) fn chat_command(&mut self, text: &str, whisper_to: Option<&str>) -> Option<Vec<String>> {
        let (word, rest) = parse(text, whisper_to)?;
        Some(match word.as_str() {
            "banlist" => self.banlist_command(&rest),
            "draw" | "draw5" => self.draw_command(&rest, word == "draw5"),
            "where" => crate::sources::answer(&rest, &self.world.card_names, &self.world.card_sources),
            "report" => self.report_command(&rest),
            _ => help(),
        })
    }

    /// `banlist` shows the banlist the player logs in with; `banlist NAME` chooses one and `banlist default` goes back to
    /// the server's (#63). It is kept for the account (the game asks for it at login, before a player is chosen) and
    /// reaches the game at its next login, which is when the game checks its decks against it.
    fn banlist_command(&mut self, arg: &str) -> Vec<String> {
        let world = self.world.clone();
        let name_of = |id: u32| world.banlists.iter().find(|(k, _)| *k == id).map_or_else(|| format!("banlist {id}"), |(_, n)| n.clone());
        let server_default = world.settings.now().login_regulation;
        let choices: Vec<String> = world.banlists.iter().map(|(_, n)| n.clone()).collect();
        let usage = |lead: String| {
            let mut lines = vec![lead];
            lines.extend(wrap("Choices: ", &choices, ", "));
            lines.push("Type banlist NAME to choose one, or banlist default for the server's.".into());
            lines
        };
        let Some(directory) = self.directory.clone() else { return vec!["Your banlist can't be saved on this server.".into()] };
        let own = directory.locked().account_of(self.account.user_id).and_then(|a| a.banlist);
        let arg = arg.trim();
        if arg.is_empty() {
            let shown = match own {
                Some(id) => format!("Your banlist is {} (your choice).", name_of(id)),
                None => format!("Your banlist is {} (the server's).", name_of(server_default)),
            };
            return usage(shown);
        }
        let chosen = if arg.eq_ignore_ascii_case("default") {
            None
        } else if choices.is_empty() {
            return vec!["This server doesn't know the game's banlists yet: ask the host.".into()];
        } else {
            match crate::regulation_by(arg, &world.banlists) {
                Ok(id) => Some(id),
                Err(_) => return usage(format!("There is no banlist \"{}\".", arg.chars().take(40).collect::<String>())),
            }
        };
        if let Err(e) = directory.locked().set_banlist_of(self.account.user_id, chosen) {
            tracing::warn!("[chat] banlist not saved: {e}");
            return vec!["Your banlist could not be saved: try again.".into()];
        }
        self.account.banlist = chosen;
        let now = name_of(chosen.unwrap_or(server_default));
        let kind = if chosen.is_some() { "your choice" } else { "the server's" };
        vec![format!("Your banlist is now {now} ({kind})."), "It applies when you next log in: log out of the game and back in, then Select Deck and Auto Duel follow it.".into()]
    }

    /// `draw [N] [DECK]` (and `draw5`) deals N cards (5 by default, at most [`MAX_DRAW`]) at random from the player's
    /// newest saved or picked deck, or the one named DECK, to see what an opening hand could be (#58).
    fn draw_command(&mut self, arg: &str, five: bool) -> Vec<String> {
        const MAX_DRAW: usize = 10;
        let mut words = arg.split_whitespace().peekable();
        let mut count = 5;
        if !five && let Some(n) = words.peek().and_then(|w| w.parse::<usize>().ok()) {
            count = n.clamp(1, MAX_DRAW);
            words.next();
        }
        let wanted = words.collect::<Vec<_>>().join(" ").to_lowercase();
        let decks = self.uploaded_decks();
        let deck = if wanted.is_empty() {
            decks.last().cloned().or_else(|| self.world.starter_deck.clone())
        } else {
            let all = decks.iter().rev().chain(self.world.starter_deck.iter());
            all.clone().find(|d| d.name.to_lowercase() == wanted).or_else(|| all.clone().find(|d| d.name.to_lowercase().contains(&wanted))).cloned()
        };
        let Some(deck) = deck else {
            return vec![if wanted.is_empty() { "You have no deck yet: save one in Edit Deck first.".to_string() } else { format!("You have no deck named like \"{}\".", wanted.chars().take(40).collect::<String>()) }];
        };
        if deck.main.is_empty() { return vec![format!("\"{}\" has no main deck cards.", deck.name)] }
        let mut pile = deck.main.clone();
        let count = count.min(pile.len());
        for i in 0..count {
            let j = i + (crate::duel::next(&mut self.rng) % (pile.len() - i) as u64) as usize;
            pile.swap(i, j);
        }
        let names: Vec<String> = pile[..count].iter().map(|row| {
            let cid = self.world.cards.get(row).map(|c| c.cid);
            cid.and_then(|c| self.world.card_names.get(&c)).cloned().unwrap_or_else(|| format!("card #{row}"))
        }).collect();
        wrap(&format!("Draw {count} from \"{}\" ({} cards): ", deck.name, deck.main.len()), &names, ", ")
    }

    /// `report PLAYER reason` tells the staff about a player (#44): kept for them, at most `report-limit` an hour from
    /// one player. The player's name is the longest start of the line that names a player, or one in quotes.
    fn report_command(&mut self, arg: &str) -> Vec<String> {
        let settings = self.world.settings.now();
        if settings.report_limit == 0 { return vec!["Reports are turned off on this server.".into()] }
        let usage = || vec!["Usage: report PLAYER reason (a name with spaces in quotes: report \"Joe Bob\" reason).".to_string()];
        let words: Vec<&str> = arg.split_whitespace().collect();
        if words.len() < 2 { return usage() }
        let me = self.player().clone();
        // The name in quotes, else the longest run of words that is a player's name.
        let (target, reason) = match arg.strip_prefix('"').and_then(|a| a.split_once('"')) {
            Some((name, reason)) => (self.player_named(name), reason.trim().to_string()),
            None => (1..words.len()).rev().find_map(|n| self.player_named(&words[..n].join(" ")).map(|t| (Some(t), words[n..].join(" "))))
                .unwrap_or((None, String::new())),
        };
        let Some((target_id, target)) = target else {
            let named = words[0].trim_matches('"').chars().take(40).collect::<String>();
            return vec![format!("No player is named \"{named}\"."), usage().remove(0)];
        };
        if target_id == me.player_id { return vec!["You can't report yourself.".into()] }
        if reason.chars().count() < 3 { return usage() }
        let added = self.hub().reports.add((me.player_id, &me.name), (target_id, &target), &reason, settings.report_limit);
        match added {
            Ok(id) => {
                tracing::info!("[report] #{id} {} -> {target}", me.name);
                vec![format!("Thank you: your report about {target} was sent to the staff (#{id}).")]
            }
            Err(e) if e == "too many reports" => vec![format!("You have sent {} reports in the last hour: please wait before sending another.", settings.report_limit)],
            Err(e) => {
                tracing::warn!("[report] not saved: {e}");
                vec!["Your report could not be saved: try again later.".into()]
            }
        }
    }

    /// The player named `name` (ignoring case): their id and their name as spelled. Among the saved players, else
    /// online ones.
    fn player_named(&self, name: &str) -> Option<(u32, String)> {
        let name = name.trim();
        if name.is_empty() { return None }
        if let Some(d) = &self.directory && let Some((id, _, spelled)) = d.locked().player_named(name) { return Some((id, spelled)) }
        self.hub().online.values().find(|o| o.name.eq_ignore_ascii_case(name)).map(|o| (o.player_id, o.name.clone()))
    }
}

/// The help every unknown command gets.
fn help() -> Vec<String> {
    vec![
        "Server commands (whisper them to Server: /p Server banlist; or start a chat line with !):".into(),
        "banlist [NAME|default]: the banlist your Auto Duel and NPC duels use".into(),
        "draw [N] [DECK]: draw N (5) random cards from your newest deck, or draw5".into(),
        "where CARD: where a card comes from (packs, decks, Trade Card, rental)".into(),
        "report PLAYER reason: tell the staff about a player".into(),
    ]
}

/// The answer to a command, as the sender's window shows it: success, then one line each from the server.
pub(super) fn reply(lines: &[String]) -> Vec<Message> {
    std::iter::once(Message::reply(0x2456)).chain(lines.iter().map(|l| chat_note(SERVER, l))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whisper_to_the_server_is_a_command_and_a_bang_line_in_a_room_names_one() {
        assert_eq!(parse("banlist unlimited", Some("server")), Some(("banlist".into(), "unlimited".into())));
        assert_eq!(parse("/banlist", Some("Server")), Some(("banlist".into(), String::new())));
        assert_eq!(parse("whatever", Some("Server")), Some(("whatever".into(), String::new())));
        assert_eq!(parse("!draw 7", None), Some(("draw".into(), "7".into())));
        assert_eq!(parse("!!!", None), None, "not a command's name: ordinary chat");
        assert_eq!(parse("!lol that was close", None), None);
        assert_eq!(parse("banlist", None), None, "no bang, no whisper to the server");
        assert_eq!(parse("banlist", Some("Kaiba")), None);
    }

    #[test]
    fn long_answers_wrap() {
        let items: Vec<String> = (0..30).map(|i| format!("Card number {i}")).collect();
        let lines = wrap("Head: ", &items, ", ");
        assert!(lines.len() > 2 && lines.iter().all(|l| l.chars().count() <= LINE + 20), "{lines:?}");
        assert!(lines.join("").contains("Card number 29"));
    }
}
