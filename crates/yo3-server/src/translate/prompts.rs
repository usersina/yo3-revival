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
        let kind = m["type"].as_u64().unwrap_or(0);
        if self.player(&m["player"]) == CPU {
            if kind == msg::ANNOUNCE_CARD {
                return Prompt::Answer(self.cpu_card_name(m));
            }
            return Prompt::Answer(cpu_answer(kind, m));
        }
        match kind {
            msg::SELECT_IDLECMD | msg::SELECT_BATTLECMD => {
                let (lists, buttons, mode): (Lists, [(&str, u8); 2], u32) = if kind == msg::SELECT_IDLECMD {
                    (&IDLE_LISTS, [("to_bp", flag::BUTTON_BP), ("to_ep", flag::BUTTON_EP)], mode::MAIN)
                } else {
                    (&BATTLE_LISTS, [("to_m2", flag::BUTTON_M2), ("to_ep", flag::BUTTON_EP)], mode::BATTLE)
                };
                self.attacker = None;
                let mut map = self.command_map(m, lists);
                self.add_pile_views(&mut map);
                let flags: Vec<(u8, u8)> = buttons.into_iter().filter(|(k, _)| m[*k] == true).map(|(_, b)| (b, 1)).collect();
                self.pending = Some(m.clone());
                Prompt::Client(vec![command_data(&map, &flags), command(cmd::INPUT, [mode, CLIENT, 0, 0])])
            }
            msg::SELECT_EFFECTYN | msg::SELECT_YESNO => {
                let battle = BATTLE_QUESTIONS.iter().find(|(d, _)| m["description"].as_u64() == Some(*d)).map(|(_, text)| *text);
                let entries = match (kind, m["cid"].as_u64(), m["description_cid"].as_u64(), battle) {
                    (msg::SELECT_EFFECTYN, cid, _, _) => vec![entry::TEXT | text::ACTIVATE, entry::CARD_NAME | cid.unwrap_or(0) as u32],
                    (_, _, _, Some(text)) => vec![entry::TEXT | text],
                    (_, _, Some(cid), _) => vec![entry::TEXT | text::USE, entry::CARD_NAME | cid as u32],
                    _ => vec![entry::TEXT | text::USE_NOW],
                };
                // A card's own question (its string as the description) under the client's line, through the DLL.
                let mut out = vec![];
                let mut entries = entries;
                if let Some(question) = m["description_text"].as_str().filter(|t| !t.is_empty() && entries.len() <= 2) {
                    let id = self.text(&mut out, question);
                    entries.extend([entry::LINE_BREAK, entry::TEXT | id]);
                }
                out.extend(message(&entries));
                out.push(Cmd::Dialog { kind: dialog::YES_NO, message: dialog::FROM_ENTRIES, param: 0 }.message());
                self.pending = Some(m.clone());
                Prompt::Client(out)
            }
            msg::SELECT_CARD | msg::SELECT_TRIBUTE => {
                let hint = if kind == msg::SELECT_TRIBUTE { hintmsg::RELEASE } else { std::mem::take(&mut self.hint) };
                if hint == hintmsg::ATTACKTARGET
                    && let Some(out) = self.target_on_field(m)
                {
                    return Prompt::Client(out);
                }
                let mut out = vec![];
                let text = self.hint_message(&mut out, hint);
                out.extend(self.card_list(m, text));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            // Materials, one at a time: the card list until the engine lets the player finish and nothing is left.
            msg::SELECT_UNSELECT_CARD if m["select_cards"].as_array().is_some_and(|s| !s.is_empty()) => {
                let open = m["can_finish"] == true || m["can_cancel"] == true;
                let list = json!({ "selects": m["select_cards"], "min": if open { 0 } else { 1 }, "max": 1 });
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
            msg::SELECT_SUM if m["selects"].as_array().is_some_and(|s| !s.is_empty()) && sum_fits(m, &sum_choice(m)) => {
                // Kind 1 (min 0): any number of picks, confirmed with a right-click (dlg_cardlist.swf has no OK
                // button). A list of kind 0x27 with a larger max closes only at max picks (live: Machina Fortress's
                // discards held the player at two picks of six).
                let n = m["selects"].as_array().map_or(0, Vec::len);
                let (_, max) = sum_bounds(m);
                let list = json!({ "selects": m["selects"], "min": 0, "max": max.min(n).max(1) });
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
            msg::SELECT_POSITION => match self.position_dialog(m) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            msg::SELECT_OPTION => match option_list(m).or_else(|| self.worded_options(m)) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                // More options than the Select dialog's four labels, or one with no words: the card list, the option's
                // card once per option in the engine's order, rather than the first option for the player.
                None if m["options"].as_array().is_some_and(|o| o.len() > 1) => {
                    let card = |i: usize| m["option_cids"][i].as_u64().filter(|&c| c != 0)
                        .or_else(|| self.chaining.as_ref().and_then(|c| c["cid"].as_u64()))
                        .unwrap_or(0);
                    let selects: Vec<Json> = (0..m["options"].as_array().map_or(0, Vec::len))
                        .map(|i| json!({ "controller": m["player"], "cid": card(i) }))
                        .collect();
                    let out = self.card_list(&json!({ "selects": selects, "min": 1, "max": 1 }), Some(text::SELECT_EFFECT));
                    // What each option does, from the engine's card strings: the chat window, numbered as listed.
                    for (i, text) in m["option_texts"].as_array().into_iter().flatten().enumerate() {
                        if let Some(text) = text.as_str().filter(|t| !t.is_empty()) {
                            self.notes.push(format!("{}: {text}", i + 1));
                        }
                    }
                    self.option_cards = true;
                    self.picks.clear();
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            msg::SELECT_CHAIN => self.chain_chance(m),
            // An order: chain links triggered together (21; the first pick becomes chain link 1) or cards put back on
            // the Deck (25, Big Eye). The card list orders the picks itself (dlg_cardlist.swf: a badge per click, a
            // second click takes it back; kind 6 needs all of them), and each card's rank answers the engine.
            msg::SORT_CHAIN | msg::SORT_CARD if m["cards"].as_array().is_some_and(|c| c.len() >= 2) => {
                let n = m["cards"].as_array().map_or(0, Vec::len);
                let text = if kind == msg::SORT_CARD { text::SORT } else { text::SELECT_EFFECT };
                let out = self.card_list(&json!({ "selects": m["cards"], "min": n, "max": n }), Some(text));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            msg::ANNOUNCE_RACE | msg::ANNOUNCE_ATTRIB | msg::ANNOUNCE_NUMBER | msg::ANNOUNCE_CARD => match declaration(m, std::mem::take(&mut self.hint) == hintmsg::LVRANK) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None if kind == msg::ANNOUNCE_CARD => Prompt::Answer(self.cpu_card_name(m)),
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            // A zone an effect takes (Ground Collapse's, Senet Switch's move): the client's own prompt in an OK dialog, then
            // the zones as click targets (`zone_map`).
            msg::SELECT_DISFIELD if !free_zones(m).is_empty() => self.zone_prompt(m),
            // Where a summoned, Set or activated card goes, with auto card placing off and a choice to make.
            msg::SELECT_PLACE if !self.auto_place && free_zones(m).len() > m["count"].as_u64().unwrap_or(1).max(1) as usize => {
                self.zone_prompt(m)
            }
            // Where a summoned, Set or activated card goes: chosen for the player, centre out, as Konami's server did.
            _ => Prompt::Answer(cpu_answer(kind, m)),
        }
    }

    /// A zone prompt (MSG_SELECT_DISFIELD, or MSG_SELECT_PLACE with auto card placing off): the client's own prompt in
    /// an OK dialog, then the zones as click targets (`zone_map`).
    pub(super) fn zone_prompt(&mut self, m: &Json) -> Prompt {
        let hint = std::mem::take(&mut self.hint);
        let zones = free_zones(m);
        let spells = zones.iter().all(|&(_, location, _)| location == loc::SZONE);
        let mut out = vec![];
        // The client's prompts are the zone-disabling effects' ("Select Monster Card Zones to render unusable."); a
        // placement is worded by the server through the connection DLL, in the client's terms.
        let text = if m["type"] == msg::SELECT_PLACE {
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
    pub(super) fn worded_options(&mut self, m: &Json) -> Option<Vec<Message>> {
        let options = m["options"].as_array().filter(|o| !o.is_empty() && o.len() <= SELECT_MAX)?;
        let texts: Vec<Option<&str>> = (0..options.len()).map(|i| m["option_texts"][i].as_str().filter(|t| !t.is_empty())).collect();
        let own = |d: u64| OPTION_LABELS.iter().find(|(o, _, _)| *o == d).map(|&(_, label, _)| label)
            .or_else(|| CARD_OPTION_LABELS.iter().find(|(o, _)| *o == d).map(|&(_, label)| label));
        let codes: Vec<u64> = options.iter().map(|o| o.as_u64()).collect::<Option<_>>()?;
        if codes.iter().zip(&texts).any(|(&d, t)| own(d).is_none() && t.is_none()) {
            return None;
        }
        let mut out = vec![];
        let mut labels = vec![];
        for (&d, text) in codes.iter().zip(&texts) {
            let label = match own(d) {
                Some(label) => label,
                None => self.text(&mut out, &shortened(text.unwrap_or_default(), LABEL_CHARS)),
            };
            labels.push(label | dialog::LISTED);
        }
        let hint = std::mem::take(&mut self.hint);
        let title = self.hint_message(&mut out, hint).unwrap_or(text::SELECT_EFFECT);
        out.push(command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0]));
        out.extend(fours(cmd::LIST_VALUES, &labels));
        out.extend(message(&[entry::TEXT | title]));
        out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
        Some(out)
    }

    /// The CPU's card name declaration: the cards in the duel by turns, a new one each time it is asked.
    pub(super) fn cpu_card_name(&mut self, m: &Json) -> Json {
        // A card the effect's filter allows (the sidecar's `fits`, the duel's own cards first).
        if let Some(cid) = m["fits"].as_array().and_then(|f| f.first()).and_then(Json::as_u64) {
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
    pub(super) fn position_dialog(&mut self, m: &Json) -> Option<Vec<Message>> {
        let allowed = m["positions"].as_u64().unwrap_or(0);
        if allowed & (pos::FACEUP_ATTACK | pos::FACEDOWN_ATTACK) == 0 {
            return None;
        }
        let cid = m["cid"].as_u64().unwrap_or(0) as u16;
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
        let (uid, name) = match (held, self.token_card_rows(m)) {
            (Some(uid), _) => (uid, cid),
            (None, Some(rows)) => {
                // The same one when the dialog opens again (after a look at a pile).
                let (uid, row) = match self.reserved_token {
                    Some((uid, row)) if rows.contains(&row) => (uid, row),
                    _ => self.next_token_card(&rows),
                };
                self.reserved_token = Some((uid, row));
                let player = self.player(&m["player"]);
                let side = &self.sides[player as usize];
                let zone = (0..5).find(|z| side.zones.get(z).is_none_or(Vec::is_empty)).unwrap_or(0);
                out.push(Cmd::Card { uid, open: false, row: row as u32, place: table_place(player, zone, 0) }.message());
                // A token without a Konami id is named by its client card's.
                (uid, self.cards.get(&row).map_or(cid, |s| s.cid))
            }
            (None, None) => (0, cid),
        };
        let face_up = if allowed & pos::FACEUP_DEFENSE != 0 { dialog::FACE_UP_DEFENSE } else { 0 };
        out.extend(message(&[entry::TEXT | text::POSITION, entry::CARD_NAME | name as u32]));
        out.push(Cmd::Dialog { kind: dialog::POSITION, message: dialog::FROM_ENTRIES, param: uid | face_up }.message());
        Some(out)
    }

    /// MSG_SELECT_CHAIN {spe_count, forced, hint_timing, selects}: the question shows the cards, names the chain's last
    /// link, and says how much it matters, so the player's Activation Message option decides whether it shows. A forced
    /// chain of several goes straight to the card list, of one needs no choice.
    pub(super) fn chain_chance(&mut self, m: &Json) -> Prompt {
        let selects = m["selects"].as_array().map_or(&[][..], Vec::as_slice);
        let forced = m["forced"] == true;
        let phase_window = self.phase_window;
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
            return Prompt::Client(self.card_list(m, Some(text::SELECT_EFFECT)));
        }
        let rows: Vec<u32> = selects.iter().map(|loc| self.row_of(loc) as u32).collect();
        let mut out = vec![command(cmd::LIST_BEGIN, [rows.len() as u32, 0, dialog::WITH_CARDS, 0])];
        out.extend(fours(cmd::LIST_ROWS, &rows));
        let entries = match &self.chaining {
            Some(link) => {
                let spell_trap = link["location"].as_u64() == Some(loc::SZONE);
                let text = if spell_trap { text::CHAIN_CARD } else { text::CHAIN_EFFECT };
                vec![entry::TEXT | text, entry::CARD_NAME | link["cid"].as_u64().unwrap_or(0) as u32]
            }
            None => vec![entry::TEXT | text::ACTIVATE_CARD],
        };
        out.extend(message(&entries));
        let timing = m["hint_timing"].as_u64().unwrap_or(0) | m["hint_timing_other"].as_u64().unwrap_or(0);
        let level = if m["spe_count"].as_u64().unwrap_or(0) == 0 {
            dialog::LEVEL_UNFIT
        } else if self.chaining.is_some() || timing & !QUIET_TIMINGS != 0 || phase_window {
            // A phase's own window counts its optional triggers in spe_count (Stardust Dragon's return in the End
            // Phase): at the phase level the default Activation Message answered No for the player, and the chance
            // was gone (#53). The core doesn't say which cards are triggers, so the whole window shows.
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
    /// (flag set for the cards shown), so face-up cards keep their faces and Set cards are backs; no pick is needed
    /// there, so the list closing with nothing picked asks again when the choice is forced.
    pub(super) fn card_list(&mut self, m: &Json, text: Option<u32>) -> Vec<Message> {
        let engine_order = m["selects"].as_array().map_or(&[][..], Vec::as_slice);
        let mut order: Vec<usize> = (0..engine_order.len()).collect();
        order.sort_by_key(|&i| self.field_order(&engine_order[i], i));
        let shown: Vec<Json> = order.iter().map(|&i| engine_order[i].clone()).collect();
        let selects = &shown[..];
        self.list_order = order;
        let (min, max) = (m["min"].as_u64().unwrap_or(1) as u32, m["max"].as_u64().unwrap_or(1) as u32);
        // A choice the player may back out of (can_cancel, or a chain list that isn't forced) opens as kind 1: the list
        // (no OK button) closes on a right-click with the picks so far, none cancelling (dlg_cardlist.swf `onMouseUp`
        // with `_cancelable`, which 0x4C73A0 always sets).
        let cancelable = m["can_cancel"] == true || (m["type"] == msg::SELECT_CHAIN && m["forced"] != true);
        let min = if cancelable { 0 } else { min };
        let (kind, max) = match min {
            0 => (list::PICK_ANY, max),
            1 => (list::PICK_ONE, max),
            _ => (list::PICK_ALL, min),
        };
        let rows: Vec<u32> = selects.iter().map(|loc| self.list_row(loc)).collect();
        let mixed = rows.contains(&0) && min <= 1;
        self.mixed = mixed;
        let (kind, player) = if mixed { (list::PICK_MIXED, CPU) } else { (kind, CLIENT) };
        let marks: Vec<u32> = selects.iter().map(|loc| if self.player(&loc["controller"]) == CPU { list::OPPONENTS } else { 0 }).collect();
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
        let several = m["min"].as_u64().unwrap_or(1) >= 2 && max >= 2 && kind != list::PICK_MIXED;
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
    pub(super) fn list_row(&self, loc: &Json) -> u32 {
        let row = self.row_of(loc);
        self.engine_place(loc).map_or(row as u32, |place| self.shown_row(&place, row))
    }

    /// A card's CARD_Prop row: from the mirror at its place (any deck card with its id for the deck), else the
    /// first row with its id.
    pub(super) fn row_of(&self, loc: &Json) -> u16 {
        let cid = loc["cid"].as_u64().unwrap_or(0) as u16;
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
    pub(super) fn field_order(&self, loc: &Json, engine: usize) -> (u32, usize) {
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
        let asked = self.pending.clone();
        let prompt = self.dialog_result_for(count);
        if let (Some(m), Some(Prompt::Answer(a))) = (&asked, &prompt)
            && m["type"] == msg::SELECT_OPTION
            && a["type"] == resp::OPTION
        {
            let called = a["index"].as_u64().and_then(|i| m["options"][i as usize].as_u64());
            self.coin_call = match called { Some(CALL_HEADS) => Some(true), Some(CALL_TAILS) => Some(false), _ => self.coin_call };
        }
        prompt.map(|p| self.with_ids(p))
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
        let kind = self.pending.as_ref()?["type"].as_u64()?;
        if matches!(kind, msg::SELECT_DISFIELD | msg::SELECT_PLACE) && !matches!(self.asking, Asking::Zones(_)) {
            self.asking = Asking::Zones(vec![]);
            let m = self.pending.clone()?;
            return Some(Prompt::Client(self.zone_map(&m)));
        }
        const DIALOGS: [u64; 15] = [
            msg::SORT_CHAIN,
            msg::SORT_CARD,
            msg::SELECT_SUM,
            msg::SELECT_CARD,
            msg::SELECT_UNSELECT_CARD,
            msg::SELECT_TRIBUTE,
            msg::SELECT_EFFECTYN,
            msg::SELECT_YESNO,
            msg::SELECT_POSITION,
            msg::SELECT_OPTION,
            msg::SELECT_CHAIN,
            msg::ANNOUNCE_RACE,
            msg::ANNOUNCE_ATTRIB,
            msg::ANNOUNCE_NUMBER,
            msg::ANNOUNCE_CARD,
        ];
        if !DIALOGS.contains(&kind) {
            return None;
        }
        let m = self.pending.take()?;
        let selects = m["selects"].as_array().or(m["select_cards"].as_array()).map_or(0, Vec::len);
        let answer = match kind {
            msg::SELECT_EFFECTYN => Response::EffectYn(count == 1).json(),
            msg::SELECT_YESNO => Response::YesNo(count == 1).json(),
            msg::SELECT_POSITION => {
                let allowed = m["positions"].as_u64().unwrap_or(0);
                let position = match count {
                    0 if allowed & pos::FACEUP_ATTACK != 0 => pos::FACEUP_ATTACK,
                    0 => pos::FACEDOWN_ATTACK,
                    _ if allowed & pos::FACEUP_DEFENSE != 0 => pos::FACEUP_DEFENSE,
                    _ => pos::FACEDOWN_DEFENSE,
                };
                match allowed & position {
                    0 => cpu_answer(kind, &m),
                    _ => Response::Position(position).json(),
                }
            }
            msg::SELECT_OPTION if std::mem::take(&mut self.option_cards) => match std::mem::take(&mut self.picks)[..] {
                [index] if index < m["options"].as_array().map_or(0, Vec::len) => Response::Option(index).json(),
                _ => cpu_answer(kind, &m),
            },
            msg::SELECT_OPTION if (count as usize) < m["options"].as_array().map_or(0, Vec::len) => {
                Response::Option(count as usize).json()
            }
            msg::SELECT_CHAIN if !self.chain_list && count == 1 && selects > 1 => {
                self.chain_list = true;
                self.picks.clear();
                let out = self.card_list(&m, Some(text::SELECT_EFFECT));
                self.pending = Some(m);
                return Some(Prompt::Client(out));
            }
            msg::SELECT_CHAIN if !self.chain_list => {
                Response::Chain((count == 1).then_some(0)).json()
            }
            msg::SELECT_CHAIN => match std::mem::take(&mut self.picks)[..] {
                [index] if index < selects => Response::Chain(Some(index)).json(),
                _ if m["forced"] == true => cpu_answer(kind, &m),
                _ => Response::Chain(None).json(),
            },
            // A mixed list needs no picks to close: a forced choice with nothing picked is asked again.
            msg::SELECT_CARD | msg::SELECT_TRIBUTE if self.mixed && self.picks.is_empty() && m["can_cancel"] != true && m["min"].as_u64().unwrap_or(1) >= 1 => {
                return Some(self.prompt(&m));
            }
            msg::SELECT_CARD | msg::SELECT_TRIBUTE => self.card_choice(&m, count),
            msg::SELECT_SUM => {
                let picks = std::mem::take(&mut self.picks);
                if !sum_fits(&m, &picks) {
                    return Some(self.prompt(&m));
                }
                self.sum_text = None;
                Response::Sum(picks).json()
            }
            msg::SELECT_UNSELECT_CARD => match std::mem::take(&mut self.picks)[..] {
                [index] if index < selects => Response::UnselectCard(Some(index)).json(),
                _ if m["can_finish"] == true || m["can_cancel"] == true => Response::UnselectCard(None).json(),
                _ => cpu_answer(kind, &m),
            },
            msg::ANNOUNCE_RACE => match TYPES.get((count as usize).wrapping_sub(1)) {
                Some(&race) if m["available"].as_u64().unwrap_or(0) & race != 0 => Response::Race(vec![race]).json(),
                _ => cpu_answer(kind, &m),
            },
            msg::ANNOUNCE_ATTRIB => {
                let picked: Vec<u64> = ATTRIBUTES.iter().enumerate().filter(|(i, _)| count >> (i + 1) & 1 != 0).map(|(_, &a)| a).collect();
                let allowed = m["available"].as_u64().unwrap_or(0);
                match picked.len() as u64 == m["count"].as_u64().unwrap_or(1) && picked.iter().all(|a| allowed & a != 0) {
                    true => Response::Attrib(picked).json(),
                    false => cpu_answer(kind, &m),
                }
            }
            msg::ANNOUNCE_NUMBER => {
                let options: Vec<u64> = m["options"].as_array().into_iter().flatten().filter_map(Json::as_u64).collect();
                let unit = number_unit(&options).map_or(1, |(unit, _)| unit);
                let value = count as u64 * unit;
                // The smallest option at least as large (the dialog's steps may pass between uneven options), else the
                // largest. By value: the engine may list them largest first (Card Trooper's 3, 2, 1).
                let index = options.iter().enumerate().filter(|(_, o)| **o >= value).min_by_key(|(_, o)| **o)
                    .or_else(|| options.iter().enumerate().max_by_key(|(_, o)| **o))
                    .map_or(0, |(i, _)| i);
                Response::Number(index as u64).json()
            }
            msg::ANNOUNCE_CARD => Response::CardName(count as u64).json(),
            // The clicks in order: card picks[k] gets rank k (the core reads byte i as card i's rank). Anything but each
            // card once keeps the engine's own order.
            msg::SORT_CHAIN | msg::SORT_CARD => {
                let picks = std::mem::take(&mut self.picks);
                let n = m["cards"].as_array().map_or(0, Vec::len);
                let mut order = vec![usize::MAX; n];
                let whole = picks.len() == n && picks.iter().enumerate().all(|(rank, &i)| {
                    i < n && order[i] == usize::MAX && { order[i] = rank; true }
                });
                if whole { Response::Sort(Some(json!(order))).json() } else { cpu_answer(kind, &m) }
            }
            _ => cpu_answer(kind, &m),
        };
        Some(Prompt::Answer(answer))
    }

    /// The response for a closed card list of MSG_SELECT_CARD or MSG_SELECT_TRIBUTE.
    pub(super) fn card_choice(&mut self, m: &Json, count: u32) -> Json {
        let kind = m["type"].as_u64().unwrap_or(0);
        let response = if kind == msg::SELECT_TRIBUTE { resp::TRIBUTE } else { resp::CARD };
        let picks = std::mem::take(&mut self.picks);
        let len = m["selects"].as_array().map_or(0, Vec::len);
        let (min, max) = (m["min"].as_u64().unwrap_or(1) as usize, m["max"].as_u64().unwrap_or(1) as usize);
        if count == 0 && picks.is_empty() && m["can_cancel"] == true {
            return Response::cards(response, None).json();
        }
        let mut seen = vec![];
        let fits = picks.iter().all(|&i| i < len && !seen.contains(&i) && { seen.push(i); true });
        if fits && (min..=max).contains(&picks.len()) {
            return Response::cards(response, Some(picks)).json();
        }
        tracing::warn!("card list: picks {picks:?} don't fit min {min} max {max} of {len}");
        cpu_answer(kind, m)
    }

    /// The command map of a prompt: each listed card's place and menu bits.
    pub(super) fn command_map(&self, m: &Json, lists: Lists) -> Vec<(u32, u32)> {
        let mut bits: Vec<(u32, u32)> = vec![];
        for &(list, bit, _) in lists {
            for loc in m[list].as_array().into_iter().flatten() {
                let Some(place) = self.place(loc) else { continue };
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

    /// The card-table place of an engine card location: hand (0xB, index), monster zone (0, slot), Spell/Trap
    /// zone (5, slot), and the extra deck, Graveyard and banished cards (0xC, 0xE, 0xF) at the index of a card with
    /// the location's `cid` (the engine's extra deck order isn't ours).
    pub(super) fn place(&self, loc: &Json) -> Option<u32> {
        let p = self.player(&loc["controller"]);
        let seq = loc["sequence"].as_u64()? as usize;
        let zone = match loc["location"].as_u64()? {
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
                Some(c) if cid(c) == loc["cid"].as_u64() => seq,
                _ => cards.iter().position(|c| cid(c) == loc["cid"].as_u64())?,
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
        for (player, location, sequence) in free_zones(m) {
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
        if cmd.index == 0 && free_zones(m).contains(&(player, location, sequence)) && !picks.contains(&place) {
            picks.push(place);
        }
        let count = (m["count"].as_u64().unwrap_or(1) as usize).max(1);
        if picks.len() >= count {
            self.pending = None;
            let answer = if m["type"] == msg::SELECT_PLACE { Response::Place(picks) } else { Response::Disfield(picks) };
            return Prompt::Answer(answer.json());
        }
        self.asking = Asking::Zones(picks);
        Prompt::Client(self.zone_map(m))
    }

}
