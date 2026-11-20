#ifndef YO3_CARD_PROFILE_LOGIC_H
#define YO3_CARD_PROFILE_LOGIC_H
/* Issue #60, without the game: a card profile (what the client should show for some cards), its text format v1, the
 * patch of CARD_Prop/CARD_Genre rows, exact ATK/DEF, effect texts and artwork choice. card-profile.h installs it.
 *
 *   yo3-card-profile 1
 *   profile <name>
 *   hash <16 hex>
 *   card <row> <cid> <a_hex> <b_hex> <genre_hex64> <atk> <def>     atk/def: a decimal number, ? (shows "?") or - (the a word)
 *   text <row> <json string>                                       replaces that card's effect text
 *   art <cid>                                                      its picture may come from the art folder
 *
 * UTF-8 with LF or CRLF lines; `#` starts a comment line; unknown lines are skipped. Everything is bounded: a bad file
 * costs its own lines, never the game. */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <wchar.h>

#define YO3_PROFILE_MAX_CARDS 2048
#define YO3_PROFILE_MAX_TEXTS 1024
#define YO3_PROFILE_MAX_ART 2048
#define YO3_PROFILE_TEXT_CHARS 2048
#define YO3_PROFILE_MAX_BYTES (4u << 20)
#define YO3_STAT_KEEP 0     /* the a word's own value */
#define YO3_STAT_EXACT 1    /* `value` is shown */
#define YO3_STAT_UNKNOWN 2  /* "?" (0xFFFF in the client) */
/* The client's getters answer a 16 bit word and the sort reads it as a signed one (`movsx esi, ax`): above this a number would
 * turn negative or collide with "?" (0xFFFF), so it is shown as this at most. No card of the anime comes near it. */
#define YO3_STAT_MAX_SHOWN 32767

typedef struct { unsigned row, cid, a, b; uint64_t genre; unsigned atk_mode, def_mode; int atk, def; } Yo3ProfileCard;
typedef struct { unsigned row; wchar_t *text; } Yo3ProfileText;
typedef struct {
    int valid;
    char name[48], hash[24];
    unsigned ncards, ntexts, narts, skipped;
    Yo3ProfileCard cards[YO3_PROFILE_MAX_CARDS];
    Yo3ProfileText texts[YO3_PROFILE_MAX_TEXTS];
    unsigned arts[YO3_PROFILE_MAX_ART];
} Yo3Profile;

