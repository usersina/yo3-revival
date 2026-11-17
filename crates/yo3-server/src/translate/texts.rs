//! Message entries, option lists, declarations and the hints' words.
use super::*;

/// A data command per four values, which the pop appends in order (0x522A50).
pub(super) fn fours(id: u32, values: &[u32]) -> Vec<Message> {
    let command_of = |four: &[u32]| {
        let mut params = [0; 4];
        params[..four.len()].copy_from_slice(four);
        command(id, params)
    };
    values.chunks(4).map(command_of).collect()
}

/// The option list for MSG_SELECT_OPTION {options}, when the client has a label for every option (at most four).
/// A card's own strings (code << 20 | n) have labels only where CARD_OPTION_LABELS pairs them.
pub(super) fn option_list(m: &Json) -> Option<Vec<Message>> {
    let options = m["options"].as_array()?;
    let label = |d: u64| {
        let system = OPTION_LABELS.iter().find(|(o, _, _)| *o == d).map(|&(_, label, title)| (label, title));
        system.or_else(|| CARD_OPTION_LABELS.iter().find(|(o, _)| *o == d).map(|&(_, label)| (label, text::SELECT_EFFECT)))
    };
    let found: Option<Vec<(u32, u32)>> = options.iter().map(|o| label(o.as_u64()?)).collect();
    let found = found.filter(|f| !f.is_empty() && f.len() <= SELECT_MAX)?;
    let labels: Vec<u32> = found.iter().map(|(label, _)| label | dialog::LISTED).collect();
    let mut out = vec![command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0])];
    out.extend(fours(cmd::LIST_VALUES, &labels));
    out.extend(message(&[entry::TEXT | found[0].1]));
    out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
    Some(out)
}

/// The message entries (at most four) a dialog shows: 0x3EC {count}, 0x3ED {entries}.
pub(super) fn message(entries: &[u32]) -> Vec<Message> {
    let mut params = [0; 4];
    params[..entries.len()].copy_from_slice(entries);
    vec![command(cmd::MESSAGE_LENGTH, [entries.len() as u32, 0, 0, 0]), command(cmd::MESSAGE, params)]
}

/// The client's dialog for a declaration (MSG_ANNOUNCE_RACE / ATTRIB {count, available}, NUMBER {options}, CARD):
/// None when it can't ask it (several types at once, numbers outside the number dialog's range). `level`: the number is
/// a Level (hint 567 before it), which the dialog says; Duel.AnnounceLevel is only an alias of AnnounceNumberRange, and
/// the client's other cards declare counts and die rolls with it.
pub(super) fn declaration(m: &Json, level: bool) -> Option<Vec<Message>> {
    let kind = m["type"].as_u64()?;
    let allowed = m["available"].as_u64().unwrap_or(0);
    let bits = |table: &[u64]| table.iter().enumerate().filter(|(_, b)| allowed & **b != 0).fold(0u32, |f, (i, _)| f | 1 << (i + 1));
    let (dialog, text, param3) = match kind {
        msg::ANNOUNCE_RACE if m["count"].as_u64() == Some(1) => (dialog::TYPE, text::TYPE, bits(&TYPES)),
        msg::ANNOUNCE_ATTRIB => (dialog::ATTRIBUTE, text::ATTRIBUTE, bits(&ATTRIBUTES) << 16 | m["count"].as_u64().unwrap_or(1) as u32),
        msg::ANNOUNCE_NUMBER => {
            let options: Vec<u64> = m["options"].as_array()?.iter().filter_map(Json::as_u64).collect();
            let (unit, format) = number_unit(&options)?;
            let (min, max) = (options.iter().min()? / unit, options.iter().max()? / unit);
            let step = options.windows(2).map(|w| w[1].abs_diff(w[0]) / unit).filter(|&d| d > 0).min().unwrap_or(1);
            if min > 15 || step > 15 || max > 255 {
                return None;
            }
            let param3 = (max as u32) << 24 | (step as u32) << 20 | (min as u32) << 16 | format;
            // A Level gets "Declare monster level."; other numbers no message.
            if level {
                let mut out = message(&[entry::TEXT | text::LEVEL]);
                out.push(Cmd::Dialog { kind: dialog::NUMBER, message: dialog::FROM_ENTRIES, param: param3 }.message());
                return Some(out);
            }
            return Some(vec![Cmd::Dialog { kind: dialog::NUMBER, message: 0, param: param3 }.message()]);
        }
        msg::ANNOUNCE_CARD => return Some(vec![command(cmd::LIST_BEGIN, [0, 1, 0, 0]), command(cmd::CARD_LIST, [CLIENT, list::SEARCH, 0, 0])]),
        _ => return None,
    };
    let mut out = message(&[entry::TEXT | text]);
    out.push(Cmd::Dialog { kind: dialog, message: dialog::FROM_ENTRIES, param: param3 }.message());
    Some(out)
}

