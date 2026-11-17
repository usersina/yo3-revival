//! The client's answers to a prompt: card commands, phase buttons, dialog results, picks, the field's right-click.
use super::*;

impl Translator {
    /// The engine response for a phase button (0 DP .. 5 EP), if the pending prompt allows that phase.
    pub fn phase(&mut self, button: u32) -> Option<Json> {
        let ask = Ask::lenient(self.pending.as_ref()?).ok()??;
        let (kind, action) = match (&ask, button as u8) {
            (Ask::IdleCmd(p), flag::BUTTON_BP) if p.to_bp => (resp::IDLECMD, idle::TO_BP),
            (Ask::IdleCmd(p), flag::BUTTON_EP) if p.to_ep => (resp::IDLECMD, idle::TO_EP),
            (Ask::BattleCmd(p), flag::BUTTON_M2) if p.to_m2 => (resp::BATTLECMD, battle::TO_M2),
            (Ask::BattleCmd(p), flag::BUTTON_EP) if p.to_ep => (resp::BATTLECMD, battle::TO_EP),
            _ => return None,
        };
        self.pending = None;
        Some(Response::command(kind, action, None).json())
    }

    /// What the client's card command leads to: an attack with no monster to target asks "Attack your opponent
    /// directly?" first (225, the original's dialog: EDOPro asks only when the attacker could also target a monster;
    /// live, an attack on an empty field just went through), anything else answers the engine.
    pub fn command(&mut self, cmd: CardCommand) -> Option<Prompt> {
        let prompt = self.command_for(cmd);
        prompt.map(|p| self.with_ids(p))
    }

    /// A client request that answers the pending prompt (a card command, a phase button, a dialog's result, a list's
    /// pick, the field's right-click): what follows, for the engine or the client. The duel's own requests (janken,
    /// options, Card Swap, the settle answer) are no prompt's.
    pub fn request(&mut self, request: ClientRequest) -> Option<Prompt> {
        match request {
            ClientRequest::Command(cmd) => self.command(cmd),
            ClientRequest::Phase(button) => self.phase(button).map(Prompt::Answer),
            ClientRequest::DialogResult(count) => self.dialog_result(count),
            ClientRequest::Pick(index) => {
                self.pick(index);
                None
            }
            ClientRequest::RightClick => self.finish(),
            ClientRequest::Hand(_) | ClientRequest::Order(_) | ClientRequest::Swapped | ClientRequest::Options { .. } | ClientRequest::Settled(_) => None,
        }
    }

    /// A client prompt's card table entries with their Konami ids (see [`with_card_ids`]).
    pub(super) fn with_ids(&self, p: Prompt) -> Prompt {
        match p {
            Prompt::Client(mut out) => {
                with_card_ids(&self.cards, &mut out);
                Prompt::Client(out)
            }
            answer => answer,
        }
    }

    pub(super) fn command_for(&mut self, cmd: CardCommand) -> Option<Prompt> {
        let m = self.pending.clone()?;
        let ask = Ask::lenient(&m).ok().flatten();
        if self.asking == Asking::FieldTarget {
            return self.target_click(&m, cmd);
        }
        if matches!(ask, Some(Ask::Disfield(_) | Ask::Place(_))) && matches!(self.asking, Asking::Zones(_)) {
            return Some(self.zone_click(&m, cmd));
        }
        if cmd.command == menu::CONFIRM
            && let Some(out) = self.pile_view(cmd.player, cmd.zone)
        {
            self.asking = Asking::PileView;
            return Some(Prompt::Client(out));
        }
        if cmd.zone >= zone::EXTRA && cmd.zone != zone::DECK {
            return self.pile_command(&m, cmd);
        }
        // A card that offers this command more than once (two effects it can activate now): the card list asks which,
        // the card once per effect in the engine's order.
        if let Some((list, kind, action, indices)) = self.listed(&cmd).filter(|l| l.3.len() > 1) {
            let cards = ask.as_ref().map_or(vec![], |a| command_list(a, list));
            let selects: Vec<Loc> = indices.iter().filter_map(|&i| cards.get(i)).map(|(loc, _)| (*loc).clone()).collect();
            let out = self.card_list(&CardList { selects, min: 0, max: 1, cancelable: false }, Some(text::SELECT_EFFECT));
            self.asking = Asking::Choice { kind, action, candidates: indices };
            self.picks.clear();
            return Some(Prompt::Client(out));
        }
        let response = self.answer(cmd)?;
        self.chose(&m, &response);
        let index = response["index"].as_u64().unwrap_or(u64::MAX) as usize;
        let attack = match &ask { Some(Ask::BattleCmd(p)) => p.attacks.get(index), _ => None };
        let opponent = 1 - attack.map_or(self.side(0), |a| self.side(a.loc.controller)) as usize;
        let no_target = (0..5).all(|zone| self.sides[opponent].zones.get(&zone).is_none_or(Vec::is_empty));
        if response["type"] != resp::BATTLECMD || response["action"] != battle::ATTACK || !attack.is_some_and(|a| a.can_direct) || !no_target {
            return Some(Prompt::Answer(response));
        }
        self.pending = Some(m);
        self.asking = Asking::DirectAttack(response);
        let mut out = message(&[entry::TEXT | text::DIRECT]);
        out.push(Cmd::Dialog { kind: dialog::YES_NO, message: dialog::FROM_ENTRIES, param: 0 }.message());
        Some(Prompt::Client(out))
    }

