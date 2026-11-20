/* Issue #60: the card profile's hooks, run for real (card-profile.h itself, on synthetic code and memory; no game): the
 * ATK/DEF getter wrappers over a synthetic getter, the range filter's stubs, the blank maximum's patch, and the deck
 * editor's sort keys and range search on crafted rows, in standard mode (no profile) and with a profile. The synthetic code
 * is written here from what the client's getter does (docs/client-internals.md "Sorting and range search"); it is not game code. */
#include <windows.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

static unsigned char *fake_image;                       /* the "game": 1 MiB at the preferred base 0x400000 */
static void *game_address(uintptr_t preferred) { return fake_image + (preferred - 0x400000u); }
static char cookie_hex[65];
static int helper_request(const char *request, int length, char *reply, int capacity, DWORD wait) { (void)request; (void)length; (void)reply; (void)capacity; (void)wait; return 0; }
static char log_text[4096];
static void starter_log(const char *text) { strncat(log_text, text, sizeof log_text - strlen(log_text) - 1); }
#pragma GCC diagnostic ignored "-Wunused-function"
#include "card-profile.h"

/* The synthetic 0x537230: EAX the Konami id in, EAX the row's a word out. */
enum { CIDS = 6000 };
unsigned fake_a[CIDS];
void fake_rows(void);
__asm__(".text\n.globl _fake_rows\n_fake_rows:\n mov _fake_a(,%eax,4),%eax\n ret\n");

/* The getter, as 0x5373C0 (shift 14) and 0x5373F0 (shift 23) do: rows call, the nine bits, "?" is 0xFFFF, else ten times. */
static unsigned char *plant_getter(unsigned at, int shift) {
    unsigned char *c = (unsigned char *)game_address(at), *p = c;
    *p++ = 0x83; *p++ = 0xEC; *p++ = 0x0C;                                  /* sub esp,0xc */
    *p++ = 0xE8; { int32_t rel = (int32_t)((intptr_t)fake_rows - (intptr_t)(p + 4)); memcpy(p, &rel, 4); p += 4; }
    *p++ = 0xC1; *p++ = 0xE8; *p++ = (unsigned char)shift;                  /* shr eax,shift */
    *p++ = 0x25; *p++ = 0xFF; *p++ = 0x01; *p++ = 0; *p++ = 0;              /* and eax,0x1ff */
    *p++ = 0x3D; *p++ = 0xFF; *p++ = 0x01; *p++ = 0; *p++ = 0;              /* cmp eax,0x1ff */
    *p++ = 0x75; *p++ = 0x09;                                               /* jne +9 */
    *p++ = 0xB8; *p++ = 0xFF; *p++ = 0xFF; *p++ = 0; *p++ = 0;              /* mov eax,0xffff */
    *p++ = 0x83; *p++ = 0xC4; *p++ = 0x0C; *p++ = 0xC3;                     /* add esp,0xc; ret */
    *p++ = 0x8D; *p++ = 0x04; *p++ = 0x80; *p++ = 0x01; *p++ = 0xC0;        /* lea eax,[eax+eax*4]; add eax,eax */
    *p++ = 0x83; *p++ = 0xC4; *p++ = 0x0C; *p++ = 0xC3;
    return c;
}
static int call_eax(void *fn, unsigned eax) {
    int r;
    __asm__ volatile("call *%1" : "=a"(r) : "r"(fn), "0"(eax) : "ecx", "edx", "memory", "cc");
    return r;
}
/* The filter's inline math as the client runs it at 0x418AB7 and the others: note the id (the stub on CALL 0x537230), take the
 * nine bits, "?" is 0 before anything, else the stub answers (the five bytes' replacement). */
static int client_filter_stat(unsigned cid, int def) {
    unsigned a = (unsigned)call_eax((void *)card_stub_note_cid, cid);               /* the rows call, id noted */
    unsigned nine = a >> (def ? 23 : 14) & 0x1FF;
    if (nine == 0x1FF) return 0;
    return call_eax((void *)(def ? card_filter_def_eax : card_filter_atk_eax), nine);
}

