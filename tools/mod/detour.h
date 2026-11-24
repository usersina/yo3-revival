#ifndef YO3_DETOUR_H
#define YO3_DETOUR_H
/* Detouring a game function by its first bytes, shared by the hooks that wrap one (ui-probe.h, duel-numbers.h). */
#include <windows.h>
#include <string.h>
#include "game-image.h"

/* Detours `address` when its first bytes are `prologue` (`n` ≥ 5 bytes, no relative addresses): a jump to `hook`,
 * and `*original` calls the copied bytes and jumps back. */
static int yo3_detour(uint32_t address,const BYTE *prologue,size_t n,void *hook,void **original) {
    BYTE *site=(BYTE *)game_address(address);
    if(n<5 || n>16 || memcmp(site,prologue,n)) return 0;
    BYTE *trampoline=VirtualAlloc(NULL,32,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);
    if(!trampoline) return 0;
    memcpy(trampoline,prologue,n);
    trampoline[n]=0xe9; LONG back=(LONG)((site+n)-(trampoline+n+5)); memcpy(trampoline+n+1,&back,4);
    FlushInstructionCache(GetCurrentProcess(),trampoline,32);
    *original=trampoline;
    DWORD previous; if(!VirtualProtect(site,n,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    BYTE jump[16]; memset(jump,0x90,sizeof jump); jump[0]=0xe9;
    LONG to=(LONG)((BYTE *)hook-(site+5)); memcpy(jump+1,&to,4);
    memcpy(site,jump,n);
    FlushInstructionCache(GetCurrentProcess(),site,n);
    DWORD ignored; VirtualProtect(site,n,previous,&ignored);
    return 1;
}

#endif
