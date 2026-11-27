//! For the duel check: whether every answer the engine offers is one the client's windows can give.
use super::*;

impl Translator {
    /// The duel check's round trip for the pending idle or battle prompt: every answer the engine offers that the
    /// client can't give through its menus, described. The client's side is played on copies of the translator: each
    /// command of the command map as the client sends it (a field card from its own slot, `answer`), the card list a
    /// pile's command opens and each of its picks, the direct attack's Yes, and the phase buttons. Empty when every
    /// option is reachable (or nothing such is pending).
    pub fn unreachable_answers(&self) -> Vec<String> {
        let Some(m) = self.pending.as_ref() else { return vec![] };
        let Ok(Some(ask)) = Ask::lenient(m) else { return vec![] };
        let (lists, kind, phases): (Lists, u64, Vec<(bool, &str, u64)>) = match &ask {
            Ask::IdleCmd(p) => (&IDLE_LISTS, resp::IDLECMD, vec![(p.to_bp, "to_bp", idle::TO_BP), (p.to_ep, "to_ep", idle::TO_EP)]),
            Ask::BattleCmd(p) => (&BATTLE_LISTS, resp::BATTLECMD, vec![(p.to_m2, "to_m2", battle::TO_M2), (p.to_ep, "to_ep", battle::TO_EP)]),
            Ask::Disfield(p) if self.side(p.player) == CLIENT => return self.unreachable_zones(p),
            Ask::Card(c) if self.asking == Asking::FieldTarget => return self.unreachable_targets(c),
            Ask::Card(c) if matches!(self.asking, Asking::EffectTarget(_)) => return self.unreachable_effect_targets(c),
            _ => return self.unreachable_choices(&ask),
        };
        let mut reachable: Vec<Json> = vec![];
        let mut reach = |p: Option<Prompt>, t: &Translator| match p {
            Some(Prompt::Answer(r)) => reachable.push(r),
            Some(Prompt::Client(_)) if matches!(t.asking, Asking::Choice { .. }) => {
                let n = if let Asking::Choice { candidates, .. } = &t.asking { candidates.len() } else { 0 };
                for i in 0..n {
                    let mut pick = t.clone();
                    pick.pick(i as u32);
                    if let Some(Prompt::Answer(r)) = pick.dialog_result(1) {
                        reachable.push(r);
                    }
                }
            }
            Some(Prompt::Client(_)) if matches!(t.asking, Asking::DirectAttack(_)) => {
                if let Some(Prompt::Answer(r)) = t.clone().dialog_result(1) {
                    reachable.push(r);
                }
            }
            _ => {}
        };
        for (place, bits) in self.command_map(&ask) {
            let (player, zone, index) = (place & 0xFF, place >> 8 & 0xFF, place >> 16 & 0xFF);
            // As the client sends it: a field card from its own slot, index 0 (see `answer`).
            let (zone, index) = match (zone, index) {
                (0, i @ 1..=4) => (i, 0),
                (5, i @ 0..=4) => (5 + i, 0),
                other => other,
            };
            for bit in (0..32).filter(|b| bits >> b & 1 != 0) {
                let mut t = self.clone();
                let p = t.command(CardCommand { command: bit, player, zone, index });
                reach(p, &t);
            }
        }
        for button in [flag::BUTTON_BP, flag::BUTTON_M2, flag::BUTTON_EP] {
            if let Some(r) = self.clone().phase(button as u32) {
                reachable.push(r);
            }
        }
        let mut offered: Vec<(Json, String)> = vec![];
        let mut seen_lists = vec![];
        for &(list, _, action) in lists {
            if seen_lists.contains(&list) {
                continue;
            }
            seen_lists.push(list);
            for (i, (loc, description)) in command_list(&ask, list).into_iter().enumerate() {
                let what = format!("{list}[{i}] (card {:?} at location {} sequence {}, effect {description})", loc.cid, loc.location, loc.sequence);
                offered.push((Response::command(kind, action, Some(i)).json(), what));
            }
        }
        for (on, flag, action) in phases {
            if on {
                offered.push((Response::command(kind, action, None).json(), flag.to_string()));
            }
        }
        offered.into_iter().filter(|(answer, _)| !reachable.contains(answer)).map(|(_, what)| what).collect()
    }

