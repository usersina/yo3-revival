#ifndef YO3_DUEL_NUMBERS_H
#define YO3_DUEL_NUMBERS_H
/* The infinite ATK and LP display: an ATK or DEF of 9,999,999 or more (the engine's infinity) under a field card shows as "∞"
 * instead of its last five digits, 99999. The game's code talks to its Flash movies through one invoke, 0x5CB1B0
 * (thiscall movie, name, args, count); this detours it and hands the field info's setProp the rewritten arguments
 * (duel-numbers-logic.h). Every other call goes through untouched. The detour also reports each call, as the movie gets
 * it, to an observer: the UI probe's Flash log (ui-probe.h) watches through it.
 * Evidence: docs/client-internals.md, "Numbers in the duel: LP and ATK". */
#include "detour.h"
#include "gfx.h"
#include "duel-numbers-logic.h"
#include "lp-infinity-png.h"

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

/* The LP "∞" (duel-numbers-logic.h, yo3_lp_call): the picture the DLL carries (lp-infinity-png.h, made by
 * make-lp-infinity.py) is written once to the temporary folder and loaded into a clip of the cockpit, `yo3Infinity`
 * beside the counter, through the movie's own loadMovie (the game's Flash screens load PNG files so, e.g. the card list's
 * thumbnails); the game's file system opens an absolute path no archive holds as a file on disk. The clip is placed
 * over the digits and shown, and the counter hidden, while the LP is beyond 99,999; the other way round otherwise. */
