#ifndef YO3_NETLOG_H
#define YO3_NETLOG_H
/* The client's network error logger (EasyMultiPlayWriteLog, global 0x82D214) is made only by 0x5339A0, which this
 * build never runs, yet every failed receive logs through it first (recv_nonblock 0x535190: "%s %d" at 0x53520B,
 * 0x53538C, 0x5353BD; recv 0x534C10/0x534CE0; connect 0x535760; close 0x5357F0). So any lost connection (the host
 * stopped or crashed, the network dropped) read address 0 and the game closed (live, 2026-11-13: the server killed
 * mid-duel, exception 0xC0000005 at 0x535210). A logger that writes nothing lets the client's own disconnect handling
 * run. Each call site is `(*logger)[0x28](logger, format, ...)` with the caller cleaning the stack (cdecl, ADD ESP),
 * and 0x5339A0 also calls [0x24](); every slot is the same do-nothing function. Only a null global is filled. */
static int __attribute__((cdecl)) netlog_ignore(void *self, ...) { (void)self; return 0; }
static void *netlog_table[16];
static struct { void **vtable; BYTE fields[0x34]; } netlog_object;
static int netlog_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* The contract fixtures end long before these addresses (their hook sites stop at 0x116000); the real game's
     * login, starter and duel guards have already identified v1.153 by then. */
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return 1;
    /* recv_nonblock's first log: mov eax,[0x82D214]; mov ecx,[eax]; mov edx,[ecx+0x28] (the address relocated). */
    const BYTE *site=(const BYTE *)game_address(0x53520b);
    static const BYTE tail[]={0x8b,0x08,0x8b,0x51,0x28};
    DWORD slot_address=(DWORD)(uintptr_t)game_address(0x82d214);
    if(site[0]!=0xa1 || memcmp(site+1,&slot_address,4) || memcmp(site+5,tail,sizeof tail)) return 0;
    for(unsigned i=0;i<16;++i) netlog_table[i]=(void *)netlog_ignore;
    netlog_object.vtable=netlog_table;
    void *volatile *slot=(void *volatile *)game_address(0x82d214);
    InterlockedCompareExchangePointer(slot,&netlog_object,NULL);
    return 1;
}
#endif
