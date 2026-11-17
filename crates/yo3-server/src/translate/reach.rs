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
        let (lists, kind, phases): (Lists, u64, &[(&str, u64)]) = match m["type"].as_u64() {
            Some(msg::SELECT_IDLECMD) => (&IDLE_LISTS, resp::IDLECMD, &[("to_bp", idle::TO_BP), ("to_ep", idle::TO_EP)]),
            Some(msg::SELECT_BATTLECMD) => (&BATTLE_LISTS, resp::BATTLECMD, &[("to_m2", battle::TO_M2), ("to_ep", battle::TO_EP)]),
            Some(msg::SELECT_DISFIELD) if self.player(&m["player"]) == CLIENT => return self.unreachable_zones(m),
            Some(msg::SELECT_CARD) if self.asking == Asking::FieldTarget => return self.unreachable_targets(m),
            _ => return self.unreachable_choices(m),
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
        for (place, bits) in self.command_map(m, lists) {
            let (player, zone, index) = (place & 0xFF, place >> 8 & 0xFF, place >> 16 & 0xFF);
            // As the client sends it: a field card from its own slot, index 0 (see `answer`).
            let (zone, index) = match (zone, index) {
                (0, i @ 1..=4) => (i, 0),
                (5, 5) => (zone::FIELD_SPELL, 0),
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
            for (i, loc) in m[list].as_array().into_iter().flatten().enumerate() {
                let what = format!("{list}[{i}] (card {} at location {} sequence {}, effect {})", loc["cid"], loc["location"], loc["sequence"], loc["description"]);
                offered.push((Response::command(kind, action, Some(i)).json(), what));
            }
        }
        for &(flag, action) in phases {
            if m[flag] == true {
                offered.push((Response::command(kind, action, None).json(), flag.to_string()));
            }
        }
        offered.into_iter().filter(|(answer, _)| !reachable.contains(answer)).map(|(_, what)| what).collect()
    }

    /// [`unreachable_answers`] for an attack target picked on the field: each target's Confirm, and the right-click.
    pub(super) fn unreachable_targets(&self, m: &Json) -> Vec<String> {
        let mut missing = vec![];
        for (i, card) in m["selects"].as_array().into_iter().flatten().enumerate() {
            let Some(place) = self.place(card) else { continue };
            let click = CardCommand { command: menu::CONFIRM, player: place & 0xFF, zone: place >> 16 & 0xFF, index: 0 };
            if self.clone().command(click) != Some(Prompt::Answer(Response::Card(Some(vec![i])).json())) {
                missing.push(format!("selects[{i}] (attack target on the field)"));
            }
        }
        if m["can_cancel"] == true && self.clone().finish().is_none() {
            missing.push("taking the attack back (right-click)".into());
        }
        missing
    }

    /// [`unreachable_answers`] for a zone choice: each free zone through the OK dialog and its click (and, for several,
    /// the first ones in order).
    pub(super) fn unreachable_zones(&self, m: &Json) -> Vec<String> {
        let zones = free_zones(m);
        let count = (m["count"].as_u64().unwrap_or(1) as usize).max(1);
        let click = |t: &mut Translator, (player, location, sequence): (u64, u64, u64)| {
            let zone = if location == loc::MZONE { sequence as u32 } else { 5 + sequence as u32 };
            t.command(CardCommand { command: ZONE_PICK_MONSTER, player: t.player(&json!(player)), zone, index: 0 })
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
    pub(super) fn unreachable_choices(&self, m: &Json) -> Vec<String> {
        let kind = m["type"].as_u64().unwrap_or(0);
        let len = |k: &str| m[k].as_array().map_or(0, Vec::len);
        // What, the clicks (each: the picks, then the dialog's result), the answer expected.
        type Case = (String, Vec<(Vec<u32>, u32)>, Json);
        let mut cases: Vec<Case> = vec![];
        match kind {
            msg::SELECT_CARD | msg::SELECT_TRIBUTE => {
                let response = if kind == msg::SELECT_TRIBUTE { resp::TRIBUTE } else { resp::CARD };
                let (min, max) = (m["min"].as_u64().unwrap_or(1), m["max"].as_u64().unwrap_or(1));
                if min <= 1 && max >= 1 {
                    for i in 0..len("selects") as u32 {
                        cases.push((format!("selects[{i}]"), vec![(vec![i], 1)], Response::cards(response, Some(vec![i as usize])).json()));
                    }
                } else {
                    let picks: Vec<u32> = (0..min as u32).collect();
                    cases.push((format!("the first {min} of selects"), vec![(picks.clone(), min as u32)], Response::cards(response, Some(picks.iter().map(|&p| p as usize).collect())).json()));
                }
                // Backing out (a right-click with nothing picked) when the engine allows it.
                if m["can_cancel"] == true {
                    cases.push(("cancelling".into(), vec![(vec![], 0)], Response::cards(response, None).json()));
                }
            }
            msg::SELECT_UNSELECT_CARD => {
                for i in 0..len("select_cards") as u32 {
                    cases.push((format!("select_cards[{i}]"), vec![(vec![i], 1)], Response::UnselectCard(Some(i as usize)).json()));
                }
                if m["can_cancel"] == true || m["can_finish"] == true {
                    let what = if m["can_cancel"] == true { "cancelling" } else { "finishing" };
                    cases.push((what.into(), vec![(vec![], 0)], Response::UnselectCard(None).json()));
                }
            }
            msg::SELECT_CHAIN => {
                let n = len("selects") as u32;
                for i in 0..n {
                    // Yes, then the card list when there are several (or a forced chain's list right away).
                    let clicks = match (m["forced"] == true, n) {
                        (true, _) => vec![(vec![i], 1)],
                        (false, 1) => vec![(vec![], 1)],
                        (false, _) => vec![(vec![], 1), (vec![i], 1)],
                    };
                    cases.push((format!("selects[{i}] (chain)"), clicks, Response::Chain(Some(i as usize)).json()));
                }
                if m["forced"] != true {
                    cases.push(("passing the chain".into(), vec![(vec![], 0)], Response::Chain(None).json()));
                }
            }
            msg::SELECT_SUM => {
                let picks: Vec<u32> = sum_choice(m).into_iter().map(|i| i as u32).collect();
                let n = picks.len() as u32;
                cases.push((format!("the sum {picks:?}"), vec![(picks.clone(), n)], Response::Sum(picks.iter().map(|&p| p as usize).collect()).json()));
            }
            msg::SELECT_EFFECTYN | msg::SELECT_YESNO => {
                let response = if kind == msg::SELECT_EFFECTYN { resp::EFFECTYN } else { resp::YESNO };
                cases.push(("yes".into(), vec![(vec![], 1)], Response::yes(response, true).json()));
                cases.push(("no".into(), vec![(vec![], 0)], Response::yes(response, false).json()));
            }
            msg::SELECT_OPTION => {
                for i in 0..len("options") as u32 {
                    let clicks = if self.option_cards { vec![(vec![i], 1)] } else { vec![(vec![], i)] };
                    cases.push((format!("options[{i}]"), clicks, Response::Option(i as usize).json()));
                }
            }
            // Each allowed position: the dialog's button 0 (attack) or 1 (defense).
            msg::SELECT_POSITION => {
                let allowed = m["positions"].as_u64().unwrap_or(0);
                for (bit, button) in [(pos::FACEUP_ATTACK, 0), (pos::FACEDOWN_ATTACK, 0), (pos::FACEUP_DEFENSE, 1), (pos::FACEDOWN_DEFENSE, 1)] {
                    if allowed & bit != 0 {
                        cases.push((format!("position {bit}"), vec![(vec![], button)], Response::Position(bit).json()));
                    }
                }
            }
            // Each allowed Type and Attribute, one declared (the client's number, or its bit for an attribute).
            msg::ANNOUNCE_RACE if m["count"].as_u64() == Some(1) => {
                let allowed = m["available"].as_u64().unwrap_or(0);
                for (i, &race) in TYPES.iter().enumerate().filter(|(_, r)| allowed & **r != 0) {
                    cases.push((format!("type {race:#x}"), vec![(vec![], i as u32 + 1)], Response::Race(vec![race]).json()));
                }
            }
            msg::ANNOUNCE_ATTRIB if m["count"].as_u64() == Some(1) => {
                let allowed = m["available"].as_u64().unwrap_or(0);
                for (i, &attribute) in ATTRIBUTES.iter().enumerate().filter(|(_, a)| allowed & **a != 0) {
                    cases.push((format!("attribute {attribute:#x}"), vec![(vec![], 1 << (i + 1))], Response::Attrib(vec![attribute]).json()));
                }
            }
            // Each number offered, as the dialog sends it (in its unit).
            msg::ANNOUNCE_NUMBER => {
                let options: Vec<u64> = m["options"].as_array().into_iter().flatten().filter_map(Json::as_u64).collect();
                let unit = number_unit(&options).map_or(1, |(unit, _)| unit);
                for (i, &o) in options.iter().enumerate() {
                    cases.push((format!("number {o}"), vec![(vec![], (o / unit) as u32)], Response::Number(i as u64).json()));
                }
            }
            // An order: the cards clicked last to first.
            msg::SORT_CHAIN | msg::SORT_CARD if len("cards") >= 2 => {
                let n = len("cards") as u32;
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
            .map(|(what, _, _)| format!("{what} of a {} prompt (card {})", kind, m["cid"]))
            .collect()
    }

}