/// The client's number for EDOPro's Type or Attribute bits: its place in the client's list, plus one (CARD_Prop and the
/// declaration dialogs number them so). None without bits or for one the client lacks.
pub(super) fn client_number(bits: Option<u64>, table: &[u64]) -> Option<u32> {
    let bits = bits.filter(|&b| b != 0)?;
    table.iter().position(|&t| bits & t != 0).map(|i| i as u32 + 1)
}

/// The number dialog's unit and format for these options: the largest unit all of them are multiples of.
pub(super) fn number_unit(options: &[u64]) -> Option<(u64, u32)> {
    if options.is_empty() {
        return None;
    }
    NUMBER_FORMATS.iter().copied().find(|(unit, _)| options.iter().all(|o| o % unit == 0))
}

/// The duel prompt table's text for an EDOPro selection hint (HINTMSG_*), where one says the same thing.
pub(super) fn hint_text(hint: u64) -> Option<u32> {
    Some(match hint {
        hintmsg::RELEASE => text::TRIBUTE,
        501 => 347, // HINTMSG_DISCARD: "Select the @2card(s)@0 in your hand you want to discard."
        502 => 281, // HINTMSG_DESTROY: "Select @3card@0 to @3destroy@0."
        503 => 70,  // HINTMSG_REMOVE: "Select @3card(s)@0 to @3remove from play@0."
        504 => 58,  // HINTMSG_TOGRAVE: "Select the @3card@0 to send to the @2Graveyard@0."
        505 => 348, // HINTMSG_RTOHAND: "Select the @3card(s)@0 to return to the @2hand@0."
        506 => 88,  // HINTMSG_ATOHAND: "Select the @3card(s)@0 to add to your @2hand@0."
        507 => 78,  // HINTMSG_TODECK: "Select a @3card@0 to be returned the owner's @2Deck@0."
        508 => 39,  // HINTMSG_SUMMON: "Select a @3monster@0 to @3Normal Summon or Set@0."
        509 => 25,  // HINTMSG_SPSUMMON: "Select the @3monster(s)@0 to @3Special Summon@0."
        510 => 46,  // HINTMSG_SET: "Select a @3Spell or Trap Card@0 to @3Set@0 on the field."
        511 => 92,  // HINTMSG_FMATERIAL: "Select @3monsters@0 to use as @3Fusion Material@0."
        // HINTMSG_FACEUP: the client has no "face-up card" prompt; EDOPro asks it for targets (Spellbinding Circle).
        514 => 94,
        515 => 273, // HINTMSG_FACEDOWN: "Select the @3monster(s)@0 in @3face-down@0 Position."
        518 => 101, // HINTMSG_EQUIP: "Select a @3monster@0 to @3equip@0."
        520 => 119, // HINTMSG_CONTROL: "Select the @3monster(s)@0 your opponent controls to take control."
        526 => 341, // HINTMSG_CONFIRM: "Select the @3card(s)@0 to show your opponent."
        528 => 357, // HINTMSG_POSCHANGE: "Select the @3monster@0 to change its battle position."
        549 => 222, // HINTMSG_ATTACKTARGET: "Select an attack target @3monster@0."
        550 => text::SELECT_EFFECT, // HINTMSG_EFFECT: "Select effect to @3activate@0."
        551 => 94,  // HINTMSG_TARGET: "Select the @3card(s)@0 to target."
        _ => return None,
    })
}

