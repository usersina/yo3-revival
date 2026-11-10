/* Start our 32-bit game suspended, require every DLL to initialize, and resume only after success.
 * Owns the child in a kill-on-close job so terminating the launcher cannot leave an unguarded game behind.
 */
#include <windows.h>
#include <stdio.h>
#include <wchar.h>
#include <string.h>
#include "metadata.h"

static DWORD inject(HANDLE process, const wchar_t *dll) {
    SIZE_T length = (wcslen(dll) + 1) * sizeof(wchar_t);
    void *remote = VirtualAllocEx(process, NULL, length, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!remote) return 0;
    FARPROC load = GetProcAddress(GetModuleHandleA("kernel32.dll"), "LoadLibraryW");
    HANDLE thread = NULL;
    if (load && WriteProcessMemory(process, remote, dll, length, NULL))
        thread = CreateRemoteThread(process, NULL, 0, (LPTHREAD_START_ROUTINE)(void *)load, remote, 0, NULL);
    DWORD loaded = 0;
    int complete = thread && WaitForSingleObject(thread, 15000) == WAIT_OBJECT_0 && GetExitCodeThread(thread, &loaded);
    if (thread) CloseHandle(thread);
    /* If the loader timed out it might still be using this buffer; the caller terminates the whole process. */
    if (complete) VirtualFreeEx(process, remote, 0, MEM_RELEASE);
    return complete ? loaded : 0;
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
        FreeLibrary(module); return NULL;
    }
    return module;
}
static int initialized(HANDLE process, DWORD base, DWORD offset) {
    HANDLE thread = CreateRemoteThread(process, NULL, 0, (LPTHREAD_START_ROUTINE)(base + offset), NULL, 0, NULL);
    DWORD status = 0;
    int ok = thread && WaitForSingleObject(thread, 10000) == WAIT_OBJECT_0 && GetExitCodeThread(thread, &status) && status == YO3_MOD_ABI;
    if (thread) CloseHandle(thread);
    return ok;
}

int wmain(int argc, wchar_t **argv) {
    if (argc < 3) { fputs("usage: yo3-auth-launch GAME.exe AUTH.dll [TRACE.dll]\n", stderr); return 2; }
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
    if (!job || !SetInformationJobObject(job, JobObjectExtendedLimitInformation, &limits, sizeof limits)) return 1;
    if (!CreateProcessW(argv[1], command, NULL, NULL, TRUE, CREATE_SUSPENDED, NULL, slash ? directory : NULL, &startup, &child)) {
        fprintf(stderr, "cannot create game (%lu)\n", GetLastError()); CloseHandle(job); return 1;
    }
    int ready = AssignProcessToJobObject(job, child.hProcess);
    for (int i = 2; ready && i < argc; ++i) {
        DWORD loaded = inject(child.hProcess, argv[i]);
        ready = loaded && (i != 2 || initialized(child.hProcess, loaded, ready_offset));
    }
    if (!ready) {
        fputs("connection mod did not initialize; game terminated before startup\n", stderr);
        TerminateProcess(child.hProcess, 1);
        CloseHandle(child.hThread); CloseHandle(child.hProcess); CloseHandle(job); return 1;
    }
    fputs("connection mod loaded\n", stderr);
    if (controlled) {
        fputs("YO3_MOD_READY\n", stdout); fflush(stdout);
        char command[16] = {0}; DWORD length = 0;
        if (!ReadFile(GetStdHandle(STD_INPUT_HANDLE), command, sizeof command - 1, &length, NULL) || length != 7 || memcmp(command, "resume\n", 7)) {
            TerminateProcess(child.hProcess, 1); CloseHandle(child.hThread); CloseHandle(child.hProcess); CloseHandle(job); return 1;
        }
    }
    ResumeThread(child.hThread); CloseHandle(child.hThread);
    WaitForSingleObject(child.hProcess, INFINITE);
    DWORD code = 1; GetExitCodeProcess(child.hProcess, &code);
    CloseHandle(child.hProcess); CloseHandle(job); return (int)code;
}
