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
    /* Above the five digits shown (the game counts an ATK up to infinity through these): "∞" too; 99,999 itself not. */
    Yo3Gfx counting[] = {text("setProp"), num(6), num(9999988)};
    assert(yo3_numbers_rewrite("call", counting, 3, out, texts, 8) && !strcmp(out[2].v.text, yo3_infinity));
    Yo3Gfx edge[] = {text("setProp"), num(6), num(99999)};
    assert(!yo3_numbers_rewrite("call", edge, 3, out, texts, 8));
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
    /* The field each setProp sets, and whether it shows "∞". */
    const char *path = NULL; int infinite = -1;
    assert(yo3_numbers_field("call", atk, 6, &path, &infinite) && !strcmp(path, "_root.current.txAtk") && infinite == 1);
    assert(yo3_numbers_field("call", plain, 3, &path, &infinite) && !strcmp(path, "_root.current.txAtk") && infinite == 0);
    assert(yo3_numbers_field("call", def, 3, &path, &infinite) && !strcmp(path, "_root.current.txDef") && infinite == 1);
    assert(!yo3_numbers_field("call", org, 3, &path, &infinite) && !yo3_numbers_field("call", position, 4, &path, &infinite));
    assert(!yo3_numbers_field("call", other, 3, &path, &infinite) && !yo3_numbers_field("respond", atk, 6, &path, &infinite));
    /* Scaled up from 100 and back: the middle stays where it was. */
    Yo3Place up = yo3_numbers_place(10, 20, 40, 14, 100, 140);
    assert(up.x == 10 + 20 - 28 && up.y == 20 + 7 - 9.8);
    Yo3Place back = yo3_numbers_place(up.x, up.y, 56, 19.6, 140, 100);
    assert(back.x > 9.999 && back.x < 10.001 && back.y > 19.999 && back.y < 20.001);
    Yo3Place same = yo3_numbers_place(10, 20, 40, 14, 100, 100);
    assert(same.x == 10 && same.y == 20);
    /* 16 bytes each, as the game's values. */
    assert(sizeof(Yo3Gfx) == 16);
    puts("PASS: an infinite field ATK or DEF reaches the movie as \xe2\x88\x9e, its originals as text");
    return 0;
}
