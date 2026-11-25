#ifndef YO3_DUEL_NUMBERS_H
#define YO3_DUEL_NUMBERS_H
/* The infinite ATK, LP and damage display: an ATK or DEF of 9,999,999 or more (the engine's infinity) under a field card shows as "∞"
 * instead of its last five digits, 99999. The game's code talks to its Flash movies through one invoke, 0x5CB1B0
 * (thiscall movie, name, args, count); this detours it and hands the field info's setProp the rewritten arguments
 * (duel-numbers-logic.h). Every other call goes through untouched. The detour also reports each call, as the movie gets
 * it, to an observer: the UI probe's Flash log (ui-probe.h) watches through it.
 * Evidence: docs/client-internals.md, "Numbers in the duel: LP and ATK". */
#include "detour.h"
#include "gfx.h"
#include "duel-numbers-logic.h"
#include "infinity-pictures.h"

static int (__attribute__((thiscall)) *numbers_invoke)(void *,const char *,const void *,unsigned);
static void (*yo3_invoke_observer)(const char *,const void *,unsigned);
static int duel_numbers_hooked;

/* "∞" larger than the digits: the ATK's or DEF's text field scaled up while it shows it, and back for a number, kept in
 * place by its right edge and vertical middle (duel-numbers-logic.h). Read and set through the movie's GetVariable and
 * SetVariable; the movie's own script never changes a field's scale or place. */
static void numbers_scale(void *movie,const char *field,int infinite) {
    char path[64]; double now,x,y,width,height,to=infinite?YO3_INFINITY_SCALE:100;
#define NUMBERS_PATH(prop) (snprintf(path,sizeof path,"%s." prop,field),path)
    if(!gfx_number(movie,NUMBERS_PATH("_xscale"),&now) || now==to || now<=0) return;
    if(!gfx_number(movie,NUMBERS_PATH("_x"),&x) || !gfx_number(movie,NUMBERS_PATH("_y"),&y)
        || !gfx_number(movie,NUMBERS_PATH("_width"),&width) || !gfx_number(movie,NUMBERS_PATH("_height"),&height)) return;
    Yo3Place place=yo3_numbers_place(x,y,width,height,now,to);
    gfx_set(movie,NUMBERS_PATH("_xscale"),gfx_num(to));
    gfx_set(movie,NUMBERS_PATH("_yscale"),gfx_num(to));
    gfx_set(movie,NUMBERS_PATH("_x"),gfx_num(place.x));
    gfx_set(movie,NUMBERS_PATH("_y"),gfx_num(place.y));
#undef NUMBERS_PATH
}

/* The "∞" pictures the DLL carries (infinity-pictures.h, made by make-infinity-pictures.py from tools/mod/art/): each
 * is written once to the temporary folder and loaded into a clip of the cockpit through the movie's own loadMovie (the
 * game's Flash screens load PNG files so, e.g. the card list's thumbnails); the game's file system opens an absolute path
 * no archive holds as a file on disk. */
