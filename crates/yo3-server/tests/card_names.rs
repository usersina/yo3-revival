//! `scenario::card`: a card's client id from its name, for scenario tests (ids typed by hand went wrong), from the
//! repository's tables alone (no game files, no fetched engine).
use yo3_server::duelcheck::scenario::card;

#[test]
fn a_name_gives_the_id_the_client_knows() {
    assert_eq!(card("Giant Axe Mummy"), 5420, "a card of the game");
    assert_eq!(card("gene-warped warwolf"), 6960, "any case");
    assert_eq!(card("Necro Wall"), 5464, "an anime-only added card, by its stand-in");
    assert_eq!(card("Shooting Quasar Dragon"), 9709, "a printed added card, by its Konami id");
    assert_eq!(card("Steam Token"), 5174, "an added Token");
}

#[test]
#[should_panic(expected = "are all named \"Dark Magician\"")]
fn a_name_of_several_cards_asks_for_the_id() {
    card("Dark Magician");
}

#[test]
#[should_panic(expected = "no card the client has is named")]
fn an_unknown_name_says_so() {
    card("Blue-Eyes Ultimate Dragon Of Nowhere");
}