static int yo3_profile_hex(const char *s, size_t n, uint64_t *out) {
    uint64_t v = 0;
    if (!n || n > 16) return 0;
    for (size_t i = 0; i < n; ++i) {
        char c = s[i]; unsigned d;
        if (c >= '0' && c <= '9') d = (unsigned)(c - '0'); else if (c >= 'a' && c <= 'f') d = (unsigned)(c - 'a' + 10);
        else if (c >= 'A' && c <= 'F') d = (unsigned)(c - 'A' + 10); else return 0;
        v = v << 4 | d;
    }
    *out = v; return 1;
}
static int yo3_profile_dec(const char *s, size_t n, unsigned *out) {
    unsigned long long v = 0;
    if (!n || n > 9) return 0;
    for (size_t i = 0; i < n; ++i) { if (s[i] < '0' || s[i] > '9') return 0; v = v * 10 + (unsigned)(s[i] - '0'); }
    *out = (unsigned)v; return 1;
}
/* "-", "?" or a number (negative allowed); the stat's mode and value. */
static int yo3_profile_stat(const char *s, size_t n, unsigned *mode, int *value) {
    *value = 0;
    if (n == 1 && s[0] == '-') { *mode = YO3_STAT_KEEP; return 1; }
    if (n == 1 && s[0] == '?') { *mode = YO3_STAT_UNKNOWN; return 1; }
    int negative = n > 1 && s[0] == '-'; unsigned v;
    if (!yo3_profile_dec(s + negative, n - (size_t)negative, &v) || v > 99999) return 0;
    *mode = YO3_STAT_EXACT; *value = negative ? -(int)v : (int)v; return 1;
}
/* A UTF-8 JSON string body (quotes included) as UTF-16, bounded; NULL when it is not one. */
static wchar_t *yo3_profile_json_text(const char *s, size_t n) {
    if (n < 2 || s[0] != '"' || s[n - 1] != '"') return NULL;
    wchar_t *out = (wchar_t *)calloc(YO3_PROFILE_TEXT_CHARS + 1, sizeof *out);
    if (!out) return NULL;
    size_t o = 0; const unsigned char *p = (const unsigned char *)s + 1, *end = (const unsigned char *)s + n - 1;
    while (p < end) {
        unsigned c = *p++;
        if (c == '\\') {
            if (p >= end) goto bad;
            c = *p++;
            switch (c) {
                case 'n': c = '\n'; break; case 'r': c = '\r'; break; case 't': c = '\t'; break;
                case '"': case '\\': case '/': break;
                case 'u': {
                    uint64_t v; if (end - p < 4 || !yo3_profile_hex((const char *)p, 4, &v)) goto bad;
                    p += 4; c = (unsigned)v;
                    if (c >= 0xD800 && c < 0xDC00 && end - p >= 6 && p[0] == '\\' && p[1] == 'u') {
                        uint64_t lo; if (yo3_profile_hex((const char *)p + 2, 4, &lo) && lo >= 0xDC00 && lo < 0xE000) {
                            p += 6; if (o + 2 > YO3_PROFILE_TEXT_CHARS) goto bad;
                            out[o++] = (wchar_t)c; c = (unsigned)lo;
                        }
                    }
                    break;
                }
                default: goto bad;
            }
        } else if (c >= 0x80) { /* UTF-8 to UTF-16 */
            unsigned extra = c >= 0xF0 ? 3 : c >= 0xE0 ? 2 : c >= 0xC0 ? 1 : 99;
            if (extra == 99 || (size_t)(end - p) < extra) goto bad;
            c &= 0x3F >> extra;
            for (unsigned i = 0; i < extra; ++i) { if ((p[i] & 0xC0) != 0x80) goto bad; c = c << 6 | (p[i] & 0x3F); }
            p += extra;
            if (c >= 0x10000) {
                if (c > 0x10FFFF || o + 2 > YO3_PROFILE_TEXT_CHARS) goto bad;
                c -= 0x10000; out[o++] = (wchar_t)(0xD800 + (c >> 10)); c = 0xDC00 + (c & 0x3FF);
            }
        }
        if (o + 1 > YO3_PROFILE_TEXT_CHARS) goto bad;
        out[o++] = (wchar_t)c;
    }
    out[o] = 0; return out;
bad:
    free(out); return NULL;
}
static __attribute__((unused)) void yo3_profile_free(Yo3Profile *p) {
    for (unsigned i = 0; i < p->ntexts; ++i) free(p->texts[i].text);
    memset(p, 0, sizeof *p);
}
/* Parses a profile. Returns 1 when the header is right (the profile may still hold nothing); `skipped` counts the lines
 * that were not understood. */
