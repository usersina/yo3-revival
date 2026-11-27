//! The windows the client gets for the engine's prompts: command maps, dialogs, card lists, zone choices.
use super::*;

impl Translator {
    /// What to do with the engine's prompt (the last message when it awaits).
    pub fn prompt(&mut self, m: &Json) -> Prompt {
        let staged = self.staged.take();
        self.list_order.clear();
        self.asking = Asking::Prompt;
        let prompt = self.prompt_for(m);
        // A window's hint is for the prompt right after it, whichever it was.
        self.phase_window = false;
        self.after_chain = false;
        match prompt {
            Prompt::Client(out) => {
                let mut all = self.unstage_n(staged);
                if std::mem::take(&mut self.taken_back) {
                    all.insert(0, Cmd::BattleEnd.message());
                }
                all.extend(out);
                with_card_ids(&self.cards, &mut all);
                Prompt::Client(all)
            }
            answer => {
                self.staged = self.staged.or(staged);
                answer
            }
        }
    }

    pub(super) fn prompt_for(&mut self, m: &Json) -> Prompt {
        let ask = match Ask::lenient(m) {
            Ok(Some(ask)) => ask,
            _ => return Prompt::Answer(cpu_answer(m)),
        };
        if self.side(ask.player()) == CPU {
            if let Ask::CardName(_) = ask {
                return Prompt::Answer(self.cpu_card_name(m));
            }
            return Prompt::Answer(cpu_answer(m));
        }
        match &ask {
            Ask::IdleCmd(_) | Ask::BattleCmd(_) => {
                let (buttons, mode): ([(bool, u8); 2], u32) = match &ask {
                    Ask::IdleCmd(p) => ([(p.to_bp, flag::BUTTON_BP), (p.to_ep, flag::BUTTON_EP)], mode::MAIN),
                    Ask::BattleCmd(p) => ([(p.to_m2, flag::BUTTON_M2), (p.to_ep, flag::BUTTON_EP)], mode::BATTLE),
                    _ => unreachable!(),
                };
                self.attacker = None;
                let mut map = self.command_map(&ask);
                self.add_pile_views(&mut map);
                self.add_surrender(&mut map);
                let flags: Vec<(u8, u8)> = buttons.into_iter().filter(|(on, _)| *on).map(|(_, b)| (b, 1)).collect();
                self.pending = Some(m.clone());
                Prompt::Client(vec![command_data(&map, &flags), command(cmd::INPUT, [mode, CLIENT, 0, 0])])
            }
            Ask::EffectYn(_) | Ask::YesNo(_) => {
                let (description, description_cid, description_text) = match &ask {
                    Ask::EffectYn(p) => (p.effect.description, p.effect.description_cid, p.effect.description_text.clone()),
                    Ask::YesNo(p) => (p.description, p.description_cid, p.description_text.clone()),
                    _ => unreachable!(),
                };
                let battle = BATTLE_QUESTIONS.iter().find(|(d, _)| description == *d).map(|(_, text)| *text);
                let entries = match (&ask, battle) {
                    (Ask::EffectYn(p), _) => vec![entry::TEXT | text::ACTIVATE, entry::CARD_NAME | p.effect.loc.cid.unwrap_or(0) as u32],
                    (_, Some(text)) => vec![entry::TEXT | text],
                    _ => match description_cid {
                        Some(cid) => vec![entry::TEXT | text::USE, entry::CARD_NAME | cid as u32],
                        None => vec![entry::TEXT | text::USE_NOW],
                    },
                };
                // A card's own question (its string as the description) under the client's line, through the DLL.
                let mut out = self.pile_views_under_question();
                let mut entries = entries;
                if let Some(question) = description_text.as_deref().filter(|t| !t.is_empty() && entries.len() <= 2) {
                    let id = self.text(&mut out, question);
                    entries.extend([entry::LINE_BREAK, entry::TEXT | id]);
                }
                out.extend(message(&entries));
                out.push(Cmd::Dialog { kind: dialog::YES_NO, message: dialog::FROM_ENTRIES, param: 0 }.message());
                self.pending = Some(m.clone());
                Prompt::Client(out)
            }
            Ask::Card(choice) | Ask::Tribute(choice) => {
                let hint = if matches!(ask, Ask::Tribute(_)) { hintmsg::RELEASE } else { std::mem::take(&mut self.hint) };
                if hint == hintmsg::ATTACKTARGET
                    && let Some(out) = self.target_on_field(m)
                {
                    return Prompt::Client(out);
                }
                if matches!(ask, Ask::Card(_))
                    && let Some(out) = self.effect_target_on_field(m, choice, hint)
                {
                    return Prompt::Client(out);
                }
                let view = if matches!(ask, Ask::Tribute(_)) { tribute_view(m, choice) } else { choice.clone() };
                Prompt::Client(self.choice_list(m, &view, hint))
            }
            // Materials, one at a time: the card list until the engine lets the player finish and nothing is left.
            Ask::UnselectCard(u) if !u.select_cards.is_empty() => {
                let open = u.can_finish || u.can_cancel;
                let list = CardList { selects: u.select_cards.clone(), min: if open { 0 } else { 1 }, max: 1, cancelable: false };
                let mut out = vec![];
                let hint = std::mem::take(&mut self.hint);
                let text = self.hint_message(&mut out, hint);
                out.extend(self.card_list(&list, text));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            // A sum (tributes or discards that make a total Level): the card list, as many picks as the bounds allow;
            // picks that don't make the sum bring the list back. Only when some picks can make it: a list nothing
            // satisfies would hold the player forever, so that one gets the automatic answer (and the core's
            // refusals end the duel, see `Repeats`).
            Ask::Sum(sum) if !sum.selects.is_empty() && sum_fits(sum, &sum_choice(sum)) => {
                // Kind 1 (min 0): any number of picks, confirmed with a right-click (dlg_cardlist.swf has no OK
                // button). A list of kind 0x27 with a larger max closes only at max picks (live: Machina Fortress's
                // discards held the player at two picks of six).
                let n = sum.selects.len();
                let (_, max) = sum_bounds(sum);
                let list = CardList { selects: sum.selects.iter().map(|c| c.loc.clone()).collect(), min: 0, max: max.min(n).max(1) as u64, cancelable: false };
                // The engine's hint (Machina Fortress: "Select the card(s) in your hand you want to discard."), kept for
                // the list shown again, else the prompt table's 207 "Tribute the necessary monsters to match the
                // required number of Level.".
                let mut out = vec![];
                let hint = std::mem::take(&mut self.hint);
                let text = self.hint_message(&mut out, hint).or(self.sum_text).unwrap_or(text::SUM);
                self.sum_text = Some(text);
                out.extend(self.card_list(&list, Some(text)));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            Ask::Position(p) => match self.position_dialog(p) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(m)),
            },
            Ask::Option(o) => match option_list(o).or_else(|| self.worded_options(o)) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                // More options than the Select dialog's four labels, or one with no words: the card list, the option's
                // card once per option in the engine's order, rather than the first option for the player.
                None if o.options.len() > 1 => {
                    let card = |i: usize| o.option_cids.get(i).copied().filter(|&c| c != 0)
                        .or_else(|| self.chaining.as_ref().and_then(|c| c.loc.cid))
                        .unwrap_or(0);
                    let selects: Vec<Loc> = (0..o.options.len()).map(|i| Loc { controller: o.player, cid: Some(card(i)), ..Loc::default() }).collect();
                    let out = self.card_list(&CardList { selects, min: 1, max: 1, cancelable: false }, Some(text::SELECT_EFFECT));
                    // What each option does, from the engine's card strings: the chat window, numbered as listed.
                    for (i, text) in o.option_texts.iter().enumerate() {
                        if let Some(text) = text.as_deref().filter(|t| !t.is_empty()) {
                            self.notes.push(format!("{}: {text}", i + 1));
                        }
                    }
                    self.option_cards = true;
                    self.picks.clear();
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(m)),
            },
            Ask::Chain(chain) => self.chain_chance(m, chain),
            // An order: chain links triggered together (21; the first pick becomes chain link 1) or cards put back on
            // the Deck (25, Big Eye). The card list orders the picks itself (dlg_cardlist.swf: a badge per click, a
            // second click takes it back; kind 6 needs all of them), and each card's rank answers the engine.
            Ask::SortChain(sort) | Ask::SortCard(sort) if sort.cards.len() >= 2 => {
                let n = sort.cards.len() as u64;
                let text = if matches!(ask, Ask::SortCard(_)) { text::SORT } else { text::SELECT_EFFECT };
                let out = self.card_list(&CardList { selects: sort.cards.clone(), min: n, max: n, cancelable: false }, Some(text));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            Ask::Race(_) | Ask::Attrib(_) | Ask::Number(_) | Ask::CardName(_) => {
                let hint = std::mem::take(&mut self.hint);
                match declaration(&ask, hint == hintmsg::LVRANK) {
                    Some(out) => {
                        self.pending = Some(m.clone());
                        Prompt::Client(out)
                    }
                    None if matches!(ask, Ask::CardName(_)) => Prompt::Answer(self.cpu_card_name(m)),
                    // Several Types, numbers beyond the number dialog: asked in steps where the dialogs can.
                    None => match self.declaration_steps(&ask, hint) {
                        Some(out) => {
                            self.pending = Some(m.clone());
                            Prompt::Client(out)
                        }
                        None => Prompt::Answer(cpu_answer(m)),
                    },
                }
            }
            // A zone an effect takes (Ground Collapse's, Senet Switch's move): the client's own prompt in an OK dialog, then
            // the zones as click targets (`zone_map`).
            Ask::Disfield(p) if !free_zones(p).is_empty() => self.zone_prompt(m),
            // Where a summoned, Set or activated card goes, with auto card placing off and a choice to make.
            Ask::Place(p) if !self.auto_place && free_zones(p).len() > p.count.max(1) as usize => self.zone_prompt(m),
            // Where a summoned, Set or activated card goes: chosen for the player, centre out, as Konami's server did.
            _ => Prompt::Answer(cpu_answer(m)),
        }
    }

