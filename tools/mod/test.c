/* Native/Wine contract test of the imported APIs, independent of Konami's binary. */
#include <winsock2.h>
#include <windows.h>
#include <wininet.h>
#include <mmsystem.h>
#include <shlobj.h>
#include <stdio.h>
#include <string.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr, "FAIL line %d: %s\n", __LINE__, #x); return 1; } } while (0)
#ifdef YO3_TEST_BAD_GAME_HOOK
static volatile char mismatched_game[0x140000];
#endif
/* A request to the helper's HTTP port with Winsock itself, as the DLL's own imports do (no cookie preface). */
static void helper_http(const char *request, char *reply, int capacity) {
    char text[16] = ""; GetEnvironmentVariableA("YO3_LOCAL_HTTP", text, sizeof text);
    HMODULE ws = GetModuleHandleA("ws2_32.dll");
    int (WSAAPI *plain_connect)(SOCKET,const struct sockaddr *,int) = (void *)GetProcAddress(ws, "connect");
    int (WSAAPI *plain_send)(SOCKET,const char *,int,int) = (void *)GetProcAddress(ws, "send");
    struct sockaddr_in address = {0}; address.sin_family = AF_INET; address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    address.sin_port = htons((unsigned short)strtoul(text, NULL, 10));
    SOCKET s = socket(AF_INET, SOCK_STREAM, 0); int got = 0; reply[0] = 0;
    DWORD wait = 30000; setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, (const char *)&wait, sizeof wait);
    if (!plain_connect(s, (struct sockaddr *)&address, sizeof address) && plain_send(s, request, (int)strlen(request), 0) > 0)
        for (;;) { int n = recv(s, reply + got, capacity - 1 - got, 0); if (n <= 0) break; got += n; }
    reply[got] = 0; closesocket(s);
}
int main(int argc, char **argv) {
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    BYTE *image=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *headers=(IMAGE_NT_HEADERS32 *)(image+((IMAGE_DOS_HEADER *)image)->e_lfanew);
    if(headers->OptionalHeader.SizeOfImage>=0xf44c2)
        fprintf(stderr,"fixture image base=0x%08lx header-base=0x%08lx\n",(unsigned long)(uintptr_t)image,headers->OptionalHeader.ImageBase);
#ifdef YO3_TEST_BAD_GAME_HOOK
    mismatched_game[sizeof mismatched_game-1]=1; /* dispatcher RVA lies in this synthetic image */
#endif
    CHECK(argc == 2 || argc == 3);
    if (argc == 3 && !strcmp(argv[1], "direct")) {
        /* Direct start (#33), as the game does it: the gate's redirect, then the front's opener and login (0x1F47),
         * with frames tools/mod/direct_frames.py made; the replies go to argv[2] for it to read. */
        CHECK(timeGetTime() > 0); /* force the same automatic WINMM import as yo3.exe */
        volatile int never = 0; /* the game's Documents lookups, which the DLL answers per server (documents.h) */
        if (never) { wchar_t w[MAX_PATH]; char a[MAX_PATH]; SHGetFolderPathW(NULL, 0, NULL, 0, w); SHGetFolderPathA(NULL, 0, NULL, 0, a); }
        FILE *out = fopen(argv[2], "w"); CHECK(out != NULL);
        WSADATA data; CHECK(WSAStartup(MAKEWORD(2,2), &data) == 0);
        char host[256];
        GetPrivateProfileStringA("Yo3", "GateServerAddress", "stock.example", host, sizeof host, "yo3.ini");
        CHECK(!strcmp(host, "127.0.0.1"));
        UINT port = GetPrivateProfileIntA("Yo3", "GateServerPort", 5730, "yo3.ini"); CHECK(port);
        fprintf(out, "mod %s\n", GetModuleHandleA("yo3-auth.dll") ? "loaded" : "absent");
        unsigned char opener[64], login[128]; int opener_n = 0, login_n = 0; char hex[512];
        if (GetEnvironmentVariableA("YO3_TEST_OPENER", hex, sizeof hex))
            for (char *h = hex; h[0] && h[1] && opener_n < (int)sizeof opener; h += 2) { unsigned v; sscanf(h, "%2x", &v); opener[opener_n++] = (unsigned char)v; }
        if (GetEnvironmentVariableA("YO3_TEST_LOGIN", hex, sizeof hex))
            for (char *h = hex; h[0] && h[1] && login_n < (int)sizeof login; h += 2) { unsigned v; sscanf(h, "%2x", &v); login[login_n++] = (unsigned char)v; }
        CHECK(opener_n > 8 && login_n > 8);
        if (GetEnvironmentVariableA("YO3_TEST_IDLE_MS", hex, sizeof hex)) Sleep(strtoul(hex, NULL, 10)); /* the title screen */
        struct sockaddr_in address = {0}; address.sin_family = AF_INET; address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
        for (int step = 0; step < 2; ++step) {
            if (step) { char text[16] = ""; GetEnvironmentVariableA("YO3_LOCAL_FRONT", text, sizeof text); port = strtoul(text, NULL, 10); }
            char typed[256];
            if (step && GetEnvironmentVariableA("YO3_TEST_TYPED", typed, sizeof typed)) {
                /* The game's own login form (#34), as tools/mod/login-form.h drives it: ask, then hand over the entries. */
                char cookie[80] = "", request[1024], reply[512]; GetEnvironmentVariableA("YO3_LOCAL_COOKIE", cookie, sizeof cookie);
                snprintf(request, sizeof request, "GET /session/%s/revival/login-state HTTP/1.1\r\nHost: localhost\r\n\r\n", cookie);
                helper_http(request, reply, sizeof reply);
                const char *body = strstr(reply, "\r\n\r\n"); fprintf(out, "state %s\n", body ? body + 4 : "none");
                snprintf(request, sizeof request, "POST /session/%s/revival/login HTTP/1.1\r\nHost: localhost\r\nContent-Length: %d\r\n\r\n%s",
                    cookie, (int)strlen(typed), typed);
                helper_http(request, reply, sizeof reply); fprintf(out, "typed %.12s\n", reply);
            }
            SOCKET s = socket(AF_INET, SOCK_STREAM, 0); address.sin_port = htons((unsigned short)port);
            DWORD wait = 70000; setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, (const char *)&wait, sizeof wait);
            if (connect(s, (struct sockaddr *)&address, sizeof address)) { fprintf(out, "%s refused %d\n", step ? "front" : "gate", WSAGetLastError()); break; }
            for (int message = 0; message < (step ? 2 : 1); ++message) {
                CHECK(send(s, (const char *)(message ? login : opener), message ? login_n : opener_n, 0) == (message ? login_n : opener_n));
                unsigned char frame[4096]; int got = 0, need = 4;
                while (got < need) {
                    int n = recv(s, (char *)frame + got, need - got, 0); if (n <= 0) break; got += n;
                    if (got == 4) { need = 4 + (int)((((unsigned)frame[0] << 24 | frame[1] << 16 | frame[2] << 8 | frame[3]) ^ 0x77777777u)); CHECK(need <= (int)sizeof frame); }
                }
                fprintf(out, "%s ", step ? "front" : "gate");
                for (int i = 0; i < got; ++i) fprintf(out, "%02x", frame[i]);
                fputc('\n', out);
            }
            closesocket(s);
        }
        if (GetEnvironmentVariableA("YO3_TEST_SIGN_OUT", hex, sizeof hex)) {
            /* Delete Login Data's request, as tools/mod/signout.h sends it. */
            char cookie[80] = "", text[16] = "", request[256], reply[64] = "";
            GetEnvironmentVariableA("YO3_LOCAL_COOKIE", cookie, sizeof cookie); GetEnvironmentVariableA("YO3_LOCAL_HTTP", text, sizeof text);
            snprintf(request, sizeof request, "GET /session/%s/revival/sign-out HTTP/1.1\r\nHost: localhost\r\n\r\n", cookie);
            /* The DLL's own imports, not the game's hooked ones (no cookie preface on this port). */
            HMODULE ws = GetModuleHandleA("ws2_32.dll");
            int (WSAAPI *plain_connect)(SOCKET,const struct sockaddr *,int) = (void *)GetProcAddress(ws, "connect");
            int (WSAAPI *plain_send)(SOCKET,const char *,int,int) = (void *)GetProcAddress(ws, "send");
            SOCKET s = socket(AF_INET, SOCK_STREAM, 0); address.sin_port = htons((unsigned short)strtoul(text, NULL, 10));
            if (!plain_connect(s, (struct sockaddr *)&address, sizeof address) && plain_send(s, request, (int)strlen(request), 0) > 0) recv(s, reply, 12, 0);
            closesocket(s); fprintf(out, "signout %.12s\n", reply);
            Sleep(3000); /* the helper revokes the sign-in while the game still runs */
        }
        fclose(out);
        puts("PASS: direct start reached the game's own login"); return 0;
    }
    if (argc == 3 && !strcmp(argv[2], "missing-env")) {
        SetEnvironmentVariableA("YO3_LOCAL_COOKIE", NULL);
        CHECK(LoadLibraryA(argv[1]) == NULL);
        puts("PASS: no launch cookie means no DLL initialization");
        return 0;
    }
    volatile int use_wininet = 0;
    if (use_wininet) InternetOpenUrlA(NULL, "", NULL, 0, 0, 0); /* keep the same imported API as the game */
    WSADATA data; CHECK(WSAStartup(MAKEWORD(2,2), &data) == 0);
    SOCKET listener = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in address = {0}; address.sin_family = AF_INET; address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    CHECK(bind(listener, (struct sockaddr *)&address, sizeof address) == 0);
    CHECK(listen(listener, 1) == 0);
    int size = sizeof address; CHECK(getsockname(listener, (struct sockaddr *)&address, &size) == 0);
    char port[16]; snprintf(port, sizeof port, "%u", ntohs(address.sin_port));
    SetEnvironmentVariableA("YO3_LOCAL_GATE", port); SetEnvironmentVariableA("YO3_LOCAL_FRONT", port);
    SetEnvironmentVariableA("YO3_LOCAL_DUEL", port); SetEnvironmentVariableA("YO3_LOCAL_HTTP", port);
    SetEnvironmentVariableA("YO3_LOCAL_COOKIE", "0707070707070707070707070707070707070707070707070707070707070707");
    if(argc==3 && (!strcmp(argv[2],"mismatched-game") || !strcmp(argv[2],"mismatched-login")
        || !strcmp(argv[2],"mismatched-duel") || !strcmp(argv[2],"mismatched-update") || !strcmp(argv[2],"resolution"))) {
#ifdef YO3_TEST_BAD_GAME_HOOK
        if(strcmp(argv[2],"mismatched-login")) {
            /* Let the login guard pass so the independent starter-dispatch guard is exercised too. */
            BYTE *site=(BYTE *)GetModuleHandleA(NULL)+0x06964b; DWORD previous, ignored;
            const BYTE call[]={0xe8,0x50,0x6c,0x08,0x00};
            CHECK(VirtualProtect(site,sizeof call,PAGE_EXECUTE_READWRITE,&previous));
            memcpy(site,call,sizeof call); CHECK(VirtualProtect(site,sizeof call,previous,&ignored));
        }
        if(!strcmp(argv[2],"mismatched-duel") || !strcmp(argv[2],"mismatched-update")) {
            BYTE *base=(BYTE *)GetModuleHandleA(NULL); DWORD previous, ignored;
            const BYTE starter[]={0xe8,0x3e,0x05,0,0};
            CHECK(VirtualProtect(base+0xf44bd,5,PAGE_EXECUTE_READWRITE,&previous));
            memcpy(base+0xf44bd,starter,5);CHECK(VirtualProtect(base+0xf44bd,5,previous,&ignored));
            /* One observer site matches, the other is invalid: neither path may initialize the DLL. */
            const BYTE call[]={0xe8,0x79,0,0,0},update[]={0xb8,0x30,0x5a,0x51,0};
            BYTE *site=base+(!strcmp(argv[2],"mismatched-duel")?0x1159ea:0x114212);
            CHECK(VirtualProtect(site,5,PAGE_EXECUTE_READWRITE,&previous));
            memcpy(site,!strcmp(argv[2],"mismatched-duel")?update:call,5);
            CHECK(VirtualProtect(site,5,previous,&ignored));
        }
#endif
        CHECK(LoadLibraryA(argv[1])==NULL);
        printf("PASS: %s prevents DLL readiness before login\n",argv[2]);
        ExitProcess(0); /* partial import hooks must never be used after a refused DLL load */
    }
    char directory[MAX_PATH], ini[MAX_PATH + 64], text[512];
    GetTempPathA(sizeof directory, directory);
    snprintf(ini, sizeof ini, "%syo3-auth-%lu.ini", directory, GetCurrentProcessId());
    CHECK(WritePrivateProfileStringA("Yo3", "GateServerAddress", "original.example", ini));
    CHECK(WritePrivateProfileStringA("Yo3", "GateServerPort", "5730", ini));
    CHECK(LoadLibraryA(argv[1]) != NULL);
    GetPrivateProfileStringA("Yo3", "GateServerAddress", "", text, sizeof text, ini);
    CHECK(strcmp(text, "127.0.0.1") == 0);
    CHECK(GetPrivateProfileIntA("Yo3", "GateServerPort", 0, ini) == ntohs(address.sin_port));
    GetPrivateProfileStringA("Yo3", "PatchUpdateUrl", "", text, sizeof text, ini);
    CHECK(strstr(text, "/session/07070707") && strstr(text, "/yo3/"));
    SOCKET client = socket(AF_INET, SOCK_STREAM, 0);
    CHECK(connect(client, (struct sockaddr *)&address, sizeof address) == 0);
    SOCKET accepted = accept(listener, NULL, NULL); CHECK(accepted != INVALID_SOCKET);
    CHECK(send(client, "game", 4, 0) == 4);
    unsigned char received[36]; int got = 0;
    while (got < 36) { int n = recv(accepted, (char *)received + got, 36 - got, 0); CHECK(n > 0); got += n; }
    for (int i = 0; i < 32; ++i) CHECK(received[i] == 7);
    CHECK(memcmp(received + 32, "game", 4) == 0);
    SOCKET external = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in outside = {0}; outside.sin_family = AF_INET;
    outside.sin_addr.s_addr = inet_addr("192.0.2.1"); outside.sin_port = htons(443);
    CHECK(connect(external, (struct sockaddr *)&outside, sizeof outside) == SOCKET_ERROR);
    CHECK(WSAGetLastError() == WSAEACCES);
    closesocket(external);
    closesocket(client); closesocket(accepted); closesocket(listener);
    DeleteFileA(ini);
    puts("PASS: INI overrides, authenticated local socket preface, and original game bytes");
    return 0;
}