static int yo3_profile_parse(const char *data, size_t length, Yo3Profile *p) {
    memset(p, 0, sizeof *p);
    if (!data || length > YO3_PROFILE_MAX_BYTES) return 0;
    size_t at = 0; int header = 0;
    while (at < length) {
        size_t end = at; while (end < length && data[end] != '\n') ++end;
        const char *line = data + at; size_t n = end - at; at = end + 1;
        if (n && line[n - 1] == '\r') --n;
        if (!n || line[0] == '#') continue;
        /* fields: split on single spaces, at most 9 */
        const char *f[9]; size_t fl[9]; unsigned nf = 0; size_t i = 0;
        while (i < n && nf < 9) {
            size_t s = i; while (i < n && line[i] != ' ') ++i;
            f[nf] = line + s; fl[nf] = i - s; ++nf;
            /* a text's string is the rest of the line */
            if (nf == 3 && fl[0] == 4 && !memcmp(f[0], "text", 4)) { f[2] = line + s; fl[2] = n - s; i = n; }
            if (i < n) ++i;
        }
        if (!header) {
            if (nf == 2 && fl[0] == 16 && !memcmp(f[0], "yo3-card-profile", 16) && fl[1] == 1 && f[1][0] == '1') { header = 1; p->valid = 1; continue; }
            memset(p, 0, sizeof *p); return 0;
        }
        #define IS(word) (fl[0] == sizeof(word) - 1 && !memcmp(f[0], word, sizeof(word) - 1))
        if (IS("profile") && nf >= 2) { size_t c = fl[1] < sizeof p->name - 1 ? fl[1] : sizeof p->name - 1; memcpy(p->name, f[1], c); p->name[c] = 0; }
        else if (IS("hash") && nf >= 2) { size_t c = fl[1] < sizeof p->hash - 1 ? fl[1] : sizeof p->hash - 1; memcpy(p->hash, f[1], c); p->hash[c] = 0; }
        else if (IS("card") && nf == 8) {
            Yo3ProfileCard c = {0}; uint64_t a, b;
            if (p->ncards < YO3_PROFILE_MAX_CARDS && yo3_profile_dec(f[1], fl[1], &c.row) && yo3_profile_dec(f[2], fl[2], &c.cid)
                && yo3_profile_hex(f[3], fl[3], &a) && yo3_profile_hex(f[4], fl[4], &b) && yo3_profile_hex(f[5], fl[5], &c.genre)
                && yo3_profile_stat(f[6], fl[6], &c.atk_mode, &c.atk) && yo3_profile_stat(f[7], fl[7], &c.def_mode, &c.def)
                && a <= 0xFFFFFFFFu && b <= 0xFFFFFFFFu && c.cid && c.cid < 0x4000) { c.a = (unsigned)a; c.b = (unsigned)b; p->cards[p->ncards++] = c; }
            else ++p->skipped;
        } else if (IS("text") && nf == 3) {
            unsigned row; wchar_t *t;
            if (p->ntexts < YO3_PROFILE_MAX_TEXTS && yo3_profile_dec(f[1], fl[1], &row) && (t = yo3_profile_json_text(f[2], fl[2]))) {
                p->texts[p->ntexts].row = row; p->texts[p->ntexts].text = t; ++p->ntexts;
            } else ++p->skipped;
        } else if (IS("art") && nf == 2) {
            unsigned cid;
            if (p->narts < YO3_PROFILE_MAX_ART && yo3_profile_dec(f[1], fl[1], &cid) && cid && cid < 0x4000) p->arts[p->narts++] = cid; else ++p->skipped;
        } else ++p->skipped;
        #undef IS
    }
    if (!header) { memset(p, 0, sizeof *p); return 0; }
    return 1;
}

/* The body of a helper's HTTP reply (`reply`, `got` bytes, NUL ended): the text after the headers of a 200 answer, moved to the
 * start of the buffer; its length, or -1 for any other answer (404 when the server has no profile, a refusal, garbage). */
static long yo3_profile_http_body(char *reply, size_t got) {
    if (got < 16 || memcmp(reply, "HTTP/1.", 7) || reply[8] != ' ' || memcmp(reply + 9, "200", 3)) return -1;
    reply[got] = 0;
    char *body = strstr(reply, "\r\n\r\n");
    if (!body) return -1;
    body += 4;
    size_t length = (size_t)(reply + got - body);
    memmove(reply, body, length + 1);
    return (long)length;
}

/* The patch: `rows` is CARD_Prop (a, b per row), `genre` CARD_Genre (two words per row). A line whose row is outside the
 * table or whose cid is not the row's is refused. Returns how many cards were patched; `refused` counts the others. */
static unsigned yo3_profile_apply(const Yo3Profile *p, uint32_t *rows, unsigned nrows, uint32_t *genre, unsigned ngenre, unsigned *refused) {
    unsigned done = 0; if (refused) *refused = 0;
    for (unsigned i = 0; i < p->ncards; ++i) {
        const Yo3ProfileCard *c = &p->cards[i];
        if (c->row >= nrows || (rows[c->row * 2] & 0x3FFF) != c->cid) { if (refused) ++*refused; continue; }
        rows[c->row * 2] = c->a; rows[c->row * 2 + 1] = c->b;
        if (genre && c->row < ngenre) { genre[c->row * 2] = (uint32_t)c->genre; genre[c->row * 2 + 1] = (uint32_t)(c->genre >> 32); }
        ++done;
    }
    return done;
}
/* The card's exact ATK or DEF (`def` selects) as the client's getter would answer; 0 when the profile leaves it to
 * the a word. */