static char lp_picture[MAX_PATH * 2 + 64];
static int lp_picture_tried, lp_logged;
static const char *lp_picture_path(void) {
    if (lp_picture_tried) return lp_picture[0] ? lp_picture : NULL;
    lp_picture_tried = 1;
    char folder[MAX_PATH * 2]; DWORD n = GetTempPathA(sizeof folder, folder);
    if (!n || n >= sizeof folder) return NULL;
    snprintf(lp_picture, sizeof lp_picture, "%syo3-revival-lp-infinity-%u.png", folder, (unsigned)sizeof yo3_lp_infinity_png);
    HANDLE f = CreateFileA(lp_picture, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD written = 0;
    BOOL ok = f != INVALID_HANDLE_VALUE && WriteFile(f, yo3_lp_infinity_png, sizeof yo3_lp_infinity_png, &written, NULL)
        && written == sizeof yo3_lp_infinity_png;
    if (f != INVALID_HANDLE_VALUE) CloseHandle(f);
    if (!ok) { starter_log("[YO3 MOD] LP infinity: could not write its picture, LP beyond 99999 shows 99999\r\n"); lp_picture[0] = 0; return NULL; }
    for (char *c = lp_picture; *c; ++c) if (*c == '\\') *c = '/';
    return lp_picture;
}

static void lp_show(void *movie, unsigned player, int infinite) {
    char counter[64], clip[64], path[128]; double v;
    snprintf(counter, sizeof counter, "_root.cockpit%u.lpCounter", player);
    snprintf(clip, sizeof clip, "_root.cockpit%u.yo3Infinity", player);
#define LP_PATH(base, prop) (snprintf(path, sizeof path, "%s." prop, base), path)
    int exists = gfx_number(movie, LP_PATH(clip, "_x"), &v);
    if (!infinite) {
        if (exists) { gfx_set(movie, LP_PATH(clip, "_visible"), gfx_bool(0)); gfx_set(movie, LP_PATH(counter, "_visible"), gfx_bool(1)); }
        return;
    }
    if (!exists) {
        const char *file = lp_picture_path();
        double x, y, scale, width, top = 0, digit_h = 14, figures = 5; char anchor[16] = "center"; GfxValue got;
        if (!file || !gfx_number(movie, LP_PATH(counter, "_x"), &x) || !gfx_number(movie, LP_PATH(counter, "_y"), &y)
            || !gfx_number(movie, LP_PATH(counter, "_xscale"), &scale) || !gfx_number(movie, LP_PATH(counter, "fontWidth"), &width)) return;
        gfx_number(movie, LP_PATH(counter, "num0._y"), &top);
        gfx_number(movie, LP_PATH(counter, "num0._height"), &digit_h);
        gfx_number(movie, LP_PATH(counter, "_figure"), &figures);
        if (gfx_get(movie, LP_PATH(counter, "anchor"), &got)) {
            /* A string: its characters (type 4), or a managed one's by reference (0x44, as yo3_gfx_string reads it). */
            const char *text = (got.type & 0x8f) != 4 ? NULL : (got.type & 0x40) ? (got.v.data ? *(const char **)got.v.data : NULL) : got.v.text;
            if (text) snprintf(anchor, sizeof anchor, "%s", text);
            gfx_release(&got);
        }
        GfxValue make[2] = { gfx_text("yo3Infinity"), gfx_num(16000 + player) };
        GfxValue inner[2] = { gfx_text("pic"), gfx_num(1) };
        GfxValue load[1] = { gfx_text(file) };
        char method[96];
        snprintf(method, sizeof method, "_root.cockpit%u.createEmptyMovieClip", player);
        if (!gfx_invoke(movie, method, make, 2)) return;
        snprintf(method, sizeof method, "%s.createEmptyMovieClip", clip); gfx_invoke(movie, method, inner, 2);
        snprintf(method, sizeof method, "%s.pic.loadMovie", clip); gfx_invoke(movie, method, load, 1);
        double k = scale / 100;
        Yo3PicPlace place = yo3_lp_infinity_place(anchor, x, y + top * k, width * k, digit_h * k, (unsigned)figures,
                                                  YO3_LP_INFINITY_W, YO3_LP_INFINITY_H, k);
        gfx_set(movie, LP_PATH(clip, "_x"), gfx_num(place.x));
        gfx_set(movie, LP_PATH(clip, "_y"), gfx_num(place.y));
        gfx_set(movie, LP_PATH(clip, "_xscale"), gfx_num(place.scale));
        gfx_set(movie, LP_PATH(clip, "_yscale"), gfx_num(place.scale));
        if (!lp_logged++) {
            char line[256];
            snprintf(line, sizeof line, "[YO3 MOD] LP infinity: counter at %.1f,%.1f scale %.0f anchor %s, digits %.1f wide %.1f high from %.1f, %d of them; picture %s at %.1f,%.1f %.1f%%\r\n",
                     x, y, scale, anchor, width, digit_h, top, (int)figures, file, place.x, place.y, place.scale);
            starter_log(line);
        }
    }
    gfx_set(movie, LP_PATH(clip, "_visible"), gfx_bool(1));
    gfx_set(movie, LP_PATH(counter, "_visible"), gfx_bool(0));
#undef LP_PATH
}

static int __attribute__((thiscall,force_align_arg_pointer)) numbers_invoke_hook(void *movie,const char *name,const void *args,unsigned count) {
    Yo3Gfx out[12]; char texts[12][16]; const char *field; int infinite;
    unsigned player; double lp;
    int scale=yo3_numbers_field(name,(const Yo3Gfx *)args,count,&field,&infinite);
    int lp_call=yo3_lp_call(name,(const Yo3Gfx *)args,count,&player,&lp);
    if(yo3_numbers_rewrite(name,(const Yo3Gfx *)args,count,out,texts,12)) args=out;
    else if(lp_call && lp>YO3_SHOWN_MAX && count<=12) {
        memcpy(out,args,count*sizeof *out); out[2].v.number=YO3_SHOWN_MAX; args=out;
    }
    if(yo3_invoke_observer) yo3_invoke_observer(name,args,count);
    int result=numbers_invoke(movie,name,args,count);
    if(scale) numbers_scale(movie,field,infinite);
    if(lp_call) lp_show(movie,player,lp>YO3_SHOWN_MAX);
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
