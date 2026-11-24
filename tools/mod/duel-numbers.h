#ifndef YO3_DUEL_NUMBERS_H
#define YO3_DUEL_NUMBERS_H
/* The infinite ATK display: an ATK or DEF of 9,999,999 or more (the engine's infinity) under a field card shows as "∞"
 * instead of its last five digits, 99999. The game's code talks to its Flash movies through one invoke, 0x5CB1B0
 * (thiscall movie, name, args, count); this detours it and hands the field info's setProp the rewritten arguments
 * (duel-numbers-logic.h). Every other call goes through untouched. The detour also reports each call, as the movie gets
 * it, to an observer: the UI probe's Flash log (ui-probe.h) watches through it.
 * Evidence: docs/client-internals.md, "Numbers in the duel: LP and ATK". */
#include "detour.h"
#include "gfx.h"
#include "duel-numbers-logic.h"

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

static int __attribute__((thiscall,force_align_arg_pointer)) numbers_invoke_hook(void *movie,const char *name,const void *args,unsigned count) {
    Yo3Gfx out[12]; char texts[12][16]; const char *field; int infinite;
    int scale=yo3_numbers_field(name,(const Yo3Gfx *)args,count,&field,&infinite);
    if(yo3_numbers_rewrite(name,(const Yo3Gfx *)args,count,out,texts,12)) args=out;
    if(yo3_invoke_observer) yo3_invoke_observer(name,args,count);
    int result=numbers_invoke(movie,name,args,count);
    if(scale) numbers_scale(movie,field,infinite);
    return result;
}

/* Never a reason to refuse the game: without it, an infinite ATK shows as 99999. */
static void duel_numbers_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return; /* contract fixtures: no game to look at */
    /* 0x5CB1B0: sub esp,0x38; push esi; mov esi,ecx. */
    static const BYTE invoke[]={0x83,0xec,0x38,0x56,0x8b,0xf1};
    duel_numbers_hooked=yo3_detour(0x5cb1b0,invoke,sizeof invoke,(void *)numbers_invoke_hook,(void **)&numbers_invoke);
}
#endif