    /// The piles' Confirm entries as a command map with the input wait, sent before a question's dialog ("Activate X?"):
    /// the field takes clicks only in a command wait, so without it the Graveyards and banished piles could only be
    /// hovered while the question was up (#16). A pile's click opens its view (`pile_view`), whose closing brings the
    /// question back. Empty when no pile has a card.
    pub(super) fn pile_views_under_question(&self) -> Vec<Message> {
        let mut map = vec![];
        self.add_pile_views(&mut map);
        // The Extra Deck's "See Details" has nothing to do with a question.
        map.retain(|(place, _)| place >> 8 & 0xFF != zone::EXTRA);
        if map.is_empty() { vec![] } else { vec![command_data(&map, &[]), command(cmd::INPUT, [mode::MAIN, CLIENT, 0, 0])] }
    }

    /// A zone prompt (MSG_SELECT_DISFIELD, or MSG_SELECT_PLACE with auto card placing off): the client's own prompt in
    /// an OK dialog, then the zones as click targets (`zone_map`).
    pub(super) fn zone_prompt(&mut self, m: &Json) -> Prompt {
        let hint = std::mem::take(&mut self.hint);
        let zones = free_zones(&typed::<SelectPlace>(m));
        let spells = zones.iter().all(|&(_, location, _)| location == loc::SZONE);
        let mut out = vec![];
        // The client's prompts are the zone-disabling effects' ("Select Monster Card Zones to render unusable."); a
        // placement is worded by the server through the connection DLL, in the client's terms.
        let text = if matches!(Ask::lenient(m), Ok(Some(Ask::Place(_)))) {
            self.text(&mut out, if spells { PLACE_SPELL_WORDS } else { PLACE_MONSTER_WORDS })
        } else if hint == hintmsg::TOZONE {
            text::ADJACENT_ZONE
        } else if spells {
            text::SPELL_ZONE
        } else {
            text::MONSTER_ZONE
        };
        out.extend(message(&[entry::TEXT | text]));
        out.push(Cmd::Dialog { kind: dialog::OK, message: dialog::FROM_ENTRIES, param: 0 }.message());
        self.pending = Some(m.clone());
        self.asking = Asking::Prompt;
        Prompt::Client(out)
    }

    /// The Select dialog for options the client has no label for, worded by the server through the connection DLL
    /// (`text`): each option the client's own label where it has one, else the engine's text for it (the card's
    /// string). None for more than the dialog's four labels or an option with no words. The title is the hint's, else
    /// "Select effect to activate." (276).
    pub(super) fn worded_options(&mut self, o: &SelectOption) -> Option<Vec<Message>> {
        if o.options.is_empty() || (0..o.options.len()).any(|i| option_words(o, i).is_none()) {
            return None;
        }
        let mut out = vec![];
        let hint = std::mem::take(&mut self.hint);
        let title = self.hint_message(&mut out, hint).unwrap_or(text::SELECT_EFFECT);
        if o.options.len() > SELECT_MAX {
            // More than the dialog's four labels: pages of three and "More choices", the last page's going round to the
            // first (#16; before, the card list with the options' texts in the chat window).
            out.extend(self.option_page(o, 0, title));
            return Some(out);
        }
        let labels: Vec<u32> = (0..o.options.len()).map(|i| self.option_label(&mut out, o, i)).collect();
        out.extend(select_dialog(&labels, title));
        Some(out)
    }

