/* Start our 32-bit game suspended, require every DLL to initialize, and resume only after success.
 * Owns the child in a kill-on-close job so terminating the launcher cannot leave an unguarded game behind.
 */
#include <windows.h>
#include <shlobj.h>
#include <stdio.h>
#include <wchar.h>
#include <string.h>
#include "metadata.h"

static int failure(const char *stage, DWORD error, const char *help) {
    fprintf(stderr, "stage=%s windows_error=%lu: %s\n", stage, error, help);
    return 0;
}

/* Read the native client's current Documents location, including redirection. This
 * command neither creates folders nor reads credentials, loads mods or starts a game. */
static int game_data_directory(void) {
    wchar_t folder[MAX_PATH+32]={0};
    HRESULT result=SHGetFolderPathW(NULL,CSIDL_PERSONAL,NULL,SHGFP_TYPE_CURRENT,folder);
    if(result!=S_OK || !folder[0]) {
        fprintf(stderr,"stage=game-data-dir hresult=0x%08lx: The current Documents folder is unavailable. Check its location and permissions.\n",(unsigned long)result);return 1;
    }
    wcscat(folder,L"\\Yu-Gi-Oh! ONLINE 3");
    char text[sizeof folder*2];int length=WideCharToMultiByte(CP_UTF8,WC_ERR_INVALID_CHARS,folder,-1,text,sizeof text,NULL,NULL);
    if(!length) {failure("game-data-dir",GetLastError(),"The Documents path could not be encoded as UTF-8.");return 1;}
    text[length-1]='\n';DWORD written=0;
    if(!WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),text,(DWORD)length,&written,NULL) || written!=(DWORD)length) {
        failure("game-data-dir",GetLastError(),"The Documents path could not be returned.");return 1;
    }
    return 0;
}

/* Probe only the game's known legacy runtimes, never its WINMM automatic loader or game entry point.
 * The game manifest selects the VC90 side-by-side assembly; the helper's own CRT is unrelated. */
/* A pointer to `need` bytes at `rva` of a PE file read as data, or NULL when that range is not in one section. */
static const BYTE *at_rva(const BYTE *file, DWORD size, const IMAGE_NT_HEADERS32 *nt, DWORD rva, DWORD need) {
    const IMAGE_SECTION_HEADER *section = IMAGE_FIRST_SECTION(nt);
    for (unsigned i = 0; i < nt->FileHeader.NumberOfSections; ++i, ++section) {
        DWORD start = section->VirtualAddress, length = section->SizeOfRawData;
        if (rva < start || rva - start >= length || length - (rva - start) < need) continue;
        DWORD offset = section->PointerToRawData + (rva - start);
        if (offset < section->PointerToRawData || offset > size || size - offset < need) return NULL;
        return file + offset;
    }
    return NULL;
}

/* What the help for a function missing from `dll` says: the runtimes the game ships for, or a Windows too old or
 * changed for it (issue #41: Windows 10 LTSC 2019 stopped the game with 0xc0000139 before it began). */
static const char *import_help(const char *dll) {
    if (!_stricmp(dll, "d3dx9_40.dll")) return "Install Microsoft's DirectX End-User Runtimes (June 2010), remove any other d3dx9_40.dll from the game folder, then validate again.";
    if (!_strnicmp(dll, "MSVC", 4)) return "Install or repair Microsoft Visual C++ 2008 Redistributable (x86), remove any other copy of this DLL from the game folder, then validate again.";
    return "This Windows lacks a function the game needs, or a different copy of the DLL is in the game folder. Remove such copies; the supported systems are 64-bit Windows 10 and 11.";
}

/* Loads `name` for a check with the safe search flags. Windows 7 without KB2533623 (today only in its 2016 rollup)
 * rejects those flags with ERROR_INVALID_PARAMETER although the DLL is there and the game loads it; there, look where
 * the game's own loader does: Windows' folder for a system DLL, else the normal order (the game folder first, set by
 * SetDllDirectoryW). */
