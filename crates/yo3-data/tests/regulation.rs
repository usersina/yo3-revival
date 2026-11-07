//! How many copies of a card a regulation allows, as the client decides it (0x51A930, 0x51A9A0).
use yo3_data::Regulation;
use yo3_proto::Message;

fn regulation(lists: [Vec<u16>; 4], default_limit: u32) -> Regulation {
    Regulation { id: 1, kind: 0, status: 1, name_id: 0, lists, default_limit, record: Message::default() }
}

#[test]
fn a_card_may_appear_as_often_as_the_index_of_its_list() {
    let r = regulation([vec![100], vec![101], vec![102], vec![]], 3);
    assert_eq!([r.limit(100), r.limit(101), r.limit(102), r.limit(103)], [0, 1, 2, 3]);
    assert!(r.allows([101, 102, 102, 103, 103, 103]));
    assert!(!r.allows([100]));
    assert!(!r.allows([101, 101]));
    assert!(!r.allows([103, 103, 103, 103]));
}

#[test]
fn the_list_at_the_default_index_is_ignored() {
    // Format regulations: default 0 and card3 lists the cards allowed three times.
    let r = regulation([vec![100], vec![101], vec![], vec![103]], 0);
    assert_eq!([r.limit(100), r.limit(101), r.limit(103), r.limit(104)], [0, 1, 3, 0]);
    assert!(r.allows([101, 103, 103, 103]));
    assert!(!r.allows([104]));
}

#[test]
fn three_cards_are_never_allowed() {
    let r = regulation([vec![], vec![], vec![], vec![]], 3);
    for cid in Regulation::NEVER_ALLOWED {
        assert!(!r.allows([cid]), "{cid}");
    }
}

#[test]
fn a_text_table_maps_ids_to_utf16_strings() {
    // u32 count, count pairs {u32 id, u32 byte offset}, 4 bytes, then UTF-16LE NUL-terminated strings from 8 + 8 * count.
    let strings: Vec<u8> = ["Hi", "GADGET BOX"].iter().flat_map(|s| s.encode_utf16().chain([0]).flat_map(u16::to_le_bytes)).collect();
    let mut table = 2u32.to_le_bytes().to_vec();
    for (id, off) in [(7u32, 0u32), (24114, 6)] {
        table.extend(id.to_le_bytes());
        table.extend(off.to_le_bytes());
    }
    table.extend([0; 4]);
    table.extend(strings);
    let texts = yo3_data::parse_texts(&table);
    assert_eq!(texts.get(&7).map(String::as_str), Some("Hi"));
    assert_eq!(texts.get(&24114).map(String::as_str), Some("GADGET BOX"));
}
