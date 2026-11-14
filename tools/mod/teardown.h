#ifndef YO3_TEARDOWN_H
#define YO3_TEARDOWN_H
/* Installs teardown-guard.h: the 18 bytes at 0x50E632 become a call to a thunk (and NOPs) that does the same work
 * through yo3_teardown_release, with edi (DuelManager) as its argument and every register kept. Byte-guarded: another
 * executable refuses to start (stage=mod-teardown). Always on: the fault is the game's own; the higher resolution only
 * made it show (native Windows, closing mid-duel). */
#include "teardown-guard.h"

void __attribute__((cdecl)) teardown_release(BYTE *manager) { yo3_teardown_release(manager); }
void teardown_thunk(void);
__asm__(
    ".text\n"
    ".globl _teardown_thunk\n"
    "_teardown_thunk:\n"
    "    pushal\n"
    "    pushl %edi\n"
    "    call _teardown_release\n"
    "    addl $4,%esp\n"
    "    popal\n"
    "    ret\n");

static int teardown_install(void) {
    BYTE *base = (BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As deck-upload.h: the contract fixtures are small executables; the other guards have identified v1.153. */
    if (nt->OptionalHeader.SizeOfImage < 0x42e000) return 1;
    BYTE *site = (BYTE *)game_address(YO3_TEARDOWN_SITE);
    if (!yo3_teardown_ok(site, sizeof yo3_teardown_site)) return 0;
    BYTE code[sizeof yo3_teardown_site];
    memset(code, 0x90, sizeof code);
    code[0] = 0xe8; LONG to = (LONG)((BYTE *)teardown_thunk - (site + 5)); memcpy(code + 1, &to, 4);
    DWORD previous;
    if (!VirtualProtect(site, sizeof code, PAGE_EXECUTE_READWRITE, &previous)) return 0;
    memcpy(site, code, sizeof code);
    FlushInstructionCache(GetCurrentProcess(), site, sizeof code);
    DWORD ignored; return VirtualProtect(site, sizeof code, previous, &ignored) != 0;
}
#endif
