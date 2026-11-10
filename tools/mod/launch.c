/* Start our 32-bit game suspended, require every DLL to initialize, and resume only after success.
 * Owns the child in a kill-on-close job so terminating the launcher cannot leave an unguarded game behind.
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

static int inject(HANDLE process, const char *dll) {
    SIZE_T length = strlen(dll) + 1;
    void *remote = VirtualAllocEx(process, NULL, length, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!remote) return 0;
    FARPROC load = GetProcAddress(GetModuleHandleA("kernel32.dll"), "LoadLibraryA");
    HANDLE thread = NULL;
    if (load && WriteProcessMemory(process, remote, dll, length, NULL))
        thread = CreateRemoteThread(process, NULL, 0, (LPTHREAD_START_ROUTINE)(void *)load, remote, 0, NULL);
    DWORD loaded = 0;
    int complete = thread && WaitForSingleObject(thread, 15000) == WAIT_OBJECT_0 && GetExitCodeThread(thread, &loaded);
    if (thread) CloseHandle(thread);
    /* If the loader timed out it might still be using this buffer; the caller terminates the whole process. */
    if (complete) VirtualFreeEx(process, remote, 0, MEM_RELEASE);
    return complete && loaded != 0;
}

int main(int argc, char **argv) {
    if (argc < 3) { fputs("usage: yo3-auth-launch GAME.exe AUTH.dll [TRACE.dll]\n", stderr); return 2; }
    char command[MAX_PATH + 4], directory[MAX_PATH];
    if (strlen(argv[1]) >= MAX_PATH) { fputs("game path too long\n", stderr); return 2; }
    snprintf(command, sizeof command, "\"%s\"", argv[1]);
    snprintf(directory, sizeof directory, "%s", argv[1]);
    char *slash = strrchr(directory, '\\'); if (slash) *slash = 0;
    STARTUPINFOA startup = {0}; PROCESS_INFORMATION child = {0}; startup.cb = sizeof startup;
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    startup.hStdOutput = GetStdHandle(STD_OUTPUT_HANDLE); startup.hStdError = GetStdHandle(STD_ERROR_HANDLE);
    HANDLE job = CreateJobObjectA(NULL, NULL);
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits = {0};
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if (!job || !SetInformationJobObject(job, JobObjectExtendedLimitInformation, &limits, sizeof limits)) return 1;
    if (!CreateProcessA(argv[1], command, NULL, NULL, TRUE, CREATE_SUSPENDED, NULL, slash ? directory : NULL, &startup, &child)) {
        fprintf(stderr, "cannot create game (%lu)\n", GetLastError()); CloseHandle(job); return 1;
    }
    int ready = AssignProcessToJobObject(job, child.hProcess);
    for (int i = 2; ready && i < argc; ++i) ready = inject(child.hProcess, argv[i]);
    if (!ready) {
        fputs("connection mod did not initialize; game terminated before startup\n", stderr);
        TerminateProcess(child.hProcess, 1);
        CloseHandle(child.hThread); CloseHandle(child.hProcess); CloseHandle(job); return 1;
    }
    fputs("connection mod loaded\n", stderr);
    ResumeThread(child.hThread); CloseHandle(child.hThread);
    WaitForSingleObject(child.hProcess, INFINITE);
    DWORD code = 1; GetExitCodeProcess(child.hProcess, &code);
    CloseHandle(child.hProcess); CloseHandle(job); return (int)code;
}
