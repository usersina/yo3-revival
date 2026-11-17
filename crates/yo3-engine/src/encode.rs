//! An answer to the core's prompt, from the server's JSON (an OcgResponse: `type` = OcgResponseType) to the bytes
//! the core reads, as ocgcore-wasm 0.1.6's `createResponse` writes them.
use serde_json::Value as Json;
use yo3_ocg::resp;

/// JavaScript's `ToInt32`/`ToUint32` of a JSON number: a missing or non-numeric value is 0, others wrap.
fn int(v: &Json) -> i64 {
    v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)).or_else(|| v.as_f64().filter(|f| f.is_finite()).map(|f| f.trunc() as i64)).unwrap_or(0)
}

fn i32s(out: &mut Vec<u8>, v: i64) {
    out.extend_from_slice(&(v as i32).to_le_bytes());
}

fn list(v: &Json) -> Option<&Vec<Json>> {
    v.as_array()
}

/// The bytes for `response`.
pub fn response(r: &Json) -> Vec<u8> {
    let mut out = vec![];
    let null = Json::Null;
    let field = |k: &str| r.get(k).unwrap_or(&null);
    let kind = u64::try_from(int(field("type"))).unwrap_or(u64::MAX);
    match kind {
        // SELECT_BATTLECMD, SELECT_IDLECMD
        resp::BATTLECMD | resp::IDLECMD => i32s(&mut out, int(field("action")) | (int(field("index")) << 16)),
        // SELECT_EFFECTYN, SELECT_YESNO
        resp::EFFECTYN | resp::YESNO => i32s(&mut out, truthy(field("yes")) as i64),
        // SELECT_OPTION, SELECT_POSITION, ANNOUNCE_CARD, ANNOUNCE_NUMBER, ROCK_PAPER_SCISSORS
        resp::OPTION => i32s(&mut out, int(field("index"))),
        resp::POSITION => i32s(&mut out, int(field("position"))),
        resp::CARD_NAME => i32s(&mut out, int(field("card"))),
        resp::NUMBER | resp::RPS => i32s(&mut out, int(field("value"))),
        // SELECT_CARD, SELECT_TRIBUTE, SELECT_SUM; SELECT_CARD_CODES
        resp::CARD | resp::TRIBUTE | resp::SUM | resp::CARD_CODES => match list(field(if kind == resp::CARD_CODES { "codes" } else { "indicies" })) {
            Some(items) => {
                i32s(&mut out, 0);
                i32s(&mut out, items.len() as i64);
                for i in items {
                    i32s(&mut out, int(i));
                }
            }
            None => i32s(&mut out, -1),
        },
        // SELECT_UNSELECT_CARD
        resp::UNSELECT_CARD => {
            if field("index").is_null() && r.get("index").is_some() {
                i32s(&mut out, -1);
            } else {
                i32s(&mut out, 1);
                i32s(&mut out, int(field("index")));
            }
        }
        // SELECT_DISFIELD, SELECT_PLACE
        resp::DISFIELD | resp::PLACE => {
            for place in list(field("places")).into_iter().flatten() {
                for k in ["player", "location", "sequence"] {
                    out.push(int(place.get(k).unwrap_or(&null)) as u8);
                }
            }
        }
        // SELECT_CHAIN
        resp::CHAIN => {
            if field("index").is_null() && r.get("index").is_some() {
                i32s(&mut out, -1);
            } else {
                i32s(&mut out, int(field("index")));
            }
        }
        // SELECT_COUNTER
        resp::COUNTER => {
            for c in list(field("counters")).into_iter().flatten() {
                out.extend_from_slice(&(int(c) as i16).to_le_bytes());
            }
        }
        // SORT_CARD
        // Byte i is card i's rank, no count before them (playerop.cpp SortCard reads returns from byte 0); 0xFF keeps
        // the engine's order.
        resp::SORT => match list(field("order")).filter(|_| truthy(field("order"))) {
            Some(order) => {
                for i in order {
                    out.push(int(i) as u8);
                }
            }
            None => out.push(0xFF),
        },
        // ANNOUNCE_RACE: the races OR'd, 64-bit
        resp::RACE => {
            let race = list(field("races")).into_iter().flatten().fold(0u64, |a, r| a | int(r) as u64);
            out.extend_from_slice(&race.to_le_bytes());
        }
        // ANNOUNCE_ATTRIB
        resp::ATTRIB => {
            let attribute = list(field("attributes")).into_iter().flatten().fold(0u32, |a, r| a | int(r) as u32);
            out.extend_from_slice(&attribute.to_le_bytes());
        }
        _ => {}
    }
    out
}

