#ifndef YO3_LOADER_START_H
#define YO3_LOADER_START_H
#include "hooks.h"
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
static void setup_error(const LoaderConfig *config, DWORD status) {
    wchar_t message[LOADER_PATH + 700];
    if (config->dll[0] && (status == ERROR_MOD_NOT_FOUND || status == ERROR_CRC)) {
        swprintf(message, sizeof message / sizeof *message,
            L"This server requires the matching Revival connection DLL.\n\nExpected path:\n%ls\n\nThe file is missing or differs from the installed release. Select the matching DLL now? You must restart the game afterwards.\n\nChoose No to repair setup using YO3 Revival → Settings → Enable direct start.", config->dll);
        if (MessageBoxW(NULL, message, L"YO3 Revival — connection blocked", MB_YESNO | MB_ICONERROR) == IDYES) {
            wchar_t selected[LOADER_PATH] = L""; OPENFILENAMEW pick = {0}; pick.lStructSize = sizeof pick;
            pick.lpstrFilter = L"Connection DLL (*.dll)\0*.dll\0"; pick.lpstrFile = selected; pick.nMaxFile = LOADER_PATH;
            pick.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
            if (GetOpenFileNameW(&pick)) {
                int valid = mod_hash_matches(selected, config->sha256);
                int copied = valid && (!lstrcmpiW(selected, config->dll) || CopyFileW(selected, config->dll, TRUE));
                MessageBoxW(NULL, copied ? L"Matching DLL selected. Close and restart the game." :
                    L"The selected DLL does not match, or the destination is protected/already exists. Repair the complete application bundle using the launcher, then enable direct start again.",
                    L"YO3 Revival", MB_OK | (copied ? MB_ICONINFORMATION : MB_ICONERROR));
            }
        }
    } else {
        swprintf(message, sizeof message / sizeof *message,
            L"Revival setup is incomplete (error %lu). The game cannot connect.\n\nOpen YO3 Revival, choose the game folder and server/account, and select Settings → Enable direct start. Then restart the game.\n\nThe original yo3.exe is unchanged.", status);
        MessageBoxW(NULL, message, L"YO3 Revival — setup required", MB_OK | MB_ICONERROR);
    }
}
static BOOL CALLBACK bootstrap(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once; (void)parameter; (void)context;
    LoaderConfig config = {0}; DWORD status = validate_config(&config);
    if (status || !hooks_ok) { setup_error(&config, status ? status : ERROR_PROC_NOT_FOUND); ExitProcess(31); }
    wchar_t exe[LOADER_PATH], directory[LOADER_PATH], ready[LOADER_PATH], ack[LOADER_PATH];
    DWORD length = GetModuleFileNameW(NULL, exe, LOADER_PATH);
    if (!length || length >= LOADER_PATH) { setup_error(&config, ERROR_FILENAME_EXCED_RANGE); boot_state = -1; return TRUE; }
    lstrcpyW(directory, exe); wchar_t *slash = wcsrchr(directory, L'\\'); if (!slash) { boot_state = -1; return TRUE; } *slash = 0;
    HCRYPTPROV rng; BYTE random[8];
    if (!CryptAcquireContextW(&rng, NULL, NULL, PROV_RSA_AES, CRYPT_VERIFYCONTEXT)) { boot_state = -1; return TRUE; }
    int random_ok = CryptGenRandom(rng, sizeof random, random); CryptReleaseContext(rng, 0);
    if (!random_ok) { boot_state = -1; return TRUE; }
    wchar_t nonce[17]; for (unsigned i = 0; i < sizeof random; ++i) swprintf(nonce + i*2, 3, L"%02x", random[i]);
    if (wcslen(config.state) + 100 >= LOADER_PATH) { setup_error(&config, ERROR_FILENAME_EXCED_RANGE); boot_state = -1; return TRUE; }
    swprintf(ready, LOADER_PATH, L"%ls\\direct-%lu-%ls.json", config.state, GetCurrentProcessId(), nonce);
    swprintf(ack, LOADER_PATH, L"%ls.ack", ready);
    wchar_t command[32768] = L"", pid[16]; swprintf(pid, 16, L"%lu", GetCurrentProcessId());
    const wchar_t *args[] = {config.launcher, L"--proxy-only", L"--attach-game", pid, L"--game-dir", directory,
        L"--dll", config.dll, L"--injector", config.injector, L"--profile", config.profile, L"--ready-file", ready, L"--mod-ready-file", ack,
        L"--diagnostics-dir", config.logs[0] ? config.logs : config.state};
    for (unsigned i = 0; i < sizeof args / sizeof *args; ++i) if (!argument(command, 32768, args[i])) { boot_state = -1; return TRUE; }
    SECURITY_ATTRIBUTES inherit = {sizeof inherit, NULL, TRUE};
    HANDLE output = CreateFileW(L"NUL", GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, &inherit, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    STARTUPINFOW startup = {0}; startup.cb = sizeof startup; startup.dwFlags = STARTF_USESHOWWINDOW; startup.wShowWindow = SW_HIDE;
    if (output != INVALID_HANDLE_VALUE) { startup.dwFlags |= STARTF_USESTDHANDLES; startup.hStdOutput = startup.hStdError = output; startup.hStdInput = GetStdHandle(STD_INPUT_HANDLE); }
    PROCESS_INFORMATION child = {0};
    int started = CreateProcessW(config.launcher, command, NULL, NULL, output != INVALID_HANDLE_VALUE, CREATE_NO_WINDOW, NULL, NULL, &startup, &child);
    if (output != INVALID_HANDLE_VALUE) CloseHandle(output);
    if (!started) { setup_error(&config, GetLastError()); boot_state = -1; return TRUE; }
    CloseHandle(child.hThread);
    DWORD start = GetTickCount(); int loaded = 0, success = 0, explained = 0; char cookie[65] = "";
    while (GetTickCount() - start < 45000) {
        char json[8192], state[32];
        if (read_json(ready, json, sizeof json) && json_string(json, "state", state, sizeof state)) {
            if (!strcmp(state, "error") && json_number(json, "helper_pid") == child.dwProcessId) {
                char error[1400]; wchar_t message[2000];
                if (!json_string(json, "error", error, sizeof error)) strcpy(error, "The connection check failed. Open the launcher Activity page for details.");
                MultiByteToWideChar(CP_UTF8, 0, error, -1, message, 1700);
                lstrcatW(message, L"\n\nConnection blocked before login. Repair setup in YO3 Revival, then restart the game.");
                MessageBoxW(NULL, message, L"YO3 Revival — server check failed", MB_OK | MB_ICONERROR); explained = 1; break;
            }
            if (json_number(json, "helper_pid") == child.dwProcessId && json_number(json, "game_pid") == GetCurrentProcessId()
                && json_number(json, "abi") == YO3_MOD_ABI) {
                if (!loaded && !strcmp(state, "prepared") && json_string(json, "cookie", cookie, sizeof cookie) && strlen(cookie) == 64) {
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
                    if (!mod_ready || mod_ready(NULL) != YO3_MOD_ABI || !mod_api) break;
                    wchar_t pending[LOADER_PATH]; swprintf(pending, LOADER_PATH, L"%ls.pending", ack);
                    HANDLE f = CreateFileW(pending, GENERIC_WRITE, 0, NULL, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, NULL); DWORD written = 0;
                    if (f == INVALID_HANDLE_VALUE) break;
                    int ok = WriteFile(f, cookie, 64, &written, NULL) && written == 64; CloseHandle(f);
                    if (!ok || !MoveFileW(pending, ack)) { DeleteFileW(pending); break; }
                    loaded = 1;
                }
                if (loaded && !strcmp(state, "authenticated")) { success = 1; break; }
            }
        }
        if (WaitForSingleObject(child.hProcess, 0) != WAIT_TIMEOUT) break;
        Sleep(25);
    }
    if (!success) {
        if (!explained) MessageBoxW(NULL, L"The connection helper did not complete the DLL/server check. Open YO3 Revival → Activity for details, repair setup, then restart the game. Login was blocked.", L"YO3 Revival — connection blocked", MB_OK | MB_ICONERROR);
        TerminateProcess(child.hProcess, 1);
        WaitForSingleObject(child.hProcess, 2000);
        char last[8192];
        if (read_json(ready, last, sizeof last) && json_number(last, "helper_pid") == child.dwProcessId) DeleteFileW(ready);
        if (loaded) DeleteFileW(ack);
        /* Never return to a partially hooked game after failure. A restart is mandatory. */
        CloseHandle(child.hProcess); ExitProcess(31);
    }
    CloseHandle(child.hProcess); boot_state = 2; return TRUE;
}
static int ensure_bootstrap(void) { InitOnceExecuteOnce(&bootstrap_once, bootstrap, NULL, NULL); return boot_state == 2; }
static int relevant(LPCSTR section, LPCSTR key) { return section && key && !_stricmp(section, "Yo3") &&
    (!_stricmp(key, "GateServerAddress") || !_stricmp(key, "GateServerPort") || !_stricmp(key, "PatchUpdateUrl") || !_stricmp(key, "PatchDataUrl") || !_stricmp(key, "InfoURL")); }
static DWORD WINAPI boot_string(LPCSTR s, LPCSTR k, LPCSTR d, LPSTR out, DWORD n, LPCSTR f) {
    if (boot_state == 2 || !relevant(s, k)) return real_string(s, k, d, out, n, f);
    if (!ensure_bootstrap()) { if (out && n) *out = 0; SetLastError(ERROR_ACCESS_DENIED); return 0; }
    DWORD (WINAPI *fn)(LPCSTR,LPCSTR,LPCSTR,LPSTR,DWORD,LPCSTR) = mod_api(YO3_API_STRING); return fn(s,k,d,out,n,f);
}
static UINT WINAPI boot_int(LPCSTR s, LPCSTR k, INT d, LPCSTR f) {
    if (boot_state == 2 || !relevant(s, k)) return real_int(s,k,d,f);
    if (!ensure_bootstrap()) return 0;
    UINT (WINAPI *fn)(LPCSTR,LPCSTR,INT,LPCSTR) = mod_api(YO3_API_INT); return fn(s,k,d,f);
}
static HINTERNET WINAPI boot_url(HINTERNET h,LPCSTR u,LPCSTR a,DWORD n,DWORD flags,DWORD_PTR c) {
    if (boot_state == 2) return real_url(h,u,a,n,flags,c);
    if (!ensure_bootstrap()) { SetLastError(ERROR_ACCESS_DENIED); return NULL; }
    HINTERNET (WINAPI *fn)(HINTERNET,LPCSTR,LPCSTR,DWORD,DWORD,DWORD_PTR) = mod_api(YO3_API_URL); return fn(h,u,a,n,flags,c);
}
static int WSAAPI boot_connect(SOCKET s,const struct sockaddr *a,int n) {
    if (boot_state == 2) return real_connect(s,a,n);
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
