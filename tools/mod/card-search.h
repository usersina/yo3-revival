#ifndef YO3_CARD_SEARCH_H
#define YO3_CARD_SEARCH_H
/* Issue #54: the card search ignores case. Edit Deck's Search Text and the card lists at Blister's (trade and rental)
 * both call TextSearchExecute (0x4955C0, 0x466FA0), which filters the list shown with 0x416C10: a card stays when the
 * text is in its name (0x5374B0, or its reading 0x537570) or, with Card Text ticked, its text (0x537510), compared by
 * msvcrt's wcsstr. So "red" finds nothing while "Red" finds Red-Eyes. The matcher loads wcsstr into edi twice
 * (0x416D40, 0x416DDD: `mov edi, [0x758580]`) and is called from nowhere else; both loads are redirected to
 * yo3_wcsistr (card-search-logic.h). A search still filters the list shown, as Konami's did: Reset shows every card.
 * Live: docs/client-internals.md "Card search". */
#include "card-search-logic.h"

static const DWORD yo3_card_search_loads[]={0x16d40,0x16ddd};

static int card_search_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As deck-upload.h: the contract fixtures keep their own code at this address. */
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return 1;
    unsigned long iat=(unsigned long)(uintptr_t)game_address(0x758580);
    for(size_t i=0;i<sizeof yo3_card_search_loads/sizeof *yo3_card_search_loads;++i)
        if(!yo3_card_search_load_ok(base+yo3_card_search_loads[i],iat)) return 0;
    for(size_t i=0;i<sizeof yo3_card_search_loads/sizeof *yo3_card_search_loads;++i) {
        BYTE *site=base+yo3_card_search_loads[i]; DWORD previous, ignored;
        if(!VirtualProtect(site,6,PAGE_EXECUTE_READWRITE,&previous)) return 0;
        yo3_card_search_load(site,(unsigned long)(uintptr_t)yo3_wcsistr);
        FlushInstructionCache(GetCurrentProcess(),site,6);
        if(!VirtualProtect(site,6,previous,&ignored)) return 0;
    }
    return 1;
}
#endif