static void *hooked_atk, *hooked_def;
typedef struct { unsigned cid, a; int monster; } Row;
#define A(nine_atk, nine_def, cid) ((cid) | (unsigned)(nine_atk) << 14 | (unsigned)(nine_def) << 23)
static const Row rows[12] = {
    {5001, A(0, 0, 5001), 1},       {5002, A(0, 0, 5002), 1},       {5003, A(0, 0, 5003), 1},      {5004, A(1, 1, 5004), 1},
    {5005, A(250, 250, 5005), 1},   {5006, A(250, 250, 5006), 1},   {5007, A(0x1FE, 0x1FE, 5007), 1}, {5008, A(0x1FE, 0x1FE, 5008), 1},
    {5009, A(0x1FF, 0x1FF, 5009), 1}, {5010, A(0x1FF, 0x1FF, 5010), 1}, {5011, A(250, 250, 5011), 1}, {5012, A(0x1FF, 0x1FF, 5012), 0},
};
/* the profile covers 5001..5009: 0, 1/1, 9, (5004: the game's 10), 2500, 2501, 5100, 6000, "?" */
static const char *profile_text =
    "yo3-card-profile 1\nprofile sorting\nhash 0000000000000000\n"
    "card 0 5001 0 0 0 0 0\ncard 1 5002 0 0 0 1 1\ncard 2 5003 0 0 0 9 9\ncard 4 5005 0 0 0 2500 2500\ncard 5 5006 0 0 0 2501 2501\n"
    "card 6 5007 0 0 0 5100 5100\ncard 7 5008 0 0 0 6000 6000\ncard 8 5009 0 0 0 ? ?\n";

static void sort_order(int def, int descending, unsigned order[12]) {
    long keys[12];
    for (unsigned i = 0; i < 12; ++i) {
        int word = call_eax(def ? hooked_def : hooked_atk, rows[i].cid);
        keys[i] = yo3_card_sort_key(word, rows[i].monster, descending, (int)i); order[i] = i;
    }
    for (unsigned i = 1; i < 12; ++i) for (unsigned j = i; j > 0 && keys[order[j - 1]] > keys[order[j]]; --j) { unsigned t = order[j]; order[j] = order[j - 1]; order[j - 1] = t; }
}

static void check_order(const unsigned order[12], const unsigned want[12], const char *what) {
    for (unsigned i = 0; i < 12; ++i) if (order[i] != want[i]) {
        fprintf(stderr, "FAIL %s: position %u is row %u, wanted %u\n", what, i, order[i], want[i]); exit(1);
    }
}

