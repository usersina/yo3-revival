#ifndef YO3_DECK_UPLOAD_H
#define YO3_DECK_UPLOAD_H
/* Edit Deck's saves reach the server, so a deck appears on the player's other machines before any duel uses it.
 * The stock editor only writes the local file: Save and New (DeckEditTest, save 0x4442E0) call 0x4159B0, which makes
 * the deck record with 0x415610 (a YO3Deck, type 6: deck_name, discription, card_protector, main, extra, side) and
 * writes it with 0x52EE60. That call to 0x415610 (0x415A4D, reached from nowhere else) is redirected here: the record
 * also goes to the front as Select Deck sends one (DuelManager 0x50EEAD-0x50EF8C), 0x1F4F {update_type 1} with the
 * deck item and no duel_id, which the server keeps unanswered (crates/yo3-server/src/front.rs, the 0x1F4F arm).
 * Uses starter.h's game string helpers. Evidence: docs/protocol.md, "Decks follow the player". */
#define YO3_DECK_SAVE_SITE 0x15a4c
static int yo3_deck_save_call_ok(const unsigned char *bytes, size_t length) {
    static const unsigned char expected[]={0x56,0xe8,0xbe,0xfb,0xff,0xff}; /* push esi; call 0x415610 */
    return length==sizeof expected && !memcmp(bytes,expected,sizeof expected);
}
typedef void *(__attribute__((thiscall)) *GameLookup)(void *,unsigned);
static void deck_upload(BYTE *record) {
    if(!record || record[0x34]!=6) return;
    /* The front connection as the editor finds it (0x441340): UserData's component registry, id -10507. */
    BYTE *user=((BYTE *(*)(void))game_address(0x40a500))();
    if(!user) return;
    void *registry=user+0x60;
    BYTE *front=((GameLookup)(*(DWORD **)registry)[0x30/4])(registry,0xffffd8f5);
    void *sender=front?*(void **)(front+0x74):NULL;
    if(!sender) return;
    __attribute__((aligned(8))) BYTE request[0x80]; /* YO3Request is 0x6C bytes (0x52FCC0 allocates that much) */
    memset(request,0,sizeof request);
    ((void *(__attribute__((stdcall)) *)(void *))game_address(0x40a190))(request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4); /* YO3Request's vtable */
    request[0x34]=4;
    ((void (__attribute__((thiscall)) *)(void *,unsigned))game_address(0x41bca0))(request,0x1f4f);
    void *map=*(void **)(request+0x60);
    GameString key; game_string(&key,"update_type");
    ((void (__attribute__((thiscall)) *)(void *,GameString,unsigned))(*(DWORD **)map)[0x14/4])(map,key,1);
    DWORD parent=*(DWORD *)(record+0xc);
    ((void (__attribute__((thiscall)) *)(void *,void *))game_address(0x40a320))(request,record);
    /* The request must not delete the record, which 0x4159B0 still writes and deletes: clear the item list's ownership
     * byte as Select Deck does for its deck (0x50EF63). */
    (*(BYTE **)(request+0x68))[0x50]=0;
    ((void (__attribute__((thiscall)) *)(void *,void *))(*(DWORD **)sender)[0x50/4])(sender,request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4);
    ((void (__attribute__((thiscall)) *)(void *))game_address(0x40a210))(request);
    *(DWORD *)(record+0xc)=parent;
    starter_log("[YO3 MOD] Deck saved in Edit Deck; sent to the server.\r\n");
}
static void *__attribute__((stdcall,force_align_arg_pointer)) deck_record(void *model) {
    void *record=((void *(__attribute__((stdcall)) *)(void *))game_address(0x415610))(model);
    deck_upload(record);
    return record;
}
static int deck_upload_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As netlog.h: the contract fixtures keep their own code at this address, and the login, starter and duel guards
     * have already identified v1.153 in the real game. */
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return 1;
    BYTE *site=base+YO3_DECK_SAVE_SITE;
    if(!yo3_deck_save_call_ok(site,6)) return 0;
    BYTE *call=site+1; DWORD previous;
    if(!VirtualProtect(call,5,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    LONG offset=(BYTE *)deck_record-(call+5); memcpy(call+1,&offset,4);
    FlushInstructionCache(GetCurrentProcess(),call,5);
    DWORD ignored; return VirtualProtect(call,5,previous,&ignored)!=0;
}
#endif