static int yo3_profile_stat_of(const Yo3Profile *p, unsigned cid, int def, int *value) {
    for (unsigned i = 0; i < p->ncards; ++i) {
        if (p->cards[i].cid != cid) continue;
        unsigned mode = def ? p->cards[i].def_mode : p->cards[i].atk_mode; int v = def ? p->cards[i].def : p->cards[i].atk;
        if (mode == YO3_STAT_EXACT) { *value = v < 0 ? 0 : v > YO3_STAT_MAX_SHOWN ? YO3_STAT_MAX_SHOWN : v; return 1; }
        if (mode == YO3_STAT_UNKNOWN) { *value = 0xFFFF; return 1; }
        return 0;
    }
    return 0;
}
/* The effect text for a Konami id: the profile's text of any row of that card (rows map to ids through `rows`). */
static const wchar_t *yo3_profile_text_of(const Yo3Profile *p, const uint32_t *rows, unsigned nrows, unsigned cid) {
    for (unsigned i = 0; i < p->ntexts; ++i)
        if (p->texts[i].row < nrows && (rows[p->texts[i].row * 2] & 0x3FFF) == cid) return p->texts[i].text;
    return NULL;
}
static int yo3_profile_has_art(const Yo3Profile *p, unsigned cid) {
    for (unsigned i = 0; i < p->narts; ++i) if (p->arts[i] == cid) return 1;
    return 0;
}

/* The art folder's file for a texture path the game asks for: "data/card/<cid>_<art>.png" (the full face, folder
 * card/<cid>.png) or "data/mini/<cid>_<art>.png" (folder mini/<cid>.png). `folder` ends without a separator. Returns 1
 * and writes the file's path when the path is one of those, the card has `art` in the profile; `exists` then decides
 * (the caller checks the file). 0 for anything else. */
static __attribute__((unused)) int yo3_profile_art_path(const Yo3Profile *p, const char *path, const char *folder, char *out, size_t capacity, int *mini) {
    const char *name; *mini = 0;
    if (!path) return 0;
    while (*path == '.' || *path == '/' || *path == '\\') ++path;
    if (!strncmp(path, "data/card/", 10)) name = path + 10; else if (!strncmp(path, "data/mini/", 10)) { name = path + 10; *mini = 1; } else return 0;
    while (*name == ' ') ++name; /* the game pads the id with spaces ("%4d") */
    unsigned cid = 0, digits = 0;
    while (*name >= '0' && *name <= '9' && digits < 6) { cid = cid * 10 + (unsigned)(*name++ - '0'); ++digits; }
    if (!digits || *name != '_' || !cid || !yo3_profile_has_art(p, cid)) return 0;
    int n = snprintf(out, capacity, "%s/%s/%u.png", folder, *mini ? "mini" : "card", cid);
    return n > 0 && (size_t)n < capacity;
}

/* The card panels that read an ATK or DEF from the row's a word themselves (docs/client-internals.md "Card profiles"):
 * `call 0x537230` (EAX the Konami id), `shr eax, 0xE` / `0x17`, then ten times the nine bits as `lea R,[eax+eax*4];
 * add R,R` (five bytes; R is eax, ecx or edx). The first is aimed at a stub that notes the id; the five bytes become
 * a CALL to a stub that answers the profile's number when it has one (card-profile.h). */
typedef struct { unsigned call, mul; int reg; int def; int filter; } Yo3CardMulSite;
static const Yo3CardMulSite yo3_card_mul_sites[] = {
    { 0x4565c6, 0x4565e9, 0, 0, 0 }, { 0x4565f4, 0x45660d, 1, 1, 0 },
    { 0x4c7955, 0x4c7977, 2, 0, 0 }, { 0x4c7982, 0x4c799f, 0, 1, 0 },
    { 0x4c876d, 0x4c878f, 0, 0, 0 }, { 0x4c879a, 0x4c87b7, 1, 1, 0 },
    /* The deck editor's ATK and DEF range filters (a card is kept when min <= its stat <= max; "?" is 0, taken before the
     * ten times, so these sites are reached for a number only): each stat is read twice, once for the minimum and once for the
     * maximum, so a card with 1 ATK is found by a range that starts at 1. */
    { 0x418ab7, 0x418acf, 0, 0, 1 }, { 0x418ade, 0x418af6, 0, 0, 1 },
    { 0x418cbf, 0x418cd2, 0, 1, 1 }, { 0x418ce1, 0x418cf4, 0, 1, 1 },
};
/* `lea R,[eax+eax*4]; add R,R` for register `reg` (0 eax, 1 ecx, 2 edx). */
static int yo3_card_mul_bytes_ok(const unsigned char *b, int reg) {
    static const unsigned char lea[3] = { 0x04, 0x0c, 0x14 }, add[3] = { 0xc0, 0xc9, 0xd2 };
    return reg >= 0 && reg < 3 && b[0] == 0x8d && b[1] == lea[reg] && b[2] == 0x80 && b[3] == 0x03 && b[4] == add[reg];
}
/* `call target` written at `site` (the displacement is relative to the next instruction). */
static void yo3_card_write_call(unsigned char *site, unsigned long target) {
    unsigned long rel = target - ((unsigned long)site + 5);
    site[0] = 0xe8; memcpy(site + 1, &rel, 4);
}
/* What the panel would have shown for the nine bits: ten times them. */
static int yo3_card_times_ten(unsigned nine) { return (int)(nine * 10u); }

