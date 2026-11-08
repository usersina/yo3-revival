//! What the monkey sends: plans (a list of steps against one port) made from a seed, the corpus of real requests and
//! mutations. A plan is also a text file, so a failing one can be replayed and cut down without the seed.
use yo3_proto::{MAX_FRAME, Message, Value};

use crate::wire::{Corpus, Rng, Target, hex, unhex, version_plain};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// A plain body, framed and encrypted on the way out.
    Plain(Vec<u8>),
    /// Bytes exactly as they go on the wire (a bad length, half a frame).
    Wire(Vec<u8>),
    /// Wire bytes in tiny pieces with pauses.
    Drip(Vec<u8>),
    Sleep(u64),
    /// Close the connection and open another to the same port.
    Reconnect,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub seed: u64,
    /// The plan's client is 127.0.0.`from` (0: whatever the OS picks, 127.0.0.1).
    pub from: u8,
    pub target: Target,
    pub steps: Vec<Step>,
}

impl Plan {
    pub fn to_text(&self) -> String {
        let mut s = format!("# yo3-monkey plan, seed {}\ntarget {}\nfrom {}\n", self.seed, self.target.name(), self.from);
        for step in &self.steps {
            match step {
                Step::Plain(b) => s += &format!("plain {}\n", hex(b)),
                Step::Wire(b) => s += &format!("wire {}\n", hex(b)),
                Step::Drip(b) => s += &format!("drip {}\n", hex(b)),
                Step::Sleep(ms) => s += &format!("sleep {ms}\n"),
                Step::Reconnect => s += "reconnect\n",
            }
        }
        s
    }

    pub fn from_text(text: &str) -> Option<Plan> {
        let mut plan = Plan { seed: 0, from: 0, target: Target::Front, steps: vec![] };
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("# yo3-monkey plan, seed ") {
                plan.seed = rest.trim().parse().unwrap_or(0);
            }
            let (word, arg) = line.split_once(' ').unwrap_or((line, ""));
            match word {
                "target" => plan.target = Target::parse(arg.trim())?,
                "from" => plan.from = arg.trim().parse().ok()?,
                "plain" => plan.steps.push(Step::Plain(unhex(arg)?)),
                "wire" => plan.steps.push(Step::Wire(unhex(arg)?)),
                "drip" => plan.steps.push(Step::Drip(unhex(arg)?)),
                "sleep" => plan.steps.push(Step::Sleep(arg.trim().parse().ok()?)),
                "reconnect" => plan.steps.push(Step::Reconnect),
                _ => {}
            }
        }
        Some(plan)
    }

    /// Messages sent in all (plain steps).
    pub fn messages(&self) -> usize {
        self.steps.iter().filter(|s| matches!(s, Step::Plain(_) | Step::Wire(_) | Step::Drip(_))).count()
    }
}

/// A recording of the real login on the front: what every session starts with.
fn login(pid: u32) -> Vec<Message> {
    use crate::wire::{req, req_with};
    vec![req(0x1F47), req_with(0x1F48, &[("lang", 1)]), req(0x1F49), req_with(0x1F4C, &[("player_id", pid)]), req_with(0x1FA4, &[("player_id", pid)])]
}

const INTERESTING: [u32; 14] = [0, 1, 2, 3, 0xFF, 0x100, 0x7FFF, 0xFFFF, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF, 1001];

pub fn random_u32(rng: &mut Rng) -> u32 {
    match rng.below(4) {
        0 => *rng.pick(&INTERESTING),
        1 => rng.range(0, 64),
        2 => rng.range(900, 1100),
        _ => rng.next() as u32,
    }
}

fn random_text(rng: &mut Rng) -> Vec<u8> {
    // Mostly short; the huge ones are rare, because a message is built whole before it is cut to a frame.
    let len = match rng.below(100) {
        0..=2 if rng.1 => 300_000,
        3..=8 if rng.1 => 70_000,
        9..=20 if rng.1 => 4096,
        _ => *rng.pick(&[0usize, 1, 7, 8, 31, 32, 255, 256, 1000]),
    };
    match rng.below(7) {
        0 => vec![b'A'; len],
        1 => (0..len).map(|_| b'a' + rng.below(26) as u8).collect(),
        2 => rng.bytes(len),
        3 => "あいうえお🂡é".as_bytes().iter().copied().cycle().take(len).collect(),
        4 => b"%s%n%x../../etc/passwd'; DROP TABLE players;--\0hidden".iter().copied().cycle().take(len.max(12)).collect(),
        5 => {
            let mut v = vec![b'x'; len];
            if len > 2 {
                v[len / 2] = 0;
            }
            v
        }
        _ => format!("{}", random_u32(rng)).into_bytes(),
    }
}

