/* The exit guard (teardown-guard.h), without the game: DuelManager's duel component is flagged only while its memory is
 * there, and the manager forgets it either way. */
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "teardown-guard.h"

int main(void) {
    BYTE site[sizeof yo3_teardown_site];
    memcpy(site, yo3_teardown_site, sizeof site);
    assert(yo3_teardown_ok(site, sizeof site));
    assert(!yo3_teardown_ok(site, sizeof site - 1));
    site[10] = 0x02; /* mov byte [eax+0x28],2: another executable */
    assert(!yo3_teardown_ok(site, sizeof site));

    SYSTEM_INFO info; GetSystemInfo(&info);
    BYTE manager[0xd0]; memset(manager, 0, sizeof manager);

    /* A live component: flagged for removal, forgotten. */
    BYTE *live = VirtualAlloc(NULL, info.dwPageSize, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
    assert(live);
    *(BYTE **)(manager + 0x70) = live;
    yo3_teardown_release(manager);
    assert(live[0x28] == 1 && *(DWORD *)(manager + 0x70) == 0);

    /* Its pool decommitted first (the game's exit on native Windows): no write, no fault, forgotten. */
    BYTE *gone = VirtualAlloc(NULL, info.dwPageSize, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
    assert(gone && VirtualFree(gone, info.dwPageSize, MEM_DECOMMIT));
    *(BYTE **)(manager + 0x70) = gone;
    yo3_teardown_release(manager);
    assert(*(DWORD *)(manager + 0x70) == 0);

    /* Memory that is there but can't be written: left alone. */
    BYTE *fixed = VirtualAlloc(NULL, info.dwPageSize, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
    DWORD previous; assert(fixed && VirtualProtect(fixed, info.dwPageSize, PAGE_READONLY, &previous));
    *(BYTE **)(manager + 0x70) = fixed;
    yo3_teardown_release(manager);
    assert(fixed[0x28] == 0 && *(DWORD *)(manager + 0x70) == 0);

    /* No component: nothing to flag. */
    yo3_teardown_release(manager);
    assert(*(DWORD *)(manager + 0x70) == 0);

    puts("PASS: the exit guard flags a live duel component and skips a released one");
    return 0;
}