/// The server's words for EDOPro's selection hints (HINTMSG_*) that the client's prompt table has no text for, in
/// the table's style (`@3` highlights, `@2` places). Shown through the connection DLL (`Translator::text`).
pub(super) const HINT_WORDS: [(u64, &str); 38] = [
    (512, "Select the @3monster(s)@0 to use as @3Synchro Material@0."),
    (513, "Select the @3monsters@0 to use as @3Xyz Material@0."),
    (516, "Select a @3monster@0 in @3Attack Position@0."),
    (517, "Select a @3monster@0 in @3Defense Position@0."),
    (519, "Select the @3Xyz Material@0 to detach."),
    (521, "Select the @3card@0 to destroy instead."),
    (522, "Select a @3monster@0 in face-up @3Attack Position@0."),
    (523, "Select a @3monster@0 in face-up @3Defense Position@0."),
    (524, "Select a @3monster@0 in face-down @3Attack Position@0."),
    (525, "Select a @3monster@0 in face-down @3Defense Position@0."),
    (527, "Select the @3card@0 to place on the field."),
    (529, "Select a @3card@0 you control."),
    (530, "Select a @3card@0 your opponent controls."),
    (531, "Select the @3monster(s)@0 to @3Tribute@0."),
    (532, "Select the @3Xyz Monster@0 to detach material from."),
    (533, "Select the @3monsters@0 to use as @3Link Material@0."),
    (552, "Call heads or tails."),
    (553, "Select a die result."),
    (554, "Declare a @3card type@0."),
    (555, "Select an option."),
    (556, "Select the effect to apply."),
    (560, "Select a @3card@0."),
    (561, "Select a battle position."),
    (562, "Declare an @3Attribute@0."),
    (563, "Declare a @3Type@0."),
    (564, "Declare a @3card name@0."),
    (565, "Declare a number."),
    (566, "Select the @3effect@0 to activate."),
    (567, "Declare a @3Level@0."),
    (568, "Select the @3card@0 whose effect to apply."),
    (569, "Select a @3zone@0."),
    (570, "Select a @3zone@0 to make unusable."),
    (572, "Select the @3card@0 to place a @3counter@0 on."),
    (575, "Select the @3card@0 to negate."),
    (576, "Select the @3monster@0 whose @3ATK/DEF@0 changes."),
    (577, "Select the @3card@0 to apply the effect to."),
    (578, "Select the @3card(s)@0 to attach as @3Xyz Material@0."),
    (579, "Select the @3banished card(s)@0 to return to the @2Graveyard@0."),
];

/// How many characters fit on one of the Select dialog's labels (one line; live, 57 of Pyramid Energy's 61 showed).
pub(super) const LABEL_CHARS: usize = 56;

/// `text` cut to `max` characters at a word, with "…" to say so.
pub(super) fn shortened(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max - 1).collect();
    let cut = cut.rfind(' ').filter(|&at| at > max / 2).map_or(cut.as_str(), |at| &cut[..at]);
    format!("{}…", cut.trim_end_matches([' ', ',', '.', ';']))
}

/// The client's duel-end reason (text 1999 + reason, 0x508650) for an EDOPro win reason.
pub(super) fn end_reason(reason: u64) -> u32 {
    match reason {
        0 => 12,    // "has surrendered" (12, not 5: 106 of Konami's 227 logs end on 12, none on 5)
        1 => 1,     // "Life Points have been reduced to 0"
        2 => 2,     // "has run out of cards in the Deck"
        3 => 11,    // "has ran out of time"
        4 => 14,    // "has disconnected"
        0x10 => 3,  // Exodia the Forbidden One
        0x11 => 7,  // Final Countdown
        0x12 => 9,  // Vennominaga the Deity of Poisonous Snakes
        0x14 => 10, // Exodius the Ultimate Forbidden Lord
        0x15 => 4,  // Destiny Board
        0x16 => 6,  // Last Turn
        _ => 8,     // "has won with" a card
    }
}

/// The panel's phase (command 5) for an ocgcore phase: 0 Draw, 1 Standby, 2 Main 1, 3 Battle (all its steps),
/// 4 Main 2, 5 End.
pub(super) fn client_phase(phase: u64) -> Option<u32> {
    Some(match phase {
        1 => 0,
        2 => 1,
        4 => 2,
        0x08..=0x80 => 3,
        0x100 => 4,
        0x200 => 5,
        _ => return None,
    })
}
