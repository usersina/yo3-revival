#ifndef YO3_DUEL_NUMBERS_LOGIC_H
#define YO3_DUEL_NUMBERS_LOGIC_H
/* The pure part of the infinite ATK display (duel-numbers.h), tested without the game by duel-numbers-test.c.
 * Evidence: docs/client-internals.md, "Numbers in the duel: LP and ATK".
 *  - The ATK and DEF under a field card are text fields of data/gfx/duel_fieldinfo.swf bound to the card info's `atk`
 *    and `def`, which the game sets through FxDelegate's `call` ("setProp", prop, value), from 0x50C190: prop 6 the ATK,
 *    7 the DEF, 4 and 5 the original ATK and DEF. An ATK of 9,999,999 (the engine's infinity) showed as its last five
 *    digits, 99999.
 *  - So such an ATK or DEF, or any above the five digits the field shows, goes to the movie as the string "∞". Its colour compares it with the original (white when
 *    equal, blue above, red below): the original values always go as their decimal string, so that ActionScript 2
 *    (SWF 8) compares "∞" with them as strings, where "∞" (U+221E) is above every digit, and a number with them as
 *    numbers, as before. The movie does nothing else with them. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

/* A GFxValue (GFx 3): +4 type (3 number, 4 char *, 0x44 char ** ; 0x40 managed), +8 the value. */
typedef struct { void *iface; uint32_t type; union { double number; const char *text; const char **ref; void *data; } v; } Yo3Gfx;

#define YO3_INFINITE_ATK 9999999.0
/* Beyond what the fields show (five digits): "∞" too. The game counts an ATK up to infinity through such values. */
#define YO3_SHOWN_MAX 99999.0
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
    if (current && value <= YO3_SHOWN_MAX) return 0;
    memcpy(out, args, count * sizeof *out);
    out[2].iface = NULL; out[2].type = 4;
    if (value > YO3_SHOWN_MAX) out[2].v.text = yo3_infinity;
    else { snprintf(texts[2], 16, "%.0f", value); out[2].v.text = texts[2]; }
    return 1;
}

/* The infinite ATK's "∞" is drawn larger than the digits: its text field is scaled by this much (percent) while it shows
 * it, and back to 100 for a number. */
#define YO3_INFINITY_SCALE 180.0

/* The field a setProp call sets (the ATK's for prop 6, the DEF's for 7) and whether it now shows "∞": 1, or 0 for any
 * other call. */
static int yo3_numbers_field(const char *name, const Yo3Gfx *args, unsigned count, const char **path, int *infinite) {
    if (!name || strcmp(name, "call") || count < 3) return 0;
    const char *method = yo3_gfx_string(&args[0]);
    if (!method || strcmp(method, "setProp") || (args[1].type & 0x8f) != 3 || (args[2].type & 0x8f) != 3) return 0;
    if (args[1].v.number == 6) *path = "_root.current.txAtk";
    else if (args[1].v.number == 7) *path = "_root.current.txDef";
    else return 0;
    *infinite = args[2].v.number > YO3_SHOWN_MAX;
    return 1;
}

/* A text field's place for `to` percent, from its place and size now (`x`, `y`, `width`, `height` at `now` percent): it
 * grows from its top left, so it is moved to keep its middle (the fields' text is centred, DefineEditText align 2). */
typedef struct { double x, y; } Yo3Place;
static Yo3Place yo3_numbers_place(double x, double y, double width, double height, double now, double to) {
    double w = width * 100 / now, h = height * 100 / now;
    Yo3Place p = { x + width / 2 - w * to / 200, y + height / 2 - h * to / 200 };
    return p;
}
/* LP beyond the counter (Mystik Wok on an infinite ATK). The LP counter is data/gfx/duel_cockpit.swf's
 * `cockpit<player>.lpCounter`, a NumberSprite of five picture digits (the font sprites duel_font_b_00/01, 13x14 frames
 * num0..num9 and minus): it has no "∞". The server sends LP above 99,999 as 100,000 (wire.rs LP_BEYOND); set LP 0xF
 * stores it as sent and hands it on through FxDelegate's `call` ("setLp", player, lp, colour) from 0x5035A0, where the
 * counter would clamp it to 99999. Such an LP goes to the movie as 99999, and the DLL hides the digits and shows its own
 * picture of "∞" in their place (duel-numbers.h). 1 for a setLp call, with its player and LP. */
static int yo3_lp_call(const char *name, const Yo3Gfx *args, unsigned count, unsigned *player, double *lp) {
    if (!name || strcmp(name, "call") || count < 3) return 0;
    const char *method = yo3_gfx_string(&args[0]);
    if (!method || strcmp(method, "setLp") || (args[1].type & 0x8f) != 3 || (args[2].type & 0x8f) != 3) return 0;
    if (args[1].v.number != 0 && args[1].v.number != 1) return 0;
    *player = (unsigned)args[1].v.number;
    *lp = args[2].v.number;
    return 1;
}

/* Where the "∞" picture goes, in the cockpit's coordinates: centred on the counter's digits, at `scale` (the counter's
 * own, times YO3_LP_INFINITY_SCALE: the picture is drawn at four times the screen's size, so a 14-pixel digit is 56 of
 * its pixels). The counter at `x` lays its `figures` digits (`digit_w` apart, `digit_h` high from `top`) by its `anchor`
 * (NumberSprite.onFigureChanged): "right" ends them at `x`, "left" starts them there, "center" (the LP counter's, live)
 * centres them on it. `pic_w` x `pic_h` is the picture's size in pixels. */