pub fn random_value(rng: &mut Rng) -> Value {
    match rng.below(10) {
        0..=3 => Value::U32(random_u32(rng)),
        4 => Value::U32Alt(random_u32(rng)),
        5 | 6 => Value::Text(random_text(rng)),
        7 => {
            let n = if rng.1 && rng.chance(0.1) { 100_000 } else { *rng.pick(&[0usize, 1, 3, 40, 248, 1000, 20_000]) };
            Value::Bin(rng.bytes(n))
        }
        8 => Value::TextAlt(random_text(rng)),
        _ => Value::Bin(random_text(rng)),
    }
}

fn random_name(rng: &mut Rng, c: &Corpus) -> String {
    match rng.below(10) {
        0 => String::new(),
        1 => "x".repeat(255),
        2 => format!("f{}", rng.below(1000)),
        _ if c.names.is_empty() => "player_id".into(),
        _ => rng.pick(&c.names).clone(),
    }
}

/// Create Player and Delete Player (0x1F4A, 0x1F4B) change the account the other monkeys play as: a deleted Kaiba
/// leaves the canary and the duel pairs with nobody to log in as. They are left out of what the monkey sends.
pub const CHANGES_THE_ACCOUNT: [u32; 2] = [0x1F4A, 0x1F4B];

fn random_id(rng: &mut Rng, c: &Corpus) -> u32 {
    let id = random_id_any(rng, c);
    if CHANGES_THE_ACCOUNT.contains(&id) { 0x1F49 } else { id }
}

fn random_id_any(rng: &mut Rng, c: &Corpus) -> u32 {
    match rng.below(10) {
        0..=4 if !c.ids.is_empty() => *rng.pick(&c.ids),
        5 | 6 if !c.ids.is_empty() => (*rng.pick(&c.ids)).wrapping_add(rng.range(0, 32)).wrapping_sub(16),
        7 => rng.range(0x1F40, 0x3500),
        _ => random_u32(rng),
    }
}

/// A message built from nothing: a random request id, a few fields, and maybe records.
pub fn random_message(rng: &mut Rng, c: &Corpus, depth: usize) -> Message {
    let mut m = Message::reply(random_id(rng, c));
    if depth > 0 {
        m.tag = *rng.pick(&[3u8, 4, 6, 7, 0, 255]);
        m.fields.clear();
    }
    for _ in 0..rng.below(8) {
        m.fields.push((random_name(rng, c), random_value(rng)));
    }
    if depth < 3 {
        let n = if rng.chance(0.1) { 40 } else { 4 };
        for _ in 0..rng.below(n) {
            m.items.push(random_message(rng, c, depth + 1));
        }
    }
    m
}

fn nest(rng: &mut Rng, depth: usize, c: &Corpus) -> Message {
    let mut inner = random_message(rng, c, 1);
    for _ in 0..depth {
        let mut outer = Message::record();
        outer.items.push(inner);
        inner = outer;
    }
    inner
}

