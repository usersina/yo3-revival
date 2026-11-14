//! Duel clocks between players. The duel connection's timers message (duelRequestName 0x14, pump case 0x52396A) sets
//! the engine's five clock slots (0x520A00): count1 / count2 are the turn clocks of the client (PlayerNum 0) and its
//! opponent, total1 / total2 their total clocks, each with a _max; timerKind 0 runs (start 1) or stops the client's
//! pair, 1 the opponent's. DuelCockpit (0x502660) counts a running pair down by wall-clock seconds and shows it.
use std::time::{Duration, Instant};

use yo3_proto::{Message, Value};
use yo3_server::clock::Clocks;

fn fields(m: &Message) -> [u32; 12] {
    let keys = ["timerKind", "start", "count1", "total1", "count2", "total2", "other", "count1_max", "total1_max",
        "count2_max", "total2_max", "other_max"];
    keys.map(|k| m.get(k).and_then(Value::as_u32).unwrap_or_else(|| panic!("{k}")))
}

#[test]
fn the_waited_on_players_clocks_run_and_each_client_sees_its_own_first() {
    let t0 = Instant::now();
    let s = Duration::from_secs;
    let mut c = Clocks::new(180, 600).expect("clocks");
    c.wait_on(Some(0), t0);
    let m = c.message(0, t0 + s(10));
    assert_eq!(m.get("duelRequestName"), Some(&Value::U32(0x14)));
    assert_eq!(fields(&m), [0, 1, 170, 590, 180, 600, 0, 180, 600, 180, 600, 0]);
    // Seat 1's client: its own clocks first, the opponent's (running) second.
    assert_eq!(fields(&c.message(1, t0 + s(10))), [1, 1, 180, 600, 170, 590, 0, 180, 600, 180, 600, 0]);
    // The engine now waits on seat 1: seat 0's time is charged, seat 1's runs.
    c.wait_on(Some(1), t0 + s(10));
    assert_eq!(fields(&c.message(0, t0 + s(25)))[..6], [1, 1, 170, 590, 165, 585]);
    // Seat 0's next turn fills its turn clock again; its total stays.
    c.new_turn(0, t0 + s(25));
    // Waiting on nobody stops both.
    c.wait_on(None, t0 + s(25));
    assert_eq!(fields(&c.message(0, t0 + s(99)))[..6], [0, 0, 180, 590, 165, 585]);
}

#[test]
fn whoever_runs_out_of_turn_or_total_time_is_out() {
    // 5 s after 0: the client starts counting only once it takes the timers message off its queue.
    let t0 = Instant::now();
    let s = Duration::from_secs;
    let mut c = Clocks::new(180, 600).unwrap();
    c.wait_on(Some(1), t0);
    assert_eq!(c.expired(t0 + s(184)), None);
    assert_eq!(c.expired(t0 + s(185)), Some(1), "the turn clock");
    // A total clock: 600 s over several turns.
    let mut c = Clocks::new(180, 600).unwrap();
    let mut t = t0;
    for _ in 0..3 {
        c.new_turn(0, t);
        c.wait_on(Some(0), t);
        t += s(170);
        c.wait_on(None, t);
    }
    c.new_turn(0, t);
    c.wait_on(Some(0), t);
    assert_eq!(c.expired(t + s(94)), None);
    assert_eq!(c.expired(t + s(95)), Some(0), "510 s used before");
    // Nobody waited on: nobody runs out.
    c.wait_on(None, t + s(10));
    assert_eq!(c.expired(t + s(1000)), None);
}

#[test]
fn a_limit_of_0_turns_that_clock_off() {
    // Create Duel's time_turn and time_total (0: off); with both off there are no clocks.
    assert!(Clocks::new(0, 0).is_none());
    let t0 = Instant::now();
    let mut c = Clocks::new(0, 600).unwrap();
    c.wait_on(Some(0), t0);
    assert_eq!(c.expired(t0 + Duration::from_secs(604)), None);
    assert_eq!(fields(&c.message(0, t0))[..4], [0, 1, 0, 600]);
}

