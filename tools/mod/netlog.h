#ifndef YO3_NETLOG_H
#define YO3_NETLOG_H
#include <stdarg.h>
/* The client's network error logger (EasyMultiPlayWriteLog, global 0x82D214) is made only by 0x5339A0, which this
 * build never runs, yet every failed receive logs through it first (recv_nonblock 0x535190: "%s %d" at 0x53520B,
 * 0x53538C, 0x5353BD; recv 0x534C10/0x534CE0; connect 0x535760; close 0x5357F0). So any lost connection (the host
 * stopped or crashed, the network dropped) read address 0 and the game closed (live, 2026-11-13: the server killed
 * mid-duel, exception 0xC0000005 at 0x535210). A logger that writes nothing lets the client's own disconnect handling
 * run. Each call site is `(*logger)[0x28](logger, format, ...)` with the caller cleaning the stack (cdecl, ADD ESP),
 * and 0x5339A0 also calls [0x24](); every slot is the same do-nothing function. Only a null global is filled.
 * #65: slot 0x28 writes the game's own message on standard error (the game's session log), so a report says which
 * receive step gave up ("EasyMultiPlayPacketBase::recv_nonblock 2106" is 0x83A, a frame read short of its peek; 2124
 * 0x84C, its header; 2067 0x813, a closed connection) with the code that logged it. A few dozen lines at most. */
static int __attribute__((cdecl)) netlog_ignore(void *self, ...) { (void)self; return 0; }
static LONG netlog_lines, netlog_waits;
static int __attribute__((cdecl)) netlog_write(void *self, const char *format, ...) {
    (void)self;
    if (!format) return 0;
    char text[160];
    va_list args; va_start(args, format);
    if (vsnprintf(text, sizeof text, format, args) < 0) text[0] = 0;
    va_end(args);
    /* The session log hides 20 letters and digits in a row as a credential: "EasyMultiPlay PacketBase" stays readable. */
    if (!strncmp(text, "EasyMultiPlay", 13) && text[13] && strlen(text) < sizeof text - 1) { memmove(text + 14, text + 13, strlen(text + 13) + 1); text[13] = ' '; }
    /* 0x83A and 0x84C only wait for the rest of a frame (recv_nonblock returns 0): a few, not the whole allowance. */
    size_t length = strlen(text);
    int waiting = length > 5 && (!strcmp(text + length - 5, " 2106") || !strcmp(text + length - 5, " 2124"));
    LONG n = InterlockedIncrement(waiting ? &netlog_waits : &netlog_lines);
    if (n > (waiting ? 5 : 40)) return 0;
    uintptr_t caller = (uintptr_t)__builtin_return_address(0);
    char line[256];
    int at = snprintf(line, sizeof line, "[YO3 MOD] Game network: %s%s (at 0x%06lx)%s\r\n", text,
        waiting ? ", waiting for the rest of a frame" : "", (unsigned long)(caller - yo3_game_image_base + 0x400000u),
        n == (waiting ? 5 : 40) ? "; further lines like this are not logged" : "");
    if (at < 0) return 0;
    if (at >= (int)sizeof line) at = (int)sizeof line - 1;
    DWORD written; WriteFile(GetStdHandle(STD_ERROR_HANDLE), line, (DWORD)at, &written, NULL);
    return 0;
}
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
    netlog_table[0x28/4]=(void *)netlog_write;
    netlog_object.vtable=netlog_table;
    void *volatile *slot=(void *volatile *)game_address(0x82d214);
    InterlockedCompareExchangePointer(slot,&netlog_object,NULL);
    return 1;
}
#endif
