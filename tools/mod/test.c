/* Native/Wine contract test of the imported APIs, independent of Konami's binary. */
#include <winsock2.h>
#include <windows.h>
#include <wininet.h>
#include <mmsystem.h>
#include <stdio.h>
#include <string.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr, "FAIL line %d: %s\n", __LINE__, #x); return 1; } } while (0)
#ifdef YO3_TEST_BAD_GAME_HOOK
static volatile char mismatched_game[0x140000];
#endif
int main(int argc, char **argv) {
#ifdef YO3_TEST_BAD_GAME_HOOK
    mismatched_game[sizeof mismatched_game-1]=1; /* dispatcher RVA lies in this synthetic image */
#endif
    CHECK(argc == 2 || argc == 3);
    if (argc == 2 && !strcmp(argv[1], "direct")) {
        CHECK(timeGetTime() > 0); /* force the same automatic WINMM import as yo3.exe */
        WSADATA data; CHECK(WSAStartup(MAKEWORD(2,2), &data) == 0);
        char host[256];
        GetPrivateProfileStringA("Yo3", "GateServerAddress", "stock.example", host, sizeof host, "yo3.ini");
        CHECK(!strcmp(host, "127.0.0.1"));
        UINT port = GetPrivateProfileIntA("Yo3", "GateServerPort", 5730, "yo3.ini"); CHECK(port && port != 5730);
        HMODULE mod = GetModuleHandleA("yo3-auth.dll"); CHECK(mod != NULL);
        DWORD (WINAPI *ready)(void *) = (void *)GetProcAddress(mod, "Yo3ModReady"); CHECK(ready && ready(NULL) == 2);
        SOCKET client = socket(AF_INET, SOCK_STREAM, 0);
        struct sockaddr_in address = {0}; address.sin_family = AF_INET;
        address.sin_addr.s_addr = htonl(INADDR_LOOPBACK); address.sin_port = htons((unsigned short)port);
        CHECK(connect(client, (struct sockaddr *)&address, sizeof address) == 0); closesocket(client);
        puts("PASS: original process automatically loads the verified mod before authenticating"); return 0;
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
        || !strcmp(argv[2],"mismatched-duel") || !strcmp(argv[2],"mismatched-update"))) {
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
