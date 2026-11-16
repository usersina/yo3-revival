#ifndef YO3_LOADER_START_H
#define YO3_LOADER_START_H
#include "hooks.h"
#include "game-image.h"
/* Offline, the login skip never shows the game's form: it has no helper to sign in with. */
static int login_decision __attribute__((used));
static int __attribute__((used,noinline)) login_decide(void *window) { (void)window; return login_decision = 0; }
#include "login.h"
static DWORD (WINAPI *real_string)(LPCSTR,LPCSTR,LPCSTR,LPSTR,DWORD,LPCSTR);
static UINT (WINAPI *real_int)(LPCSTR,LPCSTR,INT,LPCSTR);
static HINTERNET (WINAPI *real_url)(HINTERNET,LPCSTR,LPCSTR,DWORD,DWORD,DWORD_PTR);
static int (WSAAPI *real_connect)(SOCKET,const struct sockaddr *,int);
static void *(WINAPI *mod_api)(unsigned);
static INIT_ONCE bootstrap_once = INIT_ONCE_STATIC_INIT;
static int boot_state, hooks_ok;

/* The helper writes bounded, flat JSON of numbers and ASCII hex. Never accept a stale helper's result. */
static const char *json_value(const char *json, const char *key) {
    char needle[80]; snprintf(needle, sizeof needle, "\"%s\"", key);
    const char *p = strstr(json, needle); if (!p) return NULL;
    p += strlen(needle); while (*p == ' ' || *p == '\r' || *p == '\n') ++p;
    if (*p++ != ':') return NULL;
    while (*p == ' ') ++p;
    return p;
}
static unsigned long json_number(const char *json, const char *key) {
    const char *p = json_value(json, key); if (!p || *p < '0' || *p > '9') return 0;
    char *end; unsigned long n = strtoul(p, &end, 10); return (*end == ',' || *end == '}' || *end == ' ') ? n : 0;
}
static int json_string(const char *json, const char *key, char *out, size_t capacity) {
    const char *p = json_value(json, key); if (!p || *p++ != '"') return 0;
    size_t n = 0;
    for (; *p && *p != '"'; ++p) {
        char c = *p;
        if (c == '\\') { c = *++p; if (c == 'n' || c == 'r') c = ' '; else if (c != '\\' && c != '"' && c != '/') return 0; }
        if ((unsigned char)c < 32 || n + 1 >= capacity) return 0;
        out[n++] = c;
    }
    out[n] = 0; return *p == '"';
}
static int read_json(const wchar_t *path, char *out, DWORD capacity) {
    HANDLE f = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, NULL, OPEN_EXISTING, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) return 0;
    LARGE_INTEGER size; DWORD got = 0; int ok = GetFileSizeEx(f, &size) && size.QuadPart > 0 && size.QuadPart < capacity
        && ReadFile(f, out, capacity - 1, &got, NULL);
    CloseHandle(f); out[got] = 0; return ok;
}
/* CommandLineToArgvW quoting, without cmd.exe/PowerShell interpretation. */
static int argument(wchar_t *command, size_t capacity, const wchar_t *arg) {
    size_t n = wcslen(command); if (n + 4 >= capacity) return 0;
    if (n) command[n++] = L' ';
    command[n++] = L'"';
    for (;;) {
        unsigned slashes = 0; while (*arg == L'\\') { ++slashes; ++arg; }
        unsigned count = slashes * ((*arg == L'"' || !*arg) ? 2 : 1);
        if (n + count + 4 >= capacity) return 0;
        while (count--) command[n++] = L'\\';
        if (!*arg) break;
        if (*arg == L'"') command[n++] = L'\\';
        command[n++] = *arg++;
    }
    command[n++] = L'"'; command[n] = 0; return 1;
}
/* Direct start can't connect without the helper (#33): YO3 Revival moved, uninstalled or mid-update, its setup gone or
 * changed. The game then runs as itself, and ONLINE ends in its own dialog: its patch check (InternetOpenUrlA) fails, so
 * it says "You will need an active Internet connection to play ONLINE."; a gate connection (the mod's login skip,
 * installed here too) is refused, "Failed to connect to server.". Either way it returns to the title screen, and this
 * one line goes with it, once. */
