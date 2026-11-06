//! Codec behaviour pinned against vectors produced by the original Python implementation,
//! which the real client accepts.
use proptest::prelude::*;
use yo3_proto::{Error, Message, Packet, Value, decrypt, encrypt, frame, frame_len};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

fn login_reply() -> Message {
    Message::reply(0x2201).with("regulation_id", Value::U32(1)).item(
        Message::record()
            .with("user_id", Value::U32(1001))
            .with("uname", Value::text("test"))
            .with("dpoint", Value::U32(0)),
    )
}

const LOGIN_REPLY_HEX: &str = "040002077265717565737405000022010d726567756c6174696f6e5f69640500000001000103000307757365725f696405000003e905756e616d650700000004746573740664706f696e7405000000000000";

#[test]
fn encrypt_matches_reference() {
    let plain: Vec<u8> = (0..16).collect();
    assert_eq!(encrypt(&plain), hex("c150ff4b609d1847faf3227f7c1190bb"));
}

#[test]
fn encrypt_zero_pads_to_the_block_size() {
    let plain: Vec<u8> = (0..11).collect();
    assert_eq!(encrypt(&plain), hex("c150ff4b609d184709f7061fa30b1ec5"));
}

#[test]
fn decrypt_inverts_encrypt_up_to_padding() {
    let plain: Vec<u8> = (0..11).collect();
    let back = decrypt(&encrypt(&plain)).unwrap();
    assert_eq!(&back[..11], &plain[..]);
    assert!(back[11..].iter().all(|&b| b == 0));
}

#[test]
fn decrypt_rejects_bodies_that_are_not_whole_blocks() {
    assert!(matches!(decrypt(&[0; 7]), Err(Error::BadBodyLength(7))));
}

#[test]
fn message_encodes_like_reference() {
    assert_eq!(login_reply().encode(), hex(LOGIN_REPLY_HEX));
}

#[test]
fn frame_matches_reference() {
    let expected = "7777772fc6507a69380111e68769f583045e2cb585d88411b43b13f64df8d9e3b71e27bdd9fcb47437f9dcd79b6eb298713352d8f382a98eb04b690ffc6e6d94722c82835dbfe6e838413fbf946c6809f3ae28dbec49e7a84d6317aa";
    assert_eq!(frame(&login_reply().encode()), hex(expected));
    assert_eq!(frame(&Message::reply(0x219F).encode()), hex("7777776f82f65a53e19a93a14bdfc3e72daf3247ec49e7a84d6317aa"));
}

#[test]
fn frame_len_unmasks_the_header() {
    let f = frame(&login_reply().encode());
    assert_eq!(frame_len(f[..4].try_into().unwrap()), f.len() - 4);
}

#[test]
fn decodes_reference_bytes_back_to_the_message() {
    let (msg, used) = Message::decode(&hex(LOGIN_REPLY_HEX)).unwrap();
    assert_eq!(msg, login_reply());
    assert_eq!(used, LOGIN_REPLY_HEX.len() / 2);
    assert_eq!(msg.request(), Some(0x2201));
    assert_eq!(msg.items[0].get("uname"), Some(&Value::text("test")));
}

#[test]
fn decodes_the_client_version_string() {
    let plain = [b"\x00\x122010.03.23 skelton".as_slice(), &[0; 4]].concat();
    assert_eq!(Packet::decode(&plain).unwrap(), Packet::Version("2010.03.23 skelton".into()));
}

#[test]
fn decodes_a_padded_message_packet() {
    let mut plain = hex(LOGIN_REPLY_HEX);
    plain.extend([0; 5]);
    assert_eq!(Packet::decode(&plain).unwrap(), Packet::Message(login_reply()));
}

#[test]
fn every_truncation_is_an_error_not_a_panic() {
    let bytes = hex(LOGIN_REPLY_HEX);
    for n in 0..bytes.len() {
        assert!(Message::decode(&bytes[..n]).is_err(), "prefix of {n} bytes decoded");
    }
}

#[test]
fn unknown_value_tags_are_rejected() {
    // tag 4, one field "x" with value tag 0x42
    let bytes = [4, 0, 1, 1, b'x', 0x42, 0, 0, 0, 0, 0, 0];
    assert!(matches!(Message::decode(&bytes), Err(Error::UnknownValueTag { tag: 0x42, .. })));
}

#[test]
fn u16_arrays_are_big_endian_binary() {
    assert_eq!(Value::u16_be_array(&[1, 0x1234]), Value::Bin(vec![0, 1, 0x12, 0x34]));
}

fn arb_value() -> impl Strategy<Value = Value> + Clone {
    prop_oneof![
        any::<u32>().prop_map(Value::U32),
        any::<u32>().prop_map(Value::U32Alt),
        prop::collection::vec(any::<u8>(), 0..40).prop_map(Value::Text),
        prop::collection::vec(any::<u8>(), 0..40).prop_map(Value::Bin),
        prop::collection::vec(any::<u8>(), 0..40).prop_map(Value::TextAlt),
    ]
}

fn arb_message() -> impl Strategy<Value = Message> {
    let fields = prop::collection::vec(("[a-z_]{1,12}", arb_value()), 0..6);
    let leaf = (any::<u8>(), fields.clone()).prop_map(|(tag, fields)| Message { tag, fields, items: vec![] });
    leaf.prop_recursive(2, 12, 4, move |inner| {
        (any::<u8>(), fields.clone(), prop::collection::vec(inner, 0..4))
            .prop_map(|(tag, fields, items)| Message { tag, fields, items })
    })
}

proptest! {
    #[test]
    fn any_message_round_trips(msg in arb_message()) {
        let bytes = msg.encode();
        let (back, used) = Message::decode(&bytes).unwrap();
        prop_assert_eq!(back, msg);
        prop_assert_eq!(used, bytes.len());
    }

    #[test]
    fn any_payload_survives_the_wire(payload in prop::collection::vec(any::<u8>(), 0..300)) {
        let f = frame(&payload);
        let n = frame_len(f[..4].try_into().unwrap());
        let plain = decrypt(&f[4..4 + n]).unwrap();
        prop_assert_eq!(&plain[..payload.len()], &payload[..]);
    }

    #[test]
    fn decoding_garbage_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..200)) {
        let _ = Packet::decode(&bytes);
    }
}