#define YO3_LP_INFINITY_SCALE 0.25
typedef struct { double x, y, scale; } Yo3PicPlace;
static Yo3PicPlace yo3_lp_infinity_place(const char *anchor, double x, double top, double digit_w, double digit_h,
                                         unsigned figures, double pic_w, double pic_h, double counter_scale) {
    double scale = counter_scale * YO3_LP_INFINITY_SCALE, wide = digit_w * figures, centre = x;
    if (anchor && !strcmp(anchor, "right")) centre = x - wide / 2;
    else if (anchor && !strcmp(anchor, "left")) centre = x + wide / 2;
    Yo3PicPlace p = { centre - pic_w * scale / 2, top + digit_h / 2 - pic_h * scale / 2, scale * 100 };
    return p;
}
/* The damage pop-up (DuelCockpit 0x503AD0, the cockpit's `damage<player>`, a NumberAnimation of the animated fonts
 * duel_font_e_anim_00 for a gain and _01 for a loss, with a minus but no ∞): FxDelegate's `call` ("showDamage", player,
 * change, x, y) shows a change, ("setDamage", player, left) counts it down while the LP counter moves, and ("hideDamage",
 * player) ends it. The server sends an ∞ amount's change as ±100,000, more than any finite one it shows (wire.rs
 * LP_BEYOND); the DLL shows its own "-∞" or "∞" in place of the digits from such a showDamage to the next showDamage or
 * hideDamage. 1 show, 2 set, 3 hide, with the player and the change (0 for hide); 0 for any other call. */
#define YO3_DAMAGE_INFINITE 100000.0
static int yo3_damage_call(const char *name, const Yo3Gfx *args, unsigned count, unsigned *player, double *change) {
    if (!name || strcmp(name, "call") || count < 2) return 0;
    const char *method = yo3_gfx_string(&args[0]);
    if (!method || (args[1].type & 0x8f) != 3 || (args[1].v.number != 0 && args[1].v.number != 1)) return 0;
    int kind = !strcmp(method, "showDamage") ? 1 : !strcmp(method, "setDamage") ? 2 : !strcmp(method, "hideDamage") ? 3 : 0;
    if (!kind || (kind < 3 && (count < 3 || (args[2].type & 0x8f) != 3))) return 0;
    *player = (unsigned)args[1].v.number;
    *change = kind < 3 ? args[2].v.number : 0;
    return kind;
}

/* The pop-up's ∞ after a damage call of `kind` with `change`, from `now` (-1 a loss shown as ∞, 1 a gain, 0 none). */
static int yo3_damage_infinity(int now, int kind, double change) {
    if (kind == 1) return change <= -YO3_DAMAGE_INFINITE ? -1 : change >= YO3_DAMAGE_INFINITE ? 1 : 0;
    if (kind == 3) return 0;
    return now;
}
/* The card panel (data/gfx/infowin.swf): the game invokes `setDuelCardText(text, name, ..., atkdef, def text)` itself,
 * the tenth argument the line it builds as "ATK 19999998/DEF 0" (HTML), which the panel shows as it is. Any number of
 * 9,999,999 or more in it is the engine's one ∞ (guards.lua) and becomes "∞"; the rest of the text is left alone. Copies
 * `in` into `out` (`cap` characters with its end) rewritten: 1 when something changed. Narrow (UTF-8) and wide (UTF-16)
 * texts alike. */
#define YO3_PANEL_ATKDEF 9
/* Where the ATK/DEF line is in an invoke: FxDelegate's `call` ("setDuelCardText", ...) puts the method first (as the game
 * sends it, live), a direct invoke of the method would not. -1 for any other invoke. */
static int yo3_panel_call(const char *name, const Yo3Gfx *args, unsigned count) {
    if (!name) return -1;
    if (!strcmp(name, "setDuelCardText")) return count > YO3_PANEL_ATKDEF ? YO3_PANEL_ATKDEF : -1;
    if (strcmp(name, "call") || count <= YO3_PANEL_ATKDEF + 1) return -1;
    const char *method = yo3_gfx_string(&args[0]);
    return method && !strcmp(method, "setDuelCardText") ? YO3_PANEL_ATKDEF + 1 : -1;
}
#define YO3_PANEL_INFINITY(type, name, infinity_units, n_units) \
static __attribute__((unused)) int name(const type *in, type *out, size_t cap) { \
    static const type sign[] = infinity_units; \
    size_t o = 0; int changed = 0; \
    for (size_t i = 0; in[i];) { \
        if (in[i] >= '0' && in[i] <= '9') { \
            size_t j = i; double v = 0; \
            while (in[j] >= '0' && in[j] <= '9') v = v * 10 + (in[j++] - '0'); \
            if (v >= YO3_INFINITE_ATK) { \
                if (o + n_units >= cap) return 0; \
                for (size_t k = 0; k < n_units; ++k) out[o++] = sign[k]; \
                changed = 1; \
            } else { \
                if (o + (j - i) >= cap) return 0; \
                while (i < j) out[o++] = in[i++]; \
            } \
            i = j; \
        } else { \
            if (o + 1 >= cap) return 0; \
            out[o++] = in[i++]; \
        } \
    } \
    out[o] = 0; \
    return changed; \
}
#define YO3_UTF8_INFINITY { (char)0xe2, (char)0x88, (char)0x9e }
#define YO3_UTF16_INFINITY { (wchar_t)0x221e }
YO3_PANEL_INFINITY(char, yo3_panel_infinity, YO3_UTF8_INFINITY, 3)
YO3_PANEL_INFINITY(wchar_t, yo3_panel_infinity_w, YO3_UTF16_INFINITY, 1)
#endif