    /// One option's label in the Select dialog: the client's own, else the engine's text for it through the DLL.
    fn option_label(&mut self, out: &mut Vec<Message>, o: &SelectOption, i: usize) -> u32 {
        let label = match option_words(o, i) {
            Some(OptionWords::Own(label)) => label,
            Some(OptionWords::Text(text)) => self.text(out, &shortened(text, LABEL_CHARS)),
            None => text::SELECT_EFFECT,
        };
        label | dialog::LISTED
    }

    /// A page of an option prompt too long for one Select dialog: up to `OPTION_PAGE` options from `start`, then "More
    /// choices", which `Steps::Options` turns into the next page.
    pub(super) fn option_page(&mut self, o: &SelectOption, start: usize, title: u32) -> Vec<Message> {
        let mut out = vec![];
        let end = (start + OPTION_PAGE).min(o.options.len());
        let mut labels: Vec<u32> = (start..end).map(|i| self.option_label(&mut out, o, i)).collect();
        let pages = o.options.len().div_ceil(OPTION_PAGE);
        let more = self.text(&mut out, &format!("More choices ({}/{pages})", start / OPTION_PAGE + 1));
        labels.push(more | dialog::LISTED);
        out.extend(select_dialog(&labels, title));
        self.asking = Asking::Steps(Steps::Options { start, title });
        out
    }

    /// The CPU's card name declaration: the cards in the duel by turns, a new one each time it is asked.
    pub(super) fn cpu_card_name(&mut self, m: &Json) -> Json {
        // A card the effect's filter allows (the sidecar's `fits`, the duel's own cards first).
        if let Some(&cid) = typed::<AnnounceCard>(m).fits.first() {
            return Response::CardName(cid).json();
        }
        let mut cids: Vec<u16> =
            self.sides.iter().flat_map(|s| s.zones.values()).flatten().filter_map(|c| self.cards.get(&c.row)).map(|c| c.cid).collect();
        cids.sort_unstable();
        cids.dedup();
        let cid = cids.get(self.declared % cids.len().max(1)).copied().unwrap_or(0);
        self.declared += 1;
        Response::CardName(cid as u64).json()
    }

    /// The position dialog for MSG_SELECT_POSITION {cid, positions}, with the card's image (any uid with its id) and
    /// name. None without an attack position: the first button is always attack.
    ///
    /// A token is asked its position before it exists (MSG_SELECT_PLACE, MSG_SELECT_POSITION, then its MSG_MOVE from
    /// nowhere): the dialog finds no card by uid and shows nothing to click, so the duel stops there (Gorz the
    /// Emissary of Darkness's token, live). It gets the uid and token card it will appear as now, in the card table at
    /// a free monster zone of its player (`token` puts it at its own zone).
    pub(super) fn position_dialog(&mut self, p: &SelectPosition) -> Option<Vec<Message>> {
        let allowed = p.positions;
        let (attacks, defenses) = position_sides(allowed);
        // One position is no choice; two of one side are asked in steps (see `Steps`).
        if attacks.is_empty() && defenses.len() < 2 {
            return None;
        }
        let cid = p.cid.unwrap_or(0) as u16;
        let mut out = vec![];
        // The message names only the card's id, so the dialog shows a copy of it: one the client can see. The
        // opponent's hidden copies (hand, deck, extra deck) have row 0 there and draw nothing (live: Doom Shaman
        // summoned from an NPC's Graveyard while another sat in its hand). When every copy is hidden, the one shown
        // gets its row first: it is about to be summoned from where it is.
        let mut copies: Vec<(u32, u16, Place)> = vec![];
        for (p, side) in self.sides.iter().enumerate() {
            for (&zone, cards) in &side.zones {
                for (index, c) in cards.iter().enumerate() {
                    if cid != 0 && self.cards.get(&c.row).map(|s| s.cid) == Some(cid) {
                        copies.push((c.uid, c.row, Place { player: p as u32, zone, index, face_up: true, defense: false }));
                    }
                }
            }
        }
        copies.sort_by_key(|(uid, ..)| *uid);
        let seen = copies.iter().find(|(_, row, place)| self.shown_row(place, *row) != 0);
        let held = match seen.or(copies.first()) {
            Some(&(uid, row, ref place)) => {
                if self.shown_row(place, row) == 0 {
                    out.push(Cmd::Card { uid, open: false, row: row as u32, place: table_place(place.player, place.zone, place.index) }.message());
                }
                Some(uid)
            }
            None => None,
        };
        let (uid, name) = match (held, self.token_card_rows(p.cid, p.name.as_deref())) {
            (Some(uid), _) => (uid, cid),
            (None, Some(rows)) => {
                // The same one when the dialog opens again (after a look at a pile).
                let (uid, row) = match self.reserved_token {
                    Some((uid, row)) if rows.contains(&row) => (uid, row),
                    _ => self.next_token_card(&rows),
                };
                self.reserved_token = Some((uid, row));
                let player = self.side(p.player);
                let side = &self.sides[player as usize];
                let zone = (0..5).find(|z| side.zones.get(z).is_none_or(Vec::is_empty)).unwrap_or(0);
                out.push(Cmd::Card { uid, open: false, row: row as u32, place: table_place(player, zone, 0) }.message());
                // A token without a Konami id is named by its client card's.
                (uid, self.cards.get(&row).map_or(cid, |s| s.cid))
            }
            (None, None) => (0, cid),
        };
        let face_up = if allowed & pos::FACEUP_DEFENSE != 0 { dialog::FACE_UP_DEFENSE } else { 0 };
        // Two positions of the only side there is: no side to choose, the Select dialog at once.
        let only = if attacks.is_empty() { Some(defenses.clone()) } else if defenses.is_empty() && attacks.len() > 1 { Some(attacks.clone()) } else { None };
        if let Some(options) = only {
            out.extend(self.position_choice(&options, name as u32));
            self.asking = Asking::Steps(Steps::Position { options });
            return Some(out);
        }
        out.extend(message(&[entry::TEXT | text::POSITION, entry::CARD_NAME | name as u32]));
        out.push(Cmd::Dialog { kind: dialog::POSITION, message: dialog::FROM_ENTRIES, param: uid | face_up }.message());
        if attacks.len() > 1 || defenses.len() > 1 {
            self.asking = Asking::Steps(Steps::PositionSide { name: name as u32 });
        }
        Some(out)
    }