/// Changes a message the way a broken or hostile client might.
pub fn mutate(rng: &mut Rng, m: &mut Message, c: &Corpus) {
    for _ in 0..1 + rng.below(3) {
        match rng.below(14) {
            0 | 1 if !m.fields.is_empty() => {
                let i = rng.below(m.fields.len());
                m.fields[i].1 = random_value(rng);
            }
            2 if !m.fields.is_empty() => {
                // The same value with another type: a number sent as text, text as a number.
                let i = rng.below(m.fields.len());
                m.fields[i].1 = match m.fields[i].1.clone() {
                    Value::U32(n) | Value::U32Alt(n) => Value::text(&n.to_string()),
                    Value::Text(b) | Value::TextAlt(b) | Value::Bin(b) => Value::U32(b.len() as u32),
                };
            }
            3 if !m.fields.is_empty() => {
                m.fields.remove(rng.below(m.fields.len()));
            }
            4 => m.fields.push((random_name(rng, c), random_value(rng))),
            5 if !m.fields.is_empty() => {
                let f = m.fields[rng.below(m.fields.len())].clone();
                m.fields.push(f);
            }
            6 => {
                let id = random_id(rng, c);
                match m.fields.iter_mut().find(|(k, _)| k == "request") {
                    Some((_, v)) => *v = Value::U32(id),
                    None => m.fields.insert(0, ("request".into(), Value::U32(id))),
                }
            }
            7 => m.tag = *rng.pick(&[0u8, 3, 4, 5, 6, 7, 255]),
            8 => {
                for _ in 0..rng.below(30) {
                    m.items.push(random_message(rng, c, 1));
                }
            }
            9 if !m.items.is_empty() => {
                m.items.remove(rng.below(m.items.len()));
            }
            10 => {
                let d = rng.below(14);
                m.items.push(nest(rng, d, c))
            }
            11 if !m.fields.is_empty() => {
                let i = rng.below(m.fields.len());
                m.fields[i].1 = Value::U32(*rng.pick(&INTERESTING));
            }
            12 => {
                for _ in 0..rng.range(100, 2000) {
                    m.fields.push((random_name(rng, c), Value::U32(rng.next() as u32)));
                }
            }
            13 if !m.items.is_empty() => {
                let i = rng.below(m.items.len());
                mutate(rng, &mut m.items[i], c);
            }
            _ => m.fields.push((random_name(rng, c), random_value(rng))),
        }
    }
}

fn encoded(m: &Message) -> Vec<u8> {
    let mut plain = m.encode();
    plain.truncate(MAX_FRAME - 64);
    plain
}

/// Bytes on the wire that are not a well-formed message.
fn garbage(rng: &mut Rng, c: &Corpus, real: &[Message]) -> Vec<Step> {
    let body = |rng: &mut Rng| -> Vec<u8> {
        if real.is_empty() { random_message(rng, c, 0).encode() } else { real[rng.below(real.len())].encode() }
    };
    match rng.below(14) {
        // A header and then nothing, or a lying length.
        0 => vec![Step::Wire((rng.next() as u32).to_be_bytes().to_vec())],
        1 => vec![Step::Wire((0x7777_7777u32).to_be_bytes().to_vec())], // length 0
        2 => vec![Step::Wire((0x7777_7777u32 ^ (MAX_FRAME as u32 + 8)).to_be_bytes().to_vec()), Step::Wire(rng.bytes(64))],
        3 => {
            let n = rng.range(1, 15);
            let mut w = (n ^ 0x7777_7777).to_be_bytes().to_vec();
            w.extend(rng.bytes(n as usize));
            vec![Step::Wire(w)] // not a whole number of blocks, or a few random ones
        }
        4 => {
            // Random ciphertext of a legal length: it decrypts to junk.
            let n = 8 * rng.range(1, 64);
            let mut w = (n ^ 0x7777_7777).to_be_bytes().to_vec();
            w.extend(rng.bytes(n as usize));
            vec![Step::Wire(w)]
        }
        // A real message cut short, or with bits flipped, or with junk inserted.
        5 => {
            let mut p = body(rng);
            p.truncate(rng.below(p.len().max(1)));
            vec![Step::Plain(p)]
        }
        6 => {
            let mut p = body(rng);
            for _ in 0..1 + rng.below(6) {
                if !p.is_empty() {
                    let i = rng.below(p.len());
                    p[i] ^= 1 << rng.below(8);
                }
            }
            vec![Step::Plain(p)]
        }
        7 => {
            let mut p = body(rng);
            let at = rng.below(p.len() + 1);
            for (i, b) in rng.blob(1, 40).into_iter().enumerate() {
                p.insert(at + i, b);
            }
            vec![Step::Plain(p)]
        }
        8 => vec![Step::Plain(rng.blob(1, 600))],
        // Nesting far deeper than a client does.
        9 => {
            let depth = *rng.pick(&[15usize, 16, 17, 64, 1000, 20_000]);
            let mut p = vec![];
            for _ in 0..depth {
                p.extend([3u8, 0, 0, 0, 1]);
            }
            vec![Step::Plain(p)]
        }
        // Lengths inside the message that claim more than there is.
        10 => {
            let mut p = vec![4u8, 0, 1, 7];
            p.extend(b"request");
            p.push(*rng.pick(&[5u8, 7, 8, 9, 10]));
            p.extend(rng.blob(0, 7));
            p.extend([0xFF, 0xFF, 0xFF, 0xFF]);
            vec![Step::Plain(p)]
        }
        11 => {
            let mut p = vec![4u8, 0xFF, 0xFF];
            p.extend(rng.blob(0, 299));
            vec![Step::Plain(p)]
        }
        // A real frame in pieces: dripped, split across two writes, or two in one write.
        12 => {
            let b = body(rng);
            let mut w = crate::wire::framed(&b);
            match rng.below(3) {
                0 => vec![Step::Drip(w)],
                1 => {
                    let cut = rng.below(w.len());
                    let rest = w.split_off(cut);
                    vec![Step::Wire(w), Step::Sleep(rng.range(1, 300) as u64), Step::Wire(rest)]
                }
                _ => {
                    let again = crate::wire::framed(&body(rng));
                    w.extend(again);
                    vec![Step::Wire(w)]
                }
            }
        }
        // A frame that stops in the middle, then the connection is dropped.
        _ => {
            let mut w = crate::wire::framed(&body(rng));
            w.truncate(rng.range(1, w.len().max(2) as u32 - 1) as usize);
            vec![Step::Wire(w), Step::Sleep(rng.range(0, 200) as u64), Step::Reconnect]
        }
    }
}

