#ifndef YO3_DUEL_NUMBERS_LOGIC_H
#define YO3_DUEL_NUMBERS_LOGIC_H
/* The pure part of the infinite ATK display (duel-numbers.h), tested without the game by duel-numbers-test.c.
 * Evidence: docs/client-internals.md, "Numbers in the duel: LP and ATK".
 *  - The ATK and DEF under a field card are text fields of data/gfx/duel_fieldinfo.swf bound to the card info's `atk`
 *    and `def`, which the game sets through FxDelegate's `call` ("setProp", prop, value), from 0x50C190: prop 6 the ATK,
 *    7 the DEF, 4 and 5 the original ATK and DEF. An ATK of 9,999,999 (the engine's infinity) showed as its last five
 *    digits, 99999.
 *  - So such an ATK or DEF goes to the movie as the string "∞". Its colour compares it with the original (white when
 *    equal, blue above, red below): the original values always go as their decimal string, so that ActionScript 2
 *    (SWF 8) compares "∞" with them as strings, where "∞" (U+221E) is above every digit, and a number with them as
 *    numbers, as before. The movie does nothing else with them. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* A GFxValue (GFx 3): +4 type (3 number, 4 char *, 0x44 char ** ; 0x40 managed), +8 the value. */
typedef struct { void *iface; uint32_t type; union { double number; const char *text; const char **ref; void *data; } v; } Yo3Gfx;

#define YO3_INFINITE_ATK 9999999.0
static const char yo3_infinity[] = "\xe2\x88\x9e"; /* ∞ in UTF-8, as GFx 3 reads a char * */

static const char *yo3_gfx_string(const Yo3Gfx *v) {
    if (v->type == 0x44) return v->v.ref ? *v->v.ref : NULL; /* by reference, as ui-state.h reads it */
    if (v->type == 4) return v->v.text;
    return NULL;
}

/* Rewrites an invoke's arguments into `out` (which has room for `cap` values; `texts` holds `cap` strings of 16 bytes for
 * the decimal originals) when it is the field info's setProp of an ATK, a DEF or an original: 1 if `out` is to be used,
 * 0 to pass the call on untouched. */
static int yo3_numbers_rewrite(const char *name, const Yo3Gfx *args, unsigned count, Yo3Gfx *out, char (*texts)[16], unsigned cap) {
    if (!name || strcmp(name, "call") || count < 3 || count > cap) return 0;
    const char *method = yo3_gfx_string(&args[0]);
    if (!method || strcmp(method, "setProp")) return 0;
    if ((args[1].type & 0x8f) != 3 || (args[2].type & 0x8f) != 3) return 0;
    double prop = args[1].v.number, value = args[2].v.number;
    int original = prop == 4 || prop == 5, current = prop == 6 || prop == 7;
    if (!original && !current) return 0;
    if (current && value < YO3_INFINITE_ATK) return 0;
    memcpy(out, args, count * sizeof *out);
    out[2].iface = NULL; out[2].type = 4;
    if (value >= YO3_INFINITE_ATK) out[2].v.text = yo3_infinity;
    else { snprintf(texts[2], 16, "%.0f", value); out[2].v.text = texts[2]; }
    return 1;
}
#endif
