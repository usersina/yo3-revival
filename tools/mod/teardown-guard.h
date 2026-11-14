#ifndef YO3_TEARDOWN_GUARD_H
#define YO3_TEARDOWN_GUARD_H
/* The game's exit, guarded where it writes into memory it has already released (teardown.h installs it; this part has
 * no game in it, so teardown-test.c can run it).
 *
 * At the end of WinMain the game destroys its components one by one. DuelManager's destructor (0x50E5C0) still marks
 * its duel component ([this+0x70], made per duel at 0x50F030) for removal: `mov byte [eax+0x28],1` at 0x50E639. By
 * then the pool that component lives in may already be decommitted: on native Windows 11 with the higher resolution,
 * closing the window mid-duel faulted there every time (eax in a MEM_RESERVE region, PAGE_NOACCESS; the stack ran back
 * to 0x401E95). The write is only bookkeeping for a component that is going away anyway; it is kept when the memory is
 * still there and skipped when it is not. */
#include <windows.h>

/* 0x50E632: mov eax,[edi+0x70]; test eax,eax; jz 0x50E644; mov byte [eax+0x28],1; mov dword [edi+0x70],0. */
static const BYTE yo3_teardown_site[] = {0x8b,0x47,0x70, 0x85,0xc0, 0x74,0x0b, 0xc6,0x40,0x28,0x01, 0xc7,0x47,0x70,0,0,0,0};
#define YO3_TEARDOWN_SITE 0x50e632u

static int yo3_teardown_ok(const BYTE *bytes, size_t length) {
    return length == sizeof yo3_teardown_site && !memcmp(bytes, yo3_teardown_site, sizeof yo3_teardown_site);
}

/* Whether one byte at `at` can be written: committed, writable and not a guard page. */
static int yo3_writable(const void *at) {
    MEMORY_BASIC_INFORMATION m;
    if (!VirtualQuery(at, &m, sizeof m) || m.State != MEM_COMMIT || (m.Protect & (PAGE_GUARD | PAGE_NOACCESS))) return 0;
    return (m.Protect & (PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY)) != 0;
}

/* What the site does, with the component's flag written only while its memory is there. */
static void yo3_teardown_release(BYTE *manager) {
    BYTE *component = *(BYTE **)(manager + 0x70);
    if (component && yo3_writable(component + 0x28)) component[0x28] = 1;
    *(DWORD *)(manager + 0x70) = 0;
}
#endif
