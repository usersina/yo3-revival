/* The infinite ATK display (duel-numbers-logic.h), without the game: which setProp calls are rewritten and how. */
#include <assert.h>
#include <stdio.h>
#include "duel-numbers-logic.h"

static Yo3Gfx text(const char *s) { Yo3Gfx v = {0}; v.type = 4; v.v.text = s; return v; }
static Yo3Gfx num(double n) { Yo3Gfx v = {0}; v.type = 3; v.v.number = n; return v; }

int main(void) {
    Yo3Gfx out[8]; char texts[8][16];
    /* An infinite ATK (prop 6) and DEF (7) become "∞". */
    Yo3Gfx atk[] = {text("setProp"), num(6), num(9999999), num(0), num(0), num(0)};
    assert(yo3_numbers_rewrite("call", atk, 6, out, texts, 8));
    assert(out[2].type == 4 && !strcmp(out[2].v.text, "\xe2\x88\x9e"));
    assert(out[1].type == 3 && out[1].v.number == 6 && out[3].type == 3);
    Yo3Gfx def[] = {text("setProp"), num(7), num(10000000)};
    assert(yo3_numbers_rewrite("call", def, 3, out, texts, 8) && !strcmp(out[2].v.text, yo3_infinity));
    /* An ordinary ATK goes as it came. */
    Yo3Gfx plain[] = {text("setProp"), num(6), num(4000)};
    assert(!yo3_numbers_rewrite("call", plain, 3, out, texts, 8));
    /* The originals (4, 5) always go as text: decimal, or "∞". */
    Yo3Gfx org[] = {text("setProp"), num(4), num(4000)};
    assert(yo3_numbers_rewrite("call", org, 3, out, texts, 8) && out[2].type == 4 && !strcmp(out[2].v.text, "4000"));
    Yo3Gfx negative[] = {text("setProp"), num(5), num(-2)};
    assert(yo3_numbers_rewrite("call", negative, 3, out, texts, 8) && !strcmp(out[2].v.text, "-2"));
    Yo3Gfx org_inf[] = {text("setProp"), num(4), num(9999999)};
    assert(yo3_numbers_rewrite("call", org_inf, 3, out, texts, 8) && !strcmp(out[2].v.text, yo3_infinity));
    /* Other props, other methods, other invokes, a method by reference (0x44), short or too long calls. */
    Yo3Gfx position[] = {text("setProp"), num(0), num(9999999), num(5)};
    assert(!yo3_numbers_rewrite("call", position, 4, out, texts, 8));
    Yo3Gfx other[] = {text("setLp"), num(6), num(9999999)};
    assert(!yo3_numbers_rewrite("call", other, 3, out, texts, 8));
    assert(!yo3_numbers_rewrite("respond", atk, 6, out, texts, 8));
    const char *name = "setProp"; Yo3Gfx ref = {0}; ref.type = 0x44; ref.v.ref = &name;
    Yo3Gfx by_ref[] = {ref, num(6), num(9999999)};
    assert(yo3_numbers_rewrite("call", by_ref, 3, out, texts, 8) && !strcmp(out[2].v.text, yo3_infinity));
    assert(!yo3_numbers_rewrite("call", atk, 2, out, texts, 8));
    assert(!yo3_numbers_rewrite("call", atk, 6, out, texts, 4));
    Yo3Gfx textual[] = {text("setProp"), num(6), text("12")};
    assert(!yo3_numbers_rewrite("call", textual, 3, out, texts, 8));
    /* 16 bytes each, as the game's values. */
    assert(sizeof(Yo3Gfx) == 16);
    puts("PASS: an infinite field ATK or DEF reaches the movie as \xe2\x88\x9e, its originals as text");
    return 0;
}