/// The base of a plan's messages: real recorded requests in order, shuffled, or invented.
#[derive(Clone, Copy, Debug)]
enum Source {
    Ordered,
    Shuffled,
    Random,
}

/// Makes the plan of a seed. The target and the mix are chosen from the seed too.
pub fn plan(seed: u64, c: &Corpus, only: Option<Target>) -> Plan {
    let mut rng = Rng::new(seed);
    rng.1 = rng.chance(0.04);
    let target = only.unwrap_or_else(|| match rng.below(20) {
        0 => Target::Gate,
        1..=4 => Target::Duel,
        _ => Target::Front,
    });
    let mut steps = vec![];
    if target != Target::Http {
        steps.push(Step::Plain(version_plain()));
    }
    let sessions = c.of(target);
    let source = if sessions.is_empty() {
        Source::Random
    } else {
        *rng.pick(&[Source::Ordered, Source::Ordered, Source::Shuffled, Source::Random])
    };
    // How hostile this plan is.
    let mutate_p = *rng.pick(&[0.0, 0.0, 0.1, 0.3, 0.8]);
    let dup_p = *rng.pick(&[0.0, 0.0, 0.05, 0.2]);
    let garbage_p = *rng.pick(&[0.0, 0.0, 0.0, 0.05, 0.3, 1.0]);
    let logged_in = target == Target::Front && rng.chance(0.75);
    // Another account's client now and then (its players' ids are in the corpus); mostly its own players.
    let accounts = if c.accounts.is_empty() { vec![[1001, 1002]] } else { c.accounts.clone() };
    let from = if rng.chance(0.4) { 1 + rng.below(accounts.len()) as u8 } else { 1 };
    let mine = accounts[from as usize - 1];
    let pid = match rng.below(12) {
        0 => *rng.pick(&[999u32, 0, 0xFFFF_FFFF]),
        1 | 2 => *rng.pick(&accounts.concat()),
        _ => *rng.pick(&mine),
    };
    let mut msgs: Vec<Message> = vec![];
    let real_only = |m: &Message| !m.request().is_some_and(|id| CHANGES_THE_ACCOUNT.contains(&id));
    if logged_in {
        msgs.extend(login(pid));
    }
    let want = rng.range(3, 60) as usize;
    match source {
        Source::Random => {
            for _ in 0..want {
                msgs.push(random_message(&mut rng, c, 0));
            }
        }
        Source::Ordered | Source::Shuffled => {
            let s = rng.pick(&sessions);
            let start = rng.below(s.msgs.len().saturating_sub(want).max(1));
            let mut window: Vec<Message> = s.msgs.iter().skip(start).take(want).cloned().collect();
            if matches!(source, Source::Shuffled) {
                for i in (1..window.len()).rev() {
                    window.swap(i, rng.below(i + 1));
                }
            }
            msgs.extend(window.into_iter().filter(real_only));
        }
    }
    // Duplicates and out-of-order pairs.
    let mut out = vec![];
    for m in msgs {
        if rng.chance(dup_p) {
            out.push(m.clone());
            if rng.chance(0.5) {
                out.push(m.clone());
            }
        }
        out.push(m);
    }
    if rng.chance(dup_p) && out.len() > 2 {
        let i = rng.below(out.len() - 1);
        out.swap(i, i + 1);
    }
    let real: Vec<Message> = out.clone();
    for mut m in out {
        if rng.chance(garbage_p) {
            steps.extend(garbage(&mut rng, c, &real));
        }
        if rng.chance(mutate_p) {
            mutate(&mut rng, &mut m, c);
        }
        steps.push(Step::Plain(encoded(&m)));
        if rng.chance(0.02) {
            steps.push(Step::Sleep(rng.range(1, 30) as u64));
        }
        // Dropping the connection in the middle, then (maybe) carrying on as the same player on a new one.
        if rng.chance(0.01) {
            steps.push(Step::Reconnect);
            if rng.chance(0.7) {
                steps.push(Step::Plain(version_plain()));
                if logged_in {
                    steps.extend(login(pid).iter().map(|m| Step::Plain(m.encode())));
                }
            }
        }
    }
    if target == Target::Http {
        steps = http_steps(&mut rng);
    }
    if rng.chance(0.1) {
        steps.push(Step::Reconnect);
    }
    // Plans stay small enough to keep a few hundred of them (the recent ring) and to replay quickly.
    let mut bytes = 0usize;
    steps.retain(|s| {
        bytes += match s {
            Step::Plain(b) | Step::Wire(b) | Step::Drip(b) => b.len(),
            _ => 0,
        };
        bytes < 4 << 20
    });
    Plan { seed, from, target, steps }
}

