/* Revival connection mod (32-bit). Verified runtime hooks; the original game executable and files stay unchanged.
 * Only loopback endpoints owned by this launch are allowed. Account credentials remain in the Rust TLS launcher.
 */
#include <winsock2.h>
#include <windows.h>
#include <wininet.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "metadata.h"
#include "game-image.h"

__declspec(dllexport) const Yo3ModMetadata Yo3ModInfo __attribute__((section(".yo3"), used)) = {
    {'Y','O','3','M','O','D','0','2'}, YO3_MOD_ABI, YO3_BUILD_VERSION, YO3_BUILD_COMMIT
};
static int mod_ready;
static LONG reported_ready;
__declspec(dllexport) DWORD WINAPI Yo3ModReady(void *unused) {
    (void)unused;
    if (mod_ready && InterlockedCompareExchange(&reported_ready, 1, 0) == 0) {
        const char message[] = "[YO3 MOD] " YO3_BUILD_VERSION " interface ready; network hooks installed\r\n";
        DWORD written; WriteFile(GetStdHandle(STD_ERROR_HANDLE), message, sizeof message - 1, &written, NULL);
    }
    return mod_ready ? YO3_MOD_ABI : 0;
}

static DWORD (WINAPI *profile_string)(LPCSTR,LPCSTR,LPCSTR,LPSTR,DWORD,LPCSTR);
static UINT (WINAPI *profile_int)(LPCSTR,LPCSTR,INT,LPCSTR);
static int (WSAAPI *socket_connect)(SOCKET,const struct sockaddr *,int);
static int (WSAAPI *socket_send)(SOCKET,const char *,int,int);
static int (WSAAPI *socket_close)(SOCKET);
static HINTERNET (WINAPI *open_url)(HINTERNET,LPCSTR,LPCSTR,DWORD,DWORD,DWORD_PTR);
static unsigned short gate_port, front_port, duel_port, http_port;
static unsigned char cookie[32];
static char cookie_hex[65];
static CRITICAL_SECTION guard;
static struct { SOCKET socket; unsigned sent; } connections[64];

static unsigned short port_from(const char *name) {
    char text[16], *end;
    DWORD n = GetEnvironmentVariableA(name, text, sizeof text);
    if (!n || n >= sizeof text) return 0;
    unsigned long port = strtoul(text, &end, 10);
    return !*end && port > 0 && port <= 65535 ? (unsigned short)port : 0;
}