/* The deck editor's search and sort (issue #60, "Sorting and range search" in docs/client-internals.md), as the client does them,
 * so the test can run them with crafted rows:
 *   - the range filter (0x418930, the Attack/Defense cases) keeps a Monster whose stat is min <= stat <= max: the nine bits'
 *     ten times, "?" (all ones) being 0, or, when the profile has the card's number, that number (a profile's own "?" is 0 too).
 *   - the sort (0x419050) gives every card a key: Attack/Defense getter's word (0xFFFF for "?", read as signed 16 bits) minus 2
 *     for a Monster (0x416340), times 10000, plus the card's place in the list; descending is 0xFFFF minus 2 for a Monster
 *     minus the word. The list is then ordered by the key, so ties keep the list's order and "?" is lowest ascending (-3)
 *     and highest descending. The getters are wrapped (card_cb_atk, card_cb_def), so the keys already hold the profile's number.
 * The blank maximum of the search fields is 5000 (4 sites: the immediate of `mov eax, 0x1388`), which would hide a card of 5100 or
 * 6000; a profile with such a card makes it YO3_STAT_NO_LIMIT. */
static int yo3_card_filter_stat(const Yo3Profile *p, unsigned cid, int def, unsigned nine) {
    int v;
    if (nine == 0x1FF) return 0;
    if (p && p->valid && yo3_profile_stat_of(p, cid, def, &v)) return v == 0xFFFF ? 0 : v;
    return yo3_card_times_ten(nine);
}
static __attribute__((unused)) int yo3_card_in_range(int stat, int min, int max) { return stat >= min && stat <= max; }
/* The getter's word: the profile's number, else 0xFFFF for "?" or ten times the nine bits. */
static __attribute__((unused)) int yo3_card_getter_word(const Yo3Profile *p, unsigned cid, int def, unsigned nine) {
    int v;
    if (p && p->valid && yo3_profile_stat_of(p, cid, def, &v)) return v;
    return nine == 0x1FF ? 0xFFFF : yo3_card_times_ten(nine);
}
static __attribute__((unused)) int yo3_card_sort_key(int word, int monster, int descending, int place) {
    int signed_word = (int16_t)word;
    return descending ? (0xFFFF - 2 * monster - signed_word) * 10000 + place : (signed_word - 2 * monster) * 10000 + place;
}
#define YO3_STAT_DEFAULT_MAX 5000
#define YO3_STAT_NO_LIMIT 100000
/* The blank maximum's four sites: `mov eax, imm32` (B8 88 13 00 00) before `push; push eax`. */
static const unsigned yo3_card_default_max_sites[4] = { 0x465f73, 0x465fbc, 0x4953ac, 0x4953ed };
static int yo3_card_default_max_ok(const unsigned char *b) { return b[0] == 0xB8 && b[1] == 0x88 && b[2] == 0x13 && !b[3] && !b[4]; }
/* Whether any card of the profile shows more than the standard blank maximum. */
static int yo3_profile_needs_open_max(const Yo3Profile *p) {
    for (unsigned i = 0; i < p->ncards; ++i)
        for (int def = 0; def < 2; ++def) {
            int v;
            if (yo3_profile_stat_of(p, p->cards[i].cid, def, &v) && v != 0xFFFF && v > YO3_STAT_DEFAULT_MAX) return 1;
        }
    return 0;
}
#endif
