//! The engine's messages as the client's commands: draws, moves, summons, chains, battles, counters, reveals.
use super::*;

impl Translator {
    /// The client commands for the engine's events (prompts excluded).
    pub fn events(&mut self, messages: &[Json]) -> Vec<Message> {
        let staged = self.staged.take();
        let mut out = self.unstage_n(staged);
        for (i, m) in messages.iter().enumerate() {
            let event = match Event::lenient(m) {
                Ok(Some(event)) => event,
                Ok(None) => {
                    self.unshown(m);
                    continue;
                }
                Err(e) => {
                    tracing::warn!("engine message doesn't fit its struct: {e}");
                    continue;
                }
            };
            match event {
                Event::Field(f) => out.extend(self.field(&f)),
                Event::Draw(d) => {
                    let p = self.side(d.player);
                    for drawn in &d.drawn {
                        out.extend(self.draw(p, drawn.cid.unwrap_or(0) as u16));
                    }
                }
                Event::NewTurn(t) => {
                    // The turn's end speaks for the player whose turn it was.
                    if self.turns > 0 {
                        out.push(command(cmd::SPEAK_TURN_END, [self.turn_player, 0, 0, 0]));
                    }
                    self.turn_player = self.side(t.player);
                    self.turns += 1;
                    // The client starts on turn 1; 6 would count it again.
                    if self.turns > 1 {
                        out.push(command(cmd::TURN, [self.turn_player, 0, 0, 0]));
                    }
                }
                Event::NewPhase(n) => {
                    self.summoning = None;
                    // Leaving a Battle Phase ends its battle mode even with no attack: the swords the client drew over
                    // the possible attackers (effect 0x3C) go only with BattleSelect, BattleAttack or BattleEnd.
                    let battle = (0x08..=0x80).contains(&n.phase);
                    let ended = self.end_battle();
                    if ended.is_empty() && self.battle_phase && !battle {
                        out.push(Cmd::BattleEnd.message());
                    }
                    out.extend(ended);
                    self.battle_phase = battle;
                    if let Some(phase) = client_phase(n.phase) {
                        out.push(command(cmd::PHASE, [self.turn_player, phase, 0, 0]));
                        // The turn player's draw line, as the Draw Phase starts.
                        if phase == 0 {
                            out.push(command(cmd::SPEAK_DRAW, [self.turn_player, 1, 0, 0]));
                        }
                    }
                }
                Event::Move(mv) => {
                    out.extend(self.summon_before_move(&mv, &messages[i..]));
                    out.extend(self.move_card(&mv));
                }
                Event::FieldDisabled(f) => out.extend(self.disabled_zones(&f)),
                Event::Hint(h) if h.hint_type == hint::EVENT && self.side(h.player) == CLIENT => {
                    self.phase_window = chain_window(h.hint) == Some(ChainWindow::Phase);
                }
                Event::Hint(h) if h.hint_type == hint::SELECTMSG && self.side(h.player) == CLIENT => {
                    self.hint = h.hint;
                    self.hint_note = h.hint_text.filter(|t| !t.is_empty()).map(|t| (self.hint, t));
                }
                Event::Hint(_) => {}
                // Also a Set Spell/Trap turning face-up as it is activated.
                Event::PosChange(c) => out.extend(self.turn(&c.loc, c.loc.position.unwrap_or(0))),
                // A Flip Summon as Konami's logs show it: the summon line with kind 4 before the flip, RunSummon with 1
                // after it (DuelScene reads only its place and uid: the summon effect at the card).
                Event::FlipSummoning(f) => {
                    let mut turned = self.turn(&f.loc, f.loc.position.unwrap_or(0));
                    let card = self.engine_place(&f.loc).and_then(|p| Some((p, self.sides[p.player as usize].zones.get(&p.zone)?.first()?.uid)));
                    if let Some((place, uid)) = card {
                        let flip = turned.iter().position(|c| c.get("id").and_then(Value::as_u32) == Some(cmd::FLIP_TURN)).unwrap_or(turned.len());
                        turned.insert(flip, command(cmd::SPEAK_SUMMON, [place.player, 0x8000 | uid, SUMMON_FLIP, 0]));
                        let place = Place { index: 0, face_up: true, ..place };
                        turned.push(command(cmd::RUN_SUMMON, [place.packed(), uid, 1, 0]));
                    }
                    out.extend(turned);
                }
                Event::Chaining(c) => {
                    let cid = c.loc.cid.unwrap_or(0);
                    if self.side(c.loc.controller) == CLIENT {
                        match self.card_kinds.get(&(cid as u16)) {
                            Some(13) => self.stats.spells += 1,
                            Some(14) => self.stats.traps += 1,
                            _ => {}
                        }
                    }
                    out.extend(self.activation(&c));
                    let link = self.chain_link(&c);
                    let param = |k: &str| link.get(k).and_then(Value::as_u32).unwrap_or(0);
                    self.chain.push((param("param1"), param("param2")));
                    out.push(link);
                    // A hand card's effect: its avatar's reveal line after the ChainSet (Konami's logs).
                    if c.loc.location == loc::HAND {
                        out.push(command(cmd::SPEAK_REVEAL, [self.side(c.loc.controller), 1, cid as u32, 0]));
                    }
                    self.chaining = Some(c);
                }
                // The link's own {player, zone, link}, as its ChainSet (Konami's logs; the client reads only the link).
                Event::ChainSolving(c) => {
                    let n = c.chain_size as u32;
                    let (player, zone) = self.chain.get((n as usize).wrapping_sub(1)).copied().unwrap_or((0, 0));
                    out.push(command(cmd::CHAIN_RUN, [player, zone, n, 0]));
                }
                Event::Attack(a) => {
                    out.extend(self.end_battle());
                    out.extend(self.attack(&a));
                }
                Event::Battle(b) => out.extend(self.battle(&b)),
                Event::DamageStepEnd | Event::AttackDisabled => out.extend(self.end_battle()),
                Event::Summoning(ref a) | Event::SpSummoning(ref a) => {
                    let id = if matches!(event, Event::Summoning(_)) { cmd::RUN_SUMMON } else { cmd::RUN_SP_SUMMON };
                    // The effect plays at the place's zone (0x514E5A), so the place, not only the player.
                    let card = self.engine_place(&a.loc).and_then(|p| Some((p, self.sides[p.player as usize].zones.get(&p.zone)?.first()?.uid)));
                    // The place as moves give it, with its face and position bits (Konami's logs: 0x4000 | place for a
                    // Normal Summon, 0x4000/0xC000/0x8000 for a Special Summon).
                    // A Fusion or Synchro Summon said its line as it arrived; a Tribute Summon's line has its own kind.
                    let cid = a.loc.cid.unwrap_or(0) as u16;
                    if self.side(a.loc.controller) == CLIENT {
                        self.stats.summoned.push(cid);
                        if self.cards.values().find(|c| c.cid == cid).is_some_and(|c| c.level >= crate::titles::HIGH) {
                            self.stats.high_level += 1;
                        }
                    }
                    let summoning = self.summoning.take();
                    let spoken = summoning.as_ref().is_some_and(|s| s.arrived && s.cid == cid);
                    let tribute = summoning.is_some_and(|s| s.family == Family::Tribute);
                    if let Some((place, uid)) = card {
                        // The summon line first: 0 a Normal Summon, 1 a Tribute Summon, 5 a Special Summon (Konami's).
                        let kind = match event {
                            Event::Summoning(_) if tribute => SUMMON_TRIBUTE,
                            Event::Summoning(_) => SUMMON_NORMAL,
                            _ => SUMMON_SPECIAL,
                        };
                        if !spoken {
                            out.push(command(cmd::SPEAK_SUMMON, [place.player, 0x8000 | uid, kind, 0]));
                        }
                        out.push(command(id, [Place { index: 0, ..place }.packed(), uid, 0, 0]));
                    }
                }
                Event::ConfirmCards(c) => out.extend(self.confirm(&c)),
                Event::ConfirmDecktop(c) => out.extend(self.reveal_deck_top(c.player, &c.cards)),
                Event::ReverseDeck => {
                    self.decks_reversed = !self.decks_reversed;
                    if !self.decks_reversed {
                        out.extend((0..2).flat_map(|p| self.face_down_deck_top(p)).collect::<Vec<_>>());
                    }
                }
                Event::DeckTop(d) => out.extend(self.face_up_deck_top(&d)),
                Event::ShuffleHand(h) => out.extend(self.shuffle_hand(&h)),
                Event::ShuffleDeck(d) => out.push(command(cmd::DECK_SHUFFLE, [self.side(d.player), 0, 0, 0])),
                Event::BecomeTarget(t) => {
                    out.extend(self.target_lockons(&t.cards));
                    out.extend(self.sync_relations());
                }
                Event::ChainSolved(c) => {
                    self.target_marks.retain(|&(link, ..)| link != c.chain_size as u32);
                    out.extend(self.sync_relations());
                }
                Event::Swap(w) => out.extend(self.swap(&w)),
                Event::ShuffleSetCard(sh) => out.extend(self.shuffle_set(&sh)),
                Event::SwapGraveDeck(g) => out.extend(self.swap_grave_deck(&g)),
                Event::ChainEnd => {
                    self.chaining = None;
                    self.chain.clear();
                    self.target_marks.clear();
                }
                Event::Win(w) => {
                    let result = match w.player {
                        TEAM_DRAW => result::DRAW,
                        p if self.side(p) == CLIENT => result::WON,
                        _ => result::LOST,
                    };
                    if result == result::WON {
                        self.stats.win_reason = Some(w.reason);
                    }
                    let reason = end_reason(w.reason);
                    out.push(command(cmd::DUEL_END, [result, reason, 0, 0]));
                    self.result = Some(result);
                }
                Event::Damage(ref c) | Event::Recover(ref c) | Event::PayLpCost(ref c) => {
                    let amount = c.amount as i32;
                    let change = if matches!(event, Event::Recover(_)) { amount } else { -amount };
                    let damage = match event {
                        Event::Recover(_) => DAMAGE_GAIN,
                        Event::PayLpCost(_) => DAMAGE_COST,
                        _ if self.in_battle => DAMAGE_BATTLE,
                        _ => DAMAGE_EFFECT,
                    };
                    // As Konami's server sent it: the change shown (param3's low byte 1: its number and sound), the
                    // damage line, then the change applied (low byte 0). One message with 0 changed the LP silently.
                    let p = self.side(c.player);
                    if p == CLIENT {
                        self.lp_change += change as i64;
                        self.stats.lowest_change = self.stats.lowest_change.min(self.lp_change);
                    }
                    out.push(command(cmd::CHANGE_LP, [p, change as u32, damage << 8 | 1, 0]));
                    out.push(command(cmd::SPEAK_DAMAGE, [p, change as u32, damage, 0]));
                    out.push(command(cmd::CHANGE_LP, [p, change as u32, damage << 8, 0]));
                }
                Event::AddCounter(c) => out.extend(self.counter(&c, true)),
                Event::RemoveCounter(c) => out.extend(self.counter(&c, false)),
                Event::LpUpdate(u) => out.push(command(cmd::SET_LP, [self.side(u.player), u.lp as u32, 0, 0])),
                Event::Equip(l) => self.link(&l, msg::EQUIP),
                Event::CardTarget(l) => self.link(&l, msg::CARD_TARGET),
                Event::CancelTarget(l) => self.link(&l, msg::CANCEL_TARGET),
                Event::TossCoin(t) => {
                    let p = self.side(t.player);
                    let wins = self.coin_call.take().unwrap_or(true) as u32;
                    for coins in t.results.chunks(COINS_A_TOSS) {
                        let faces = coins.iter().enumerate().fold(0, |bits, (i, &h)| bits | (h as u32) << i);
                        out.push(command(cmd::RUN_COIN, [p | (coins.len() as u32) << 6 | wins << 14, 0, faces, 0]));
                    }
                }
                Event::TossDice(t) => {
                    let p = self.side(t.player);
                    for &face in &t.results {
                        out.push(command(cmd::RUN_DICE, [p, 0, face as u32, 0]));
                    }
                }
                // Shown by its move (`move_card` plays the Set effect, 0x1E): nothing of its own.
                Event::Set(_) => {}
            }
        }
        out.extend(self.sync_relations());
        with_card_ids(&self.cards, &mut out);
        self.note_deck_tops(&out);
        out
    }

