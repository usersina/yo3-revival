/* The rest of WINMM, forwarded to the system's winmm.dll (#41).
 *
 * Windows looks for winmm.dll in the game folder first, for the game and for Windows' own DLLs in its process alike.
 * On Windows 10 1809, DSOUND.dll imports waveInGetDevCapsW from it, so a loader with only the game's seven functions
 * stopped the process at startup (0xc0000139). Each export in winmm-exports.h is a jump through its own slot. The slot
 * first points at a resolver that looks the function up in the system's winmm.dll (loader.c, on first use, never in
 * DllMain), stores it and jumps there, so later calls go straight through. A jump keeps the caller's stack and
 * registers as they were, whatever the function's arguments.
 */
#include <windows.h>

FARPROC yo3_winmm_resolve(const char *name, FARPROC *slot);

__asm__(
    ".text\n"
    /* Stack: the name, the slot, the caller's return address, its arguments. ecx and edx are saved for the call. */
    "_yo3_forward_common:\n"
    "\tpushl %ecx\n"
    "\tpushl %edx\n"
    "\tpushl 12(%esp)\n"
    "\tpushl 12(%esp)\n"
    "\tcall _yo3_winmm_resolve\n"
    "\taddl $8, %esp\n"
    "\tpopl %edx\n"
    "\tpopl %ecx\n"
    "\taddl $8, %esp\n"
    "\tjmp *%eax\n");

#define FORWARD(name) \
    extern char yo3_resolve_##name[]; \
    __attribute__((used)) const char yo3_name_##name[] = #name; \
    __attribute__((used)) FARPROC yo3_slot_##name = (FARPROC)yo3_resolve_##name; \
    __asm__(".text\n" \
        ".globl _yo3_fwd_" #name "\n_yo3_fwd_" #name ":\n\tjmp *_yo3_slot_" #name "\n" \
        "_yo3_resolve_" #name ":\n\tpushl $_yo3_slot_" #name "\n\tpushl $_yo3_name_" #name "\n\tjmp _yo3_forward_common\n" \
        ".section .drectve\n\t.ascii \" -export:" #name "=yo3_fwd_" #name "\"\n" \
        ".text\n");
#include "winmm-exports.h"