fn http_steps(rng: &mut Rng) -> Vec<Step> {
    let paths = [
        "/yo3.idx".to_string(),
        "/app/x".into(),
        "/duellogs/1.html".into(),
        "/duellogs/../../etc/passwd".into(),
        "/duellogs/..%2f..%2f..%2fetc%2fpasswd".into(),
        "/duellogs/".into(),
        "/duellogs/%00".into(),
        format!("/{}", "a".repeat(100_000)),
        "/duellogs/\u{1F602}".into(),
        "//yo3.idx".into(),
        "/".into(),
        String::new(),
    ];
    let mut steps = vec![];
    for _ in 0..rng.range(1, 6) {
        let path = rng.pick(&paths).clone();
        let method = *rng.pick(&["GET", "GET", "HEAD", "POST", "BREW", "", "G\0T"]);
        let mut req = format!("{method} {path} HTTP/1.1\r\nHost: x\r\n").into_bytes();
        if rng.chance(0.3) {
            req.extend(format!("Content-Length: {}\r\n", random_u32(rng)).bytes());
        }
        if rng.chance(0.2) {
            req.extend(format!("X: {}\r\n", "h".repeat(rng.range(1, 20_000) as usize)).bytes());
        }
        match rng.below(6) {
            0 => req.truncate(rng.below(req.len().max(1))), // never finished
            1 => req = rng.blob(1, 3000),
            _ => req.extend(b"\r\n"),
        }
        steps.push(if rng.chance(0.2) { Step::Drip(req) } else { Step::Wire(req) });
        steps.push(Step::Sleep(rng.range(0, 40) as u64));
        steps.push(Step::Reconnect);
    }
    steps
}

/// What identifies a trouble line (a panic's location, an ERROR's text) when looking for it again.
pub fn signature_of(line: &str) -> String {
    match line.split("panicked at ").nth(1) {
        Some(rest) => rest.split_whitespace().next().unwrap_or(rest).to_string(),
        None => line.split(" ERROR ").nth(1).unwrap_or(line).chars().take(40).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_makes_the_same_plan_and_a_plan_survives_its_text_file() {
        let c = Corpus::default();
        for seed in 0..10 {
            let a = plan(seed, &c, None);
            let b = plan(seed, &c, None);
            assert_eq!(a.steps, b.steps, "seed {seed} is not repeatable");
            let back = Plan::from_text(&a.to_text()).expect("a plan file reads back");
            assert_eq!((back.from, back.target, back.steps), (a.from, a.target, a.steps));
        }
    }

    #[test]
    fn mutated_messages_stay_inside_a_frame() {
        let c = Corpus::default();
        let mut rng = Rng::new(7);
        for _ in 0..100 {
            let mut m = random_message(&mut rng, &c, 0);
            mutate(&mut rng, &mut m, &c);
            assert!(encoded(&m).len() < MAX_FRAME);
        }
    }
}
