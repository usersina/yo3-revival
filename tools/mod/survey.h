#ifndef YO3_SURVEY_H
#define YO3_SURVEY_H
/* What else runs inside the game (#68, evidence): the facts survey-logic.h judges and writes. Only reads; nothing on the
 * player's PC is changed, and nothing here can stop the game from starting.
 *   - survey_scan_imports(), in DllMain before this DLL hooks anything: for each import of the network (WSOCK32, WS2_32) and
 *     of the three functions we redirect besides, where the game's import table points. Inside a Windows DLL it is as
 *     Windows made it; inside another module, or in no module, another program has redirected it ("import"). In a Windows
 *     DLL whose first bytes jump into another module, a detour ("detour"). Our own loader's redirects are ours.
 *   - survey_run(), from Yo3ModReady (outside DllMain's lock, before the game runs): the modules loaded from outside
 *     Windows' folders and the game folder's other .dll and .asi files, with their size and SHA-256; the 32-bit Winsock
 *     providers (the game goes through that catalog; the 64-bit helper never does, #65). It writes them to the log
 *     (stderr, which the helper keeps in the game's session log and the report) and, when the helper names a file in
 *     YO3_SURVEY_FILE, as JSON for it to read. */
#include <tlhelp32.h>
#include <ws2spi.h>
#include "survey-logic.h"

static Yo3Survey survey;
static HMODULE survey_self;
static char survey_windows_dir[3 * MAX_PATH], survey_game_dir[3 * MAX_PATH];

static void survey_utf8(const wchar_t *wide, char *out, size_t capacity) {
    if (!capacity) return;
    out[0] = 0;
    if (!WideCharToMultiByte(CP_UTF8, 0, wide, -1, out, (int)capacity, NULL, NULL)) out[capacity - 1] = 0;
}
static void survey_directories(void) {
    wchar_t windows[MAX_PATH], exe[MAX_PATH];
    if (GetWindowsDirectoryW(windows, MAX_PATH)) survey_utf8(windows, survey_windows_dir, sizeof survey_windows_dir);
    DWORD n = GetModuleFileNameW(NULL, exe, MAX_PATH);
    if (n && n < MAX_PATH) { wchar_t *slash = wcsrchr(exe, L'\\'); if (slash) { *slash = 0; survey_utf8(exe, survey_game_dir, sizeof survey_game_dir); } }
}
/* A module of ours: this DLL, or the game-local WINMM loader (it exports Yo3LoaderInfo). */
static int survey_ours(HMODULE module) {
    return module == survey_self || GetProcAddress(module, "Yo3LoaderInfo") || GetProcAddress(module, "Yo3ModInfo");
}
/* The module holding `address`: its UTF-8 path, and whether it is ours. 0 when no module holds it (code in allocated memory). */
static int survey_owner(const void *address, char *path, size_t capacity, int *ours) {
    HMODULE module = NULL; wchar_t wide[MAX_PATH];
    path[0] = 0; *ours = 0;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, (LPCWSTR)address, &module)) return 0;
    DWORD n = GetModuleFileNameW(module, wide, MAX_PATH);
    if (n && n < MAX_PATH) survey_utf8(wide, path, capacity);
    *ours = survey_ours(module);
    return 1;
}
static int survey_read(uintptr_t address, void *out, size_t n) {
    MEMORY_BASIC_INFORMATION info;
    if (!VirtualQuery((const void *)address, &info, sizeof info) || info.State != MEM_COMMIT || (info.Protect & (PAGE_NOACCESS | PAGE_GUARD))) return 0;
    if (address + n > (uintptr_t)info.BaseAddress + info.RegionSize) return 0;
    memcpy(out, (const void *)address, n); return 1;
}
static void survey_hooked(const char *function, const char *how, const char *path) {
    if (survey.hooks_n >= (int)(sizeof survey.hooks / sizeof *survey.hooks)) return;
    Yo3Hook *hook = &survey.hooks[survey.hooks_n++];
    const char *module = path[0] ? yo3_path_base(path) : "";
    YO3_SET(hook->function, function); YO3_SET(hook->how, how); YO3_SET(hook->module, module); YO3_SET(hook->path, path);
    const char *product = yo3_security_product(module); YO3_SET(hook->product, product);
}
/* One import's target: redirected, detoured, or as Windows left it. */
static void survey_target(const char *function, const void *target) {
    char path[YO3_SURVEY_PATH]; int ours;
    if (!survey_owner(target, path, sizeof path, &ours)) { survey_hooked(function, "import", ""); return; }
    if (ours) return;
    if (!yo3_path_under(path, survey_windows_dir)) { survey_hooked(function, "import", path); return; }
    unsigned char code[12] = {0}; uintptr_t destination;
    if (!survey_read((uintptr_t)target, code, sizeof code) || !yo3_jump_target(code, (uintptr_t)target, survey_read, &destination)) return;
    if (!survey_owner((const void *)destination, path, sizeof path, &ours)) { survey_hooked(function, "detour", ""); return; }
    if (!ours && !yo3_path_under(path, survey_windows_dir)) survey_hooked(function, "detour", path);
}
static void survey_scan_imports(void) {
    survey_directories();
    unsigned char *base = (unsigned char *)GetModuleHandleA(NULL);
    IMAGE_DOS_HEADER *dos = (IMAGE_DOS_HEADER *)base;
    if (dos->e_magic != IMAGE_DOS_SIGNATURE) return;
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + dos->e_lfanew);
    if (nt->Signature != IMAGE_NT_SIGNATURE || nt->FileHeader.Machine != IMAGE_FILE_MACHINE_I386) return;
    DWORD rva = nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress;
    if (!rva) return;
    for (IMAGE_IMPORT_DESCRIPTOR *imports = (IMAGE_IMPORT_DESCRIPTOR *)(base + rva); imports->Name; ++imports) {
        if (!imports->OriginalFirstThunk) continue;
        const char *dll = (const char *)(base + imports->Name);
        IMAGE_THUNK_DATA32 *names = (IMAGE_THUNK_DATA32 *)(base + imports->OriginalFirstThunk);
        IMAGE_THUNK_DATA32 *iat = (IMAGE_THUNK_DATA32 *)(base + imports->FirstThunk);
        for (; names->u1.AddressOfData; ++names, ++iat) {
            char numbered[16]; const char *function;
            if (IMAGE_SNAP_BY_ORDINAL32(names->u1.Ordinal)) {
                unsigned ordinal = IMAGE_ORDINAL32(names->u1.Ordinal);
                function = yo3_name_is(dll, "WSOCK32.dll") ? yo3_wsock32_name(ordinal) : NULL;
                if (!function) { snprintf(numbered, sizeof numbered, "#%u", ordinal); function = numbered; }
            } else function = (const char *)((IMAGE_IMPORT_BY_NAME *)(base + names->u1.AddressOfData))->Name;
            if (yo3_import_matters(dll, function) && iat->u1.Function) survey_target(function, (const void *)(uintptr_t)iat->u1.Function);
        }
    }
}