    /// Every answer a stepped number question can end in: each label of its Select dialog, and of each next dialog in
    /// turn, down to `depth` steps.
    fn reached_by_dialogs(&self, depth: usize) -> Vec<Json> {
        let mut found = vec![];
        let labels = if let Asking::Steps(Steps::Number { groups, .. }) = &self.asking { groups.len() } else { 0 };
        for result in 0..labels as u32 {
            let mut t = self.clone();
            match t.dialog_result(result) {
                Some(Prompt::Answer(a)) => found.push(a),
                Some(Prompt::Client(_)) if depth > 0 => found.extend(t.reached_by_dialogs(depth - 1)),
                _ => {}
            }
        }
        found
    }

    /// [`unreachable_answers`] for an attack target picked on the field: each target's Confirm, and the right-click.
    pub(super) fn unreachable_targets(&self, choice: &SelectCard) -> Vec<String> {
        let mut missing = vec![];
        for (i, card) in choice.selects.iter().enumerate() {
            let Some(place) = self.place(card) else { continue };
            let click = CardCommand { command: menu::CONFIRM, player: place & 0xFF, zone: place >> 16 & 0xFF, index: 0 };
            if self.clone().command(click) != Some(Prompt::Answer(Response::Card(Some(vec![i])).json())) {
                missing.push(format!("selects[{i}] (attack target on the field)"));
            }
        }
        if choice.can_cancel && self.clone().finish().is_none() {
            missing.push("taking the attack back (right-click)".into());
        }
        missing
    }

    /// [`unreachable_answers`] for an effect's target on the field: each candidate's Confirm click, and the right-click's
    /// list.
    pub(super) fn unreachable_effect_targets(&self, choice: &SelectCard) -> Vec<String> {
        let mut missing = vec![];
        for (i, card) in choice.selects.iter().enumerate() {
            let Some(place) = self.place(card) else { continue };
            let (zone, slot) = (place >> 8 & 0xFF, place >> 16 & 0xFF);
            let zone = if zone == 0 || zone == 5 { zone + slot } else { zone };
            let click = CardCommand { command: menu::CONFIRM, player: place & 0xFF, zone, index: 0 };
            if self.clone().command(click) != Some(Prompt::Answer(Response::Card(Some(vec![i])).json())) {
                missing.push(format!("selects[{i}] (an effect's target on the field)"));
            }
        }
        if !matches!(self.clone().finish(), Some(Prompt::Client(_))) {
            missing.push("the card list (right-click)".into());
        }
        missing
    }

    /// [`unreachable_answers`] for a zone choice: each free zone through the OK dialog and its click (and, for several,
    /// the first ones in order).
    pub(super) fn unreachable_zones(&self, p: &SelectPlace) -> Vec<String> {
        let zones = free_zones(p);
        let count = (p.count as usize).max(1);
        let click = |t: &mut Translator, (player, location, sequence): (u64, u64, u64)| {
            let zone = if location == loc::MZONE { sequence as u32 } else { 5 + sequence as u32 };
            t.command(CardCommand { command: ZONE_PICK_MONSTER, player: t.side(player), zone, index: 0 })
        };
        let place = |(p, l, s): (u64, u64, u64)| json!({ "player": p, "location": l, "sequence": s });
        let mut missing = vec![];
        let picks: Vec<Vec<(u64, u64, u64)>> = if count == 1 { zones.iter().map(|&z| vec![z]).collect() } else { vec![zones.iter().copied().take(count).collect()] };
        for pick in picks {
            let mut t = self.clone();
            t.dialog_result(0);
            let mut last = None;
            for &z in &pick {
                last = click(&mut t, z);
            }
            let expected = Response::Disfield(pick.iter().map(|&z| place(z)).collect()).json();
            if last != Some(Prompt::Answer(expected)) {
                missing.push(format!("zones {pick:?}"));
            }
        }
        missing
    }

