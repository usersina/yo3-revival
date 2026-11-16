#ifndef YO3_SIGNOUT_H
#define YO3_SIGNOUT_H
/* Delete Login Data forgets this device's saved sign-in for the server (signout-logic.h, #33): the Yes branch's save call
 * (0x4CA488) goes through signout_save, which first asks the helper, over this launch's local HTTP port, to revoke the
 * device token on the server and delete it from the credential vault, then saves as the game would (which blanks the
 * game's saved User ID). The next direct start then has no saved sign-in. Singleplayer has none: nothing changes. */
#include "signout-logic.h"

static void signout_direct_start(void) {
    char cookie[80], port[16], request[256];
    DWORD n = GetEnvironmentVariableA("YO3_LOCAL_COOKIE", cookie, sizeof cookie);
    if (!n || n >= sizeof cookie || !GetEnvironmentVariableA("YO3_LOCAL_HTTP", port, sizeof port)) return;
    int length = yo3_signout_request(request, sizeof request, cookie);
    unsigned long number = strtoul(port, NULL, 10);
    if (!length || !number || number > 65535) return;
    SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == INVALID_SOCKET) return;
    DWORD wait = 3000; setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, (const char *)&wait, sizeof wait);
    struct sockaddr_in helper = {0}; helper.sin_family = AF_INET; helper.sin_port = htons((u_short)number);
    helper.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    char reply[64]; int got = 0;
    if (connect(s, (struct sockaddr *)&helper, sizeof helper) == 0 && send(s, request, length, 0) == length)
        got = recv(s, reply, sizeof reply - 1, 0);
    closesocket(s);
    if (got > 12 && !memcmp(reply, "HTTP/1.1 204", 12)) starter_log("[YO3 MOD] Delete Login Data: the saved sign-in is forgotten.\r\n");
}

/* 0x51D1C0, UserSettings' save (thiscall, ECX the settings), after the sign-out. */
static char __attribute__((thiscall, force_align_arg_pointer)) signout_save(void *settings) {
    signout_direct_start();
    return ((char (__attribute__((thiscall)) *)(void *))game_address(YO3_SIGNOUT_SAVE))(settings);
}

/* Another build of the game keeps its own Delete Login Data: nothing is patched, never a reason to refuse it. */
static void signout_install(void) {
    BYTE *base = (BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if (nt->OptionalHeader.SizeOfImage < 0x42e000) return;
    BYTE *site = (BYTE *)game_address(YO3_SIGNOUT_SITE);
    if (!yo3_signout_site_ok(site)) return;
    BYTE *call = site + 2;
    LONG to = (LONG)((BYTE *)signout_save - (call + 5));
    DWORD previous;
    if (!VirtualProtect(call + 1, 4, PAGE_EXECUTE_READWRITE, &previous)) return;
    memcpy(call + 1, &to, 4);
    FlushInstructionCache(GetCurrentProcess(), call, 5);
    DWORD ignored; VirtualProtect(call + 1, 4, previous, &ignored);
}
#endif