static HMODULE probe(const wchar_t *name, int system) {
    HMODULE module = LoadLibraryExW(name, NULL, system ? LOAD_LIBRARY_SEARCH_SYSTEM32 : LOAD_LIBRARY_SEARCH_USER_DIRS | LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (module || GetLastError() != ERROR_INVALID_PARAMETER) return module;
    if (!system) return LoadLibraryExW(name, NULL, 0);
    wchar_t path[MAX_PATH];
    UINT length = GetSystemDirectoryW(path, MAX_PATH);
    if (!length || length + 1 + wcslen(name) >= MAX_PATH) { SetLastError(ERROR_INVALID_PARAMETER); return NULL; }
    wcscat(path, L"\\"); wcscat(path, name);
    return LoadLibraryExW(path, NULL, 0);
}

/* Every function yo3.exe imports, looked up where the game's loader would find it: a missing one stops the game with
 * 0xc0000139 before any of our code runs, so name it here instead. WINMM is checked in Windows' copy: the game
 * folder's is our loader, which forwards to it. Runs inside the game's runtime activation context. */
static int check_imports(const wchar_t *game, const wchar_t *directory) {
    HANDLE handle = CreateFileW(game, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
    if (handle == INVALID_HANDLE_VALUE) return !failure("runtime-image", GetLastError(), "Cannot read the game executable. Validate the original game files.");
    DWORD size = GetFileSize(handle, NULL), read = 0;
    BYTE *file = size != INVALID_FILE_SIZE && size <= 64u << 20 ? HeapAlloc(GetProcessHeap(), 0, size ? size : 1) : NULL;
    int loaded = file && ReadFile(handle, file, size, &read, NULL) && read == size;
    DWORD error = GetLastError(); CloseHandle(handle);
    if (!loaded) { if (file) HeapFree(GetProcessHeap(), 0, file); return !failure("runtime-image", file ? error : ERROR_FILE_TOO_LARGE, "Cannot read the game executable. Validate the original game files."); }
    const IMAGE_DOS_HEADER *dos = (const void *)file;
    const IMAGE_NT_HEADERS32 *nt = size >= sizeof *dos && dos->e_magic == IMAGE_DOS_SIGNATURE && dos->e_lfanew > 0
        && (DWORD)dos->e_lfanew < size && size - (DWORD)dos->e_lfanew >= sizeof *nt ? (const void *)(file + dos->e_lfanew) : NULL;
    if (!nt || nt->Signature != IMAGE_NT_SIGNATURE || nt->OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR32_MAGIC
        || (DWORD)((const BYTE *)IMAGE_FIRST_SECTION(nt) - file) + nt->FileHeader.NumberOfSections * sizeof(IMAGE_SECTION_HEADER) > size) {
        HeapFree(GetProcessHeap(), 0, file);
        return !failure("runtime-image", ERROR_BAD_EXE_FORMAT, "The game executable is not a 32-bit Windows program. Validate the original game files.");
    }
    const IMAGE_DATA_DIRECTORY *directory_entry = &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT];
    int ok = 1, reported = 0;
    for (DWORD at = directory_entry->VirtualAddress; at && ok >= 0; at += sizeof(IMAGE_IMPORT_DESCRIPTOR)) {
        const IMAGE_IMPORT_DESCRIPTOR *entry = (const void *)at_rva(file, size, nt, at, sizeof *entry);
        if (!entry) { ok = -1; break; }
        if (!entry->Name && !entry->FirstThunk) break;
        const char *dll = (const char *)at_rva(file, size, nt, entry->Name, 1);
        if (!dll || !memchr(dll, 0, size - (DWORD)((const BYTE *)dll - file)) || strlen(dll) >= MAX_PATH) { ok = -1; break; }
        wchar_t name[MAX_PATH];
        if (!MultiByteToWideChar(CP_ACP, 0, dll, -1, name, MAX_PATH)) { ok = -1; break; }
        int system = !_stricmp(dll, "winmm.dll");
        HMODULE module = probe(name, system);
        if (!module) {
            fprintf(stderr, "stage=runtime-import windows_error=%lu: yo3.exe needs %s, which Windows cannot load. %s\n", GetLastError(), dll, import_help(dll));
            ok = 0; if (++reported == 3) break; continue;
        }
        wchar_t path[MAX_PATH * 2] = {0};
        DWORD path_length = GetModuleFileNameW(module, path, sizeof path / sizeof *path);
        size_t folder = wcslen(directory);
        const char *where = path_length && !_wcsnicmp(path, directory, folder) && path[folder] == L'\\' ? "the game folder's copy" : "Windows' copy";
        DWORD thunk = entry->OriginalFirstThunk ? entry->OriginalFirstThunk : entry->FirstThunk;
        for (;; thunk += sizeof(DWORD)) {
            const DWORD *slot = (const void *)at_rva(file, size, nt, thunk, sizeof(DWORD));
            if (!slot) { ok = -1; break; }
            if (!*slot) break;
            char wanted[32]; const char *function = wanted;
            FARPROC found;
            if (*slot & IMAGE_ORDINAL_FLAG32) {
                snprintf(wanted, sizeof wanted, "ordinal %lu", (unsigned long)IMAGE_ORDINAL32(*slot));
                found = GetProcAddress(module, MAKEINTRESOURCEA(IMAGE_ORDINAL32(*slot)));
            } else {
                const IMAGE_IMPORT_BY_NAME *by_name = (const void *)at_rva(file, size, nt, *slot, sizeof *by_name);
                function = by_name ? (const char *)by_name->Name : NULL;
                if (!function || !memchr(function, 0, size - (DWORD)((const BYTE *)function - file))) { ok = -1; break; }
                found = GetProcAddress(module, function);
            }
            if (!found) {
                fprintf(stderr, "stage=runtime-import windows_error=%lu: yo3.exe needs %s from %s, which %s lacks. %s\n",
                    GetLastError(), function, dll, where, import_help(dll));
                ok = 0; if (++reported == 3) break;
            }
        }
        FreeLibrary(module);
        if (reported == 3) break;
    }
    HeapFree(GetProcessHeap(), 0, file);
    if (ok < 0) return !failure("runtime-image", ERROR_BAD_EXE_FORMAT, "The game executable's import table is damaged. Validate the original game files.");
    return ok;
}

static int check_runtime(const wchar_t *game) {
    wchar_t directory[MAX_PATH];
    DWORD length = GetFullPathNameW(game, MAX_PATH, directory, NULL);
    if (!length || length >= MAX_PATH)
        return !failure("runtime-path", ERROR_FILENAME_EXCED_RANGE, "Select a shorter, complete game path.");
    wchar_t *slash = wcsrchr(directory, L'\\');
    if (!slash) return !failure("runtime-path", ERROR_PATH_NOT_FOUND, "Select the folder containing yo3.exe.");
    *slash = 0;
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX);
    if (!SetDllDirectoryW(directory)) return !failure("runtime-path", GetLastError(), "Cannot access the game folder.");
    HMODULE image = LoadLibraryExW(game, NULL, LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE);
    if (!image) return !failure("runtime-image", GetLastError(), "Cannot read the game executable's runtime manifest. Validate the original game files.");
    int has_manifest = FindResourceW(image, MAKEINTRESOURCEW(1), MAKEINTRESOURCEW(24)) != NULL;
    FreeLibrary(image);
    HANDLE activation = INVALID_HANDLE_VALUE; ULONG_PTR cookie = 0;
    if (has_manifest) {
        ACTCTXW context = {0}; context.cbSize = sizeof context; context.lpSource = game;
        context.dwFlags = ACTCTX_FLAG_RESOURCE_NAME_VALID; context.lpResourceName = MAKEINTRESOURCEW(1);
        activation = CreateActCtxW(&context);
        if (activation == INVALID_HANDLE_VALUE)
            return !failure("runtime-manifest", GetLastError(), "The game's runtime manifest could not be activated. Install or repair Microsoft Visual C++ 2008 Redistributable (x86), then validate again.");
        if (!ActivateActCtx(activation, &cookie)) {
            DWORD error = GetLastError(); ReleaseActCtx(activation);
            return !failure("runtime-manifest", error, "Cannot activate the game's Microsoft Visual C++ 2008 (x86) runtime. Repair that runtime, then validate again.");
        }
    }
    const wchar_t *names[] = {L"d3dx9_40.dll", L"MSVCR90.dll", L"MSVCP90.dll"};
    const char *help[] = {
        "d3dx9_40.dll is missing or cannot load. Install Microsoft's DirectX End-User Runtimes (June 2010), then validate again.",
        "MSVCR90.dll is missing or cannot load. Install or repair Microsoft Visual C++ 2008 Redistributable (x86), then validate again.",
        "MSVCP90.dll is missing or cannot load. Install or repair Microsoft Visual C++ 2008 Redistributable (x86), then validate again."
    };
    int ok = 1;
    for (unsigned i = 0; i < sizeof names / sizeof *names; ++i) {
        HMODULE module = probe(names[i], 0);
        if (!module) { failure("runtime-dependency", GetLastError(), help[i]); ok = 0; break; }
        FreeLibrary(module);
    }
    if (ok) ok = check_imports(game, directory);
    if (activation != INVALID_HANDLE_VALUE) { DeactivateActCtx(0, cookie); ReleaseActCtx(activation); }
    if (ok) fputs("Game runtime prerequisites available\n", stdout);
    return !ok;
}