/* ---- Yo3ModReady's part ---- */
static int survey_hash(const wchar_t *path, char hex[65]) {
    HANDLE file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) return 0;
    Yo3Sha256 sha; yo3_sha256_init(&sha);
    static unsigned char chunk[65536]; DWORD got; unsigned long long total = 0; int ok = 1;
    for (;;) {
        if (!ReadFile(file, chunk, sizeof chunk, &got, NULL)) { ok = 0; break; }
        if (!got) break;
        total += got; if (total > 128ull * 1024 * 1024) { ok = 0; break; }
        yo3_sha256_update(&sha, chunk, got);
    }
    CloseHandle(file);
    if (ok) yo3_sha256_hex(&sha, hex);
    return ok;
}
static char survey_seen[64][YO3_SURVEY_PATH]; static int survey_seen_n;
static int survey_seen_path(const char *path) {
    for (int i = 0; i < survey_seen_n; ++i) if (yo3_name_is(survey_seen[i], path)) return 1;
    return 0;
}
static void survey_remember(const char *path) {
    if (survey_seen_n < (int)(sizeof survey_seen / sizeof *survey_seen) && !survey_seen_path(path)) YO3_SET(survey_seen[survey_seen_n++], path);
}
static void survey_add_module(const wchar_t *wide, const char *path, int loaded) {
    if (survey.modules_n >= (int)(sizeof survey.modules / sizeof *survey.modules)) return;
    Yo3Module *module = &survey.modules[survey.modules_n++];
    const char *name = yo3_path_base(path);
    YO3_SET(module->name, name); YO3_SET(module->path, path);
    WIN32_FILE_ATTRIBUTE_DATA info;
    if (GetFileAttributesExW(wide, GetFileExInfoStandard, &info) && !info.nFileSizeHigh) module->size = info.nFileSizeLow;
    if (!survey_hash(wide, module->sha256)) module->sha256[0] = 0;
    module->game_folder = yo3_path_under(path, survey_game_dir); module->loaded = loaded;
    const char *product = yo3_security_product(name); YO3_SET(module->product, product);
}
static void survey_modules(void) {
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, GetCurrentProcessId());
    if (snapshot != INVALID_HANDLE_VALUE) {
        MODULEENTRY32W entry; entry.dwSize = sizeof entry;
        for (BOOL more = Module32FirstW(snapshot, &entry); more; more = Module32NextW(snapshot, &entry)) {
            char path[YO3_SURVEY_PATH];
            survey_utf8(entry.szExePath, path, sizeof path);
            if (entry.hModule == GetModuleHandleA(NULL) || yo3_path_under(path, survey_windows_dir) || !path[0]) continue;
            survey_remember(path); /* ours, or listed: the folder's files below must not repeat it */
            if (!survey_ours(entry.hModule)) survey_add_module(entry.szExePath, path, 1);
        }
        CloseHandle(snapshot);
    }
    /* The game folder's other programs' files, loaded or not: what a plugin loader would pick up. */
    wchar_t pattern[MAX_PATH + 8], folder[MAX_PATH];
    DWORD n = GetModuleFileNameW(NULL, folder, MAX_PATH);
    wchar_t *slash = n && n < MAX_PATH ? wcsrchr(folder, L'\\') : NULL;
    if (!slash) return;
    *slash = 0; snwprintf(pattern, MAX_PATH + 8, L"%ls\\*", folder);
    WIN32_FIND_DATAW found; HANDLE find = FindFirstFileW(pattern, &found);
    if (find == INVALID_HANDLE_VALUE) return;
    do {
        size_t length = wcslen(found.cFileName);
        if ((found.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) || length < 5) continue;
        const wchar_t *extension = found.cFileName + length - 4;
        if (_wcsicmp(extension, L".dll") && _wcsicmp(extension, L".asi")) continue;
        wchar_t full[MAX_PATH]; char path[YO3_SURVEY_PATH];
        if (snwprintf(full, MAX_PATH, L"%ls\\%ls", folder, found.cFileName) < 0) continue;
        survey_utf8(full, path, sizeof path);
        if (!survey_seen_path(path)) { survey_remember(path); survey_add_module(full, path, 0); }
    } while (FindNextFileW(find, &found));
    FindClose(find);
}
static void survey_providers(void) {
    HMODULE ntdll = GetModuleHandleA("ntdll.dll"), sockets = GetModuleHandleA("ws2_32.dll");
    int wine = ntdll && GetProcAddress(ntdll, "wine_get_version"); /* its own providers are not a finding */
    int (WSAAPI *provider_path)(LPGUID, WCHAR *, LPINT, LPINT) = sockets && !wine ? /* Wine answers it with another text */ (void *)GetProcAddress(sockets, "WSCGetProviderPath") : NULL;
    WSADATA data;
    if (WSAStartup(MAKEWORD(2, 2), &data)) return;
    DWORD size = 0; WSAEnumProtocolsW(NULL, NULL, &size);
    WSAPROTOCOL_INFOW *list = size && size < 1 << 20 ? HeapAlloc(GetProcessHeap(), 0, size) : NULL;
    int count = list ? WSAEnumProtocolsW(NULL, list, &size) : SOCKET_ERROR;
    for (int i = 0; i < count && survey.providers_n < (int)(sizeof survey.providers / sizeof *survey.providers); ++i) {
        int repeated = 0;
        for (int j = 0; j < i; ++j) if (!memcmp(&list[j].ProviderId, &list[i].ProviderId, sizeof(GUID))) repeated = 1;
        if (repeated) continue;
        Yo3Provider *provider = &survey.providers[survey.providers_n++];
        survey_utf8(list[i].szProtocol, provider->name, sizeof provider->name);
        wchar_t raw[MAX_PATH], expanded[MAX_PATH]; INT length = MAX_PATH, error = 0; GUID id = list[i].ProviderId;
        if (provider_path && !provider_path(&id, raw, &length, &error) && ExpandEnvironmentStringsW(raw, expanded, MAX_PATH)) survey_utf8(expanded, provider->path, sizeof provider->path);
        provider->microsoft = wine || yo3_provider_microsoft(provider->name, provider->path);
    }
    if (list) HeapFree(GetProcessHeap(), 0, list);
    survey.providers_known = count != SOCKET_ERROR;
    WSACleanup();
}
static void survey_emit(const char *line, void *context) {
    (void)context;
    char text[2 * YO3_SURVEY_PATH + 410]; int n = snprintf(text, sizeof text, "%s\r\n", line);
    DWORD written; if (n > 0) WriteFile(GetStdHandle(STD_ERROR_HANDLE), text, (DWORD)n, &written, NULL);
}
static void survey_write(void) {
    wchar_t file[MAX_PATH + 8], part[MAX_PATH + 16];
    DWORD n = GetEnvironmentVariableW(L"YO3_SURVEY_FILE", file, MAX_PATH);
    if (!n || n >= MAX_PATH) return;
    static char json[98304];
    size_t length = yo3_survey_json(&survey, json, sizeof json);
    if (!length) return;
    snwprintf(part, MAX_PATH + 16, L"%ls.part", file);
    HANDLE out = CreateFileW(part, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (out == INVALID_HANDLE_VALUE) return;
    DWORD written; int ok = WriteFile(out, json, (DWORD)length, &written, NULL) && written == length;
    CloseHandle(out);
    if (!ok || !MoveFileExW(part, file, MOVEFILE_REPLACE_EXISTING)) DeleteFileW(part);
}
/* The rest of the survey and its two outputs, once. */
static void survey_run(void) {
    survey_directories();
    survey_modules();
    survey_providers();
    yo3_survey_lines(&survey, survey_emit, NULL);
    survey_write();
}
#endif
