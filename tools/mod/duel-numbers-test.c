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
    /* setLp (player, LP, colour): recognised with its player and LP; anything else not. */
    unsigned player = 9; double lp = -1;
    Yo3Gfx set_lp[] = {text("setLp"), num(1), num(100000), num(0)};
    assert(yo3_lp_call("call", set_lp, 4, &player, &lp) && player == 1 && lp == 100000);
    Yo3Gfx set_lp_small[] = {text("setLp"), num(0), num(8000)};
    assert(yo3_lp_call("call", set_lp_small, 3, &player, &lp) && player == 0 && lp == 8000);
    Yo3Gfx set_lp_bad[] = {text("setLp"), num(2), num(8000)};
    assert(!yo3_lp_call("call", set_lp_bad, 3, &player, &lp));
    Yo3Gfx set_lp_color[] = {text("setLpColor"), num(0), num(1)};
    assert(!yo3_lp_call("call", set_lp_color, 3, &player, &lp) && !yo3_lp_call("respond", set_lp, 4, &player, &lp));
    assert(!yo3_lp_call("call", set_lp, 2, &player, &lp) && !yo3_lp_call("call", atk, 6, &player, &lp));
    /* The picture at a quarter of its pixels (116x56 shows 29x14, a digit's height), centred on the digits by the
     * counter's anchor; a counter scaled 200% shows it at half its pixels. */
    Yo3PicPlace pic = yo3_lp_infinity_place("center", 32, 27, 13, 14, 4, 116, 56, 1);
    assert(pic.scale == 25 && pic.y == 27 && pic.x == 32 - 14.5);
    pic = yo3_lp_infinity_place("right", 100, 10, 13, 14, 4, 116, 56, 1);
    assert(pic.x == 100 - 26 - 14.5 && pic.y == 10);
    pic = yo3_lp_infinity_place("left", 100, 10, 13, 14, 4, 116, 56, 1);
    assert(pic.x == 100 + 26 - 14.5);
    assert(yo3_lp_infinity_place(NULL, 32, 27, 13, 14, 4, 116, 56, 1).x == 32 - 14.5);
    pic = yo3_lp_infinity_place("center", 32, 27, 26, 28, 4, 116, 56, 2);
    assert(pic.scale == 50 && pic.x == 32 - 29 && pic.y == 27);
    /* A picture with room around the sign (160x64) is shown at a quarter too, its middle on the digits' middle. */
    pic = yo3_lp_infinity_place("center", 32, 27, 13, 14, 4, 160, 64, 1);
    assert(pic.scale == 25 && pic.x == 12 && pic.y == 26);
    /* The damage pop-up's calls, and its ∞ from a showDamage of ±100,000 or more to the next showDamage or hideDamage. */
    double change = 0;
    Yo3Gfx show_loss[] = {text("showDamage"), num(1), num(-100000), num(300), num(200)};
    assert(yo3_damage_call("call", show_loss, 5, &player, &change) == 1 && player == 1 && change == -100000);
    Yo3Gfx set_left[] = {text("setDamage"), num(0), num(38682)};
    assert(yo3_damage_call("call", set_left, 3, &player, &change) == 2 && player == 0 && change == 38682);
    Yo3Gfx hide[] = {text("hideDamage"), num(1)};
    assert(yo3_damage_call("call", hide, 2, &player, &change) == 3 && player == 1 && change == 0);
    assert(!yo3_damage_call("call", set_lp, 4, &player, &change) && !yo3_damage_call("respond", show_loss, 5, &player, &change));
    Yo3Gfx show_bad[] = {text("showDamage"), num(2), num(-100000)};
    assert(!yo3_damage_call("call", show_bad, 3, &player, &change) && !yo3_damage_call("call", show_loss, 2, &player, &change));
    assert(yo3_damage_infinity(0, 1, -100000) == -1 && yo3_damage_infinity(0, 1, 100000) == 1);
    assert(yo3_damage_infinity(0, 1, -99999) == 0 && yo3_damage_infinity(1, 1, 500) == 0);
    assert(yo3_damage_infinity(-1, 2, 38682) == -1 && yo3_damage_infinity(0, 2, 38682) == 0);
    assert(yo3_damage_infinity(-1, 3, 0) == 0 && yo3_damage_infinity(1, 3, 0) == 0);
    /* 16 bytes each, as the game's values. */
    assert(sizeof(Yo3Gfx) == 16);
    puts("PASS: an infinite field ATK or DEF reaches the movie as \xe2\x88\x9e, its originals as text; setLp and the damage pop-up are recognised");
    return 0;
}