/* Windows loads the suspended game's DLLs when its first thread runs: our loader thread. A missing DLL or function
 * ends the process there, and the thread's exit code is the process's (0xc0000139), not a module address (#41). */
static int startup_reported;
static int game_died(HANDLE process) {
    DWORD code = 0;
    if (!GetExitCodeProcess(process, &code) || code == STILL_ACTIVE) return 0;
    if (startup_reported) return 1;
    startup_reported = 1;
    const char *why = code == 0xC0000139 ? "a DLL the game uses lacks a function it needs (entry point not found). Validate the game in Revival: it names the DLL and the function."
        : code == 0xC0000135 ? "a DLL the game needs was not found. Install DirectX End-User Runtimes (June 2010) and Microsoft Visual C++ 2008 Redistributable (x86), then validate again."
        : code == 0xC0000142 ? "a DLL the game uses failed to initialize. Repair DirectX End-User Runtimes (June 2010) and Microsoft Visual C++ 2008 Redistributable (x86), then validate again."
        : code == 0xC000007B ? "a DLL in the game folder is not a 32-bit DLL. Remove copies of Windows or runtime DLLs from the game folder, then validate again."
        : "Windows closed it before the game began. Export logs for help.";
    fprintf(stderr, "stage=game-startup process_exit_code=0x%08lx: The game closed while Windows loaded its DLLs: %s\n", code, why);
    return 1;
}