    /// An engine message `events` has no case for: fine when [`ENGINE_MESSAGES`] says so, else a warning, once a duel.
    pub(super) fn unshown(&mut self, m: &Json) {
        let Some(kind) = m["type"].as_u64() else { return };
        let entry = ENGINE_MESSAGES.iter().find(|(k, _, _)| *k == kind);
        if matches!(entry, Some((_, _, Handling::Shown | Handling::Asked | Handling::Ignored(_)))) || !self.unshown.insert(kind) {
            return;
        }
        let name = entry.map_or("not in ENGINE_MESSAGES", |(_, name, _)| name);
        tracing::warn!("[duel] engine message {kind} ({name}) is not shown to the player");
    }

    /// The engine messages this duel dropped without a case for them (each warned once).
    pub fn unshown_messages(&self) -> Vec<u64> {
        let mut kinds: Vec<u64> = self.unshown.iter().copied().collect();
        kinds.sort();
        kinds
    }

    /// MSG_EQUIP {card, target} (an equip's new monster: its earlier link goes), MSG_CARD_TARGET {card, target} (a
    /// lasting target) and MSG_CANCEL_TARGET (its end): the cards' links, by uid.
    pub(super) fn link(&mut self, l: &Link, kind: u64) {
        let uid = |t: &Self, loc: &Loc| {
            let place = t.engine_place(loc).filter(|p| p.zone <= zone::FIELD_SPELL)?;
            t.sides[place.player as usize].zones.get(&place.zone)?.first().map(|c| c.uid)
        };
        let (Some(card), Some(target)) = (uid(self, &l.card), uid(self, &l.target)) else { return };
        match kind {
            msg::EQUIP => {
                self.links.retain(|&(c, _, mark)| !(c == card && mark == MARK_EQUIP));
                self.links.push((card, target, MARK_EQUIP));
            }
            msg::CARD_TARGET if !self.links.contains(&(card, target, MARK_TARGET)) => self.links.push((card, target, MARK_TARGET)),
            msg::CANCEL_TARGET => self.links.retain(|&l| l != (card, target, MARK_TARGET)),
            _ => {}
        }
    }

    /// The client's relations brought in line with the links, both ways round so either card shows it: a link whose
    /// card left the field goes, one whose cards moved is re-sent at their new places. Removals, additions, then a redraw.
    pub(super) fn sync_relations(&mut self) -> Vec<Message> {
        let place_of = |t: &Self, uid: u32| {
            (0..2u32).find_map(|p| {
                (0..=zone::FIELD_SPELL).find(|z| t.sides[p as usize].zones.get(z).and_then(|c| c.first()).is_some_and(|c| c.uid == uid)).map(|z| p | z << 8)
            })
        };
        let mut wanted = vec![];
        let want = |wanted: &mut Vec<_>, (a, b, mark): (u32, u32, u32)| {
            for pair in [(a, b, mark), (b, a, mark)] {
                if !wanted.contains(&pair) {
                    wanted.push(pair);
                }
            }
        };
        let mut kept = vec![];
        for &link in &self.links {
            let (Some(a), Some(b)) = (place_of(self, link.0), place_of(self, link.1)) else { continue };
            kept.push(link);
            want(&mut wanted, (a, b, link.2));
        }
        self.links = kept;
        // The targets of the chain links on the way: a marker from the activating card until the link is solved.
        let marks: Vec<_> = self.target_marks.iter().copied().filter(|&(_, a, b)| place_of(self, a).is_some() && place_of(self, b).is_some()).collect();
        for &(_, a, b) in &marks {
            want(&mut wanted, (place_of(self, a).unwrap_or(0), place_of(self, b).unwrap_or(0), MARK_TARGET));
        }
        self.target_marks = marks;
        let mut out: Vec<Message> = self
            .relations
            .iter()
            .filter(|r| !wanted.contains(r))
            .map(|&(a, b, mark)| command(cmd::RELATION, [a, b, mark, 0]))
            .collect();
        out.extend(wanted.iter().filter(|r| !self.relations.contains(r)).map(|&(a, b, mark)| command(cmd::RELATION, [a, b, mark, 1])));
        if !out.is_empty() {
            out.push(command(cmd::RELATIONS_REDRAW, [0; 4]));
        }
        self.relations = wanted;
        out
    }

    /// The cards an effect targets (MSG_BECOME_TARGET, which follows the link's MSG_CHAINING: the cost and target are
    /// chosen after it) as Konami's server showed them (its logs: 1,373 CardLockon with param3 0, one per target of an
    /// effect, then the target marker from the activating card to it, taken away once the link is solved):
    /// CardLockon {player, zone | index << 8, 0} puts the lock-on cursor on the card (0x4FF000 case 0x19; param3 2 is the
    /// card's pop-up instead). Konami sent them before ChainSet; here they follow it. The marker (a relation) follows in
    /// `sync_relations`, from the chain's last link's card, until MSG_CHAIN_SOLVED.
    pub(super) fn target_lockons(&mut self, targets: &[Loc]) -> Vec<Message> {
        let uid_at = |t: &Self, loc: &Loc| {
            let place = t.engine_place(loc).filter(|p| p.zone <= zone::FIELD_SPELL)?;
            t.sides[place.player as usize].zones.get(&place.zone)?.first().map(|c| c.uid)
        };
        let link = self.chaining.clone();
        let source = link.as_ref().and_then(|c| uid_at(self, &Loc { location: c.loc.location & !loc::OVERLAY, ..c.loc.clone() }));
        let mut out = vec![];
        for loc in targets {
            let Some(place) = self.engine_place(loc).filter(|p| !matches!(p.zone, zone::DECK | zone::EXTRA)) else { continue };
            out.push(command(cmd::CARD_LOCKON, [place.player, place.zone | (place.index as u32) << 8, 0, 0]));
            if let (Some(c), Some(a), Some(b)) = (&link, source, uid_at(self, loc)) {
                self.target_marks.push((c.chain_size as u32, a, b));
            }
        }
        out
    }

    /// MSG_FIELD_DISABLED {field_mask}: per player (× 16) monster zones bits 0-4, Spell/Trap zones 8-12, set when
    /// disabled. The client's 1006 {mask}: its own side low, the opponent's high, monster zones bits 0-4 and Spell/Trap
    /// zones 5-9, set when usable; DuelFieldManager applies it on the next 0x26 (0x50AA90), DuelInvalidZone drawing a
    /// zone that became unusable (effect 0x2D) or usable again (0x2E).
    pub(super) fn disabled_zones(&mut self, f: &FieldDisabled) -> Vec<Message> {
        let mask = f.field_mask as u32;
        let usable = |p: u32| {
            let dis = mask >> (16 * p);
            (0..5).filter(|z| dis & 1 << z == 0).fold(0, |a, z| a | 1 << z) | (0..5).filter(|z| dis & 1 << (8 + z) == 0).fold(0, |a, z| a | 1 << (5 + z))
        };
        let own = (0..2).find(|&p| self.side(p as u64) == CLIENT).unwrap_or(0);
        vec![command(cmd::ZONES_USABLE, [usable(own) | usable(1 - own) << 16, 0, 0, 0]), command(cmd::RELATIONS_REDRAW, [0; 4])]
    }

