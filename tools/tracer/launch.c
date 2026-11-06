/* yo3trace-launch.exe: start the game and load yo3trace.dll into it.
 *
 *   wine yo3trace-launch.exe "C:\...\Yu-Gi-Oh! ONLINE 3\yo3.exe" Z:\path\to\yo3trace.dll
 *
 * Both paths are Windows paths (winepath -w). The game runs in its own folder; its stdout/stderr
 * are inherited, so its debug trace still lands wherever the launcher's output goes. The game
 * folder is not modified.
 * Build: i686-w64-mingw32-gcc -O2 -s -o yo3trace-launch.exe launch.c
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: yo3trace-launch GAME.exe TRACER.dll\n");
        return 2;
    }
    const char *exe = argv[1], *dll = argv[2];
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    si.hStdOutput = GetStdHandle(STD_OUTPUT_HANDLE);
    si.hStdError = GetStdHandle(STD_ERROR_HANDLE);
    char cmd[MAX_PATH + 2], dir[MAX_PATH];
    snprintf(cmd, sizeof cmd, "\"%s\"", exe);
    /* The game expects its own folder as the working directory. */
    snprintf(dir, sizeof dir, "%s", exe);
    char *slash = strrchr(dir, '\\');
    if (slash) *slash = 0;
    if (!CreateProcessA(exe, cmd, NULL, NULL, TRUE, CREATE_SUSPENDED, NULL, slash ? dir : NULL, &si, &pi)) {
        fprintf(stderr, "yo3trace: CreateProcess %s failed (%lu)\n", exe, GetLastError());
        return 1;
    }
    /* Load the DLL before the game runs any code of its own. */
    SIZE_T len = strlen(dll) + 1;
    void *remote = VirtualAllocEx(pi.hProcess, NULL, len, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    FARPROC load = GetProcAddress(GetModuleHandleA("kernel32.dll"), "LoadLibraryA");
    HANDLE th = NULL;
    if (remote && WriteProcessMemory(pi.hProcess, remote, dll, len, NULL))
        th = CreateRemoteThread(pi.hProcess, NULL, 0, (LPTHREAD_START_ROUTINE)(void *)load, remote, 0, NULL);
    DWORD loaded = 0;
    if (th) {
        WaitForSingleObject(th, 15000);
        GetExitCodeThread(th, &loaded);
        CloseHandle(th);
    }
    fprintf(stderr, "yo3trace: %s\n", loaded ? "tracer loaded" : "tracer NOT loaded");
    ResumeThread(pi.hThread);
    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD code = 0;
    GetExitCodeProcess(pi.hProcess, &code);
    return (int)code;
}