static DWORD inject(HANDLE process, const wchar_t *dll) {
    SIZE_T length = (wcslen(dll) + 1) * sizeof(wchar_t);
    void *remote = VirtualAllocEx(process, NULL, length, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!remote) return failure("inject-allocation", GetLastError(), "Cannot prepare the suspended game for its connection DLL. Export logs for help.");
    FARPROC load = GetProcAddress(GetModuleHandleA("kernel32.dll"), "LoadLibraryW");
    if (!load) return failure("inject-loader", GetLastError(), "The Windows DLL loader is unavailable.");
    if (!WriteProcessMemory(process, remote, dll, length, NULL))
        return failure("inject-write", GetLastError(), "Cannot write the connection DLL path into the suspended game.");
    HANDLE thread = CreateRemoteThread(process, NULL, 0, (LPTHREAD_START_ROUTINE)(void *)load, remote, 0, NULL);
    DWORD loaded = 0;
    int complete = 0;
    if (!thread) failure("inject-thread", GetLastError(), "Cannot start the connection DLL loader. Check security-software detection history and export logs.");
    else {
        DWORD wait = WaitForSingleObject(thread, 15000);
        if (wait != WAIT_OBJECT_0) failure("inject-wait", wait == WAIT_TIMEOUT ? ERROR_TIMEOUT : GetLastError(), "The connection DLL loader did not finish; the game will be closed.");
        else if (!GetExitCodeThread(thread, &loaded)) failure("inject-result", GetLastError(), "Cannot read the connection DLL loader result.");
        else if (game_died(process)) complete = 1; /* loaded is the process's exit code: not a module */
        else {
            complete = 1;
            /* A remote LoadLibrary result does not carry that thread's GetLastError; do not invent one. */
            if (!loaded) fputs("stage=mod-load remote_result=0: Windows could not load or initialize the connection DLL. Check earlier mod-stage diagnostics, required runtimes and security-software detection history. Validate the game and export logs if it repeats.\n", stderr);
        }
    }
    if (thread) CloseHandle(thread);
    /* If the loader timed out it might still be using this buffer; the caller terminates the whole process. */
    if (complete) VirtualFreeEx(process, remote, 0, MEM_RELEASE);
    return complete && !startup_reported ? loaded : 0;
}