    /// A deck card with that id moves to the end of the hand.
    pub(super) fn draw(&mut self, p: u32, cid: u16) -> Vec<Message> {
        let hand = self.sides[p as usize].zone(zone::HAND).len();
        let from = Place { player: p, zone: zone::DECK, index: 0, face_up: false, defense: false };
        let to = Place { player: p, zone: zone::HAND, index: hand, face_up: false, defense: false };
        self.relocate(cid, from, to)
    }

    /// MSG_MOVE {card, cid, from, to}: the card leaves its place for the new one.
    /// Before a MSG_MOVE (`rest[0]`, the rest of the batch after it): a Tribute, Fusion or Synchro Summon's commands
    /// (see [`Summoning`]). The first material's move marks every material of the summon that moves in this batch; each
    /// plays its effect before its own move, but a Synchro's are all tuned at the first, the Tuners first (the client's
    /// first Tuner starts its level sums again, 0x5165A0; Konami's logs always tune the Tuner first, and the Synchro
    /// animation crashed on sums that left one out). An Extra Deck monster's arrival plays RunFusion and its summon
    /// line. The engine may report a summon's materials and its arrival in different batches (a Synchro Summon asks
    /// for its position in between).
    pub(super) fn summon_before_move(&mut self, mv: &Move, rest: &[Json]) -> Vec<Message> {
        let cid = mv.cid.unwrap_or(0) as u16;
        let reason = mv.reason;
        // This batch's moves from this one on.
        let moves: Vec<Move> = rest.iter().filter_map(|x| match Event::lenient(x) { Ok(Some(Event::Move(m))) => Some(m), _ => None }).collect();
        let mut out = vec![];
        if let Some(family) = Family::of(reason) {
            let mut s = match self.summoning.take() {
                Some(s) if s.family == family && !s.arrived => s,
                _ => Summoning { family, count: 0, tuned: 0, arrived: false, cid: 0, played: vec![], uids: vec![] },
            };
            if let Some(at) = s.played.iter().position(|(c, _)| *c == cid) {
                out.extend(s.played.remove(at).1);
                self.summoning = Some(s);
                return out;
            }
            // This move's material and the others of this batch: (Konami id, place, uid, Tuner). Each move's place is
            // the engine's as it happens, after the earlier ones: three materials from the hand all leave its index 0
            // (Ultimate Ancient Gear Golem's two Golems: both were taken for the first one's card), so each is found
            // among the cards the earlier ones left.
            let mut materials: Vec<(u16, Place, u32, bool)> = vec![];
            for x in &moves {
                if Family::of(x.reason) != Some(family) {
                    continue;
                }
                let cid = x.cid.unwrap_or(0) as u16;
                let Some(from) = self.engine_place(&x.from) else { continue };
                let taken: Vec<u32> = materials.iter().map(|&(_, _, uid, _)| uid).collect();
                if let Some(uid) = self.uid_left(&from, cid, &taken) {
                    materials.push((cid, from, uid, self.tuners.contains(&cid)));
                }
            }
            materials.sort_by_key(|&(.., tuner)| !(family == Family::Synchro && tuner));
            let marks = family.set_and_run();
            for &(_, from, uid, _) in materials.iter().filter(|_| marks.is_some()) {
                out.push(command(marks.map_or(0, |m| m.0), [from.player, from.zone, uid, 0]));
            }
            // Each material's effect (none for a Contact Fusion's), by its Konami id.
            let mut runs: Vec<(u16, Option<Message>)> = vec![];
            for &(material, from, uid, tuner) in &materials {
                let variant = match family {
                    Family::Synchro if tuner => {
                        s.tuned += 1;
                        self.level_of(material) | if s.tuned == 1 { 0x100 } else { 0x200 }
                    }
                    Family::Synchro => self.level_of(material),
                    _ => 0,
                };
                runs.push((material, marks.map(|m| command(m.1, [from.player, from.zone, uid, variant]))));
            }
            s.count += materials.len() as u32;
            s.uids.extend(materials.iter().map(|&(_, _, uid, _)| uid));
            // A Synchro tunes them all now, the Tuners first. A Fusion's or a Tribute's material plays its effect right
            // before its own move, as in Konami's logs: run ahead of the first move, a Fusion's materials from the hand
            // crashed the client (0x4FC4A9, live 2026-11-15, issue #40).
            if family == Family::Synchro {
                for (_, run) in &mut runs {
                    out.extend(run.take());
                }
            }
            if let Some(at) = runs.iter().position(|(c, _)| *c == cid) {
                out.extend(runs.remove(at).1);
            }
            // The others' moves come later in this batch, with their effects if not yet played.
            s.played = runs;
            self.summoning = Some(s);
            return out;
        }
        // A tribute for a Set: the monster goes to its zone face down and no summon follows.
        let set = mv.to.location == loc::MZONE && mv.to.position.is_some_and(|p| p & 0xA != 0);
        if set && self.summoning.as_ref().is_some_and(|s| s.family == Family::Tribute) {
            self.summoning = None;
        }
        // The Extra Deck monster arrives: its animation and its line (Konami's logs: 8 for a Synchro, 6 for a Fusion).
        let arriving = reason & reason::SPSUMMON != 0 && mv.from.location == loc::EXTRA;
        let place = self.engine_place(&mv.from).and_then(|from| Some((from, self.uid_at(&from, cid)?)));
        if let Some(s) = self.summoning.as_mut().filter(|s| arriving && s.family != Family::Tribute && !s.arrived)
            && let Some((from, uid)) = place
        {
            let (kind, line) = match s.family {
                Family::Synchro => (2, SUMMON_SYNCHRO),
                Family::Contact => (1, SUMMON_EXTRA),
                _ => (0, SUMMON_EXTRA),
            };
            let at = |i: usize| s.uids.get(i).copied().unwrap_or(0);
            out.push(command(cmd::MATERIAL_LIST, [s.uids.len() as u32, at(0), at(1), at(2)]));
            for more in s.uids.get(3..).unwrap_or_default().chunks(4) {
                let at = |i: usize| more.get(i).copied().unwrap_or(0);
                out.push(command(cmd::MATERIAL_LIST_MORE, [at(0), at(1), at(2), at(3)]));
            }
            if from.player == CLIENT {
                match s.family {
                    Family::Synchro => self.stats.synchros += 1,
                    _ => self.stats.fusions += 1,
                }
            }
            out.push(command(cmd::RUN_FUSION, [from.player, uid, kind << 16 | s.count, 0]));
            out.push(command(cmd::SPEAK_SUMMON, [from.player, 0x8000 | uid, line, 0]));
            s.arrived = true;
            s.cid = cid;
        }
        out
    }

    /// The uid of the card with Konami id `cid` at `place` (in a list zone, at the place's index if it is there).
    pub(super) fn uid_at(&self, place: &Place, cid: u16) -> Option<u32> {
        let zone = self.sides[place.player as usize].zones.get(&place.zone)?;
        let id = |c: &Card| self.cards.get(&c.row).map(|s| s.cid);
        zone.get(place.index).filter(|c| id(c) == Some(cid)).or_else(|| zone.iter().find(|c| id(c) == Some(cid))).map(|c| c.uid)
    }

    /// [`uid_at`](Self::uid_at) once the cards `taken` have left: a place the engine gives after earlier moves of the
    /// same batch, which the translator hasn't followed yet.
    pub(super) fn uid_left(&self, place: &Place, cid: u16, taken: &[u32]) -> Option<u32> {
        let zone = self.sides[place.player as usize].zones.get(&place.zone)?;
        let id = |c: &Card| self.cards.get(&c.row).map(|s| s.cid);
        let left: Vec<&Card> = zone.iter().filter(|c| !taken.contains(&c.uid)).collect();
        left.get(place.index).filter(|c| id(c) == Some(cid)).or_else(|| left.iter().find(|c| id(c) == Some(cid))).map(|c| c.uid)
    }

    pub(super) fn level_of(&self, cid: u16) -> u32 {
        self.cards.values().find(|c| c.cid == cid).map_or(0, |c| c.level)
    }

