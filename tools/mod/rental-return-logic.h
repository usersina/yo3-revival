#ifndef YO3_RENTAL_RETURN_LOGIC_H
#define YO3_RENTAL_RETURN_LOGIC_H
/* The pure part of the Rental Card List's Return button (rental-return.h), tested without the game by
 * rental-return-test.c. */
#include <stdint.h>

/* The three RentalRecord vtable slots the button takes over, with the game's own entries (v1.153, preferred base):
 * the main vtable 0x7A7A14's tick (slot 14) and packet handler (slot 6), and the FxDelegateHandler vtable 0x7A7A54's
 * Accept (slot 1), which registers the screen's Flash callbacks. */
static const uint32_t yo3_rental_slots[3][2] = {
    {0x7a7a4c, 0x482250}, /* tick: loads the movie at state 2 */
    {0x7a7a2c, 0x482030}, /* events: front packets reach 0x482090 */
    {0x7a7a58, 0x481d70}, /* Accept: ExitRental ... AdjustRental */
};

/* An address of the game at its actual base (ASLR on native Windows). */
static uint32_t yo3_rental_at(uint32_t base, uint32_t address) { return base + (address - 0x400000u); }

/* The slots hold the game's own functions, relocated: this is v1.153's RentalRecord, and nothing else has taken them. */
static int yo3_rental_slots_ok(uint32_t base, const uint32_t values[3]) {
    for (int i = 0; i < 3; ++i)
        if (values[i] != yo3_rental_at(base, yo3_rental_slots[i][1])) return 0;
    return 1;
}

/* rentalrecord.swf's main timeline: frames 3-8 are the Borrowed tab (labels borrow_menu0, borrow_menu0_stop and
 * borrow_menu1), 9-14 the Lent one (loan_menu0 ...); frames 1 and 2 load the lists. The button shows on 3-8 only. */
static int yo3_rental_borrowed_frame(double frame) { return frame >= 3 && frame <= 8; }

/* The Borrowed row the movie names (`_root.listBorrowIndex + _root.selectList`, a number) in the borrowed list, the
 * vector of record pointers at RentalRecord +0x90/+0x94. A row only when the vector looks sane and the number is a
 * whole index inside it. */
static int yo3_rental_row(uint32_t begin, uint32_t end, double index, int *row) {
    if (end < begin || (end - begin) % 4 || (end - begin) / 4 > 1000) return 0;
    uint32_t count = (end - begin) / 4;
    if (!(index >= 0) || index >= (double)count || index != (double)(int)index) return 0;
    *row = (int)index;
    return 1;
}

/* The rentals returned since the game started: the screen's own list isn't refreshed by the server's pushes
 * (rental-return.h), so a second press on the same row sends nothing. */
typedef struct { uint32_t ids[32]; unsigned next; } Yo3RentalReturned;
static int yo3_rental_returned(const Yo3RentalReturned *r, uint32_t id) {
    for (unsigned i = 0; i < sizeof r->ids / sizeof r->ids[0]; ++i)
        if (id && r->ids[i] == id) return 1;
    return 0;
}
static void yo3_rental_remember(Yo3RentalReturned *r, uint32_t id) {
    if (!id || yo3_rental_returned(r, id)) return;
    r->ids[r->next++ % (sizeof r->ids / sizeof r->ids[0])] = id;
}

/* What a press does with the row's record: send the return only for a rental that is ours to name (a `rental_id`,
 * which v0.0.13+ servers add), not over (`confirm_to` 0) and not returned already. */
static int yo3_rental_should_return(const Yo3RentalReturned *r, uint32_t id, uint32_t over) {
    return id != 0 && over == 0 && !yo3_rental_returned(r, id);
}

/* A front packet the open screen must not act on: the rental records 0x2D6C make RentalRecord ask for its lists again
 * (0x482760), and each 0x2D6E answer is stored at +0x6C + 0x18 * state (0x48211B) and counts the state up: only states
 * 0 and 1 are the Lent and Borrowed lists, later answers overwrite the screen's card-view vectors (+0x9C..+0xE3) and,
 * from state 5, memory past them. The records arrive as pushes after any return or a term's end; the global
 * dispatcher still keeps them (dataset slot 8) for the collection. */
static int yo3_rental_skip_packet(uint32_t id, int32_t state) { return id == 0x2d6c && state != 0; }
#endif