static void json_text(const char *text) {
    putchar('"'); for (; *text; ++text) { if (*text == '"' || *text == '\\') putchar('\\'); putchar(*text); } putchar('"');
}
static HMODULE metadata(const wchar_t *path, const Yo3ModMetadata **info, FARPROC *ready) {
    HMODULE module = LoadLibraryExW(path, NULL, DONT_RESOLVE_DLL_REFERENCES);
    if (!module) return NULL;
    *info = (const void *)GetProcAddress(module, "Yo3ModInfo");
    *ready = GetProcAddress(module, "Yo3ModReady");
    if (!*info || !*ready || memcmp((*info)->magic, "YO3MOD02", 8)
        || !memchr((*info)->release, 0, sizeof (*info)->release) || !memchr((*info)->commit, 0, sizeof (*info)->commit)) {
        FreeLibrary(module); SetLastError(ERROR_BAD_EXE_FORMAT); return NULL;
    }
    return module;
}
static int initialized(HANDLE process, DWORD base, DWORD offset) {
    HANDLE thread = CreateRemoteThread(process, NULL, 0, (LPTHREAD_START_ROUTINE)(base + offset), NULL, 0, NULL);
    DWORD status = 0;
    int ok = 0;
    if (!thread) failure("mod-ready-thread", GetLastError(), "Cannot query the connection DLL's readiness.");
    else {
        DWORD wait = WaitForSingleObject(thread, 10000);
        if (wait != WAIT_OBJECT_0) failure("mod-ready-wait", wait == WAIT_TIMEOUT ? ERROR_TIMEOUT : GetLastError(), "The connection DLL readiness check did not finish.");
        else if (!GetExitCodeThread(thread, &status)) failure("mod-ready-result", GetLastError(), "Cannot read the connection DLL readiness result.");
        else if (status != YO3_MOD_ABI) fprintf(stderr, "stage=mod-ready windows_error=0 mod_status=%lu expected_abi=%u: Connection DLL did not report the required interface. Restore the complete matching release.\n", status, YO3_MOD_ABI);
        else ok = 1;
    }
    if (thread) CloseHandle(thread);
    return ok;
}

