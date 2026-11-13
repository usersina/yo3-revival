//! An answer to the core's prompt, from the server's JSON (an OcgResponse: `type` = OcgResponseType) to the bytes
//! the core reads, as ocgcore-wasm 0.1.6's `createResponse` writes them.
use serde_json::Value as Json;

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
    match int(field("type")) {
        // SELECT_BATTLECMD, SELECT_IDLECMD
        0 | 1 => i32s(&mut out, int(field("action")) | (int(field("index")) << 16)),
        // SELECT_EFFECTYN, SELECT_YESNO
        2 | 3 => i32s(&mut out, truthy(field("yes")) as i64),
        // SELECT_OPTION, SELECT_POSITION, ANNOUNCE_CARD, ANNOUNCE_NUMBER, ROCK_PAPER_SCISSORS
        4 => i32s(&mut out, int(field("index"))),
        11 => i32s(&mut out, int(field("position"))),
        18 => i32s(&mut out, int(field("card"))),
        19 | 20 => i32s(&mut out, int(field("value"))),
        // SELECT_CARD, SELECT_TRIBUTE, SELECT_SUM; SELECT_CARD_CODES
        5 | 12 | 14 | 6 => match list(field(if int(field("type")) == 6 { "codes" } else { "indicies" })) {
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
        7 => {
            if field("index").is_null() && r.get("index").is_some() {
                i32s(&mut out, -1);
            } else {
                i32s(&mut out, 1);
                i32s(&mut out, int(field("index")));
            }
        }
        // SELECT_DISFIELD, SELECT_PLACE
        9 | 10 => {
            for place in list(field("places")).into_iter().flatten() {
                for k in ["player", "location", "sequence"] {
                    out.push(int(place.get(k).unwrap_or(&null)) as u8);
                }
            }
        }
        // SELECT_CHAIN
        8 => {
            if field("index").is_null() && r.get("index").is_some() {
                i32s(&mut out, -1);
            } else {
                i32s(&mut out, int(field("index")));
            }
        }
        // SELECT_COUNTER
        13 => {
            for c in list(field("counters")).into_iter().flatten() {
                out.extend_from_slice(&(int(c) as i16).to_le_bytes());
            }
        }
        // SORT_CARD
        15 => match list(field("order")).filter(|_| truthy(field("order"))) {
            Some(order) => {
                out.push(order.len() as u8);
                for i in order {
                    out.push(int(i) as u8);
                }
            }
            None => out.push(0xFF),
        },
        // ANNOUNCE_RACE: the races OR'd, 64-bit
        16 => {
            let race = list(field("races")).into_iter().flatten().fold(0u64, |a, r| a | int(r) as u64);
            out.extend_from_slice(&race.to_le_bytes());
        }
        // ANNOUNCE_ATTRIB
        17 => {
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
        assert_eq!(response(&json!({"type": 16, "races": [1, 4]})), 5u64.to_le_bytes());
        assert_eq!(response(&json!({"type": 13, "counters": [1, 0]})), [1, 0, 0, 0]);
        assert_eq!(response(&json!({"type": 3, "yes": true})), 1i32.to_le_bytes());
    }
}