#[test]
fn the_time_the_client_spends_on_animations_is_not_charged() {
    // The client never says when it has played a batch's animations (no duel request reports it, and DuelCockpit
    // counts a running pair down whatever the scene does), so the waited-on player's clocks start after an allowance
    // for the batch it was sent (**guess**: a duration per animated command, capped). Until then they show stopped
    // (start 0); once the allowance has passed `starting` says so once, for the clients to be told they run.
    let t0 = Instant::now();
    let s = Duration::from_secs;
    let mut c = Clocks::new(180, 600).expect("clocks");
    c.wait_after(Some(0), t0, s(4));
    assert_eq!(fields(&c.message(0, t0))[..4], [0, 0, 180, 600], "stopped during the animations");
    assert!(!c.starting(t0 + s(3)));
    assert_eq!(c.message(0, t0 + s(3)).get("count1"), Some(&Value::U32(180)), "nothing charged yet");
    assert!(c.starting(t0 + s(4)), "the allowance has passed: tell the clients");
    assert!(!c.starting(t0 + s(5)), "once");
    assert_eq!(fields(&c.message(0, t0 + s(14)))[..4], [0, 1, 170, 590]);
    // A batch's allowance: the animated commands' durations, at most the cap.
    let command = |id: u32| Message::reply(0x200D).with("duelRequestName", Value::U32(17)).with("id", Value::U32(id));
    let batch = [command(1000), command(0x1A), command(0x1A), command(5)];
    assert_eq!(yo3_server::clock::allowance(&batch), Duration::from_millis(400 + 400 + 1200));
    assert_eq!(yo3_server::clock::allowance(&vec![command(6); 100]), s(15));
}

#[test]
fn the_client_says_when_it_has_played_the_batch() {
    // Issue #11: each wait ends the waited-on client's batch with a settle marker (engine command 0x5F1 {seq}); the
    // connection DLL answers duelRequestName 0x7A13 {seq} once the scene has settled after it, and the clocks count from
    // that answer: sooner than the allowance, or later (a slow machine), but not past the cap (30 s after the batch),
    // so a client that never answers keeps the allowance and can't hold its own clock.
    let t0 = Instant::now();
    let s = Duration::from_secs;
    let mut c = Clocks::new(180, 600).expect("clocks");
    c.wait_after(Some(0), t0, s(4));
    let (seat, seq) = c.awaiting().expect("a marker for the seat waited on");
    assert_eq!(seat, 0);
    assert!(!c.settled(1, seq, t0 + s(1)), "only the waited-on client's answer counts");
    assert!(!c.settled(0, seq + 1, t0 + s(1)), "and only for its own batch");
    assert!(c.settled(0, seq, t0 + s(1)), "played after 1 s: the clocks run from here");
    assert!(!c.settled(0, seq, t0 + s(2)), "once");
    assert!(!c.starting(t0 + s(4)), "the clients were told already");
    assert_eq!(fields(&c.message(0, t0 + s(11)))[..4], [0, 1, 170, 590]);
    // A slow client: the allowance started the clocks at 4 s, its answer at 9 s gives those 5 s back.
    c.wait_after(Some(1), t0 + s(20), s(4));
    let (_, seq) = c.awaiting().unwrap();
    assert!(c.starting(t0 + s(24)));
    assert!(c.settled(1, seq, t0 + s(29)));
    assert_eq!(fields(&c.message(1, t0 + s(39)))[2..4], [170, 590]);
    // One that answers after the cap keeps the allowance.
    c.wait_after(Some(0), t0 + s(100), s(2));
    let (_, seq) = c.awaiting().unwrap();
    assert!(!c.settled(0, seq, t0 + s(131)), "30 s after the batch it no longer counts");
    assert_eq!(fields(&c.message(0, t0 + s(112)))[2], 151, "charged from the allowance's end: 161 left, less 10");
    // Nobody waited on: no marker.
    c.wait_after(None, t0 + s(120), s(0));
    assert_eq!(c.awaiting(), None);
    assert_eq!(yo3_server::clock::settle_marker(7).get("id"), Some(&Value::U32(0x5F1)));
    // The marker goes after the batch's animations and before its prompt: a dialog or the input wait holds the
    // client's queue until the player answers, and the answer must come once the animations have played.
    let command = |id: u32| Message::reply(0x200D).with("duelRequestName", Value::U32(17)).with("id", Value::U32(id));
    let map = Message::reply(0x200D).with("duelRequestName", Value::U32(0x12));
    let mut batch = vec![command(0x1A), command(5), map, command(4)];
    yo3_server::clock::mark_settle(&mut batch, 9);
    let ids: Vec<u32> = batch.iter().map(|m| m.get("id").and_then(Value::as_u32).unwrap_or(0)).collect();
    assert_eq!(ids, [0x1A, 5, 0x5F1, 0, 4]);
    let mut dialog = vec![command(0x1A), command(0x3EC), command(0x3ED), command(0x36)];
    yo3_server::clock::mark_settle(&mut dialog, 9);
    assert_eq!(dialog[1].get("id"), Some(&Value::U32(0x5F1)));
}