#define LOADER_NOTICE L"Opening the game directly needs repair: open YO3 Revival once, then start the game again."
static volatile LONG notice_shown;
static void offline_notice(void) {
    if (InterlockedExchange(&notice_shown, 1)) return;
    wchar_t file[LOADER_PATH];
    DWORD n = GetEnvironmentVariableW(L"YO3_LOADER_NOTICE_FILE", file, LOADER_PATH); /* tests: the line, not a window */
    if (n && n < LOADER_PATH) {
        HANDLE f = CreateFileW(file, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
        if (f != INVALID_HANDLE_VALUE) { DWORD written; WriteFile(f, LOADER_NOTICE, sizeof LOADER_NOTICE - sizeof(wchar_t), &written, NULL); CloseHandle(f); }
        return;
    }
    MessageBoxW(GetActiveWindow(), LOADER_NOTICE, L"YO3 Revival", MB_OK | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND);
}
static void go_offline(void) {
    yo3_game_image_base = (uintptr_t)GetModuleHandleW(NULL);
    login_install(); /* another build keeps its own login form; its connection is refused all the same */
    boot_state = 3;
}
static BOOL CALLBACK bootstrap(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once; (void)parameter; (void)context;
    LoaderConfig config = {0}; DWORD status = validate_config(&config);
    if (!hooks_ok) { MessageBoxW(NULL, LOADER_NOTICE, L"YO3 Revival", MB_OK | MB_ICONERROR); ExitProcess(31); }
    if (status) { go_offline(); return TRUE; }
    wchar_t exe[LOADER_PATH], directory[LOADER_PATH], ready[LOADER_PATH], ack[LOADER_PATH];
    DWORD length = GetModuleFileNameW(NULL, exe, LOADER_PATH);
    if (!length || length >= LOADER_PATH) { go_offline(); return TRUE; }
    lstrcpyW(directory, exe); wchar_t *slash = wcsrchr(directory, L'\\'); if (!slash) { go_offline(); return TRUE; } *slash = 0;
    HCRYPTPROV rng; BYTE random[8];
    if (!CryptAcquireContextW(&rng, NULL, NULL, PROV_RSA_AES, CRYPT_VERIFYCONTEXT)) { go_offline(); return TRUE; }
    int random_ok = CryptGenRandom(rng, sizeof random, random); CryptReleaseContext(rng, 0);
    if (!random_ok) { go_offline(); return TRUE; }
    wchar_t nonce[17]; for (unsigned i = 0; i < sizeof random; ++i) swprintf(nonce + i*2, 3, L"%02x", random[i]);
    if (wcslen(config.state) + 100 >= LOADER_PATH) { go_offline(); return TRUE; }
    swprintf(ready, LOADER_PATH, L"%ls\\direct-%lu-%ls.json", config.state, GetCurrentProcessId(), nonce);
    swprintf(ack, LOADER_PATH, L"%ls.ack", ready);
    wchar_t command[32768] = L"", pid[16]; swprintf(pid, 16, L"%lu", GetCurrentProcessId());
    const wchar_t *args[] = {config.launcher, L"--proxy-only", L"--attach-game", pid, L"--game-dir", directory,
        L"--dll", config.dll, L"--injector", config.injector, L"--profile", config.profile, L"--ready-file", ready, L"--mod-ready-file", ack,
        L"--diagnostics-dir", config.logs[0] ? config.logs : config.state};
    for (unsigned i = 0; i < sizeof args / sizeof *args; ++i) if (!argument(command, 32768, args[i])) { go_offline(); return TRUE; }
    SECURITY_ATTRIBUTES inherit = {sizeof inherit, NULL, TRUE};
    HANDLE output = CreateFileW(L"NUL", GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, &inherit, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    STARTUPINFOW startup = {0}; startup.cb = sizeof startup; startup.dwFlags = STARTF_USESHOWWINDOW; startup.wShowWindow = SW_HIDE;
    if (output != INVALID_HANDLE_VALUE) { startup.dwFlags |= STARTF_USESTDHANDLES; startup.hStdOutput = startup.hStdError = output; startup.hStdInput = GetStdHandle(STD_INPUT_HANDLE); }
    PROCESS_INFORMATION child = {0};
    int started = CreateProcessW(config.launcher, command, NULL, NULL, output != INVALID_HANDLE_VALUE, CREATE_NO_WINDOW, NULL, NULL, &startup, &child);
    if (output != INVALID_HANDLE_VALUE) CloseHandle(output);
    if (!started) { go_offline(); return TRUE; }
    CloseHandle(child.hThread);
    /* The helper prepares the local ports, then this loads the DLL and acknowledges it. The sign-in happens after that,
     * while the game starts: a refusal reaches the game's own login dialog (yo3-launcher direct.rs). */
    DWORD start = GetTickCount(); int loaded = 0; char cookie[65] = "";
    while (!loaded && GetTickCount() - start < 45000) {
        char json[8192], state[32];
        if (read_json(ready, json, sizeof json) && json_string(json, "state", state, sizeof state)) {
            if (!strcmp(state, "error") && json_number(json, "helper_pid") == child.dwProcessId) break;
            if (json_number(json, "helper_pid") == child.dwProcessId && json_number(json, "game_pid") == GetCurrentProcessId()
                && json_number(json, "abi") == YO3_MOD_ABI && !strcmp(state, "prepared")
                && json_string(json, "cookie", cookie, sizeof cookie) && strlen(cookie) == 64) {
                int valid = 1; for (unsigned i = 0; i < 64; ++i) if (!((cookie[i] >= '0' && cookie[i] <= '9') || (cookie[i] >= 'a' && cookie[i] <= 'f'))) valid = 0;
                const char *names[] = {"gate", "front", "duel", "http"};
                const char *environment[] = {"YO3_LOCAL_GATE", "YO3_LOCAL_FRONT", "YO3_LOCAL_DUEL", "YO3_LOCAL_HTTP"};
                for (unsigned i = 0; i < 4; ++i) { unsigned long port = json_number(json, names[i]); char text[16];
                    if (!port || port > 65535) valid = 0;
                    snprintf(text, sizeof text, "%lu", port); SetEnvironmentVariableA(environment[i], text); }
                if (!valid) break;
                SetEnvironmentVariableA("YO3_LOCAL_COOKIE", cookie);
                /* The server's id names its game-data folder (netlog.h's neighbour, documents.h); 16 hex digits. */
                char server[24] = {0};
                if (json_string(json, "server", server, sizeof server) && strlen(server) == 16) SetEnvironmentVariableA("YO3_SERVER_ID", server);
                HMODULE mod = LoadLibraryW(config.dll);
                DWORD (WINAPI *mod_ready)(void *) = mod ? (void *)GetProcAddress(mod, "Yo3ModReady") : NULL;
                mod_api = mod ? (void *)GetProcAddress(mod, "Yo3ModApi") : NULL;
                if (!mod) break; /* nothing of it ran: the game is still only ours */
                /* A loaded DLL hooked the game's imports: never go on with a partly hooked game. */
                if (!mod_ready || mod_ready(NULL) != YO3_MOD_ABI || !mod_api) { loaded = -1; break; }
                wchar_t pending[LOADER_PATH]; swprintf(pending, LOADER_PATH, L"%ls.pending", ack);
                HANDLE f = CreateFileW(pending, GENERIC_WRITE, 0, NULL, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, NULL); DWORD written = 0;
                if (f == INVALID_HANDLE_VALUE) { loaded = -1; break; }
                int ok = WriteFile(f, cookie, 64, &written, NULL) && written == 64; CloseHandle(f);
                if (!ok || !MoveFileW(pending, ack)) { DeleteFileW(pending); loaded = -1; break; }
                loaded = 1;
            }
        }
        if (!loaded && WaitForSingleObject(child.hProcess, 0) != WAIT_TIMEOUT) break;
        if (!loaded) Sleep(25);
    }
    if (loaded != 1) {
        TerminateProcess(child.hProcess, 1);
        WaitForSingleObject(child.hProcess, 2000);
        char last[8192];
        if (read_json(ready, last, sizeof last) && json_number(last, "helper_pid") == child.dwProcessId) DeleteFileW(ready);
        CloseHandle(child.hProcess);
        if (loaded < 0) { DeleteFileW(ack); MessageBoxW(NULL, LOADER_NOTICE, L"YO3 Revival", MB_OK | MB_ICONERROR); ExitProcess(31); }
        go_offline(); return TRUE;
    }
    CloseHandle(child.hProcess); boot_state = 2; return TRUE;
}
static int ensure_bootstrap(void) { InitOnceExecuteOnce(&bootstrap_once, bootstrap, NULL, NULL); return boot_state == 2; }
/* Offline (boot_state 3): the game's own values, but its gate on loopback, where nothing of ours answers. */
static int offline(void) { InitOnceExecuteOnce(&bootstrap_once, bootstrap, NULL, NULL); return boot_state == 3; }
static int relevant(LPCSTR section, LPCSTR key) { return section && key && !_stricmp(section, "Yo3") &&
    (!_stricmp(key, "GateServerAddress") || !_stricmp(key, "GateServerPort") || !_stricmp(key, "PatchUpdateUrl") || !_stricmp(key, "PatchDataUrl") || !_stricmp(key, "InfoURL")); }
static DWORD WINAPI boot_string(LPCSTR s, LPCSTR k, LPCSTR d, LPSTR out, DWORD n, LPCSTR f) {
    if (boot_state == 2 || !relevant(s, k)) return real_string(s, k, d, out, n, f);
    if (offline()) {
        if (_stricmp(k, "GateServerAddress") || !out || n < 10) return real_string(s, k, d, out, n, f);
        lstrcpyA(out, "127.0.0.1"); return 9;
    }
    if (!ensure_bootstrap()) { if (out && n) *out = 0; SetLastError(ERROR_ACCESS_DENIED); return 0; }
    DWORD (WINAPI *fn)(LPCSTR,LPCSTR,LPCSTR,LPSTR,DWORD,LPCSTR) = mod_api(YO3_API_STRING); return fn(s,k,d,out,n,f);
}
static UINT WINAPI boot_int(LPCSTR s, LPCSTR k, INT d, LPCSTR f) {
    if (boot_state == 2 || !relevant(s, k)) return real_int(s,k,d,f);
    if (offline()) return real_int(s,k,d,f);
    if (!ensure_bootstrap()) return 0;
    UINT (WINAPI *fn)(LPCSTR,LPCSTR,INT,LPCSTR) = mod_api(YO3_API_INT); return fn(s,k,d,f);
}
static HINTERNET WINAPI boot_url(HINTERNET h,LPCSTR u,LPCSTR a,DWORD n,DWORD flags,DWORD_PTR c) {
    if (boot_state == 2) return real_url(h,u,a,n,flags,c);
    if (offline()) { offline_notice(); SetLastError(ERROR_INTERNET_CANNOT_CONNECT); return NULL; }
    if (!ensure_bootstrap()) { SetLastError(ERROR_ACCESS_DENIED); return NULL; }
    HINTERNET (WINAPI *fn)(HINTERNET,LPCSTR,LPCSTR,DWORD,DWORD,DWORD_PTR) = mod_api(YO3_API_URL); return fn(h,u,a,n,flags,c);
}
static int WSAAPI boot_connect(SOCKET s,const struct sockaddr *a,int n) {
    if (boot_state == 2) return real_connect(s,a,n);
    if (offline()) { offline_notice(); WSASetLastError(WSAECONNREFUSED); return SOCKET_ERROR; }
    if (!ensure_bootstrap()) { WSASetLastError(WSAEACCES); return SOCKET_ERROR; }
    struct sockaddr_in gate = {0};
    if (a && n >= (int)sizeof gate && a->sa_family == AF_INET) {
        char port[16]; GetEnvironmentVariableA("YO3_LOCAL_GATE", port, sizeof port);
        gate.sin_family = AF_INET; gate.sin_addr.s_addr = htonl(INADDR_LOOPBACK); gate.sin_port = htons((unsigned short)strtoul(port,NULL,10));
        a = (const struct sockaddr *)&gate; n = sizeof gate;
    }
    int (WSAAPI *fn)(SOCKET,const struct sockaddr *,int) = mod_api(YO3_API_CONNECT); return fn(s,a,n);
}
static void install_bootstrap(void) {
    wchar_t exe[LOADER_PATH]; char managed[2]; DWORD n = GetModuleFileNameW(NULL, exe, LOADER_PATH);
    if (!n || n >= LOADER_PATH || GetEnvironmentVariableA("YO3_LOCAL_COOKIE", managed, sizeof managed)) return;
    const wchar_t *name = wcsrchr(exe, L'\\'); if (!name || lstrcmpiW(name + 1, L"yo3.exe")) return;
    hooks_ok = hook("GetPrivateProfileStringA", (void *)boot_string, (void **)&real_string)
        && hook("GetPrivateProfileIntA", (void *)boot_int, (void **)&real_int)
        && hook("InternetOpenUrlA", (void *)boot_url, (void **)&real_url)
        && hook("connect", (void *)boot_connect, (void **)&real_connect);
}
#endif