    /// The Select dialog of a position prompt's positions on one side (two, face-up and face-down), under the position
    /// dialog's message, worded by the server through the connection DLL.
    pub(super) fn position_choice(&mut self, options: &[u64], name: u32) -> Vec<Message> {
        let mut out = vec![];
        let labels: Vec<u32> = options
            .iter()
            .map(|&p| POSITION_WORDS.iter().find(|(bit, _)| *bit == p).map_or("Position", |&(_, words)| words))
            .map(|words| self.text(&mut out, words) | dialog::LISTED)
            .collect();
        out.push(command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0]));
        out.extend(fours(cmd::LIST_VALUES, &labels));
        out.extend(message(&[entry::TEXT | text::POSITION, entry::CARD_NAME | name]));
        out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
        out
    }

    /// The steps of a declaration the client's dialogs can't make in one (several Types, a number out of the number
    /// dialog's range): the first window, with the state the next result needs. None when it can't be asked at all
    /// (fewer Types than declared, no options), which takes the automatic answer.
    pub(super) fn declaration_steps(&mut self, ask: &Ask, hint: u64) -> Option<Vec<Message>> {
        match ask {
            Ask::Race(a) if a.count > 1 && TYPES.iter().filter(|&&b| a.available & b != 0).count() as u64 >= a.count => {
                self.asking = Asking::Steps(Steps::Types { picked: vec![] });
                Some(self.type_step(a, &[]))
            }
            Ask::Number(n) if !n.options.is_empty() => {
                let mut out = vec![];
                let title = if hint == hintmsg::LVRANK {
                    text::LEVEL
                } else {
                    match self.hint_message(&mut out, hint) {
                        Some(title) => title,
                        None => self.text(&mut out, "Declare a number."),
                    }
                };
                let mut order: Vec<usize> = (0..n.options.len()).collect();
                order.sort_by_key(|&i| (n.options[i], i));
                out.extend(self.number_step(&n.options, order, title));
                Some(out)
            }
            _ => None,
        }
    }

    /// One Type's dialog of several: the types not declared yet, "Select Type n of m.".
    pub(super) fn type_step(&mut self, a: &Announce, picked: &[u64]) -> Vec<Message> {
        let left = picked.iter().fold(a.available, |left, p| left & !p);
        let bits = TYPES.iter().enumerate().filter(|(_, b)| left & **b != 0).fold(0u32, |f, (i, _)| f | 1 << (i + 1));
        let mut out = vec![];
        let words = self.text(&mut out, &format!("Select @3Type {} of {}@0.", picked.len() + 1, a.count));
        out.extend(message(&[entry::TEXT | words]));
        out.push(Cmd::Dialog { kind: dialog::TYPE, message: dialog::FROM_ENTRIES, param: bits }.message());
        out
    }

    /// A number question narrowed by ranges: `candidates` (indices into the engine's options, by value) in at most four
    /// labels of equal share, each a range of their values or, with four or fewer, one value each. The Select
    /// dialog's result picks the share, `Steps::Number` remembering them.
    pub(super) fn number_step(&mut self, options: &[u64], candidates: Vec<usize>, title: u32) -> Vec<Message> {
        let size = if candidates.len() <= SELECT_MAX { 1 } else { candidates.len().div_ceil(SELECT_MAX) };
        let groups: Vec<Vec<usize>> = candidates.chunks(size).map(<[usize]>::to_vec).collect();
        let mut out = vec![];
        let labels: Vec<u32> = groups
            .iter()
            .map(|g| {
                let (lo, hi) = (options[g[0]], options[g[g.len() - 1]]);
                let words = if lo == hi { lo.to_string() } else { format!("{lo}-{hi}") };
                self.text(&mut out, &words) | dialog::LISTED
            })
            .collect();
        out.push(command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0]));
        out.extend(fours(cmd::LIST_VALUES, &labels));
        out.extend(message(&[entry::TEXT | title]));
        out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
        self.asking = Asking::Steps(Steps::Number { groups, title });
        out
    }

    /// A result of a stepped choice (`Steps`): the next window, or the answer.
    pub(super) fn step_result(&mut self, steps: Steps, count: u32) -> Option<Prompt> {
        let m = self.pending.clone()?;
        let ask = Ask::lenient(&m).ok()??;
        // A result the step has no answer for (a stale click) asks the whole question again.
        let again = |t: &mut Translator| Some(t.prompt(&m));
        match (steps, &ask) {
            (Steps::PositionSide { name }, Ask::Position(p)) => {
                let (attacks, defenses) = position_sides(p.positions);
                let side = if count == 0 { attacks } else { defenses };
                match side[..] {
                    [] => again(self),
                    [one] => {
                        self.pending = None;
                        Some(Prompt::Answer(Response::Position(one).json()))
                    }
                    _ => {
                        let out = self.position_choice(&side, name);
                        self.asking = Asking::Steps(Steps::Position { options: side });
                        Some(Prompt::Client(out))
                    }
                }
            }
            (Steps::Position { options }, Ask::Position(_)) => match options.get(count as usize) {
                Some(&position) => {
                    self.pending = None;
                    Some(Prompt::Answer(Response::Position(position).json()))
                }
                None => again(self),
            },
            (Steps::Types { mut picked }, Ask::Race(a)) => {
                let race = TYPES.get((count as usize).wrapping_sub(1)).copied().filter(|r| a.available & r != 0 && !picked.contains(r));
                if let Some(race) = race {
                    picked.push(race);
                }
                if picked.len() as u64 >= a.count {
                    self.pending = None;
                    return Some(Prompt::Answer(Response::Race(picked).json()));
                }
                let out = self.type_step(a, &picked);
                self.asking = Asking::Steps(Steps::Types { picked });
                Some(Prompt::Client(out))
            }
            (Steps::Number { groups, title }, Ask::Number(n)) => match groups.get(count as usize) {
                Some(group) if group.len() == 1 => {
                    self.pending = None;
                    Some(Prompt::Answer(Response::Number(group[0] as u64).json()))
                }
                Some(group) => Some(Prompt::Client(self.number_step(&n.options, group.clone(), title))),
                // Back to this step's labels (the texts they used may be gone from the DLL's slots).
                None => Some(Prompt::Client(self.number_step(&n.options, groups.concat(), title))),
            },
            (Steps::Options { start, title }, Ask::Option(o)) => {
                let shown = OPTION_PAGE.min(o.options.len().saturating_sub(start));
                let count = count as usize;
                if count < shown {
                    self.pending = None;
                    return Some(Prompt::Answer(self.chosen_option(o, start + count)));
                }
                // "More choices" (or a stale click): the next page, after the last the first.
                let next = if count == shown && start + OPTION_PAGE < o.options.len() { start + OPTION_PAGE } else { 0 };
                Some(Prompt::Client(self.option_page(o, next, title)))
            }
            _ => None,
        }
    }

    /// MSG_SELECT_CHAIN {spe_count, forced, hint_timing, selects}: the question shows the cards, names the chain's last
    /// link, and says how much it matters, so the player's Activation Message option decides whether it shows. A forced
    /// chain of several goes straight to the card list, of one needs no choice.
    pub(super) fn chain_chance(&mut self, m: &Json, chain: &SelectChain) -> Prompt {
        let selects = &chain.selects;
        let forced = chain.forced;
        let phase_window = self.phase_window || self.after_chain;
        if selects.is_empty() {
            return Prompt::Answer(Response::Chain(None).json());
        }
        if forced && selects.len() == 1 {
            return Prompt::Answer(Response::Chain(Some(0)).json());
        }
        self.pending = Some(m.clone());
        self.picks.clear();
        self.chain_list = forced;
        if forced {
            return Prompt::Client(self.card_list(&CardList::chain(m), Some(text::SELECT_EFFECT)));
        }
        let rows: Vec<u32> = selects.iter().map(|e| self.row_of(&e.loc) as u32).collect();
        let mut out = self.pile_views_under_question();
        out.push(command(cmd::LIST_BEGIN, [rows.len() as u32, 0, dialog::WITH_CARDS, 0]));
        out.extend(fours(cmd::LIST_ROWS, &rows));
        let entries = match &self.chaining {
            Some(link) => {
                let spell_trap = link.loc.location == loc::SZONE;
                let text = if spell_trap { text::CHAIN_CARD } else { text::CHAIN_EFFECT };
                vec![entry::TEXT | text, entry::CARD_NAME | link.loc.cid.unwrap_or(0) as u32]
            }
            None => vec![entry::TEXT | text::ACTIVATE_CARD],
        };
        out.extend(message(&entries));
        let timing = chain.hint_timing | chain.hint_timing_other;
        let level = if chain.spe_count == 0 {
            dialog::LEVEL_UNFIT
        } else if self.chaining.is_some() || timing & !QUIET_TIMINGS != 0 || phase_window {
            // A phase's own window counts its optional triggers in spe_count (Stardust Dragon's return in the End
            // Phase): at the phase level the default Activation Message answered No for the player, and the chance
            // was gone (#53). The core doesn't say which cards are triggers, so the whole window shows. The same for the
            // chance right after a chain resolved: Doppelwarrior's offer after Junk Synchron's revival (2026-11-22).
            dialog::LEVEL_EVENT
        } else {
            dialog::LEVEL_PHASE
        };
        out.push(Cmd::Dialog { kind: dialog::EFFECT, message: dialog::FROM_ENTRIES, param: level }.message());
        Prompt::Client(out)
    }

    /// The card list for a card choice: its rows and marks, the message, then the dialog. The kind gives the picks OK
    /// needs: none for min 0, one for min 1, else exactly min (the list's max is lowered to it). A list holding a
    /// hidden card (a Set card has no row for the client) opens as the card-back kind with a table entry per card
    /// (flag set for the cards shown), so face-up cards keep their faces and Set cards are backs, whatever the picks
    /// needed (Brionac's several, #90: the other kinds drew a Set card blank and unclickable); no pick is needed to
    /// close it, so a forced choice closed with fewer than its picks asks again.
    pub(super) fn card_list(&mut self, list: &CardList, text: Option<u32>) -> Vec<Message> {
        let engine_order = &list.selects;
        let mut order: Vec<usize> = (0..engine_order.len()).collect();
        order.sort_by_key(|&i| self.field_order(&engine_order[i], i));
        let shown: Vec<&Loc> = order.iter().map(|&i| &engine_order[i]).collect();
        let selects = &shown[..];
        self.list_order = order;
        let (min, max) = (list.min as u32, list.max as u32);
        // A choice the player may back out of (can_cancel, or a chain list that isn't forced) opens as kind 1: the list
        // (no OK button) closes on a right-click with the picks so far, none cancelling (dlg_cardlist.swf `onMouseUp`
        // with `_cancelable`, which 0x4C73A0 always sets).
        let min = if list.cancelable { 0 } else { min };
        let (kind, max) = match min {
            0 => (list::PICK_ANY, max),
            1 => (list::PICK_ONE, max),
            _ => (list::PICK_ALL, min),
        };
        let rows: Vec<u32> = selects.iter().map(|loc| self.list_row(loc)).collect();
        let mixed = rows.contains(&0);
        self.mixed = mixed;
        let (kind, player) = if mixed { (list::PICK_MIXED, CPU) } else { (kind, CLIENT) };
        let marks: Vec<u32> = selects.iter().map(|loc| if self.side(loc.controller) == CPU { list::OPPONENTS } else { 0 }).collect();
        let mut out = vec![];
        if mixed {
            for (i, &row) in rows.iter().enumerate() {
                let flag = if row != 0 { OPEN } else { 0 };
                out.push(Cmd::Card { uid: (STAGED_UID + i as u32) | flag, open: false, row, place: table_place(CPU, zone::REMOVED, i) }.message());
            }
            self.staged = Some(rows.len());
        }
        out.push(command(cmd::LIST_BEGIN, [selects.len() as u32, max, 0, 0]));
        out.extend(fours(cmd::LIST_ROWS, &rows));
        out.extend(fours(cmd::LIST_MARKS, &marks));
        // A list that needs several picks says how many (min == max here): the hint alone left a player of Magical Hats
        // with one card marked and no hint that a second one decides.
        let several = list.min >= 2 && max >= 2;
        let entries: Vec<u32> = match (text, several) {
            (Some(text), true) => vec![entry::TEXT | text, entry::LINE_BREAK, entry::TEXT | text::SELECT_COUNT, entry::NUMBER | max],
            (None, true) => vec![entry::TEXT | text::SELECT_COUNT, entry::NUMBER | max],
            (Some(text), false) => vec![entry::TEXT | text],
            (None, false) => vec![],
        };
        if !entries.is_empty() {
            out.extend(message(&entries));
        }
        out.push(command(cmd::CARD_LIST, [player, kind, !entries.is_empty() as u32, 0]));
        out
    }

    /// The opponent's banished entries a mixed list overwrote (`n` of them), as they are: sent before anything else
    /// once it closes.
    pub(super) fn unstage_n(&self, n: Option<usize>) -> Vec<Message> {
        let Some(n) = n else { return vec![] };
        let pile = self.sides[CPU as usize].zones.get(&zone::REMOVED).map_or(&[][..], Vec::as_slice);
        pile.iter()
            .enumerate()
            .take(n)
            .map(|(i, c)| {
                let row = self.shown_row(&Place { player: CPU, zone: zone::REMOVED, index: i, face_up: true, defense: false }, c.row);
                Cmd::Card { uid: c.uid, open: true, row, place: table_place(CPU, zone::REMOVED, i) }.message()
            })
            .collect()
    }

    /// A card's row as a list shows it: none for the opponent's hidden cards when hiding.
    pub(super) fn list_row(&self, loc: &Loc) -> u32 {
        let row = self.row_of(loc);
        self.engine_place(loc).map_or(row as u32, |place| self.shown_row(&place, row))
    }

    /// A card's CARD_Prop row: from the mirror at its place (any deck card with its id for the deck), else the
    /// first row with its id.
    pub(super) fn row_of(&self, loc: &Loc) -> u16 {
        let cid = loc.cid.unwrap_or(0) as u16;
        // A token has no Konami id: the one at its place.
        let has_cid = |c: &&Card| self.cards.get(&c.row).map(|s| s.cid) == Some(cid) || (cid == 0 && self.tokens.contains(&c.uid));
        let at_place = self.engine_place(loc).and_then(|p| {
            let zone = self.sides[p.player as usize].zones.get(&p.zone)?;
            if p.zone == zone::DECK { zone.iter().find(has_cid) } else { zone.get(p.index).filter(has_cid) }.map(|c| c.row)
        });
        at_place.or_else(|| self.cards.iter().find(|(_, s)| s.cid == cid).map(|(&row, _)| row)).unwrap_or(0)
    }

    /// A pick in the card list (duelRequestName 7 {index: the list position}), kept as the engine's index.
    pub fn pick(&mut self, index: u32) {
        // A list holds a few dozen entries: more picks than that are a client making the server keep them.
        if self.picks.len() < MAX_PICKS {
            let index = index as usize;
            self.picks.push(self.list_order.get(index).copied().unwrap_or(index));
        }
    }

    /// Where a card list puts a card: as the field lies from the player's seat, top to bottom (the opponent's Spell &
    /// Trap row, their monsters, the player's monsters, the player's Spell & Trap row), each row left to right (the
    /// opponent's zones are mirrored: their zone 0 is on the player's right), the Field Spells at their rows' ends;
    /// then the cards off the field (hand, piles) in the engine's order. The engine's order (the player's cards first,
    /// each side by zone number) didn't match the field, so two Set cards in a list couldn't be told apart.
    pub(super) fn field_order(&self, loc: &Loc, engine: usize) -> (u32, usize) {
        let Some(place) = self.engine_place(loc).filter(|p| p.zone <= zone::FIELD_SPELL) else { return (4, engine) };
        let theirs = place.player == CPU;
        let (row, x) = match place.zone {
            zone::FIELD_SPELL => (if theirs { 0 } else { 3 }, 5),
            z if z >= 5 => (if theirs { 0 } else { 3 }, if theirs { 4 - (z - 5) } else { z - 5 }),
            z => (if theirs { 1 } else { 2 }, if theirs { 4 - z } else { z }),
        };
        (row, x as usize)
    }

    /// A dialog closed (duelRequestName 4 {DLGResult}): the engine response for the pending question (the button: 1
    /// yes, 0 attack / 1 defense), option (its index) or card choice (`count` picks), or for a chain chance's Yes with
    /// several cards, the card list. No pick cancels when the engine allows it; picks that don't fit the choice get
    /// the default.
    pub fn dialog_result(&mut self, count: u32) -> Option<Prompt> {
        let prompt = self.dialog_result_for(count);
        prompt.map(|p| self.with_ids(p))
    }

    /// An option answered: a call of heads or tails is kept for the coin toss that follows (Time Wizard).
    pub(super) fn chosen_option(&mut self, o: &SelectOption, index: usize) -> Json {
        self.coin_call = match o.options.get(index) { Some(&CALL_HEADS) => Some(true), Some(&CALL_TAILS) => Some(false), _ => self.coin_call };
        Response::Option(index).json()
    }

    pub(super) fn dialog_result_for(&mut self, count: u32) -> Option<Prompt> {
        match std::mem::take(&mut self.asking) {
            Asking::Choice { kind, action, candidates } => {
                // Which card of the pile: the pick answers the engine; none goes back to the prompt.
                let pick = (count == 1).then(|| self.picks.first().and_then(|&i| candidates.get(i).copied())).flatten();
                self.picks.clear();
                if let Some(index) = pick {
                    self.pending = None;
                    return Some(Prompt::Answer(Response::command(kind, action, Some(index)).json()));
                }
                let m = self.pending.clone()?;
                return Some(self.prompt(&m));
            }
            Asking::Steps(steps) => return self.step_result(steps, count),
            Asking::Surrender => return self.surrender_answer(count == 1),
            Asking::SurrenderEarly => return self.surrender_answer(false),
            Asking::PileView => {
                // A pile's view closed: the prompt it was opened from, again.
                self.picks.clear();
                let m = self.pending.clone()?;
                return Some(self.prompt(&m));
            }
            Asking::DirectAttack(response) => {
                // Yes attacks; No goes back to the battle commands.
                let m = self.pending.take()?;
                return Some(if count == 1 { Prompt::Answer(response) } else { self.prompt(&m) });
            }
            other => self.asking = other,
        }
        let ask = Ask::lenient(self.pending.as_ref()?).ok()??;
        if matches!(ask, Ask::Disfield(_) | Ask::Place(_)) && !matches!(self.asking, Asking::Zones(_)) {
            self.asking = Asking::Zones(vec![]);
            let m = self.pending.clone()?;
            return Some(Prompt::Client(self.zone_map(&m)));
        }
        // Prompts answered in a dialog or the card list; the others (a command map's, the zones') aren't.
        if matches!(ask, Ask::IdleCmd(_) | Ask::BattleCmd(_) | Ask::Place(_) | Ask::Disfield(_) | Ask::Counter(_) | Ask::RockPaperScissors(_)) {
            return None;
        }
        let m = self.pending.take()?;
        let answer = match &ask {
            Ask::EffectYn(_) => Response::EffectYn(count == 1).json(),
            Ask::YesNo(_) => Response::YesNo(count == 1).json(),
            Ask::Position(p) => {
                let allowed = p.positions;
                let position = match count {
                    0 if allowed & pos::FACEUP_ATTACK != 0 => pos::FACEUP_ATTACK,
                    0 => pos::FACEDOWN_ATTACK,
                    _ if allowed & pos::FACEUP_DEFENSE != 0 => pos::FACEUP_DEFENSE,
                    _ => pos::FACEDOWN_DEFENSE,
                };
                match allowed & position {
                    0 => cpu_answer(&m),
                    _ => Response::Position(position).json(),
                }
            }
            Ask::Option(o) if std::mem::take(&mut self.option_cards) => match std::mem::take(&mut self.picks)[..] {
                [index] if index < o.options.len() => self.chosen_option(o, index),
                _ => self.chosen_option(o, 0),
            },
            Ask::Option(o) if (count as usize) < o.options.len() => self.chosen_option(o, count as usize),
            Ask::Option(o) => self.chosen_option(o, 0),
            Ask::Chain(c) if !self.chain_list && count == 1 && c.selects.len() > 1 => {
                self.chain_list = true;
                self.picks.clear();
                let out = self.card_list(&CardList::chain(&m), Some(text::SELECT_EFFECT));
                self.pending = Some(m);
                return Some(Prompt::Client(out));
            }
            Ask::Chain(_) if !self.chain_list => Response::Chain((count == 1).then_some(0)).json(),
            Ask::Chain(c) => match std::mem::take(&mut self.picks)[..] {
                [index] if index < c.selects.len() => Response::Chain(Some(index)).json(),
                _ if c.forced => cpu_answer(&m),
                _ => Response::Chain(None).json(),
            },
            // A mixed list needs no picks to close: a forced choice with too few picked is asked again (a Tribute's
            // picks count by their monsters' release values, so there only none).
            Ask::Card(c) | Ask::Tribute(c)
                if self.mixed && !c.can_cancel && c.min >= 1
                    && (self.picks.is_empty() || (matches!(ask, Ask::Card(_)) && self.picks.len() < c.min as usize)) =>
            {
                return Some(self.prompt(&m));
            }
            Ask::Card(c) | Ask::Tribute(c) => self.card_choice(&m, matches!(ask, Ask::Tribute(_)), c, count),
            Ask::Sum(sum) => {
                let picks = std::mem::take(&mut self.picks);
                if !sum_fits(sum, &picks) {
                    return Some(self.prompt(&m));
                }
                self.sum_text = None;
                Response::Sum(picks).json()
            }
            Ask::UnselectCard(u) => match std::mem::take(&mut self.picks)[..] {
                [index] if index < u.select_cards.len() => Response::UnselectCard(Some(index)).json(),
                _ if u.can_finish || u.can_cancel => Response::UnselectCard(None).json(),
                _ => cpu_answer(&m),
            },
            Ask::Race(a) => match TYPES.get((count as usize).wrapping_sub(1)) {
                Some(&race) if a.available & race != 0 => Response::Race(vec![race]).json(),
                _ => cpu_answer(&m),
            },
            Ask::Attrib(a) => {
                let picked: Vec<u64> = ATTRIBUTES.iter().enumerate().filter(|(i, _)| count >> (i + 1) & 1 != 0).map(|(_, &a)| a).collect();
                match picked.len() as u64 == a.count && picked.iter().all(|p| a.available & p != 0) {
                    true => Response::Attrib(picked).json(),
                    false => cpu_answer(&m),
                }
            }
            Ask::Number(n) => {
                let options = &n.options;
                let unit = number_unit(options).map_or(1, |(unit, _)| unit);
                let value = count as u64 * unit;
                // The smallest option at least as large (the dialog's steps may pass between uneven options), else the
                // largest. By value: the engine may list them largest first (Card Trooper's 3, 2, 1).
                let index = options.iter().enumerate().filter(|(_, o)| **o >= value).min_by_key(|(_, o)| **o)
                    .or_else(|| options.iter().enumerate().max_by_key(|(_, o)| **o))
                    .map_or(0, |(i, _)| i);
                Response::Number(index as u64).json()
            }
            Ask::CardName(_) => Response::CardName(count as u64).json(),
            // The clicks in order: card picks[k] gets rank k (the core reads byte i as card i's rank). Anything but each
            // card once keeps the engine's own order.
            Ask::SortChain(sort) | Ask::SortCard(sort) => {
                let picks = std::mem::take(&mut self.picks);
                let n = sort.cards.len();
                let mut order = vec![usize::MAX; n];
                let whole = picks.len() == n && picks.iter().enumerate().all(|(rank, &i)| {
                    i < n && order[i] == usize::MAX && { order[i] = rank; true }
                });
                if whole { Response::Sort(Some(json!(order))).json() } else { cpu_answer(&m) }
            }
            _ => cpu_answer(&m),
        };
        Some(Prompt::Answer(answer))
    }

    /// The response for a closed card list of MSG_SELECT_CARD or MSG_SELECT_TRIBUTE.
    pub(super) fn card_choice(&mut self, m: &Json, tribute: bool, c: &SelectCard, count: u32) -> Json {
        let response = if tribute { resp::TRIBUTE } else { resp::CARD };
        let picks = std::mem::take(&mut self.picks);
        let len = c.selects.len();
        let (min, max) = (c.min as usize, c.max as usize);
        if count == 0 && picks.is_empty() && c.can_cancel {
            return Response::cards(response, None).json();
        }
        let mut seen = vec![];
        let fits = picks.iter().all(|&i| i < len && !seen.contains(&i) && { seen.push(i); true });
        // A Tribute's picks count by their `release_param`s, not one each.
        let enough = if tribute { tribute_enough(m, c, &picks) } else { (min..=max).contains(&picks.len()) };
        if fits && enough {
            return Response::cards(response, Some(picks)).json();
        }
        tracing::warn!("card list: picks {picks:?} don't fit min {min} max {max} of {len}");
        cpu_answer(m)
    }

    /// The command map of a prompt: each listed card's place and menu bits.
    pub(super) fn command_map(&self, ask: &Ask) -> Vec<(u32, u32)> {
        let mut bits: Vec<(u32, u32)> = vec![];
        for &(list, bit, _) in command_lists(ask).0 {
            for (loc, _) in command_list(ask, list) {
                let Some(place) = self.command_place(loc) else { continue };
                if list == "pos_changes" && bit != self.position_change(place) {
                    continue;
                }
                // The client looks a pile's commands up at index 0 only (0x50B510, 0x5204E0).
                let place = if place >> 8 & 0xFF >= zone::EXTRA { place & 0xFFFF } else { place };
                match bits.iter_mut().find(|(p, _)| *p == place) {
                    Some((_, b)) => *b |= 1 << bit,
                    None => bits.push((place, 1 << bit)),
                }
            }
        }
        bits
    }

    /// The menu entry a monster's position change shows (`table_place`): Flip Summon face down, to Attack Position in
    /// defense, to Defence Position in attack (live: an attack position monster offered both positions).
    pub(super) fn position_change(&self, place: u32) -> u32 {
        match self.positions.get(&(place & 1, place >> 16 & 0xFF)) {
            Some((false, _)) => 5,
            Some((true, true)) => 8,
            _ => 9,
        }
    }

    /// Where a listed card's command goes in the command map: its [`place`](Self::place), and for a card in the Deck (a
    /// Special Summon from the Deck: Red-Eyes Black Metal Dragon, Familiar-Possessed - Wynn) the Deck pile, index 0, as for
    /// the other piles: the client has no place for a card in the Deck, and the pile is clickable (Surrender's menu is on it).
    pub(super) fn command_place(&self, loc: &Loc) -> Option<u32> {
        if loc.location == loc::DECK {
            return Some(table_place(self.side(loc.controller), zone::DECK, 0));
        }
        self.place(loc)
    }

    /// The card-table place of an engine card location: hand (0xB, index), monster zone (0, slot), Spell/Trap
    /// zone (5, slot), and the extra deck, Graveyard and banished cards (0xC, 0xE, 0xF) at the index of a card with
    /// the location's `cid` (the engine's extra deck order isn't ours).
    pub(super) fn place(&self, loc: &Loc) -> Option<u32> {
        let p = self.side(loc.controller);
        let seq = loc.sequence as usize;
        // The Field Zone is the engine's Spell & Trap sequence 5 but the client's own slot, zone 10 at index 0: keyed
        // as Spell & Trap zone 5, a Set Field Spell's activation was offered where the client never looks (live: no
        // menu on it, 2026-11-26).
        if loc.location == loc::SZONE && seq == 5 {
            return Some(table_place(p, zone::FIELD_SPELL, 0));
        }
        let zone = match loc.location {
            loc::HAND => zone::HAND,
            loc::MZONE => 0,
            loc::SZONE => 5,
            loc::EXTRA => zone::EXTRA,
            loc::GRAVE => zone::GRAVE,
            loc::REMOVED => zone::REMOVED,
            _ => return None,
        };
        if zone >= zone::EXTRA {
            let cid = |c: &Card| self.cards.get(&c.row).map(|s| s.cid as u64);
            let cards = self.sides[p as usize].zones.get(&zone)?;
            let index = match cards.get(seq) {
                Some(c) if cid(c) == loc.cid => seq,
                _ => cards.iter().position(|c| cid(c) == loc.cid)?,
            };
            return Some(table_place(p, zone, index));
        }
        Some(table_place(p, zone, seq))
    }

    /// The zones a MSG_SELECT_DISFIELD may still take, as click targets: the command map keyed as a card on each zone
    /// would be (an empty zone's click sends the entry's command with the zone, 0x50B510 → 0x4F9510), then the input
    /// wait.
    pub(super) fn zone_map(&self, m: &Json) -> Vec<Message> {
        let picked = match &self.asking { Asking::Zones(picks) => picks.clone(), _ => vec![] };
        let mut map = vec![];
        for (player, location, sequence) in free_zones(&typed::<SelectPlace>(m)) {
            if picked.contains(&json!({ "player": player, "location": location, "sequence": sequence })) {
                continue;
            }
            let side = self.player(&json!(player));
            // Keyed like a card on that zone (`place`): monster zones as zone 0, Spell/Trap zones as zone 5, with the
            // zone's index; the click comes back as the zone itself (position, index 0).
            let (zone, bit) = if location == loc::MZONE { (0, ZONE_PICK_MONSTER) } else { (5, ZONE_PICK_SPELL) };
            map.push((table_place(side, zone, sequence as usize), 1 << bit));
        }
        vec![command_data(&map, &[]), command(cmd::INPUT, [mode::MAIN, CLIENT, 0, 0])]
    }

    /// A zone clicked for MSG_SELECT_DISFIELD: a free one not picked yet counts; with `count` of them the answer, else
    /// the zones again.
    pub(super) fn zone_click(&mut self, m: &Json, cmd: CardCommand) -> Prompt {
        let player = (0..2u64).find(|&p| self.player(&json!(p)) == cmd.player).unwrap_or(0);
        let (location, sequence) = match cmd.zone {
            z @ 0..=4 => (loc::MZONE, z as u64),
            z @ 5..=9 => (loc::SZONE, z as u64 - 5),
            _ => (0, 0),
        };
        let place = json!({ "player": player, "location": location, "sequence": sequence });
        let mut picks = match std::mem::take(&mut self.asking) { Asking::Zones(picks) => picks, _ => vec![] };
        if cmd.index == 0 && free_zones(&typed::<SelectPlace>(m)).contains(&(player, location, sequence)) && !picks.contains(&place) {
            picks.push(place);
        }
        let count = (typed::<SelectPlace>(m).count as usize).max(1);
        if picks.len() >= count {
            self.pending = None;
            let answer = if matches!(Ask::lenient(m), Ok(Some(Ask::Place(_)))) { Response::Place(picks) } else { Response::Disfield(picks) };
            return Prompt::Answer(answer.json());
        }
        self.asking = Asking::Zones(picks);
        Prompt::Client(self.zone_map(m))
    }
}

/// What an option of a SelectOption is called: the client's own label, or the engine's text for it.
enum OptionWords<'a> {
    Own(u32),
    Text(&'a str),
}

fn option_words(o: &SelectOption, i: usize) -> Option<OptionWords<'_>> {
    let d = *o.options.get(i)?;
    let own = OPTION_LABELS.iter().find(|(o, _, _)| *o == d).map(|&(_, label, _)| label)
        .or_else(|| CARD_OPTION_LABELS.iter().find(|(o, _)| *o == d).map(|&(_, label)| label));
    own.map(OptionWords::Own).or_else(|| o.option_texts.get(i).and_then(|t| t.as_deref()).filter(|t| !t.is_empty()).map(OptionWords::Text))
}

/// The Select dialog (kind 4) with these labels under `title`.
fn select_dialog(labels: &[u32], title: u32) -> Vec<Message> {
    let mut out = vec![command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0])];
    out.extend(fours(cmd::LIST_VALUES, labels));
    out.extend(message(&[entry::TEXT | title]));
    out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
    out
}