/// JavaScript truthiness.
fn truthy(v: &Json) -> bool {
    match v {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Json::String(s) => !s.is_empty(),
        Json::Array(_) | Json::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn answers_are_the_bytes_the_wrapper_wrote() {
        assert_eq!(response(&json!({"type": 1, "action": 2, "index": 3})), (2i32 | 3 << 16).to_le_bytes());
        assert_eq!(response(&json!({"type": 5, "indicies": [1, 4]})), [0i32, 2, 1, 4].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>());
        assert_eq!(response(&json!({"type": 5, "indicies": null})), (-1i32).to_le_bytes());
        assert_eq!(response(&json!({"type": 8, "index": null})), (-1i32).to_le_bytes());
        assert_eq!(response(&json!({"type": 7, "index": 2})), [1i32, 2].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>());
        assert_eq!(response(&json!({"type": 10, "places": [{"player": 0, "location": 4, "sequence": 2}]})), [0, 4, 2]);
        assert_eq!(response(&json!({"type": 15, "order": null})), [0xFF]);
        assert_eq!(response(&json!({"type": 15, "order": [2, 0, 1]})), [2, 0, 1]);
        assert_eq!(response(&json!({"type": 16, "races": [1, 4]})), 5u64.to_le_bytes());
        assert_eq!(response(&json!({"type": 13, "counters": [1, 0]})), [1, 0, 0, 0]);
        assert_eq!(response(&json!({"type": 3, "yes": true})), 1i32.to_le_bytes());
    }

    #[test]
    fn every_answer_the_server_builds_is_the_bytes_the_core_reads() {
        // The server and this program build answers as yo3_ocg::Response: each one's JSON, through this encoder, is
        // the core's bytes. (A card name's `cid` is the session's: it turns it into the declared card's `card`.)
        use yo3_ocg::Response;
        let ints = |v: &[i32]| v.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
        let place = json!({ "player": 1, "location": 8, "sequence": 3 });
        let cases: Vec<(Response, Vec<u8>)> = vec![
            (Response::BattleCmd { action: 1, index: Some(2) }, ints(&[1 | 2 << 16])),
            (Response::BattleCmd { action: 3, index: None }, ints(&[3])),
            (Response::IdleCmd { action: 5, index: Some(1) }, ints(&[5 | 1 << 16])),
            (Response::EffectYn(true), ints(&[1])),
            (Response::YesNo(false), ints(&[0])),
            (Response::Option(2), ints(&[2])),
            (Response::Card(Some(vec![0, 3])), ints(&[0, 2, 0, 3])),
            (Response::Card(None), ints(&[-1])),
            (Response::Tribute(Some(vec![1])), ints(&[0, 1, 1])),
            (Response::UnselectCard(Some(4)), ints(&[1, 4])),
            (Response::UnselectCard(None), ints(&[-1])),
            (Response::Chain(Some(0)), ints(&[0])),
            (Response::Chain(None), ints(&[-1])),
            (Response::Disfield(vec![place.clone()]), vec![1, 8, 3]),
            (Response::Place(vec![place]), vec![1, 8, 3]),
            (Response::Position(4), ints(&[4])),
            (Response::Counter(json!([2, 1])), vec![2, 0, 1, 0]),
            (Response::Sum(vec![0, 2]), ints(&[0, 2, 0, 2])),
            (Response::Sort(None), vec![0xFF]),
            (Response::Sort(Some(json!([1, 0]))), vec![1, 0]),
            (Response::Race(vec![0x10, 0x2]), 0x12u64.to_le_bytes().to_vec()),
            (Response::Attrib(vec![0x4]), 4u32.to_le_bytes().to_vec()),
            (Response::Number(3), ints(&[3])),
            (Response::Rps(2), ints(&[2])),
        ];
        for (answer, bytes) in cases {
            assert_eq!(response(&answer.json()), bytes, "{answer:?}");
        }
    }
}