int wmain(int argc, wchar_t **argv) {
    if(argc==2 && !wcscmp(argv[1],L"--game-data-dir")) return game_data_directory();
    if (argc < 3) { fputs("usage: yo3-auth-launch GAME.exe AUTH.dll [TRACE.dll]\n", stderr); return 2; }
    if (!wcscmp(argv[1], L"--check-runtime")) return argc == 3 ? check_runtime(argv[2]) : 2;
    int controlled = !wcscmp(argv[1], L"--control-stdin");
    if (controlled) { --argc; ++argv; if (argc < 3) return 2; }
    const Yo3ModMetadata *info = NULL; FARPROC ready_fn = NULL;
    HMODULE mapped = metadata(argv[2], &info, &ready_fn);
    if (!wcscmp(argv[1], L"--inspect")) {
        if (!mapped) { fputs("invalid 32-bit revival connection DLL\n", stderr); return 1; }
        printf("{\"abi\":%u,\"release\":", info->abi); json_text(info->release); fputs(",\"commit\":", stdout); json_text(info->commit); puts("}");
        FreeLibrary(mapped); return 0;
    }
    if (!mapped || info->abi != YO3_MOD_ABI || strcmp(info->release, YO3_BUILD_VERSION)) {
        failure("mod-metadata", mapped ? ERROR_REVISION_MISMATCH : GetLastError(), "Required connection DLL is missing, invalid or from a different release. Extract the complete matching bundle; check security-software detection history if a file disappeared.");
        fputs("connection mod did not initialize; game terminated before startup (missing or incompatible DLL)\n", stderr);
        if (mapped) FreeLibrary(mapped);
        return 1;
    }
    DWORD ready_offset = (DWORD)((BYTE *)ready_fn - (BYTE *)mapped); FreeLibrary(mapped);
    wchar_t command[MAX_PATH + 4], directory[MAX_PATH];
    if (wcslen(argv[1]) >= MAX_PATH) { fputs("game path too long\n", stderr); return 2; }
    swprintf(command, sizeof command / sizeof *command, L"\"%ls\"", argv[1]);
    wcscpy(directory, argv[1]);
    wchar_t *slash = wcsrchr(directory, L'\\'); if (slash) *slash = 0;
    STARTUPINFOW startup = {0}; PROCESS_INFORMATION child = {0}; startup.cb = sizeof startup;
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    startup.hStdOutput = GetStdHandle(STD_OUTPUT_HANDLE); startup.hStdError = GetStdHandle(STD_ERROR_HANDLE);
    HANDLE job = CreateJobObjectA(NULL, NULL);
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits = {0};
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if (!job || !SetInformationJobObject(job, JobObjectExtendedLimitInformation, &limits, sizeof limits)) {
        failure("game-job", GetLastError(), "Cannot create a protected game process. Export logs for help."); if (job) CloseHandle(job); return 1;
    }
    if (!CreateProcessW(argv[1], command, NULL, NULL, TRUE, CREATE_SUSPENDED, NULL, slash ? directory : NULL, &startup, &child)) {
        DWORD error = GetLastError();
        failure("game-create", error, error == ERROR_ELEVATION_REQUIRED ? "Windows requires administrator permission for this original game executable. Close Revival and use Run as administrator, then try again." : "Cannot start the game. Validate its executable, required runtimes and file permissions; export logs for help.");
        CloseHandle(job); return 1;
    }
    int ready = AssignProcessToJobObject(job, child.hProcess);
    if (!ready) failure("game-job-assign", GetLastError(), "Cannot protect the game process; it will be closed before startup.");
    for (int i = 2; ready && i < argc; ++i) {
        DWORD loaded = inject(child.hProcess, argv[i]);
        ready = loaded && (i != 2 || initialized(child.hProcess, loaded, ready_offset));
    }
    if (!ready) {
        if (!game_died(child.hProcess)) fputs("stage=game-before-resume process_status=suspended\n", stderr);
        fputs("connection mod did not initialize; game terminated before startup\n", stderr);
        TerminateProcess(child.hProcess, 1);
        CloseHandle(child.hThread); CloseHandle(child.hProcess); CloseHandle(job); return 1;
    }
    fputs("connection mod loaded\n", stderr);
    if (controlled) {
        fputs("YO3_MOD_READY\n", stdout); fflush(stdout);
        char command[16] = {0}; DWORD length = 0;
        if (!ReadFile(GetStdHandle(STD_INPUT_HANDLE), command, sizeof command - 1, &length, NULL) || length != 7 || memcmp(command, "resume\n", 7)) {
            failure("game-resume-control", ERROR_OPERATION_ABORTED, "Launcher closed or cancelled startup; game was not resumed.");
            TerminateProcess(child.hProcess, 1); CloseHandle(child.hThread); CloseHandle(child.hProcess); CloseHandle(job); return 1;
        }
    }
    if (ResumeThread(child.hThread) == (DWORD)-1) {
        failure("game-resume", GetLastError(), "Cannot resume the protected game process."); TerminateProcess(child.hProcess, 1);
        CloseHandle(child.hThread); CloseHandle(child.hProcess); CloseHandle(job); return 1;
    }
    CloseHandle(child.hThread);
    WaitForSingleObject(child.hProcess, INFINITE);
    DWORD code = 1; GetExitCodeProcess(child.hProcess, &code);
    if (code) fprintf(stderr, "stage=game-exit process_exit_code=0x%08lx: Game exited with an error. Export logs for help.\n", code);
    CloseHandle(child.hProcess); CloseHandle(job); return (int)code;
}