static int hex_digit(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

static int override(const char *section, const char *key, char *out, size_t size) {
    if (!section || !key || _stricmp(section, "Yo3")) return 0;
    if (!_stricmp(key, "GateServerAddress")) snprintf(out, size, "127.0.0.1");
    else if (!_stricmp(key, "GateServerPort")) snprintf(out, size, "%u", gate_port);
    else {
        const char *path;
        if (!_stricmp(key, "PatchUpdateUrl")) path = "yo3/";
        else if (!_stricmp(key, "PatchDataUrl")) path = "ygoda/patch/";
        else if (!_stricmp(key, "InfoURL")) path = "app/";
        else return 0;
        snprintf(out, size, "http://127.0.0.1:%u/session/%s/%s", http_port, cookie_hex, path);
    }
    return 1;
}

static DWORD WINAPI hooked_string(LPCSTR section, LPCSTR key, LPCSTR fallback, LPSTR out, DWORD size, LPCSTR file) {
    char value[256];
    if (!out || !size || !override(section, key, value, sizeof value))
        return profile_string(section, key, fallback, out, size, file);
    size_t n = strlen(value); if (n >= size) n = size - 1;
    memcpy(out, value, n); out[n] = 0;
    return (DWORD)n;
}

static UINT WINAPI hooked_int(LPCSTR section, LPCSTR key, INT fallback, LPCSTR file) {
    char value[256];
    if (override(section, key, value, sizeof value)) return (UINT)strtoul(value, NULL, 10);
    return profile_int(section, key, fallback, file);
}

static HINTERNET WINAPI hooked_url(HINTERNET internet, LPCSTR url, LPCSTR headers, DWORD count, DWORD flags, DWORD_PTR context) {
    char prefix[128], replacement[4096];
    snprintf(prefix, sizeof prefix, "http://127.0.0.1:%u/session/%s/", http_port, cookie_hex);
    if (url && !strncmp(url, prefix, strlen(prefix))) return open_url(internet, url, headers, count, flags, context);
    const char *authority = url ? strstr(url, "://") : NULL;
    const char *path = authority ? strchr(authority + 3, '/') : NULL;
    if (!path || strlen(path) > 3000) { SetLastError(ERROR_ACCESS_DENIED); return NULL; }
    if (strstr(path, "/yo3.idx")) path = "/yo3/yo3.idx";
    snprintf(replacement, sizeof replacement, "%s%s", prefix, path + 1);
    return open_url(internet, replacement, headers, count, flags, context);
}

static int WSAAPI hooked_connect(SOCKET socket, const struct sockaddr *to, int size) {
    if (!to || size < (int)sizeof(struct sockaddr_in) || to->sa_family != AF_INET) {
        WSASetLastError(WSAEACCES); return SOCKET_ERROR;
    }
    const struct sockaddr_in *address = (const struct sockaddr_in *)to;
    unsigned short port = ntohs(address->sin_port);
    if (address->sin_addr.s_addr != htonl(INADDR_LOOPBACK) || (port != gate_port && port != front_port && port != duel_port)) {
        WSASetLastError(WSAEACCES); return SOCKET_ERROR;
    }
    EnterCriticalSection(&guard);
    unsigned i;
    for (i = 0; i < 64; ++i) if (connections[i].socket == socket) break;
    if (i == 64) for (i = 0; i < 64; ++i) if (connections[i].socket == INVALID_SOCKET) break;
    if (i == 64) { LeaveCriticalSection(&guard); WSASetLastError(WSAENOBUFS); return SOCKET_ERROR; }
    if (connections[i].socket != socket) { connections[i].socket = socket; connections[i].sent = 0; }
    LeaveCriticalSection(&guard);
    return socket_connect(socket, to, size);
}

static int WSAAPI hooked_send(SOCKET socket, const char *data, int length, int flags) {
    EnterCriticalSection(&guard);
    unsigned i;
    for (i = 0; i < 64; ++i) if (connections[i].socket == socket) break;
    if (i == 64) { LeaveCriticalSection(&guard); WSASetLastError(WSAEACCES); return SOCKET_ERROR; }
    while (connections[i].sent < sizeof cookie) {
        int n = socket_send(socket, (const char *)cookie + connections[i].sent, sizeof cookie - connections[i].sent, 0);
        if (n <= 0) {
            int error = n < 0 ? WSAGetLastError() : WSAECONNRESET;
            LeaveCriticalSection(&guard); WSASetLastError(error); return SOCKET_ERROR;
        }
        connections[i].sent += (unsigned)n;
    }
    LeaveCriticalSection(&guard);
    /* The client sees only its own bytes/count. A partial/would-block cookie send is retried before any game byte. */
    return socket_send(socket, data, length, flags);
}

static int WSAAPI hooked_close(SOCKET socket) {
    EnterCriticalSection(&guard);
    for (unsigned i = 0; i < 64; ++i) if (connections[i].socket == socket) connections[i].socket = INVALID_SOCKET;
    LeaveCriticalSection(&guard);
    return socket_close(socket);
}

#include "hooks.h"
#include "starter.h"
#include "login.h"
#include "duel-observer.h"
#include "deck-upload.h"
#include "netlog.h"
#include "documents.h"
#include <stdarg.h>
#include "ui-probe.h"

/* The automatic loader delegates the API call that triggered initialization here once, outside DllMain. */
__declspec(dllexport) void *WINAPI Yo3ModApi(unsigned id) {
    if (!mod_ready) return NULL;
    switch (id) {
        case YO3_API_STRING: return (void *)hooked_string;
        case YO3_API_INT: return (void *)hooked_int;
        case YO3_API_URL: return (void *)hooked_url;
        case YO3_API_CONNECT: return (void *)hooked_connect;
        default: return NULL;
    }
}

static BOOL mod_failure(const char *message) {
    DWORD written; WriteFile(GetStdHandle(STD_ERROR_HANDLE), message, (DWORD)strlen(message), &written, NULL);
    return FALSE;
}

BOOL WINAPI DllMain(HINSTANCE dll, DWORD reason, LPVOID reserved) {
    (void)reserved;
    if (reason != DLL_PROCESS_ATTACH) return TRUE;
    DisableThreadLibraryCalls(dll);
    yo3_game_image_base = (uintptr_t)GetModuleHandleA(NULL);
    gate_port = port_from("YO3_LOCAL_GATE"); front_port = port_from("YO3_LOCAL_FRONT");
    duel_port = port_from("YO3_LOCAL_DUEL"); http_port = port_from("YO3_LOCAL_HTTP");
    if (!gate_port || !front_port || !duel_port || !http_port ||
        GetEnvironmentVariableA("YO3_LOCAL_COOKIE", cookie_hex, sizeof cookie_hex) != 64)
        return mod_failure("stage=mod-environment: Private launch state is missing or invalid. Start through Revival.\r\n");
    for (unsigned i = 0; i < 32; ++i) {
        int a = hex_digit(cookie_hex[i * 2]), b = hex_digit(cookie_hex[i * 2 + 1]);
        if (a < 0 || b < 0) return mod_failure("stage=mod-environment: Private launch state is invalid. Start through Revival.\r\n");
        cookie[i] = (unsigned char)(a * 16 + b);
    }
    InitializeCriticalSection(&guard);
    for (unsigned i = 0; i < 64; ++i) connections[i].socket = INVALID_SOCKET;
    /* Fail closed: the injector must terminate the suspended game if any import cannot be guarded. */
    if (!(hook("GetPrivateProfileStringA", (void *)hooked_string, (void **)&profile_string)
        && hook("GetPrivateProfileIntA", (void *)hooked_int, (void **)&profile_int)
        && hook("connect", (void *)hooked_connect, (void **)&socket_connect)
        && hook("send", (void *)hooked_send, (void **)&socket_send)
        && hook("closesocket", (void *)hooked_close, (void **)&socket_close)
        && hook("InternetOpenUrlA", (void *)hooked_url, (void **)&open_url)))
        return mod_failure("stage=mod-import-hooks: Game imports are unsupported. Validate the original v1.153 game and matching Revival files.\r\n");
    char image[96];
    int image_length = snprintf(image, sizeof image, "[YO3 MOD] Game image base=0x%08lx\r\n", (unsigned long)yo3_game_image_base);
    DWORD image_written; WriteFile(GetStdHandle(STD_ERROR_HANDLE), image, (DWORD)image_length, &image_written, NULL);
    if (!login_install()) return mod_failure("stage=mod-login-hook: Game login code is unsupported. Validate the original v1.153 executable.\r\n");
    if (!starter_install()) return mod_failure("stage=mod-starter-hook: Game deck code is unsupported. Validate the original v1.153 executable.\r\n");
    if (!duel_observer_install()) return mod_failure("stage=mod-duel-hook: Game duel code is unsupported. Validate the original v1.153 executable.\r\n");
    if (!deck_upload_install()) return mod_failure("stage=mod-deck-hook: Game deck editor code is unsupported. Validate the original v1.153 executable.\r\n");
    if (!netlog_install()) return mod_failure("stage=mod-netlog: Game network code is unsupported. Validate the original v1.153 executable.\r\n");
    if (!documents_install()) return mod_failure("stage=mod-documents: Game folder lookups are unsupported. Validate the original v1.153 executable.\r\n");
    ui_probe_install(); /* debugging aid, off unless YO3_UI_PROBE is set; never a reason to refuse the game */
    mod_ready = 1;
    return mod_ready;
}
