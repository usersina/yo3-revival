/* Issue #60: the card profile's text format, the row patch, exact stats, texts and art paths (card-profile-logic.h),
 * without the game. */
#include <assert.h>
#include <stdio.h>
#include "card-profile-logic.h"

static Yo3Profile profile;

int main(void) {
    const char *text =
        "# a profile\r\nyo3-card-profile 1\nprofile anime\nhash 0123456789abcdef\n"
        "card 2 4007 4fa7 1a8 ff 1 ?\n"
        "card 3 4008 0 0 0 - 1900\n"
        "card 9 99 0 0 0 - -\n"          /* row 9 is outside the table below */
        "card 1 4001 zz 0 0 - -\n"       /* bad hex */
        "card 1 4001 0 0 0 1 2 3\n"      /* too many fields */
        "text 2 \"Line one.\\nSecond \\\"q\\\" \\u00e9 \\ud83d\\ude00 \xc3\xa9\"\n"
        "text 3 not-json\n"
        "art 4007\nart 0\nart 99999\nfuture 1 2 3\n";
    assert(yo3_profile_parse(text, strlen(text), &profile));
    assert(!strcmp(profile.name, "anime") && !strcmp(profile.hash, "0123456789abcdef"));
    assert(profile.ncards == 3 && profile.ntexts == 1 && profile.narts == 1 && profile.skipped == 6);
    assert(profile.cards[0].a == 0x4fa7 && profile.cards[0].b == 0x1a8 && profile.cards[0].genre == 0xff);
    assert(profile.cards[0].atk_mode == YO3_STAT_EXACT && profile.cards[0].atk == 1 && profile.cards[0].def_mode == YO3_STAT_UNKNOWN);
    assert(profile.cards[1].atk_mode == YO3_STAT_KEEP && profile.cards[1].def == 1900);
    const wchar_t expect[] = L"Line one.\nSecond \"q\" \u00e9 \U0001f600 \u00e9"; /* UTF-16 in wchar_t (2 bytes) */
    assert(sizeof(wchar_t) == 2 && !wcscmp(profile.texts[0].text, expect));

    /* The patch: rows are (a, b); the cid guard refuses a row that is not that card. */
    uint32_t rows[8] = { 0,0, 0,0, 4007,0x50, 4009,0x60 }, genre[8] = {0};
    unsigned refused;
    assert(yo3_profile_apply(&profile, rows, 4, genre, 4, &refused) == 1 && refused == 2);
    assert(rows[4] == 0x4fa7 && rows[5] == 0x1a8 && genre[4] == 0xff && genre[5] == 0);
    assert(rows[6] == 4009 && rows[7] == 0x60);                      /* row 3 holds 4009, the line said 4008: untouched */
    int v;
    assert(yo3_profile_stat_of(&profile, 4007, 0, &v) && v == 1);
    assert(yo3_profile_stat_of(&profile, 4007, 1, &v) && v == 0xFFFF);
    assert(!yo3_profile_stat_of(&profile, 4008, 0, &v) && yo3_profile_stat_of(&profile, 4008, 1, &v) && v == 1900);
    assert(!yo3_profile_stat_of(&profile, 5000, 0, &v));
    assert(yo3_profile_text_of(&profile, rows, 4, 4007) == profile.texts[0].text && !yo3_profile_text_of(&profile, rows, 4, 4009));

    char path[128]; int mini;
    assert(yo3_profile_art_path(&profile, "data/card/4007_1.png", "C:/art", path, sizeof path, &mini) && !mini && !strcmp(path, "C:/art/card/4007.png"));
    assert(yo3_profile_art_path(&profile, "data/card/4007_2.png", "C:/art", path, sizeof path, &mini) && !strcmp(path, "C:/art/card/4007.png"));
    assert(yo3_profile_art_path(&profile, "data/card/ 4007_1.png", "C:/art", path, sizeof path, &mini));
    assert(yo3_profile_art_path(&profile, "data/mini/4007_1.png", "C:/art", path, sizeof path, &mini) && mini && !strcmp(path, "C:/art/mini/4007.png"));
    assert(!yo3_profile_art_path(&profile, "data/card/4008_1.png", "C:/art", path, sizeof path, &mini));  /* not listed */
    assert(!yo3_profile_art_path(&profile, "data/protector/p0001.png", "C:/art", path, sizeof path, &mini));
    assert(!yo3_profile_art_path(&profile, "data/card/4007_1.png", "C:/art", path, 8, &mini));            /* too small */
    assert(!yo3_profile_art_path(&profile, NULL, "C:/art", path, sizeof path, &mini));
    yo3_profile_free(&profile);

    /* Garbage never crashes and never yields a profile. */
    const char *bad[] = { "", "\n\n", "profile x\n", "yo3-card-profile 2\n", "yo3-card-profile\n", "\xff\xfe\x00garbage", "yo3-card-profile 1x\n" };
    for (unsigned i = 0; i < sizeof bad / sizeof *bad; ++i) assert(!yo3_profile_parse(bad[i], strlen(bad[i]), &profile));
    assert(yo3_profile_parse("yo3-card-profile 1", 18, &profile) && !profile.ncards);     /* a header alone: no changes */
    yo3_profile_free(&profile);
    assert(!yo3_profile_parse("yo3-card-profile 1\n", YO3_PROFILE_MAX_BYTES + 1, &profile));
    /* A file with too many lines keeps its first lines and skips the rest. */
    size_t cap = 64 * 1024 * 40; char *big = (char *)malloc(cap), *w = big;
    w += sprintf(w, "yo3-card-profile 1\n");
    for (unsigned i = 0; i < 5000; ++i) w += sprintf(w, "card %u 4007 0 0 0 - -\ntext %u \"x\"\nart %u\n", i, i, i + 1);
    assert(yo3_profile_parse(big, (size_t)(w - big), &profile));
    assert(profile.ncards == YO3_PROFILE_MAX_CARDS && profile.ntexts == YO3_PROFILE_MAX_TEXTS && profile.narts == YO3_PROFILE_MAX_ART);
    yo3_profile_free(&profile); free(big);
    /* An over-long text is refused, not truncated. */
    char *longline = (char *)malloc(YO3_PROFILE_TEXT_CHARS + 200), *l = longline;
    l += sprintf(l, "yo3-card-profile 1\ntext 1 \"");
    for (unsigned i = 0; i < YO3_PROFILE_TEXT_CHARS + 5; ++i) *l++ = 'a';
    l += sprintf(l, "\"\n");
    assert(yo3_profile_parse(longline, (size_t)(l - longline), &profile) && !profile.ntexts && profile.skipped == 1);
    yo3_profile_free(&profile); free(longline);
    /* The helper's answer: the body of a 200, nothing else. */
    char reply[256]; const char *ok = "HTTP/1.0 200 OK\r\nContent-Length: 19\r\n\r\nyo3-card-profile 1\n";
    strcpy(reply, ok);
    long body = yo3_profile_http_body(reply, strlen(ok));
    assert(body == 19 && !strcmp(reply, "yo3-card-profile 1\n"));
    strcpy(reply, "HTTP/1.1 404 Not Found\r\n\r\nnope"); assert(yo3_profile_http_body(reply, strlen(reply)) < 0);
    strcpy(reply, "HTTP/1.1 200 OK\r\nno blank line"); assert(yo3_profile_http_body(reply, strlen(reply)) < 0);
    strcpy(reply, "garbage garbage garbage"); assert(yo3_profile_http_body(reply, strlen(reply)) < 0);
    strcpy(reply, "HTTP/1.1 200 OK\r\n\r\n"); assert(yo3_profile_http_body(reply, strlen(reply)) == 0);

    /* The panels' inline ATK/DEF math: the five bytes, per register, and the call that replaces them. */
    {
        static const unsigned char seq[3][5] = { {0x8d,0x04,0x80,0x03,0xc0}, {0x8d,0x0c,0x80,0x03,0xc9}, {0x8d,0x14,0x80,0x03,0xd2} };
        for (int r = 0; r < 3; ++r) for (int q = 0; q < 3; ++q) assert(yo3_card_mul_bytes_ok(seq[q], r) == (q == r));
        assert(!yo3_card_mul_bytes_ok(seq[0], 3) && !yo3_card_mul_bytes_ok(seq[0], -1));
        unsigned char wrong[5] = { 0x8d,0x04,0x80,0x03,0xc9 }; assert(!yo3_card_mul_bytes_ok(wrong, 0));
        unsigned char site[8] = {0}; yo3_card_write_call(site, (unsigned long)site + 5 + 0x100);
        assert(site[0] == 0xe8 && site[1] == 0 && site[2] == 1 && site[3] == 0 && site[4] == 0);
        yo3_card_write_call(site, (unsigned long)site - 0x10);
        int32_t rel; memcpy(&rel, site + 1, 4); assert(rel == -0x15);
        assert(yo3_card_times_ten(0) == 0 && yo3_card_times_ten(255) == 2550);
        enum { SITES = 10 };
        assert(sizeof yo3_card_mul_sites / sizeof *yo3_card_mul_sites == SITES);
        for (unsigned i = 0; i < SITES; ++i) {
            const Yo3CardMulSite *s = &yo3_card_mul_sites[i];
            assert(s->reg >= 0 && s->reg < 3 && s->mul > s->call);
            /* ATK before DEF in each function, and no two patched ranges (the call's five bytes, the multiplication's) overlap. */
            for (unsigned j = 0; j < i; ++j) {
                const Yo3CardMulSite *o = &yo3_card_mul_sites[j];
                assert(s->call + 5 <= o->call || o->call + 5 <= s->call);
                assert(s->mul + 5 <= o->mul || o->mul + 5 <= s->mul);
                assert(s->mul + 5 <= o->call || o->call + 5 <= s->mul);
                assert(o->mul + 5 <= s->call || s->call + 5 <= o->mul);
            }
        }
        /* Each patched stat has a stub (card_mul_stub in card-profile.h: eax for ATK and DEF here, ecx/edx for the panels'). */
        for (unsigned i = 6; i < SITES; ++i) assert(yo3_card_mul_sites[i].reg == 0);
    }
    puts("PASS: the card profile parses its format, refuses what is wrong, patches rows by their ids and finds stats, texts and art");
    return 0;
}
