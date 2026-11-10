/* A reversible game-local WINMM proxy. The original executable imports these seven functions.
 * System API loading is deferred to API calls; DllMain performs no process, UI or network work.
 */
#define _WINMM_
#include <windows.h>
#include <mmsystem.h>
#include "metadata.h"

static INIT_ONCE system_once = INIT_ONCE_STATIC_INIT;
static HMODULE system_winmm;
static BOOL CALLBACK load_system(PINIT_ONCE once, PVOID arg, PVOID *context) {
    (void)once; (void)arg; (void)context;
    wchar_t path[MAX_PATH]; UINT n = GetSystemDirectoryW(path, MAX_PATH);
    if (!n || n + 11 >= MAX_PATH) return FALSE;
    lstrcatW(path, L"\\winmm.dll"); system_winmm = LoadLibraryW(path); return system_winmm != NULL;
}
static FARPROC system_api(const char *name) {
    if (!InitOnceExecuteOnce(&system_once, load_system, NULL, NULL)) return NULL;
    return GetProcAddress(system_winmm, name);
}
__declspec(dllexport) unsigned __cdecl Yo3LoaderAbi(void) { return YO3_MOD_ABI; }
__declspec(dllexport) UINT WINAPI joyGetNumDevs(void) {
    UINT (WINAPI *fn)(void) = (void *)system_api("joyGetNumDevs"); return fn ? fn() : 0;
}
__declspec(dllexport) MMRESULT WINAPI joyGetDevCapsA(UINT_PTR id, LPJOYCAPSA caps, UINT size) {
    MMRESULT (WINAPI *fn)(UINT_PTR, LPJOYCAPSA, UINT) = (void *)system_api("joyGetDevCapsA"); return fn ? fn(id, caps, size) : MMSYSERR_ERROR;
}
__declspec(dllexport) MMRESULT WINAPI joyGetPosEx(UINT id, LPJOYINFOEX position) {
    MMRESULT (WINAPI *fn)(UINT, LPJOYINFOEX) = (void *)system_api("joyGetPosEx"); return fn ? fn(id, position) : MMSYSERR_ERROR;
}
__declspec(dllexport) DWORD WINAPI timeGetTime(void) {
    DWORD (WINAPI *fn)(void) = (void *)system_api("timeGetTime"); return fn ? fn() : GetTickCount();
}
__declspec(dllexport) MMRESULT WINAPI timeBeginPeriod(UINT period) {
    MMRESULT (WINAPI *fn)(UINT) = (void *)system_api("timeBeginPeriod"); return fn ? fn(period) : TIMERR_NOCANDO;
}
__declspec(dllexport) MMRESULT WINAPI timeEndPeriod(UINT period) {
    MMRESULT (WINAPI *fn)(UINT) = (void *)system_api("timeEndPeriod"); return fn ? fn(period) : TIMERR_NOCANDO;
}
__declspec(dllexport) MMRESULT WINAPI timeGetDevCaps(LPTIMECAPS caps, UINT size) {
    MMRESULT (WINAPI *fn)(LPTIMECAPS, UINT) = (void *)system_api("timeGetDevCaps"); return fn ? fn(caps, size) : TIMERR_NOCANDO;
}
BOOL WINAPI DllMain(HINSTANCE dll, DWORD reason, LPVOID reserved) {
    (void)reserved; if (reason == DLL_PROCESS_ATTACH) DisableThreadLibraryCalls(dll); return TRUE;
}
