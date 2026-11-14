#ifndef YO3_NPC_DUEL_LOG_H
#define YO3_NPC_DUEL_LOG_H
/* NPC duels leave a duel log too. After a duel DuelResult's tick (0x512070) reads the `dlogURL` of the 0x2009 result and
 * starts its download (0x498700) only when MainGamen +0x804 is 0: 0x5122DF `cmp dword [eax+0x804],0` then `jne
 * 0x5123BD` (the step's end), and the flag is set for NPC duels. The server now keeps a log of NPC duels and sends its
 * URL (crates/yo3-server/src/front.rs, finish_duel); this removes that jump, so the step runs for both kinds of duel.
 * With no dlogURL the step does nothing, as before (0x512364: an empty text). Byte-guarded: the compare and the jump.
 * Evidence: docs/protocol.md, "Duel log files". */
#define YO3_NPC_LOG_SITE 0x1122df
static int yo3_npc_log_ok(const unsigned char *bytes, size_t length) {
    static const unsigned char expected[]={0x83,0xb8,0x04,0x08,0,0,0, 0x0f,0x85,0xd1,0,0,0};
    return length==sizeof expected && !memcmp(bytes,expected,sizeof expected);
}
static int npc_duel_log_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As deck-upload.h: the contract fixtures are small executables; the other guards have identified v1.153. */
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return 1;
    BYTE *site=base+YO3_NPC_LOG_SITE;
    if(!yo3_npc_log_ok(site,13)) return 0;
    BYTE *jump=site+7; DWORD previous;
    if(!VirtualProtect(jump,6,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    memset(jump,0x90,6);
    FlushInstructionCache(GetCurrentProcess(),jump,6);
    DWORD ignored; return VirtualProtect(jump,6,previous,&ignored)!=0;
}
#endif