    /// The client's answer to a prompt, as the engine gets it: an attack names the attacker its target is aimed from.
    /// The duel check calls it with the self-played answers, which reach the engine without a card command.
    pub fn answered(&mut self, prompt: &Json, response: &Json) {
        if let Ok(Some(ask)) = Ask::lenient(prompt)
            && self.side(ask.player()) == CLIENT
        {
            self.chose(prompt, response);
        }
    }

    pub(super) fn chose(&mut self, m: &Json, response: &Json) {
        if let Ok(Some(Ask::BattleCmd(p))) = Ask::lenient(m)
            && response["type"] == resp::BATTLECMD
            && response["action"] == battle::ATTACK
        {
            let attack = p.attacks.get(response["index"].as_u64().unwrap_or(u64::MAX) as usize);
            self.attacker = attack.and_then(|a| self.engine_place(&a.loc)).filter(|p| p.zone < 5).map(|p| (p.player, p.zone));
        }
    }

    /// The target of the client's own attack, picked on the field as the original did (#51): the battle's start, the
    /// attack with no target yet, BattleSelect's aiming state, Confirm on each target and the battle input wait, with
    /// the field's right-click for taking the attack back. None (the card list instead) without an attacker or with a
    /// target that is not a monster on the field.
    pub(super) fn target_on_field(&mut self, m: &Json) -> Option<Vec<Message>> {
        let (player, zone) = self.attacker?;
        let mut map = vec![];
        let choice: SelectCard = typed(m);
        for card in &choice.selects {
            let place = self.place(card).filter(|p| p >> 8 & 0xFF == 0)?;
            map.push((place, 1 << menu::CONFIRM));
        }
        self.asking = Asking::FieldTarget;
        self.pending = Some(m.clone());
        Some(vec![
            Cmd::BattleInit.message(),
            Cmd::BattleAttack { player, zone, target: Target::NotYet { player: 1 - player } }.message(),
            Cmd::BattleSelect.message(),
            command_data(&map, &[(flag::RIGHT_CLICK, 1)]),
            command(cmd::INPUT, [mode::BATTLE, CLIENT, 0, 0]),
        ])
    }

    /// A click while an attack target is picked on the field: Confirm on a target answers, anything else waits on.
    pub(super) fn target_click(&mut self, m: &Json, cmd: CardCommand) -> Option<Prompt> {
        // A field card comes from its own slot, index 0 (see `listed`); targets are keyed at (zone 0, index slot).
        let slot = if cmd.index == 0 && cmd.zone < 5 { cmd.zone as usize } else { return None };
        let place = table_place(cmd.player, 0, slot);
        let choice: SelectCard = typed(m);
        let index = choice.selects.iter().position(|c| self.place(c) == Some(place)).filter(|_| cmd.command == menu::CONFIRM)?;
        self.asking = Asking::Prompt;
        self.pending = None;
        Some(Prompt::Answer(Response::Card(Some(vec![index])).json()))
    }

    /// The field's right-click (duelRequestName 5): while an attack target is picked on the field, the attack taken
    /// back when the engine allows it.
    pub fn finish(&mut self) -> Option<Prompt> {
        let m = self.pending.as_ref().filter(|_| self.asking == Asking::FieldTarget)?;
        if !typed::<SelectCard>(m).can_cancel {
            return None;
        }
        self.asking = Asking::Prompt;
        self.taken_back = true;
        self.pending = None;
        Some(Prompt::Answer(Response::Card(None).json()))
    }

    /// A command on a pile (sent at index 0, where the client found it): the one card of the pile it fits, or with
    /// several, the card list asks which (the original's "Select the monster(s) to Special Summon from your Extra
    /// Deck.", 26).
    pub(super) fn pile_command(&mut self, m: &Json, cmd: CardCommand) -> Option<Prompt> {
        let ask = Ask::lenient(m).ok()??;
        let (lists, kind) = command_lists(&ask);
        let &(list, _, action) = lists.iter().find(|(_, bit, _)| *bit == cmd.command)?;
        let pile = table_place(cmd.player, cmd.zone, 0);
        let candidates: Vec<(usize, &Loc)> = command_list(&ask, list)
            .into_iter()
            .map(|(loc, _)| loc)
            .enumerate()
            .filter(|(_, loc)| self.place(loc).is_some_and(|p| p & 0xFFFF == pile))
            .collect();
        match candidates[..] {
            [] => None,
            [(index, _)] => {
                self.pending = None;
                Some(Prompt::Answer(Response::command(kind, action, Some(index)).json()))
            }
            _ => {
                let text = match (cmd.command, cmd.zone) {
                    (2, zone::EXTRA) => text::SPSUMMON_EXTRA,
                    (2, _) => text::SPSUMMON,
                    _ => text::SELECT_EFFECT,
                };
                let selects: Vec<Loc> = candidates.iter().map(|(_, loc)| (*loc).clone()).collect();
                let out = self.card_list(&CardList { selects, min: 0, max: 1, cancelable: false }, Some(text));
                self.asking = Asking::Choice { kind, action, candidates: candidates.iter().map(|(i, _)| *i).collect() };
                self.picks.clear();
                Some(Prompt::Client(out))
            }
        }
    }