typedef struct { const char *name; const unsigned char *bytes; unsigned size; double w, h; char path[MAX_PATH * 2 + 64]; int tried; } Yo3Picture;
static Yo3Picture yo3_pictures[] = {
    { "lp-infinity", yo3_lp_infinity_png, sizeof yo3_lp_infinity_png, YO3_LP_INFINITY_W, YO3_LP_INFINITY_H, "", 0 },
    { "damage-infinity-loss", yo3_damage_loss_png, sizeof yo3_damage_loss_png, YO3_DAMAGE_LOSS_W, YO3_DAMAGE_LOSS_H, "", 0 },
    { "damage-infinity-gain", yo3_damage_gain_png, sizeof yo3_damage_gain_png, YO3_DAMAGE_GAIN_W, YO3_DAMAGE_GAIN_H, "", 0 },
};
enum { YO3_PIC_LP, YO3_PIC_LOSS, YO3_PIC_GAIN };
static const char *picture_path(Yo3Picture *p) {
    if (p->tried) return p->path[0] ? p->path : NULL;
    p->tried = 1;
    char folder[MAX_PATH * 2]; DWORD n = GetTempPathA(sizeof folder, folder);
    if (!n || n >= sizeof folder) return NULL;
    snprintf(p->path, sizeof p->path, "%syo3-revival-%s-%u.png", folder, p->name, p->size);
    HANDLE f = CreateFileA(p->path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD written = 0;
    BOOL ok = f != INVALID_HANDLE_VALUE && WriteFile(f, p->bytes, p->size, &written, NULL) && written == p->size;
    if (f != INVALID_HANDLE_VALUE) CloseHandle(f);
    if (!ok) {
        char line[160]; snprintf(line, sizeof line, "[YO3 MOD] infinity: could not write the picture %s, its number shows instead\r\n", p->name);
        starter_log(line); p->path[0] = 0; return NULL;
    }
    for (char *c = p->path; *c; ++c) if (*c == '\\') *c = '/';
    return p->path;
}

/* A NumberSprite's `anchor` ("center" when it can't be read). */
static void numbers_anchor(void *movie, const char *sprite, char *anchor, size_t size) {
    char path[128]; GfxValue got;
    snprintf(anchor, size, "center");
    snprintf(path, sizeof path, "%s.anchor", sprite);
    if (!gfx_get(movie, path, &got)) return;
    /* A string: its characters (type 4), or a managed one's by reference (0x44, as yo3_gfx_string reads it). */
    const char *text = (got.type & 0x8f) != 4 ? NULL : (got.type & 0x40) ? (got.v.data ? *(const char **)got.v.data : NULL) : got.v.text;
    if (text) snprintf(anchor, size, "%s", text);
    gfx_release(&got);
}

/* Loads picture `pic` into `clip`.pic and places `clip` over the digits of the NumberSprite `sprite`: its sibling
 * (`inside` 0: the sprite's place and scale in the parent count) or `clip`'s parent itself (`inside` 1). 1 when done. */
static int picture_load(void *movie, const char *clip, const char *sprite, int inside, Yo3Picture *pic) {
    char path[160], method[160], line[320];
#define PIC_PATH(base, prop) (snprintf(path, sizeof path, "%s." prop, base), path)
#define PIC_FAIL(why) do { snprintf(line, sizeof line, "[YO3 MOD] infinity: no %s over %s: %s\r\n", pic->name, sprite, why); starter_log(line); return 0; } while (0)
    const char *file = picture_path(pic);
    double x = 0, y = 0, scale = 100, width, top = 0, digit_h = 14, figures = 5; char anchor[16];
    if (!file) PIC_FAIL("no picture file");
    if (!gfx_number(movie, PIC_PATH(sprite, "fontWidth"), &width)) PIC_FAIL("no fontWidth");
    if (!inside && (!gfx_number(movie, PIC_PATH(sprite, "_x"), &x) || !gfx_number(movie, PIC_PATH(sprite, "_y"), &y)
        || !gfx_number(movie, PIC_PATH(sprite, "_xscale"), &scale))) PIC_FAIL("no place");
    gfx_number(movie, PIC_PATH(sprite, "num0._y"), &top);
    gfx_number(movie, PIC_PATH(sprite, "num0._height"), &digit_h);
    gfx_number(movie, PIC_PATH(sprite, "_figure"), &figures);
    numbers_anchor(movie, sprite, anchor, sizeof anchor);
    GfxValue load[1] = { gfx_text(file) };
    snprintf(method, sizeof method, "%s.pic.loadMovie", clip);
    if (!gfx_invoke(movie, method, load, 1)) PIC_FAIL("loadMovie failed");
    double k = scale / 100;
    Yo3PicPlace place = yo3_lp_infinity_place(anchor, x, y + top * k, width * k, digit_h * k, (unsigned)figures, pic->w, pic->h, k);
    gfx_set(movie, PIC_PATH(clip, "_x"), gfx_num(place.x));
    gfx_set(movie, PIC_PATH(clip, "_y"), gfx_num(place.y));
    gfx_set(movie, PIC_PATH(clip, "_xscale"), gfx_num(place.scale));
    gfx_set(movie, PIC_PATH(clip, "_yscale"), gfx_num(place.scale));
    snprintf(line, sizeof line, "[YO3 MOD] infinity: %s over %s (at %.1f,%.1f scale %.0f anchor %s, digits %.1f wide %.1f high from %.1f, %d of them): %.1f,%.1f %.1f%%\r\n",
             pic->name, sprite, x, y, scale, anchor, width, digit_h, top, (int)figures, place.x, place.y, place.scale);
    starter_log(line);
#undef PIC_FAIL
#undef PIC_PATH
    return 1;
}

/* Makes `parent`.`name` (depth `depth`) with an empty `pic` inside, when it isn't there: 1 when it is there now, with
 * `*made` 1 when it was just made. */
static int picture_clip(void *movie, const char *parent, const char *name, int depth, int *made) {
    char path[160], method[160]; double v;
    snprintf(path, sizeof path, "%s.%s._x", parent, name);
    *made = 0;
    if (gfx_number(movie, path, &v)) return 1;
    GfxValue make[2] = { gfx_text(name), gfx_num(depth) }, inner[2] = { gfx_text("pic"), gfx_num(1) };
    snprintf(method, sizeof method, "%s.createEmptyMovieClip", parent);
    if (!gfx_invoke(movie, method, make, 2)) {
        char line[200]; snprintf(line, sizeof line, "[YO3 MOD] infinity: no clip %s.%s: createEmptyMovieClip failed\r\n", parent, name);
        starter_log(line); return 0;
    }
    snprintf(method, sizeof method, "%s.%s.createEmptyMovieClip", parent, name);
    gfx_invoke(movie, method, inner, 2);
    *made = 1;
    return 1;
}

/* The LP "∞" (duel-numbers-logic.h, yo3_lp_call): a clip of the cockpit, `yo3Infinity` beside the counter, shown and the
 * counter hidden while the LP is beyond 99,999; the other way round otherwise. */
static void lp_show(void *movie, unsigned player, int infinite) {
    char cockpit[32], counter[64], clip[64], path[128]; double v;
    snprintf(cockpit, sizeof cockpit, "_root.cockpit%u", player);
    snprintf(counter, sizeof counter, "%s.lpCounter", cockpit);
    snprintf(clip, sizeof clip, "%s.yo3Infinity", cockpit);
#define LP_PATH(base, prop) (snprintf(path, sizeof path, "%s." prop, base), path)
    if (!infinite) {
        if (gfx_number(movie, LP_PATH(clip, "_x"), &v)) { gfx_set(movie, LP_PATH(clip, "_visible"), gfx_bool(0)); gfx_set(movie, LP_PATH(counter, "_visible"), gfx_bool(1)); }
        return;
    }
    int made;
    if (!picture_clip(movie, cockpit, "yo3Infinity", 16000 + player, &made)) return;
    if (made && !picture_load(movie, clip, counter, 0, &yo3_pictures[YO3_PIC_LP])) return;
    gfx_set(movie, LP_PATH(clip, "_visible"), gfx_bool(1));
    gfx_set(movie, LP_PATH(counter, "_visible"), gfx_bool(0));
#undef LP_PATH
}

/* The damage pop-up's "∞" (duel-numbers-logic.h, yo3_damage_call): one clip inside the pop-up (`damage<player>`, which
 * the movie moves and animates), `yo3Infinity`, holding the loss or the gain picture (loaded again, and placed again, when
 * the sign changes), shown and the digits made transparent while the pop-up shows an ∞ amount (`_alpha`: the pop-up
 * reveals its digits one by one through `_visible`); the digits back and the clip hidden otherwise. */
static int damage_infinity[2], damage_applied[2], damage_loaded[2];
static void damage_show(void *movie, unsigned player, int sign) {
    char popup[32], clip[48], path[128]; double v; int made;
    snprintf(popup, sizeof popup, "_root.damage%u", player);
    snprintf(clip, sizeof clip, "%s.yo3Infinity", popup);
#define DMG_PATH(fmt, ...) (snprintf(path, sizeof path, fmt, __VA_ARGS__), path)
    if (!sign && !damage_applied[player]) return;
    if (sign) {
        if (!picture_clip(movie, popup, "yo3Infinity", 16100, &made)) return;
        if ((made || damage_loaded[player] != sign)
            && !picture_load(movie, clip, popup, 1, &yo3_pictures[sign < 0 ? YO3_PIC_LOSS : YO3_PIC_GAIN])) return;
        damage_loaded[player] = sign;
    }
    for (int i = 0; i < 8; ++i)
        if (gfx_number(movie, DMG_PATH("%s.num%d._alpha", popup, i), &v)) gfx_set(movie, path, gfx_num(sign ? 0 : 100));
    if (gfx_number(movie, DMG_PATH("%s._x", clip), &v)) gfx_set(movie, DMG_PATH("%s._visible", clip), gfx_bool(sign != 0));
    damage_applied[player] = sign != 0;
#undef DMG_PATH
}

static int __attribute__((thiscall,force_align_arg_pointer)) numbers_invoke_hook(void *movie,const char *name,const void *args,unsigned count) {
    Yo3Gfx out[12]; char texts[12][16]; const char *field=NULL; int infinite=0;
    unsigned player=0, popup=0; double lp=0, change=0;
    int scale=yo3_numbers_field(name,(const Yo3Gfx *)args,count,&field,&infinite);
    int lp_call=yo3_lp_call(name,(const Yo3Gfx *)args,count,&player,&lp);
    int damage=yo3_damage_call(name,(const Yo3Gfx *)args,count,&popup,&change);
    if(yo3_numbers_rewrite(name,(const Yo3Gfx *)args,count,out,texts,12)) args=out;
    else if(lp_call && lp>YO3_SHOWN_MAX && count<=12) {
        memcpy(out,args,count*sizeof *out); out[2].v.number=YO3_SHOWN_MAX; args=out;
    }
    else if(damage && change*change>=YO3_DAMAGE_INFINITE*YO3_DAMAGE_INFINITE && count<=12) {
        memcpy(out,args,count*sizeof *out); out[2].v.number=change<0?-YO3_SHOWN_MAX:YO3_SHOWN_MAX; args=out;
    }
    if(damage) damage_infinity[popup]=yo3_damage_infinity(damage_infinity[popup],damage,change);
    if(yo3_invoke_observer) yo3_invoke_observer(name,args,count);
    int result=numbers_invoke(movie,name,args,count);
    if(scale) numbers_scale(movie,field,infinite);
    if(lp_call) lp_show(movie,player,lp>YO3_SHOWN_MAX);
    if(damage) damage_show(movie,popup,damage_infinity[popup]);
    return result;
}

/* Change LP (DuelCockpit 0x502EB0, command 0x10) clamps the LP it adds a change to at 99,999: `cmp edi, 99999` at
 * 0x50301D, `mov edi, 99999` at 0x503025 (the binary's only 99999). Both become 100,000 (wire.rs LP_BEYOND), so a
 * change that takes the LP beyond the counter lands on that value, "∞", not on 99999 (the server sends nothing for a
 * change that leaves it beyond). 1 when patched. */
static int lp_clamp_raise(void) {
    static const BYTE cmp[]={0x81,0xff,0x9f,0x86,0x01,0x00}, mov[]={0xbf,0x9f,0x86,0x01,0x00};
    BYTE *at_cmp=(BYTE *)game_address(0x50301d), *at_mov=(BYTE *)game_address(0x503025);
    if(memcmp(at_cmp,cmp,sizeof cmp) || memcmp(at_mov,mov,sizeof mov)) return 0;
    const DWORD beyond=100000; DWORD previous,ignored;
    if(!VirtualProtect(at_cmp,16,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    memcpy(at_cmp+2,&beyond,4); memcpy(at_mov+1,&beyond,4);
    VirtualProtect(at_cmp,16,previous,&ignored);
    FlushInstructionCache(GetCurrentProcess(),at_cmp,16);
    return 1;
}

/* Never a reason to refuse the game: without it, an infinite ATK and LP beyond 99,999 show as 99999. */
static void duel_numbers_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return; /* contract fixtures: no game to look at */
    /* 0x5CB1B0: sub esp,0x38; push esi; mov esi,ecx. */
    static const BYTE invoke[]={0x83,0xec,0x38,0x56,0x8b,0xf1};
    duel_numbers_hooked=yo3_detour(0x5cb1b0,invoke,sizeof invoke,(void *)numbers_invoke_hook,(void **)&numbers_invoke);
    if(duel_numbers_hooked && !lp_clamp_raise()) starter_log("[YO3 MOD] LP infinity: change LP's clamp is not the expected code, LP beyond 99999 may show 99999 after a change\r\n");
}
#endif
