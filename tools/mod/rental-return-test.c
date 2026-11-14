/* The Rental Card List's Return button (rental-return-logic.h), without the game: which slots identify v1.153, which
 * frames are the Borrowed tab, which row a press names, when a press sends a return, and which pushes the open screen
 * must not act on. */
#include <assert.h>
#include <stdio.h>
#include "rental-return-logic.h"

int main(void) {
    /* The slots, at the preferred base and relocated by ASLR. */
    uint32_t slots[3] = {0x482250, 0x482030, 0x481d70};
    assert(yo3_rental_slots_ok(0x400000, slots));
    uint32_t moved[3] = {0xb60000 + 0x82250, 0xb60000 + 0x82030, 0xb60000 + 0x81d70};
    assert(yo3_rental_slots_ok(0xb60000, moved));
    assert(!yo3_rental_slots_ok(0x400000, moved));
    slots[2] = 0x10001000; /* Accept already taken by someone else */
    assert(!yo3_rental_slots_ok(0x400000, slots));
    assert(yo3_rental_slots[0][0] == 0x7a7a14 + 14 * 4 && yo3_rental_slots[1][0] == 0x7a7a14 + 6 * 4
        && yo3_rental_slots[2][0] == 0x7a7a54 + 4);

    /* The Borrowed tab is frames 3-8. */
    assert(!yo3_rental_borrowed_frame(1) && !yo3_rental_borrowed_frame(2));
    for (int f = 3; f <= 8; ++f) assert(yo3_rental_borrowed_frame(f));
    assert(!yo3_rental_borrowed_frame(9) && !yo3_rental_borrowed_frame(14) && !yo3_rental_borrowed_frame(0));

    /* Rows: whole indexes inside a sane vector of pointers. */
    int row = -1;
    assert(yo3_rental_row(0x1000, 0x100c, 0, &row) && row == 0);
    assert(yo3_rental_row(0x1000, 0x100c, 2, &row) && row == 2);
    assert(!yo3_rental_row(0x1000, 0x100c, 3, &row));
    assert(!yo3_rental_row(0x1000, 0x100c, -1, &row)); /* no row selected: click_return already refuses */
    assert(!yo3_rental_row(0x1000, 0x100c, 1.5, &row));
    assert(!yo3_rental_row(0x1000, 0x100c, 0.0 / 0.0, &row));
    assert(!yo3_rental_row(0x1000, 0x1000, 0, &row)); /* an empty list */
    assert(!yo3_rental_row(0x100c, 0x1000, 0, &row)); /* end before begin */
    assert(!yo3_rental_row(0x1000, 0x1006, 0, &row)); /* not whole pointers */
    assert(!yo3_rental_row(0x1000, 0x1000 + 4 * 1001, 0, &row));

    /* A press: a named, running rental, once. */
    Yo3RentalReturned returned = {0};
    assert(yo3_rental_should_return(&returned, 7, 0));
    assert(!yo3_rental_should_return(&returned, 0, 0)); /* an older server: no rental_id */
    assert(!yo3_rental_should_return(&returned, 7, 1)); /* over already */
    yo3_rental_remember(&returned, 7);
    assert(!yo3_rental_should_return(&returned, 7, 0));
    yo3_rental_remember(&returned, 7);
    assert(returned.next == 1);
    for (uint32_t id = 100; id < 140; ++id) yo3_rental_remember(&returned, id);
    assert(!yo3_rental_should_return(&returned, 139, 0) && yo3_rental_should_return(&returned, 7, 0));
    yo3_rental_remember(&returned, 0);
    assert(!yo3_rental_returned(&returned, 0));

    /* Pushed records reach the open screen: kept out once it asked for its lists. */
    assert(!yo3_rental_skip_packet(0x2d6c, 0));
    assert(yo3_rental_skip_packet(0x2d6c, 1) && yo3_rental_skip_packet(0x2d6c, 3) && yo3_rental_skip_packet(0x2d6c, 9));
    assert(!yo3_rental_skip_packet(0x2d6e, 1) && !yo3_rental_skip_packet(0x2d6d, 3));

    puts("PASS: the Return button's slots, Borrowed frames, rows, returns and pushes");
    return 0;
}