    /// Confirm (bit 11) at index 0 of each pile that can be looked through: both players' Graveyards and banished
    /// piles and the player's own extra deck, when not empty. The field highlights a pile of either player whose
    /// bits there have Activate Effect or Confirm (0x808, 0x50A690).
    pub(super) fn add_pile_views(&self, map: &mut Vec<(u32, u32)>) {
        for (player, zone) in [(CLIENT, zone::GRAVE), (CPU, zone::GRAVE), (CLIENT, zone::REMOVED), (CPU, zone::REMOVED), (CLIENT, zone::EXTRA)] {
            if self.sides[player as usize].zones.get(&zone).is_none_or(Vec::is_empty) {
                continue;
            }
            let place = table_place(player, zone, 0);
            match map.iter_mut().find(|(p, _)| *p == place) {
                Some((_, bits)) => *bits |= 1 << menu::CONFIRM,
                None => map.push((place, 1 << menu::CONFIRM)),
            }
        }
    }

    /// A pile's view: its cards from the bottom in a card list with no picks (max 0: dlg_cardlist.swf ignores card
    /// clicks, and a right-click closes it), under the pile's title.
    pub(super) fn pile_view(&self, player: u32, zone: u32) -> Option<Vec<Message>> {
        let title = PILE_TITLES.iter().find(|(z, _)| *z == zone)?.1 + (player != CLIENT) as u32;
        if zone == zone::EXTRA && player != CLIENT {
            return None;
        }
        let cards = self.sides.get(player as usize)?.zones.get(&zone).filter(|c| !c.is_empty())?;
        let rows: Vec<u32> = cards
            .iter()
            .enumerate()
            .map(|(index, c)| self.shown_row(&Place { player, zone, index, face_up: true, defense: false }, c.row))
            .collect();
        let mark = if player == CLIENT { 0 } else { list::OPPONENTS };
        let mut out = vec![command(cmd::LIST_BEGIN, [rows.len() as u32, 0, 0, 0])];
        out.extend(fours(cmd::LIST_ROWS, &rows));
        out.extend(fours(cmd::LIST_MARKS, &vec![mark; rows.len()]));
        out.extend(message(&[entry::TEXT | title]));
        out.push(command(cmd::CARD_LIST, [CLIENT, list::PICK_ANY, 1, 0]));
        Some(out)
    }

    /// The engine response for the client's card command, if it names a listed card (its first entry: `command`
    /// asks which when there are several).
    pub fn answer(&mut self, cmd: CardCommand) -> Option<Json> {
        let (_, kind, action, indices) = self.listed(&cmd)?;
        self.pending = None;
        Some(Response::command(kind, action, Some(indices[0])).json())
    }

    /// The pending prompt's entries the client's card command names: the list, the response's type and action, and
    /// the entries' indices in the list (several when the card offers the command more than once).
    pub(super) fn listed(&self, cmd: &CardCommand) -> Option<(&'static str, u64, u64, Vec<usize>)> {
        let ask = Ask::lenient(self.pending.as_ref()?).ok()??;
        let (lists, kind) = command_lists(&ask);
        // The client looks a card's commands up at (zone 0, index slot) for monsters and (zone 5, index slot) for
        // Spells/Traps (0x50A690), but sends the one picked with the card's own place, (zone slot, index 0) (live: an
        // attack from the second monster slot came as zone 1, index 0).
        let (zone, index) = match (cmd.zone, cmd.index) {
            (z @ 1..=4, 0) => (0, z as usize),
            (z @ 6..=9, 0) => (5, z as usize - 5),
            (zone::FIELD_SPELL, 0) => (5, 5),
            (z, i) => (z, i as usize),
        };
        let place = table_place(cmd.player, zone, index);
        for &(list, bit, action) in lists {
            if bit != cmd.command {
                continue;
            }
            let at: Vec<usize> = command_list(&ask, list).iter().enumerate().filter(|(_, (loc, _))| self.place(loc) == Some(place)).map(|(i, _)| i).collect();
            if !at.is_empty() {
                return Some((list, kind, action, at));
            }
        }
        None
    }
}