    pub(super) fn move_card(&mut self, mv: &Move) -> Vec<Message> {
        let cid = mv.cid.unwrap_or(0) as u16;
        if mv.to.is_material() && !mv.from.is_material() {
            return self.attach(mv, cid);
        }
        if mv.from.is_material() {
            return self.engine_place(&mv.to).map(|to| self.detach(cid, to)).unwrap_or_default();
        }
        if mv.from.location == 0 {
            return self.engine_place(&mv.to).map(|to| self.token(mv, to)).unwrap_or_default();
        }
        if let Some(from) = self.engine_place(&mv.from).filter(|f| f.zone <= zone::FIELD_SPELL)
            && let Some(card) = self.sides[from.player as usize].zones.get(&from.zone).and_then(|z| z.first()).copied()
            && self.tokens.contains(&card.uid)
            && !self.engine_place(&mv.to).is_some_and(|to| to.zone <= zone::FIELD_SPELL)
        {
            // A token leaving the field is gone (EDOPro may send it to a pile first): it shatters off the field, a monster
            // zone or, equipped as a card (Allure Queen LV3), a Spell/Trap zone. One that changes sides (a control
            // change) or becomes an equip stays and moves like any card.
            let mut out = self.drop_counters(&from);
            self.sides[from.player as usize].zone(from.zone).clear();
            self.tokens.retain(|&t| t != card.uid);
            out.push(command(cmd::CARD_VANISH, [from.player, from.zone, 0, 0]));
            return out;
        }
        match (self.engine_place(&mv.from), self.engine_place(&mv.to)) {
            (Some(from), Some(to)) => {
                let reason = mv.reason;
                let dropped = if to.zone > zone::FIELD_SPELL { self.drop_counters(&from) } else { vec![] };
                let kind = move_kind(reason, from, to);
                let mut moved = self.relocate_as(cid, from, to, kind);
                if !moved.is_empty() {
                    moved = self.presented(moved, cid, from, to, kind);
                }
                if to.zone == zone::HAND && let Some(at) = self.revealing.iter().position(|&c| c == cid) {
                    self.revealing.remove(at);
                    let index = self.sides[to.player as usize].zone(zone::HAND).len().saturating_sub(1).min(to.index);
                    moved.extend(self.show_hand(to.player, index));
                }
                // Only an actual destruction move confirms the card died. MSG_BATTLE precedes limited battle
                // protection (Fortress Warrior, Shield Wing), so its tentative destroy flag cannot hide the card.
                if moved.is_empty() || reason & reason::DESTROY == 0 || from.zone > zone::HAND {
                    return dropped.into_iter().chain(moved).collect();
                }
                let vanish = command(cmd::CARD_VANISH, [from.player, from.zone | (from.index as u32) << 8, 0, 0]);
                dropped.into_iter().chain(std::iter::once(vanish)).chain(moved).collect()
            }
            // Tokens and Xyz materials have no client place yet.
            _ => vec![],
        }
    }

