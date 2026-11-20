//! Which of a deck's lists a card belongs in, shared by the server's decks and the launcher's local deck files (issue #60).

/// Moves cards between a deck's Main and Extra lists. `want(row)` says where a CARD_Prop row belongs: `Some(true)` the
/// Extra Deck, `Some(false)` the Main Deck, `None` leaves the card where the player put it. Every card stays, so the
/// quantities do; the cards that stay keep their order and the ones that move go to the end of their new list, in
/// their order.
pub fn place(main: &[u16], extra: &[u16], want: impl Fn(u16) -> Option<bool>) -> (Vec<u16>, Vec<u16>) {
    let (leaving_main, staying): (Vec<u16>, Vec<u16>) = main.iter().partition(|&&r| want(r) == Some(true));
    let (leaving_extra, kept): (Vec<u16>, Vec<u16>) = extra.iter().partition(|&&r| want(r) == Some(false));
    (staying.into_iter().chain(leaving_extra).collect(), kept.into_iter().chain(leaving_main).collect())
}

#[cfg(test)]
mod tests {
    use super::place;

    #[test]
    fn cards_move_to_the_list_they_belong_in_and_keep_their_quantities() {
        // 1 and 2 belong in the Extra Deck, 3 in the Main Deck, 9 nowhere in particular.
        let want = |r| match r { 1 | 2 => Some(true), 3 => Some(false), _ => None };
        let (main, extra) = place(&[5, 1, 9, 1, 6], &[2, 3, 9, 3], want);
        assert_eq!((main, extra), (vec![5, 9, 6, 3, 3], vec![2, 9, 1, 1]));
        let (main, extra) = place(&[5], &[], |_| None);
        assert_eq!((main, extra), (vec![5], vec![]), "nothing to say, nothing moves");
    }
}
