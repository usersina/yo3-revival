#ifndef YO3_GFX_H
#define YO3_GFX_H
/* The game's Flash (Scaleform GFx 3) values and movie calls, shared by the hooks that talk to its movies
 * (rental-return.h, duel-numbers.h). */
#include <windows.h>
#include <string.h>
#include "game-image.h"

/* A GFxValue (GFx 3): +4 type (0 undefined, 2 bool, 3 number, 4 char *, 6 object, 8 display object; 0x40 managed),
 * +8 the value. */
typedef struct { void *iface; DWORD type; union { double number; const char *text; void *data; BYTE flag; } v; } GfxValue;
_Static_assert(sizeof(GfxValue)==16,"GFxValue is 16 bytes");
/* GFxMovie's SetVariable (+0x40, as the game's own `_root.canbtn.disabled`) and GetVariable (+0x44, `_root.xpos`); the
 * movie root's Invoke 0x5CB1B0 (name, args, count), as FxDelegate's `call` and `respond`. */
static int gfx_set(void *movie,const char *path,GfxValue value) {
    return ((char (__attribute__((thiscall)) *)(void *,const char *,const GfxValue *,int))(*(DWORD **)movie)[0x40/4])
        (movie,path,&value,0)!=0;
}
static int gfx_get(void *movie,const char *path,GfxValue *out) {
    memset(out,0,sizeof *out);
    return ((char (__attribute__((thiscall)) *)(void *,GfxValue *,const char *))(*(DWORD **)movie)[0x44/4])(movie,out,path)!=0;
}
static void gfx_release(GfxValue *value) {
    if(value->type & 0x40)
        ((void (__attribute__((thiscall)) *)(void *,GfxValue *,void *))game_address(0x5c98c0))(value->iface,value,value->v.data);
    memset(value,0,sizeof *value);
}
static int gfx_invoke(void *movie,const char *name,const GfxValue *args,unsigned count) {
    return ((char (__attribute__((thiscall)) *)(void *,const char *,const GfxValue *,unsigned))game_address(0x5cb1b0))
        (movie,name,args,count)!=0;
}
static int gfx_number(void *movie,const char *path,double *out) {
    GfxValue value; int ok=gfx_get(movie,path,&value) && (value.type & 0x8f)==3;
    if(ok) *out=value.v.number;
    gfx_release(&value);
    return ok;
}
static GfxValue gfx_text(const char *text) { GfxValue v={0}; v.type=4; v.v.text=text; return v; }
static GfxValue gfx_num(double number) { GfxValue v={0}; v.type=3; v.v.number=number; return v; }
static GfxValue gfx_bool(int flag) { GfxValue v={0}; v.type=2; v.v.flag=flag!=0; return v; }

#endif