    /// [`unreachable_answers`] for the other windows: each card of a card choice (one pick, when one may be picked),
    /// a chain chance's Yes for each card and its No, a question's Yes and No, and each option.
    pub(super) fn unreachable_choices(&self, ask: &Ask) -> Vec<String> {
        // What, the clicks (each: the picks, then the dialog's result), the answer expected.
        type Case = (String, Vec<(Vec<u32>, u32)>, Json);
        let mut cases: Vec<Case> = vec![];
        match ask {
            Ask::Card(c) | Ask::Tribute(c) => {
                let response = if matches!(ask, Ask::Tribute(_)) { resp::TRIBUTE } else { resp::CARD };
                let view = match (ask, self.pending.as_ref()) {
                    (Ask::Tribute(_), Some(m)) => tribute_view(m, c),
                    _ => c.clone(),
                };
                let (min, max) = (view.min, view.max);
                if min <= 1 && max >= 1 {
                    for i in 0..c.selects.len() as u32 {
                        // A Tribute's card that counts for less than the Tribute needs isn't a whole answer by itself.
                        if let (Ask::Tribute(_), Some(m)) = (ask, self.pending.as_ref())
                            && !tribute_enough(m, c, &[i as usize])
                        {
                            continue;
                        }
                        cases.push((format!("selects[{i}]"), vec![(vec![i], 1)], Response::cards(response, Some(vec![i as usize])).json()));
                    }
                } else {
                    let picks: Vec<u32> = (0..min as u32).collect();
                    cases.push((format!("the first {min} of selects"), vec![(picks.clone(), min as u32)], Response::cards(response, Some(picks.iter().map(|&p| p as usize).collect())).json()));
                }
                // Backing out (a right-click with nothing picked) when the engine allows it.
                if c.can_cancel {
                    cases.push(("cancelling".into(), vec![(vec![], 0)], Response::cards(response, None).json()));
                }
            }
            Ask::UnselectCard(u) => {
                for i in 0..u.select_cards.len() as u32 {
                    cases.push((format!("select_cards[{i}]"), vec![(vec![i], 1)], Response::UnselectCard(Some(i as usize)).json()));
                }
                if u.can_cancel || u.can_finish {
                    let what = if u.can_cancel { "cancelling" } else { "finishing" };
                    cases.push((what.into(), vec![(vec![], 0)], Response::UnselectCard(None).json()));
                }
            }
            Ask::Chain(c) => {
                let n = c.selects.len() as u32;
                for i in 0..n {
                    // Yes, then the card list when there are several (or a forced chain's list right away).
                    let clicks = match (c.forced, n) {
                        (true, _) => vec![(vec![i], 1)],
                        (false, 1) => vec![(vec![], 1)],
                        (false, _) => vec![(vec![], 1), (vec![i], 1)],
                    };
                    cases.push((format!("selects[{i}] (chain)"), clicks, Response::Chain(Some(i as usize)).json()));
                }
                if !c.forced {
                    cases.push(("passing the chain".into(), vec![(vec![], 0)], Response::Chain(None).json()));
                }
            }
            Ask::Sum(sum) => {
                let picks: Vec<u32> = sum_choice(sum).into_iter().map(|i| i as u32).collect();
                let n = picks.len() as u32;
                cases.push((format!("the sum {picks:?}"), vec![(picks.clone(), n)], Response::Sum(picks.iter().map(|&p| p as usize).collect()).json()));
            }
            Ask::EffectYn(_) | Ask::YesNo(_) => {
                let response = if matches!(ask, Ask::EffectYn(_)) { resp::EFFECTYN } else { resp::YESNO };
                cases.push(("yes".into(), vec![(vec![], 1)], Response::yes(response, true).json()));
                cases.push(("no".into(), vec![(vec![], 0)], Response::yes(response, false).json()));
            }
            // A page at a time: "More choices" (after the page's options) until the option's page, then its label.
            Ask::Option(o) if matches!(self.asking, Asking::Steps(Steps::Options { .. })) => {
                let n = o.options.len();
                for i in 0..n {
                    let mut clicks: Vec<(Vec<u32>, u32)> =
                        (0..i / OPTION_PAGE).map(|p| (vec![], OPTION_PAGE.min(n - p * OPTION_PAGE) as u32)).collect();
                    clicks.push((vec![], (i % OPTION_PAGE) as u32));
                    cases.push((format!("options[{i}]"), clicks, Response::Option(i).json()));
                }
            }
            Ask::Option(o) => {
                for i in 0..o.options.len() as u32 {
                    let clicks = if self.option_cards { vec![(vec![i], 1)] } else { vec![(vec![], i)] };
                    cases.push((format!("options[{i}]"), clicks, Response::Option(i as usize).json()));
                }
            }
            // Each allowed position: the dialog's button 0 (attack) or 1 (defense), then, for two positions of a side, the
            // Select dialog's label (face-up first).
            Ask::Position(p) => {
                let (attacks, defenses) = position_sides(p.positions);
                let both = !attacks.is_empty() && !defenses.is_empty();
                for (button, side) in [(0, &attacks), (1, &defenses)] {
                    for (i, &bit) in side.iter().enumerate() {
                        let mut clicks = vec![];
                        if both || side.len() == 1 {
                            clicks.push((vec![], button));
                        }
                        if side.len() > 1 {
                            clicks.push((vec![], i as u32));
                        }
                        cases.push((format!("position {bit}"), clicks, Response::Position(bit).json()));
                    }
                }
            }
            // Each allowed Type and Attribute, one declared (the client's number, or its bit for an attribute); of several,
            // each type with each other, one dialog for each.
            Ask::Race(a) => {
                let numbers: Vec<(usize, u64)> = TYPES.iter().copied().enumerate().filter(|(_, r)| a.available & r != 0).collect();
                let count = a.count.max(1) as usize;
                let mut picks: Vec<Vec<(usize, u64)>> = vec![vec![]];
                for _ in 0..count {
                    let mut longer = vec![];
                    for p in &picks {
                        for &n in numbers.iter().filter(|n| p.last().is_none_or(|l| n.0 > l.0)) {
                            longer.push([p.clone(), vec![n]].concat());
                        }
                    }
                    picks = longer;
                    // Every pair is checked; of three or more, the first types of the list in order.
                    if count > 2 {
                        picks.truncate(1);
                    }
                }
                for pick in picks {
                    let clicks = pick.iter().map(|&(i, _)| (vec![], i as u32 + 1)).collect();
                    let races: Vec<u64> = pick.iter().map(|&(_, r)| r).collect();
                    cases.push((format!("types {races:#x?}"), clicks, Response::Race(races).json()));
                }
            }
            Ask::Attrib(a) if a.count == 1 => {
                for (i, &attribute) in ATTRIBUTES.iter().enumerate().filter(|(_, b)| a.available & **b != 0) {
                    cases.push((format!("attribute {attribute:#x}"), vec![(vec![], 1 << (i + 1))], Response::Attrib(vec![attribute]).json()));
                }
            }
            // Numbers the number dialog can't show: every option must be the end of some path of the Select dialogs.
            Ask::Number(n) if matches!(self.asking, Asking::Steps(Steps::Number { .. })) => {
                let reached = self.reached_by_dialogs(12);
                for i in 0..n.options.len() as u64 {
                    if !reached.contains(&Response::Number(i).json()) {
                        cases.push((format!("number {}", n.options[i as usize]), vec![], Response::Number(i).json()));
                    }
                }
                return cases.into_iter().map(|(what, _, _)| format!("{what} of a {ask:?} prompt")).collect();
            }
            // Each number offered, as the dialog sends it (in its unit).
            Ask::Number(n) => {
                let unit = number_unit(&n.options).map_or(1, |(unit, _)| unit);
                for (i, &o) in n.options.iter().enumerate() {
                    cases.push((format!("number {o}"), vec![(vec![], (o / unit) as u32)], Response::Number(i as u64).json()));
                }
            }
            // An order: the cards clicked last to first.
            Ask::SortChain(sort) | Ask::SortCard(sort) if sort.cards.len() >= 2 => {
                let n = sort.cards.len() as u32;
                let picks: Vec<u32> = (0..n).rev().collect();
                let order: Vec<u32> = (0..n).rev().collect();
                cases.push(("the reverse order".into(), vec![(picks, n)], Response::Sort(Some(json!(order))).json()));
            }
            _ => {}
        }
        cases
            .into_iter()
            .filter(|(_, clicks, expected)| {
                let mut t = self.clone();
                let mut last = None;
                for (picks, result) in clicks {
                    // A case names the engine's indices; the client clicks where the list shows each (`field_order`).
                    for &p in picks {
                        let at = t.list_order.iter().position(|&e| e == p as usize).map_or(p, |at| at as u32);
                        t.pick(at);
                    }
                    last = t.dialog_result(*result);
                }
                last != Some(Prompt::Answer(expected.clone()))
            })
            .map(|(what, _, _)| format!("{what} of a {ask:?} prompt"))
            .collect()
    }

}