int main(void) {
    fake_image = (unsigned char *)VirtualAlloc(NULL, 0x100000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    assert(fake_image);
    for (unsigned i = 0; i < 12; ++i) fake_a[rows[i].cid] = rows[i].a;
    card_orig_rows_by_cid = (void *)fake_rows;
    /* hook the synthetic getters exactly as card_profile_install does (the bytes are checked, the CALL's target re-aimed) */
    unsigned char *atk = plant_getter(0x401000, 14), *def = plant_getter(0x401100, 23);
    unsigned char expect_atk[8], expect_def[8]; memcpy(expect_atk, atk, 8); memcpy(expect_def, def, 8);
    assert((card_tramp_atk = card_detour(0x401000, 8, expect_atk, 3, (void *)card_wrap_atk)));
    assert((card_tramp_def = card_detour(0x401100, 8, expect_def, 3, (void *)card_wrap_def)));
    hooked_atk = atk; hooked_def = def;

    /* what each getter answers and the filter's stat, per row: standard mode (no profile) then the profile's */
    const int std_word[12] = { 0, 0, 0, 10, 2500, 2500, 5100, 5100, 0xFFFF, 0xFFFF, 2500, 0xFFFF };
    const int prof_word[12] = { 0, 1, 9, 10, 2500, 2501, 5100, 6000, 0xFFFF, 0xFFFF, 2500, 0xFFFF };
    const int std_stat[12] = { 0, 0, 0, 10, 2500, 2500, 5100, 5100, 0, 0, 2500, 0 };
    const int prof_stat[12] = { 0, 1, 9, 10, 2500, 2501, 5100, 6000, 0, 0, 2500, 0 };
    /* the sorted rows (their list places): "?" first ascending (a Monster's key is 2 lower), ties in list order; the reverse going down */
    const unsigned std_up[12] = { 8, 9, 0, 1, 2, 11, 3, 4, 5, 10, 6, 7 }, std_down[12] = { 6, 7, 4, 5, 10, 3, 0, 1, 2, 8, 9, 11 };
    const unsigned prof_up[12] = { 8, 9, 0, 1, 11, 2, 3, 4, 10, 5, 6, 7 }, prof_down[12] = { 7, 6, 5, 4, 10, 3, 2, 1, 0, 8, 9, 11 };
    Yo3Profile *loaded = &card_profile;
    assert(yo3_profile_parse(profile_text, strlen(profile_text), loaded) && loaded->ncards == 8 && !loaded->skipped);

    for (int mode = 0; mode < 2; ++mode) {
        card_profile_on = mode;                              /* 0: standard mode, the client's own numbers; 1: the profile */
        const int *word = mode ? prof_word : std_word, *stat = mode ? prof_stat : std_stat;
        for (int def_side = 0; def_side < 2; ++def_side) {
            for (unsigned i = 0; i < 12; ++i) {
                int got = call_eax(def_side ? hooked_def : hooked_atk, rows[i].cid);
                if (got != word[i]) { fprintf(stderr, "FAIL getter mode %d side %d row %u: %d, wanted %d\n", mode, def_side, i, got, word[i]); return 1; }
                int filtered = client_filter_stat(rows[i].cid, def_side);
                if (rows[i].monster && filtered != stat[i]) { fprintf(stderr, "FAIL filter mode %d side %d row %u: %d, wanted %d\n", mode, def_side, i, filtered, stat[i]); return 1; }
            }
            unsigned order[12];
            sort_order(def_side, 0, order); check_order(order, mode ? prof_up : std_up, mode ? "profile ascending" : "standard ascending");
            sort_order(def_side, 1, order); check_order(order, mode ? prof_down : std_down, mode ? "profile descending" : "standard descending");
        }
        /* range searches: every pair of bounds, for ATK and DEF, against the numbers the client shows */
        static const int bounds[] = { 0, 1, 2, 9, 10, 11, 2500, 2501, 5000, 5100, 5101, 6000, 6001 };
        for (unsigned lo = 0; lo < sizeof bounds / sizeof *bounds; ++lo) for (unsigned hi = 0; hi < sizeof bounds / sizeof *bounds; ++hi)
            for (int def_side = 0; def_side < 2; ++def_side) for (unsigned i = 0; i < 12; ++i) {
                if (!rows[i].monster) continue;
                int in = yo3_card_in_range(client_filter_stat(rows[i].cid, def_side), bounds[lo], bounds[hi]);
                int want = stat[i] >= bounds[lo] && stat[i] <= bounds[hi];
                if (in != want) { fprintf(stderr, "FAIL range mode %d row %u [%d,%d]\n", mode, i, bounds[lo], bounds[hi]); return 1; }
            }
    }
    /* a card the profile does not list keeps the game's own answer even with the profile on */
    card_profile_on = 1;
    assert(call_eax(hooked_atk, 5011) == 2500 && call_eax(hooked_atk, 5004) == 10 && client_filter_stat(5004, 0) == 10);

    /* the blank maximum: four sites; opened only with a profile that has a stat above 5000, and only when the bytes are the client's */
    static const unsigned char mov5000[5] = { 0xB8, 0x88, 0x13, 0, 0 };
    for (int mode = 0; mode < 4; ++mode) {
        for (unsigned i = 0; i < 4; ++i) memcpy(game_address(yo3_card_default_max_sites[i]), mov5000, 5);
        if (mode == 3) ((unsigned char *)game_address(yo3_card_default_max_sites[2]))[1] = 0x89;      /* one site is not the client's */
        card_profile_on = mode != 0;
        if (mode == 3) assert(yo3_profile_parse(profile_text, strlen(profile_text), &card_profile));
        if (mode == 2) { static const char *low = "yo3-card-profile 1\ncard 0 5001 0 0 0 5000 5000\n"; assert(yo3_profile_parse(low, strlen(low), &card_profile)); }
        card_open_max();
        for (unsigned i = 0; i < 4; ++i) {
            int32_t v; memcpy(&v, (unsigned char *)game_address(yo3_card_default_max_sites[i]) + 1, 4);
            int opened = mode == 1;
            if (mode == 3) { opened = 0; assert(v == (i == 2 ? 0x1389 : 5000)); } else assert(v == (opened ? YO3_STAT_NO_LIMIT : YO3_STAT_DEFAULT_MAX));
            assert(*(unsigned char *)game_address(yo3_card_default_max_sites[i]) == 0xB8);
        }
        if (mode == 1) { assert(strstr(log_text, "blank maximum is open")); }
        if (mode == 2) { assert(card_profile.ncards == 1); }
    }
    assert(strstr(log_text, "not as expected, left at 5000"));
    puts("PASS: the card profile's hooks answer exact ATK/DEF to the sort and the range search, and nothing changes in standard mode");
    return 0;
}
