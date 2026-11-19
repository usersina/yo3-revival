/* The card search's matcher (card-search-logic.h), without the game: wcsstr ignoring case, and the instruction swap. */
#include <assert.h>
#include <stdio.h>
#include "card-search-logic.h"

int main(void) {
    const wchar_t *name=L"Red-Eyes B. Dragon";
    assert(yo3_wcsistr(name,L"red")==name);
    assert(yo3_wcsistr(name,L"RED")==name);
    assert(yo3_wcsistr(name,L"dragon")==name+12);
    assert(yo3_wcsistr(name,L"Eyes B")==name+4);
    assert(yo3_wcsistr(name,L"")==name);
    assert(!yo3_wcsistr(name,L"blue"));
    assert(!yo3_wcsistr(name,L"Dragons"));
    assert(!yo3_wcsistr(NULL,L"x") && !yo3_wcsistr(name,NULL));
    /* mov edi, [0x758580] -> mov edi, imm32; nop */
    unsigned char code[6]={0x8b,0x3d,0x80,0x85,0x75,0x00};
    assert(yo3_card_search_load_ok(code,0x758580));
    assert(!yo3_card_search_load_ok(code,0x758584));
    yo3_card_search_load(code,0x12345678);
    const unsigned char swapped[6]={0xbf,0x78,0x56,0x34,0x12,0x90};
    assert(!memcmp(code,swapped,6));
    assert(!yo3_card_search_load_ok(code,0x758580));
    puts("PASS: the card search matches ignoring case, and only the expected loads are swapped");
    return 0;
}
