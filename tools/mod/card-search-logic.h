#ifndef YO3_CARD_SEARCH_LOGIC_H
#define YO3_CARD_SEARCH_LOGIC_H
/* Issue #54, without the game: the card search's matcher, as wcsstr but ignoring case (card-search.h installs it). */
#include <stddef.h>
#include <string.h>
#include <wctype.h>

/* Where `needle` first occurs in `haystack`, ignoring case (towlower in the C locale: A-Z); `haystack` for
 * an empty needle, NULL when it doesn't occur or either is NULL. cdecl, as the game calls wcsstr. */
static wchar_t *__attribute__((cdecl)) yo3_wcsistr(const wchar_t *haystack, const wchar_t *needle) {
    if(!haystack || !needle) return NULL;
    if(!*needle) return (wchar_t *)haystack;
    for(; *haystack; ++haystack) {
        const wchar_t *h=haystack, *n=needle;
        while(*h && *n && towlower(*h)==towlower(*n)) { ++h; ++n; }
        if(!*n) return (wchar_t *)haystack;
    }
    return NULL;
}

/* The six bytes of `mov edi, dword ptr [wcsstr's import]` with that import slot at `iat`, the instruction the matcher
 * loads wcsstr with; and the replacement, `mov edi, imm32` (the case-blind matcher) and a nop. */
static int yo3_card_search_load_ok(const unsigned char *bytes, unsigned long iat) {
    unsigned char expected[6]={0x8b,0x3d};
    memcpy(expected+2,&iat,4);
    return !memcmp(bytes,expected,6);
}
static void yo3_card_search_load(unsigned char *out, unsigned long matcher) {
    out[0]=0xbf; memcpy(out+1,&matcher,4); out[5]=0x90;
}
#endif