    /// MSG_SHUFFLE_HAND {player, cards, cids}: the engine's hand in its new order (after a card in it was shown). The
    /// mirror takes that order and the client follows: each card's table entry at its new index, then HandShuffle. A
    /// list that doesn't match the mirror's hand (another count, an id it doesn't hold) is left alone: moves still find
    /// their card by id.
    pub(super) fn shuffle_hand(&mut self, h: &Shuffle) -> Vec<Message> {
        let p = self.side(h.player);
        let cids: Vec<u16> = h.cids.iter().map(|&c| c as u16).collect();
        let cards = &self.cards;
        let hand = self.sides[p as usize].zone(zone::HAND);
        if cids.len() != hand.len() {
            tracing::warn!("a hand shuffle of {} cards for a hand of {}", cids.len(), hand.len());
            return vec![];
        }
        let mut left = hand.clone();
        let mut order = Vec::with_capacity(left.len());
        for cid in &cids {
            let Some(at) = left.iter().position(|c| cards.get(&c.row).map(|s| s.cid) == Some(*cid)) else {
                tracing::warn!("a hand shuffle names card {cid}, which the hand doesn't hold");
                return vec![];
            };
            order.push(left.remove(at));
        }
        if order.iter().map(|c| c.uid).eq(hand.iter().map(|c| c.uid)) {
            return vec![];
        }
        *hand = order.clone();
        tracing::debug!("hand shuffle: player {p}, {} cards", order.len());
        let mut out: Vec<Message> = order
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let row = self.shown_row(&Place { player: p, zone: zone::HAND, index: i, face_up: false, defense: false }, c.row);
                Cmd::Card { uid: c.uid, open: false, row, place: table_place(p, zone::HAND, i) }.message()
            })
            .collect();
        out.push(command(cmd::HAND_SHUFFLE, [p, 0, 0, 0]));
        out
    }

    /// A field card becomes an Xyz material: the client has no place under a monster, so the card leaves the field with
    /// CardVanish (0x1F: marked off the field and hidden, 0x4FC520; the slot keeps the object until another card lands
    /// there) and the mirror keeps it among the materials. **Shape**: the shatter effect plays for it. From elsewhere (a
    /// hand or pile) it isn't followed (logged).
    pub(super) fn attach(&mut self, mv: &Move, cid: u16) -> Vec<Message> {
        let Some(from) = self.engine_place(&mv.from) else { return vec![] };
        if from.zone > zone::FIELD_SPELL {
            tracing::warn!("an Xyz material from a place the client can't take it from: {mv:?}");
            return vec![];
        }
        let Some(card) = self.sides[from.player as usize].zones.get(&from.zone).and_then(|z| z.first()).copied() else { return vec![] };
        if self.cards.get(&card.row).map(|s| s.cid) != Some(cid) && cid != 0 {
            tracing::warn!("mirror out of step: Xyz material {cid} not at {from:?}");
        }
        let mut out = self.drop_counters(&from);
        self.sides[from.player as usize].zone(from.zone).clear();
        self.positions.remove(&(from.player, from.zone));
        self.materials.push((card, from));
        out.push(command(cmd::CARD_VANISH, [from.player, from.zone, 0, 0]));
        out
    }

    /// An Xyz material leaves its monster for a place (a detach to the Graveyard, its monster leaving the field): the
    /// card goes there with a move from the field slot it left when that slot is still empty (the client clears the
    /// move's slot, 0x5000E0, and so lets go of the hidden object), else from any empty field slot (another card landed
    /// on its old one, which let go of it). **Shape**: the card flies from that slot. None empty: not followed (logged).
    pub(super) fn detach(&mut self, cid: u16, to: Place) -> Vec<Message> {
        let Some(at) = self.materials.iter().position(|(c, _)| self.cards.get(&c.row).map(|s| s.cid) == Some(cid) || cid == 0) else { return vec![] };
        let empty_at = |s: &Self, p: u32, z: u32| s.sides[p as usize].zones.get(&z).is_none_or(Vec::is_empty);
        let left = self.materials[at].1;
        let empty = |s: &Self, p: u32| (0..=zone::FIELD_SPELL).find(|&z| empty_at(s, p, z)).map(|z| (p, z));
        let slot = if empty_at(self, left.player, left.zone) { Some((left.player, left.zone)) } else { empty(self, to.player).or_else(|| empty(self, 1 - to.player)) };
        let Some((player, zone)) = slot else {
            tracing::warn!("an Xyz material leaves with no empty field slot to move it from");
            return vec![];
        };
        let (card, _) = self.materials.remove(at);
        self.sides[player as usize].zone(zone).push(card);
        let via = Place { player, zone, index: 0, face_up: false, defense: false };
        self.relocate(cid, via, to)
    }

    /// MSG_SWAP_GRAVE_DECK {player, grave, extra}: a player's Graveyard and Deck trade their cards (Exchange of the Spirit),
    /// Extra Deck monsters going back to the Extra Deck. The sidecar gives the Graveyard and Extra Deck as they are after
    /// it. Each old Graveyard card moves out, from the top: to the Extra Deck while the engine's holds more of its id than
    /// the mirror's, else to the Deck; then each new Graveyard card moves in from the Deck, in the engine's order.
    pub(super) fn swap_grave_deck(&mut self, g: &SwapGraveDeck) -> Vec<Message> {
        let p = self.side(g.player);
        let cids = |cards: &[Coded]| -> Vec<u16> { cards.iter().map(|c| c.cid.unwrap_or(0) as u16).collect() };
        let (grave, extra) = (cids(&g.grave), cids(&g.extra));
        let cid_of = |s: &Self, c: &Card| s.cards.get(&c.row).map_or(0, |s| s.cid);
        let at = |zone: u32, index: usize| Place { player: p, zone, index, face_up: true, defense: false };
        let mut out = vec![];
        while let Some(card) = self.sides[p as usize].zones.get(&zone::GRAVE).and_then(|z| z.last()).copied() {
            let cid = cid_of(self, &card);
            let index = self.sides[p as usize].zone(zone::GRAVE).len() - 1;
            let in_extra = self.sides[p as usize].zones.get(&zone::EXTRA).map_or(0, |z| z.iter().filter(|c| cid_of(self, c) == cid).count());
            let to = if extra.iter().filter(|&&c| c == cid).count() > in_extra {
                Place { face_up: false, ..at(zone::EXTRA, self.sides[p as usize].zone(zone::EXTRA).len()) }
            } else {
                Place { face_up: false, ..at(zone::DECK, 0) }
            };
            out.extend(self.relocate(cid, at(zone::GRAVE, index), to));
        }
        for (i, cid) in grave.into_iter().enumerate() {
            out.extend(self.relocate(cid, Place { face_up: false, ..at(zone::DECK, 0) }, at(zone::GRAVE, i)));
        }
        out
    }

    /// MSG_SWAP {card1, card2}: two field cards trade places (Creature Swap: two monsters change sides), each keeping its
    /// own position. Which mirror card is the engine's card1 is told by its id; then [`trade`](Self::trade).
    pub(super) fn swap(&mut self, w: &Swap) -> Vec<Message> {
        let (Some(p1), Some(p2)) = (self.engine_place(&w.card1), self.engine_place(&w.card2)) else { return vec![] };
        if p1.zone > zone::FIELD_SPELL || p2.zone > zone::FIELD_SPELL {
            return vec![];
        }
        let cid1 = w.card1.cid.unwrap_or(0) as u16;
        let holds = |p: &Place| self.sides[p.player as usize].zones.get(&p.zone).and_then(|z| z.first()).and_then(|c| self.cards.get(&c.row)).map(|s| s.cid);
        // The message may name the places before or after the swap: card1 goes to whichever place doesn't hold it now.
        let (card1_at, card2_at) = if holds(&p2) == Some(cid1) && holds(&p1) != Some(cid1) { (p2, p1) } else { (p1, p2) };
        let (card1_to, card2_to) = (Place { face_up: p1.face_up, defense: p1.defense, ..card2_at }, Place { face_up: p2.face_up, defense: p2.defense, ..card1_at });
        self.trade(card1_to, card2_to)
    }

    /// MSG_SHUFFLE_SET_CARD {location, places: [{controller, location, sequence, position, cid}]}: Set cards shuffled
    /// among their zones (Magical Hats). The sidecar lists each shuffled zone and the card it holds after the shuffle (the
    /// core's own lists give only the old places, see `fix_shuffles` in crates/yo3-engine's session.rs); each of the mirror's cards in those
    /// zones is matched by id to its new zone (identical cards are interchangeable) and traded into place.
    pub(super) fn shuffle_set(&mut self, sh: &ShuffleSet) -> Vec<Message> {
        let places: Vec<(Place, u16)> = sh
            .places
            .iter()
            .filter_map(|p| Some((self.engine_place(p)?, p.cid.unwrap_or(0) as u16)))
            .filter(|(p, _)| p.zone <= zone::FIELD_SPELL)
            .collect();
        let mut left: Vec<(u32, u16)> = places
            .iter()
            .filter_map(|(p, _)| self.sides[p.player as usize].zones.get(&p.zone).and_then(|z| z.first()))
            .map(|c| (c.uid, self.cards.get(&c.row).map_or(0, |s| s.cid)))
            .collect();
        let mut moves = vec![];
        for (to, cid) in &places {
            if let Some(i) = left.iter().position(|(_, c)| c == cid) {
                moves.push((left.remove(i).0, *to));
            }
        }
        if !left.is_empty() {
            tracing::warn!("a Set card shuffle the mirror can't follow: {sh:?}");
        }
        let mut out = vec![];
        for (uid, to) in moves {
            let at = self.sides.iter().enumerate().find_map(|(p, side)| {
                side.zones.iter().find(|(z, cards)| **z <= zone::FIELD_SPELL && cards.first().is_some_and(|c| c.uid == uid)).map(|(z, _)| (p as u32, *z))
            });
            let Some((player, zone)) = at.filter(|&(p, z)| (p, z) != (to.player, to.zone)) else { continue };
            // The card already in the new zone takes this card's old zone as it is shown now.
            let (face_up, defense) = self.positions.get(&(to.player, to.zone)).copied().unwrap_or((to.face_up, to.defense));
            out.extend(self.trade(to, Place { player, zone, index: 0, face_up, defense }));
        }
        out
    }

    /// The cards at field places `a` and `b` trade places: the one at `b` lands at `a` with `a`'s face and position, the
    /// one at `a` at `b` with `b`'s. CardSwap (0x1B) when both are there, a move (0x1A) when one place is empty; then each
    /// card's table entry and, for a monster zone, its record.
    pub(super) fn trade(&mut self, a: Place, b: Place) -> Vec<Message> {
        let at = |s: &Self, p: &Place| s.sides[p.player as usize].zones.get(&p.zone).and_then(|z| z.first()).copied();
        let (card_a, card_b) = (at(self, &a), at(self, &b));
        let (record_a, record_b) = (self.records.get(&(a.player, a.zone)).copied(), self.records.get(&(b.player, b.zone)).copied());
        let mut out = match (card_a, card_b) {
            (Some(_), Some(_)) => vec![command(cmd::CARD_SWAP, [0, a.packed(), b.packed(), 0])],
            (Some(c), None) => vec![Cmd::Move { uid: c.uid, kind: 0, from: Place { face_up: false, defense: false, ..a }, to: b }.message()],
            (None, Some(c)) => vec![Cmd::Move { uid: c.uid, kind: 0, from: Place { face_up: false, defense: false, ..b }, to: a }.message()],
            (None, None) => return vec![],
        };
        for (place, card, record) in [(a, card_b, record_b), (b, card_a, record_a)] {
            let zone = self.sides[place.player as usize].zone(place.zone);
            zone.clear();
            zone.extend(card);
            match card {
                Some(c) => {
                    self.positions.insert((place.player, place.zone), (place.face_up, place.defense));
                    let row = self.shown_row(&place, c.row);
                    out.push(Cmd::Card { uid: c.uid, open: open(place.zone), row, place: table_place(place.player, place.zone, 0) }.message());
                    if place.zone < 5 && row != 0 && place.face_up {
                        out.extend(match record {
                            Some(values) => self.record(place.player, place.zone, c.row, values),
                            None => self.monster_record(place.player, place.zone, c.row),
                        });
                    }
                }
                None => {
                    self.positions.remove(&(place.player, place.zone));
                }
            }
        }
        out
    }

    /// MSG_CONFIRM_CARDS {player, cards}: the cards are shown to `player`. The client sees one it is shown to, and its
    /// own as the other side sees them; a hand card shows its face at once, a card from elsewhere (the Deck, a search)
    /// when it reaches the hand.
    pub(super) fn confirm(&mut self, c: &Confirm) -> Vec<Message> {
        let viewer = self.side(c.player);
        let mut out = vec![];
        // A Deck's top cards shown to the client (Ancient Telescope): turned over on the pile, as MSG_CONFIRM_DECKTOP.
        let deck_cards: Vec<Loc> = c.cards.iter().filter(|c| c.location == loc::DECK).cloned().collect();
        if viewer == CLIENT && !deck_cards.is_empty() && deck_cards.len() == c.cards.len() {
            let owner = deck_cards[0].controller;
            let size = self.sides[self.side(owner) as usize].zones.get(&zone::DECK).map_or(0, Vec::len) as u64;
            let top = deck_cards.iter().all(|c| c.controller == owner && c.sequence + deck_cards.len() as u64 >= size);
            if top {
                return self.reveal_deck_top(owner, &deck_cards);
            }
        }
        for card in &c.cards {
            let owner = self.side(card.controller);
            if viewer != CLIENT && owner != CLIENT {
                continue;
            }
            let cid = card.cid.unwrap_or(0) as u16;
            match card.location {
                loc::HAND => out.extend(self.show_hand(owner, card.sequence as usize)),
                loc::MZONE | loc::SZONE => {
                    if viewer == CLIENT && owner != CLIENT {
                        out.extend(self.reveal_field(card));
                    }
                }
                _ if !self.revealing.contains(&cid) => self.revealing.push(cid),
                _ => {}
            }
        }
        out
    }

    /// A face-down field card the client can't see, shown to it and hidden again, as Konami's server did (Stern Mystic in
    /// its logs): the table entry with the row, CardCheat (0x1D: the card object reloads its face from the row,
    /// 0x4FF000), the flip face up (0x1C), the card's popup (CardLockon 0x19 {player, zone, 2}), then the entry without a
    /// row, CardCheat and the flip back.
    pub(super) fn reveal_field(&mut self, loc: &Loc) -> Vec<Message> {
        let Some(place) = self.engine_place(loc).filter(|p| p.zone <= zone::FIELD_SPELL) else { return vec![] };
        let Some(&(face_up, defense)) = self.positions.get(&(place.player, place.zone)) else { return vec![] };
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied() else { return vec![] };
        if face_up || !self.hidden[place.player as usize] {
            return vec![];
        }
        let cid = self.cards.get(&card.row).map_or(0, |s| s.cid as u32);
        let down = Place { face_up: false, defense: place.zone < 5 && defense, ..place };
        let up = Place { face_up: true, ..down };
        let key = table_place(place.player, place.zone, 0);
        vec![
            Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: key }.message(),
            command(cmd::CARD_CHEAT, [up.packed(), cid, card.uid, 0]),
            command(cmd::FLIP_TURN, [up.packed(), 0, 0, 0]),
            command(cmd::CARD_LOCKON, [place.player, place.zone, POPUP, 0]),
            Cmd::Card { uid: card.uid, open: false, row: 0, place: key }.message(),
            command(cmd::CARD_CHEAT, [down.packed(), 0, card.uid, 0]),
            command(cmd::FLIP_TURN, [down.packed(), 0, 0, 0]),
        ]
    }

    /// Remembers the table entries sent for each Deck's index 0 (see `deck_tops`).
    pub(super) fn note_deck_tops(&mut self, out: &[Message]) {
        for m in out {
            let int = |k: &str| m.get(k).and_then(Value::as_u32).unwrap_or(0);
            if int("id") != cmd::CARD {
                continue;
            }
            for p in 0..2 {
                if int("param4") == table_place(p, zone::DECK, 0) {
                    self.deck_tops[p as usize] = Some([int("param1"), int("param2"), int("param3"), int("param4")]);
                }
            }
        }
    }

    /// MSG_CONFIRM_DECKTOP {player, cards}: the Deck's top cards shown to both players, one at a time as Konami's server
    /// showed them: the card's table entry at the Deck's index 0, DeckReset (once), CardCheat, DeckFlipTop (the pile's
    /// top turns over from index 0's row) and the card's popup (CardLockon); then the top turns back and index 0 gets
    /// its last entry again, and DeckReset redraws the pile.
    pub(super) fn reveal_deck_top(&mut self, player: u64, cards: &[Loc]) -> Vec<Message> {
        let p = self.side(player);
        let key = table_place(p, zone::DECK, 0);
        let deck = self.sides[p as usize].zones.get(&zone::DECK).cloned().unwrap_or_default();
        let mut used: Vec<u32> = vec![];
        let mut out = vec![];
        for c in cards {
            let cid = c.cid.unwrap_or(0) as u16;
            let has_cid = |d: &&Card| !used.contains(&d.uid) && self.cards.get(&d.row).map(|s| s.cid) == Some(cid);
            let Some(card) = deck.iter().find(has_cid).copied() else { continue };
            used.push(card.uid);
            let hidden = self.shown_row(&Place { player: p, zone: zone::DECK, index: 0, face_up: false, defense: false }, card.row);
            // Index 0's last entry back while its card is still in the Deck; one that has left (drawn, sent) would pull
            // its table entry off its new place, so the revealed card stays there instead, hidden.
            let in_deck = |uid: u32| deck.iter().any(|d| d.uid == uid & !OPEN);
            let back = self.deck_tops[p as usize].filter(|e| in_deck(e[0]) && e[0] & OPEN == 0).unwrap_or([card.uid, hidden, 0, key]);
            out.push(Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: key }.message());
            if used.len() == 1 {
                out.push(command(cmd::DECK_RESET, [p, zone::DECK, 0, 0]));
            }
            out.extend([
                command(cmd::CARD_CHEAT, [0x4000 | move_place(p, zone::DECK, 0), cid as u32, card.uid, 0]),
                command(cmd::DECK_FLIP_TOP, [p, zone::DECK, 0, 0]),
                command(cmd::CARD_LOCKON, [p, zone::DECK, POPUP, 0]),
                command(cmd::DECK_FLIP_TOP, [p, zone::DECK, 0, 0]),
                command(cmd::CARD, back),
                command(cmd::DECK_RESET, [p, zone::DECK, 0, 0]),
            ]);
        }
        out
    }

    /// MSG_DECK_TOP {player, count, cid, position}: a face-up card on top of a Deck (Convulsion of Nature turned the
    /// Decks over): the table entry at the Deck's index 0 with its row and the public flag, then DeckReset, so the pile
    /// shows its face (0x4FE4D0), as a Graveyard shows its top card.
    pub(super) fn face_up_deck_top(&mut self, d: &DeckTop) -> Vec<Message> {
        let p = self.side(d.player);
        let cid = d.cid.unwrap_or(0) as u16;
        // A reversed Deck's top is public though its cards keep their position (field.cpp reverse_deck only reorders);
        // otherwise only a card turned face up in the Deck (ReverseInDeck).
        let face_up = self.decks_reversed || d.position & (pos::FACEUP_ATTACK | pos::FACEUP_DEFENSE) != 0;
        if d.count != 0 || !face_up {
            return vec![];
        }
        let deck = self.sides[p as usize].zones.get(&zone::DECK);
        let Some(card) = deck.and_then(|d| d.iter().find(|c| self.cards.get(&c.row).map(|s| s.cid) == Some(cid))).copied() else {
            return vec![];
        };
        vec![Cmd::Card { uid: card.uid, open: true, row: card.row as u32, place: table_place(p, zone::DECK, 0) }.message(), command(cmd::DECK_RESET, [p, zone::DECK, 0, 0])]
    }

    /// The Decks turned back: the card last shown on top of player `p`'s Deck loses its face (and, hidden, its row).
    pub(super) fn face_down_deck_top(&mut self, p: u32) -> Vec<Message> {
        let Some([uid, row, _, place]) = self.deck_tops[p as usize].filter(|e| e[0] & OPEN != 0) else { return vec![] };
        let uid = uid & !OPEN;
        // Only while that card is still in the Deck (a drawn one has its own entry by now).
        if !self.sides[p as usize].zones.get(&zone::DECK).is_some_and(|d| d.iter().any(|c| c.uid == uid)) {
            return vec![];
        }
        let shown = self.shown_row(&Place { player: p, zone: zone::DECK, index: 0, face_up: false, defense: false }, row as u16);
        vec![Cmd::Card { uid, open: false, row: shown, place }.message(), command(cmd::DECK_RESET, [p, zone::DECK, 0, 0])]
    }

    /// A hand card shows its face: its table entry with the row (the opponent's hand has none), then HandShow.
    pub(super) fn show_hand(&self, player: u32, index: usize) -> Vec<Message> {
        let Some(card) = self.sides[player as usize].zones.get(&zone::HAND).and_then(|z| z.get(index)).copied() else { return vec![] };
        vec![
            Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: table_place(player, zone::HAND, index) }.message(),
            command(cmd::HAND_SHOW, [player, index as u32, 0, 0]),
        ]
    }

    /// A token appears on a monster zone: the client's token card of its name (in turn when it has several, as for
    /// the four Sheep Tokens), a card table entry, its record and a move onto its own slot, which makes the card
    /// object (0x1A finds none by uid, 0x5003A0). A token with no card of its name isn't shown.
    pub(super) fn token(&mut self, mv: &Move, to: Place) -> Vec<Message> {
        let reserved = self.reserved_token.take();
        let Some(rows) = self.token_card_rows(mv.cid, mv.name.as_deref()) else {
            tracing::warn!("a token the client has no card for: {:?} {:?}", mv.cid, mv.name);
            return vec![];
        };
        if to.zone >= 5 {
            return vec![];
        }
        // The uid and card its position dialog showed, so the dialog's card is the one that appears.
        let (uid, row) = match reserved {
            Some((uid, row)) if rows.contains(&row) => (uid, row),
            _ => self.next_token_card(&rows),
        };
        self.tokens.push(uid);
        let zone = self.sides[to.player as usize].zone(to.zone);
        zone.clear();
        zone.push(Card { uid, row });
        self.positions.insert((to.player, to.zone), (to.face_up, to.defense));
        let mut out = vec![Cmd::Card { uid, open: false, row: row as u32, place: table_place(to.player, to.zone, 0) }.message()];
        out.extend(self.monster_record(to.player, to.zone, row));
        out.push(Cmd::Move { uid, kind: MOVE_SPECIAL_SUMMON, from: to, to }.message());
        out
    }

    /// The client's token cards (CARD_Prop rows) for an engine token: a token with a Konami id (Doomsday Token, 3940)
    /// is the client's card of that id; one without, by its name. None for a card that isn't a token the client has.
    pub(super) fn token_card_rows(&self, cid: Option<u64>, name: Option<&str>) -> Option<Vec<u16>> {
        let cid = cid.filter(|&c| c != 0).map(|c| c as u16);
        let by_id: Option<Vec<u16>> = cid.map(|c| {
            let mut rows: Vec<u16> = self.cards.iter().filter(|(_, s)| s.cid == c).map(|(&r, _)| r).collect();
            rows.sort_unstable();
            rows
        });
        let by_name = || name.and_then(|n| self.token_rows.get(n)).cloned();
        by_id.filter(|r| !r.is_empty()).or_else(by_name).filter(|r| !r.is_empty())
    }

    /// A new token's uid and card: the next of its cards in turn (the four Sheep Tokens' colours).
    pub(super) fn next_token_card(&mut self, rows: &[u16]) -> (u32, u16) {
        let row = rows[(self.next_token - FIRST_TOKEN_UID) as usize % rows.len()];
        let uid = self.next_token;
        self.next_token += 1;
        (uid, row)
    }

    /// Moves a card in the mirror and on the client: the card table entry (1000), then on screen (0x1A); the
    /// cards whose index shifted in a list zone get their table entry again.
    pub(super) fn relocate(&mut self, cid: u16, from: Place, to: Place) -> Vec<Message> {
        self.relocate_as(cid, from, to, MOVE_PLAIN)
    }

    /// A move's presentation around it, as Konami's server sent it: a card leaving the hand for the field is announced
    /// first (`0x48`: a Set speaks), a Set plays the Set effect after it (`0x1E`), and the Field zone's card changes the
    /// field's background (7: its Konami id face up, 0 when it leaves).
    pub(super) fn presented(&self, moved: Vec<Message>, cid: u16, from: Place, to: Place, kind: u32) -> Vec<Message> {
        let mut out = vec![];
        if from.zone == zone::HAND && to.zone <= zone::FIELD_SPELL && matches!(kind, MOVE_NORMAL_SUMMON | MOVE_ACTIVATE | MOVE_SET) {
            // The speech never reads the id; a Set card the client may not see goes without it.
            let shown = if kind == MOVE_SET && self.hidden[to.player as usize] { 0 } else { cid as u32 };
            out.push(command(cmd::SPEAK_PLAY, [from.player, shown, (kind != MOVE_SET) as u32, (to.zone >= 5) as u32]));
        }
        out.extend(moved);
        if kind == MOVE_SET && to.zone <= zone::FIELD_SPELL {
            out.push(command(cmd::CARD_SET, [to.player, to.zone, (to.zone < 5) as u32, 0]));
        }
        if to.zone == zone::FIELD_SPELL && to.face_up {
            out.push(command(cmd::FIELD_CHANGE, [cid as u32, 0, 0, 0]));
        } else if from.zone == zone::FIELD_SPELL && from.face_up && to.zone != zone::FIELD_SPELL {
            out.push(command(cmd::FIELD_CHANGE, [0; 4]));
        }
        out
    }

    /// An activation's presentation before its ChainSet, as Konami's server sent it: the activation line (`0x47`; for a
    /// Spell/Trap step 2 then 3, as from the hand, since a Set card is face up by its MSG_CHAINING; Konami's sent 1 for
    /// a Set one, which the client only keeps as context) and the card's activation effect (`0x22`). Nothing for a card
    /// the client has no place for.
    pub(super) fn activation(&self, c: &Chaining) -> Vec<Message> {
        let at = Loc { location: c.loc.location & !loc::OVERLAY, ..c.loc.clone() };
        let Some(place) = self.engine_place(&at) else { return vec![] };
        let index = if place.zone <= zone::FIELD_SPELL { 0 } else { place.index };
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.get(index)).copied() else { return vec![] };
        let cid = self.cards.get(&card.row).map_or(0, |c| c.cid as u32);
        let mut out = vec![];
        if (5..=zone::FIELD_SPELL).contains(&place.zone) {
            out.push(command(cmd::SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 2, 0]));
            out.push(command(cmd::SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 3, 0]));
        } else {
            out.push(command(cmd::SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 4, 0]));
        }
        let place = Place { index, face_up: true, ..place };
        out.push(command(cmd::CARD_ACTIVATE, [place.packed(), cid, card.uid, 0]));
        out
    }

    /// What the card info panel reads for a monster moving into or out of the client's hand, or into a Graveyard, which
    /// field records don't cover ([`HAND_LEVEL`], [`PILE_STATS`]). The opponent's hand isn't given levels: its cards
    /// are hidden against a player.
    pub(super) fn panel_stats(&self, card: Card, from: Place, to: Place, row: u32) -> Vec<Message> {
        let Some(s) = self.cards.get(&card.row).filter(|s| s.level > 0) else { return vec![] };
        let mut out = vec![];
        if from.zone == zone::HAND && to.zone != zone::HAND && from.player == CLIENT {
            out.push(command(cmd::HAND_LEVEL, [card.uid, NO_LEVEL, 0, 0]));
        }
        if to.zone == zone::HAND && from.zone != zone::HAND && to.player == CLIENT && row != 0 {
            out.push(command(cmd::HAND_LEVEL, [card.uid, s.level, 0, 0]));
        }
        if to.zone == zone::GRAVE && from.zone != zone::GRAVE {
            let id = s.cid as u32;
            out.extend([command(cmd::PILE_STATS, [id, id, s.attribute, s.race]), command(cmd::PILE_STATS_COMMIT, [id, s.level, 0, 0])]);
        }
        out
    }

    /// [`relocate`] with the move's kind (`move_kind`), which the client's move takes in `param1`'s high half.
    pub(super) fn relocate_as(&mut self, cid: u16, from: Place, to: Place, kind: u32) -> Vec<Message> {
        let record = (from.zone <= zone::FIELD_SPELL).then(|| self.records.get(&(from.player, from.zone)).copied()).flatten();
        let cards = &self.cards;
        let source = self.sides[from.player as usize].zone(from.zone);
        let at = if from.zone == zone::DECK || from.zone == zone::EXTRA {
            source.iter().position(|c| cards.get(&c.row).map(|s| s.cid) == Some(cid)).or(if source.is_empty() { None } else { Some(0) })
        } else if source.is_empty() {
            None
        } else {
            // The engine's sequence, unless the mirror holds another card there: a list zone that fell out of step
            // with the engine would move (and show) the wrong card, so the card with the id is taken instead.
            let at = from.index.min(source.len() - 1);
            let id_at = |i: usize| cards.get(&source[i].row).map(|s| s.cid);
            let known = cid != 0 && !self.tokens.contains(&source[at].uid);
            if known && id_at(at) != Some(cid) && let Some(found) = (0..source.len()).find(|&i| id_at(i) == Some(cid)) {
                tracing::warn!("mirror out of step: card {cid} not at {from:?}, found at {found}");
                Some(found)
            } else {
                Some(at)
            }
        };
        let Some(at) = at else { return vec![] };
        // The client takes a hand card out by the move's index (0x5000E0 → 0x5005D0), so the move names the index the
        // mirror (the client's view) found it at, not the engine's: from another index the client drops the wrong card
        // and keeps this one in its hand list, and the next renumbering of that hand crashes on it (0x516185).
        let from = if from.zone == zone::DECK || from.zone == zone::EXTRA { from } else { Place { index: at, ..from } };
        let card = source.remove(at);
        let dest = self.sides[to.player as usize].zone(to.zone);
        let index = if to.zone > zone::FIELD_SPELL { to.index.min(dest.len()) } else { 0 };
        if to.zone <= zone::FIELD_SPELL {
            dest.clear();
        }
        dest.insert(index, card);
        let to = Place { index, ..to };
        // Monsters' face and position, and Spell/Traps' face (a revealed Set card, `reveal_field`).
        if to.zone <= zone::FIELD_SPELL {
            self.positions.insert((to.player, to.zone), (to.face_up, to.defense));
        }
        let row = self.shown_row(&to, card.row);
        let mut out = self.panel_stats(card, from, to, row);
        out.push(Cmd::Card { uid: card.uid, open: open(to.zone), row, place: table_place(to.player, to.zone, index) }.message());
        if to.zone < 5 && row != 0 {
            out.extend(match record {
                Some(values) => self.record(to.player, to.zone, card.row, values),
                None => self.monster_record(to.player, to.zone, card.row),
            });
        }
        // List zones renumber: re-send the shifted cards (not the face-down deck). A pile's go before the move, whose
        // count change redraws the pile from the table's top entry (0x4FE4D0); the hand's after.
        let mut hand = vec![];
        for (p, zone, start) in [(from.player, from.zone, at), (to.player, to.zone, index + 1)] {
            if zone <= zone::FIELD_SPELL || zone == zone::DECK {
                continue;
            }
            let shifted: Vec<(usize, Card)> = self.sides[p as usize].zone(zone).iter().copied().enumerate().skip(start).collect();
            for (i, c) in shifted {
                let row = self.shown_row(&Place { player: p, zone, index: i, face_up: true, defense: false }, c.row);
                let entry = Cmd::Card { uid: c.uid, open: open(zone), row, place: table_place(p, zone, i) }.message();
                if zone == zone::HAND { hand.push(entry) } else { out.push(entry) }
            }
        }
        out.push(Cmd::Move { uid: card.uid, kind, from, to }.message());
        out.extend(hand);
        out
    }

    /// MSG_ATTACK {card, target}: the battle's start and the attack, at the target or directly.
    pub(super) fn attack(&mut self, a: &Attack) -> Vec<Message> {
        let Some(attacker) = self.engine_place(&a.card) else { return vec![] };
        let target = a.target.as_ref().and_then(|t| self.engine_place(t)).map_or(Target::Direct { player: 1 - attacker.player }, |t| Target::Monster { player: t.player, zone: t.zone });
        self.in_battle = true;
        vec![Cmd::BattleInit.message(), Cmd::BattleAttack { player: attacker.player, zone: attacker.zone, target }.message()]
    }

    /// MSG_BATTLE {card, target: {attack, defense, position, destroyed}}: per player, the battle damage they take, by
    /// ATK against ATK, or DEF when the target defends (effects that change it aren't followed), and whether their card
    /// would be destroyed. Limited protection is applied after this message, so destruction is shown only when
    /// MSG_MOVE confirms it. The client pairs the values with the cards by player (DuelBattle states 5 and 6). A direct
    /// attack's target is an empty place (location 0; ocgcore-wasm parses it like any card): no figures, since there
    /// is no target card to show them at (the LP change shows the damage).
    pub(super) fn battle(&self, b: &Battle) -> Vec<Message> {
        let (card, target) = (&b.card, &b.target);
        if target.loc.location == 0 {
            return vec![Cmd::BattleRun { damage: [0; 2], flags: 0 }.message()];
        }
        let (to_attacker, to_target) = if target.loc.position.unwrap_or(0) & (pos::FACEUP_ATTACK | pos::FACEDOWN_ATTACK) != 0 {
            let diff = card.attack as i64 - target.attack as i64;
            (-diff.min(0), diff.max(0))
        } else {
            ((target.defense as i64 - card.attack as i64).max(0), 0)
        };
        let mut damage = [0u32; 2];
        let attacker = self.side(card.loc.controller) as usize;
        damage[attacker] = to_attacker as u32;
        damage[1 - attacker] = to_target as u32;
        vec![Cmd::BattleRun { damage, flags: 0 }.message()]
    }

    /// The end of the battle shown on the client, if one is.
    pub(super) fn end_battle(&mut self) -> Vec<Message> {
        self.attacker = None;
        if std::mem::take(&mut self.in_battle) { vec![Cmd::BattleEnd.message()] } else { vec![] }
    }

    /// A field card turns in place to `position`: 0x1C with its new face and position, after its table entry and record
    /// when it shows a row it didn't before (the opponent's face-down card turning face-up when hiding).
    pub(super) fn turn(&mut self, loc: &Loc, position: u64) -> Vec<Message> {
        let Some(place) = self.engine_place(loc) else { return vec![] };
        if place.zone > zone::FIELD_SPELL {
            return vec![];
        }
        // A Spell/Trap has no battle position.
        let place = Place { face_up: position & 5 != 0, defense: place.zone < 5 && position & 12 != 0, ..place };
        let before = (place.zone < 5).then(|| self.positions.insert((place.player, place.zone), (place.face_up, place.defense)));
        let mut out = vec![];
        let card = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied();
        if place.zone >= 5 {
            // A Set Spell/Trap turning face up is activated: its avatar's reveal line first (Konami's logs: 0x46 before the
            // flip; the activation line after it stays silent, DuelCutin +0x68).
            let was_down = self.positions.insert((place.player, place.zone), (place.face_up, false)).is_some_and(|(f, _)| !f);
            if let Some(card) = card
                && was_down
                && place.face_up
            {
                let cid = self.cards.get(&card.row).map_or(0, |c| c.cid as u32);
                out.push(command(cmd::SPEAK_REVEAL, [place.player, 1, cid, 0]));
            }
        }
        // Face down, the client empties the card's counters itself (0x4FBE50); so does EDOPro.
        if let Some(card) = card
            && !place.face_up
        {
            self.counters.retain(|(uid, _), _| *uid != card.uid);
        }
        if let Some(card) = card
            && self.hidden[place.player as usize]
            && place.face_up
        {
            out.push(Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: table_place(place.player, place.zone, 0) }.message());
            if place.zone < 5 {
                out.extend(self.monster_record(place.player, place.zone, card.row));
            } else {
                // The field card takes its face from a move (a flip only turns it over), and a Spell/Trap has no
                // record to carry its id: it moves onto its own slot with its row, face up.
                let down = Place { face_up: false, ..place };
                out.push(Cmd::Move { uid: card.uid, kind: 0, from: down, to: place }.message());
                return out;
            }
        }
        // A player's own monster turned in place stayed upright on that player's screen (live, the VM round: every
        // 0x1C after its summon), while a card arriving by a move is drawn in its position: it moves onto its own
        // slot in the new position first.
        if let (Some(before), Some(card)) = (before, card) {
            let (face_up, defense) = before.unwrap_or((!place.face_up, !place.defense));
            let from = Place { face_up, defense, ..place };
            out.push(Cmd::Move { uid: card.uid, kind: 0, from, to: place }.message());
        }
        out.push(command(cmd::FLIP_TURN, [place.packed(), 0, 0, 0]));
        out
    }

    /// MSG_ADD_COUNTER / MSG_REMOVE_COUNTER {counter_type, controller, location, sequence, count} on a field card: the
    /// change in the client's kind, a removal no more than the client holds.
    pub(super) fn counter(&mut self, c: &Counter, add: bool) -> Vec<Message> {
        let at = Loc { controller: c.controller, location: c.location, sequence: c.sequence, ..Loc::default() };
        let Some(place) = self.engine_place(&at).filter(|p| p.zone <= zone::FIELD_SPELL) else { return vec![] };
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied() else { return vec![] };
        let kind = COUNTER_KINDS.iter().find(|(e, _)| *e == c.counter_type).map_or(ANY_COUNTER, |(_, k)| *k);
        let count = c.count as u32;
        let held = self.counters.entry((card.uid, kind)).or_default();
        let change = if add { count as i32 } else { -(count.min(*held) as i32) };
        *held = held.saturating_add_signed(change);
        if change == 0 {
            return vec![];
        }
        vec![command(cmd::COUNTER, [place.player | place.zone << 8, change as u32, kind, 0])]
    }

    /// A field card about to leave its place gives up its counters on the client, which would keep them (EDOPro drops
    /// them without a message).
    pub(super) fn drop_counters(&mut self, place: &Place) -> Vec<Message> {
        if place.zone > zone::FIELD_SPELL {
            return vec![];
        }
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied() else { return vec![] };
        let mut held: Vec<(u32, u32)> =
            self.counters.iter().filter(|((uid, _), n)| *uid == card.uid && **n > 0).map(|((_, kind), n)| (*kind, *n)).collect();
        held.sort_unstable();
        self.counters.retain(|(uid, _), _| *uid != card.uid);
        held.into_iter().map(|(kind, n)| command(cmd::COUNTER, [place.player | place.zone << 8, (n as i32).wrapping_neg() as u32, kind, 0])).collect()
    }

    /// The record the card info and the field label show for a monster zone (0x523F00, keyed player | zone << 8):
    /// shown id, second id, race, attribute, level, ATK, DEF, base ATK, base DEF. Data commands 0x3E9 {_, values 0,
    /// 2, 3}, 0x3EA {_, values 4, 5, 6}, 0x3EB {key, values 7, 8, 1} (which commits it; the pop 0x522A50).
    /// Stats from CARD_Prop as the monster enters; the sidecar's field report brings later changes (`field`).
    pub(super) fn monster_record(&mut self, player: u32, zone: u32, row: u16) -> Vec<Message> {
        let s = self.cards.get(&row).copied().unwrap_or_default();
        self.record(player, zone, row, [s.level, s.atk, s.def, s.atk, s.def])
    }

    /// A monster zone's record with these values (level, ATK, DEF, base ATK, base DEF); race and attribute are
    /// CARD_Prop's.
    pub(super) fn record(&mut self, player: u32, zone: u32, row: u16, values: [u32; 5]) -> Vec<Message> {
        self.kinds.remove(&(player, zone));
        self.record_with(player, zone, row, values, None)
    }

    /// [`record`] with a Type and Attribute an effect changed (the client's numbers), kept for the zone.
    pub(super) fn record_with(&mut self, player: u32, zone: u32, row: u16, values: [u32; 5], kinds: Option<(u32, u32)>) -> Vec<Message> {
        self.records.insert((player, zone), values);
        let s = self.cards.get(&row).copied().unwrap_or_default();
        let cid = s.cid as u32;
        let (race, attribute) = kinds.unwrap_or((s.race, s.attribute));
        if let Some(k) = kinds {
            self.kinds.insert((player, zone), k);
        }
        let [level, atk, def, base_atk, base_def] = values;
        vec![
            command(cmd::RECORD_BEGIN, [0, cid, race, attribute]),
            command(cmd::RECORD_MORE, [0, level, atk, def]),
            command(cmd::RECORD_COMMIT, [player | zone << 8, base_atk, base_def, cid]),
        ]
    }

    /// The sidecar's report of the monsters' stats after an engine step ({"type": "field", monsters: [{controller,
    /// sequence, position, level, attack, defense, base_attack, base_defense, race, attribute}]}): a face-up monster
    /// whose values differ from those its record last went with gets the record again, which DuelCardManager picks up
    /// after the next command (0x4FFF14) and shows on its label. A level of 0 (an Xyz monster's) keeps CARD_Prop's.
    /// A Type or Attribute other than CARD_Prop's (DNA Surgery, Zombie World) goes in the record too: the info panel
    /// reads them from it (0x453740, 0x523F00).
    pub(super) fn field(&mut self, f: &Field) -> Vec<Message> {
        let mut out = vec![];
        for s in &f.monsters {
            let stat = |v: Option<i64>| v.unwrap_or(0).max(0) as u32;
            if s.position.unwrap_or(0) & (pos::FACEUP_ATTACK | pos::FACEUP_DEFENSE) == 0 || s.sequence >= 5 {
                continue;
            }
            let (player, zone) = (self.side(s.controller), s.sequence as u32);
            let Some(card) = self.sides[player as usize].zones.get(&zone).and_then(|z| z.first()).copied() else { continue };
            let level = s.level.map(|l| l as u32).filter(|&l| l > 0).unwrap_or(self.cards.get(&card.row).map_or(0, |c| c.level));
            let values = [level, stat(s.attack), stat(s.defense), stat(s.base_attack), stat(s.base_defense)];
            let printed = self.cards.get(&card.row).map_or((0, 0), |c| (c.race, c.attribute));
            let race = client_number(s.race, &TYPES).unwrap_or(printed.0);
            let attribute = client_number(s.attribute, &ATTRIBUTES).unwrap_or(printed.1);
            let kinds = Some((race, attribute)).filter(|&k| k != printed);
            if self.records.get(&(player, zone)) != Some(&values) || self.kinds.get(&(player, zone)).copied() != kinds {
                out.extend(self.record_with(player, zone, card.row, values, kinds));
                if kinds.is_none() {
                    self.kinds.remove(&(player, zone));
                }
            }
        }
        out
    }

    /// The client place of an engine location {controller, location, sequence, position}.
    pub(super) fn engine_place(&self, loc: &Loc) -> Option<Place> {
        // An Xyz material: ocgcore-wasm gives its Xyz Monster's location with an `overlay_sequence` (a material attached
        // in the Extra Deck read as the Extra Deck, one detached as its monster's zone); the client has no place for it.
        if loc.is_material() {
            return None;
        }
        let player = self.side(loc.controller);
        let seq = loc.sequence as usize;
        let (zone, index) = match loc.location {
            loc::DECK => (zone::DECK, 0),
            loc::HAND => (zone::HAND, seq),
            loc::MZONE => (seq as u32, 0),
            loc::SZONE if seq == 5 => (zone::FIELD_SPELL, 0),
            loc::SZONE => (5 + seq as u32, 0),
            loc::GRAVE => (zone::GRAVE, seq),
            loc::REMOVED => (zone::REMOVED, seq),
            loc::EXTRA => (zone::EXTRA, seq),
            _ => return None,
        };
        // ocgcore positions: 1 face-up attack, 2 face-down attack, 4 face-up defense, 8 face-down defense. Only
        // field cards carry them; the client shows each list zone its own way. Only monsters lie sideways: a set
        // Spell/Trap comes face-down defense, which drew it like a set monster (live).
        // The Graveyard is face up: face down, its top card showed only its back (live).
        let position = match zone {
            zone::GRAVE => pos::FACEUP_ATTACK,
            zone::REMOVED => loc.position.unwrap_or(pos::FACEUP_ATTACK),
            zone if zone <= zone::FIELD_SPELL => loc.position.unwrap_or(0),
            _ => 0,
        };
        Some(Place { player, zone, index, face_up: position & 5 != 0, defense: zone < 5 && position & 12 != 0 })
    }

    /// ChainSet for MSG_CHAINING {controller, location, sequence, chain_size}: the link over its card's zone (an Xyz
    /// material's over its monster).
    pub(super) fn chain_link(&self, c: &Chaining) -> Message {
        let at = Loc { location: c.loc.location & !loc::OVERLAY, ..c.loc.clone() };
        let zone = self.engine_place(&at).map_or(FIELD_CENTRE, |p| p.zone);
        command(cmd::CHAIN_SET, [self.side(c.loc.controller), zone, c.chain_size as u32, 0])
    }

}
