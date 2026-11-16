/* Uses only Windows APIs: prove automatic loading and forwarding without Konami files. */
#include <windows.h>
#include <mmsystem.h>
#include <stdio.h>
#include <stdlib.h>
#include "build/version.h"
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "FAIL loader line %d: %s\n", __LINE__, #x); return 1; } } while (0)
int main(int argc, char **argv) {
    HMODULE loader = GetModuleHandleW(L"winmm.dll"); CHECK(loader != NULL);
    unsigned (__cdecl *abi)(void) = (void *)GetProcAddress(loader, "Yo3LoaderAbi"); CHECK(abi != NULL && abi() == YO3_MOD_ABI);
    DWORD (WINAPI *validate)(void) = (void *)GetProcAddress(loader, "Yo3LoaderValidate");
    CHECK(validate != NULL);
    DWORD expected = argc == 2 ? strtoul(argv[1], NULL, 10) : ERROR_FILE_NOT_FOUND;
    DWORD status = validate();
    if (status != expected) fprintf(stderr, "loader validation %lu, expected %lu\n", status, expected);
    CHECK(status == expected);
    CHECK(timeGetTime() > 0);
    TIMECAPS caps = {0}; CHECK(timeGetDevCaps(&caps, sizeof caps) == TIMERR_NOERROR);
    CHECK(timeBeginPeriod(caps.wPeriodMin) == TIMERR_NOERROR); CHECK(timeEndPeriod(caps.wPeriodMin) == TIMERR_NOERROR);
    (void)joyGetNumDevs(); JOYCAPSA joy = {0}; (void)joyGetDevCapsA(0, &joy, sizeof joy);
    JOYINFOEX pos = {0}; pos.dwSize = sizeof pos; pos.dwFlags = JOY_RETURNALL; (void)joyGetPosEx(0, &pos);
    /* #41: the rest of WINMM, which Windows' own DLLs may import from the game folder, goes through the loader to the
     * system's winmm.dll; the first call resolves it, the next goes straight there, and the stack stays balanced. */
    MEMORY_BASIC_INFORMATION where = {0};
    FARPROC forward = GetProcAddress(loader, "mmioStringToFOURCCA"); CHECK(forward != NULL);
    CHECK(VirtualQuery((void *)forward, &where, sizeof where) && where.AllocationBase == (void *)loader);
    for (int i = 0; i < 2; i++) CHECK(mmioStringToFOURCCA("WAVE", 0) == mmioFOURCC('W', 'A', 'V', 'E'));
    CHECK(GetProcAddress(loader, "waveInGetDevCapsW") != NULL);
    (void)waveInGetNumDevs(); WAVEINCAPSW in = {0}; MMRESULT got = waveInGetDevCapsW(WAVE_MAPPER, &in, sizeof in);
    CHECK(got == MMSYSERR_NOERROR || got == MMSYSERR_BADDEVICEID || got == MMSYSERR_NODRIVER);
    CHECK(mmioStringToFOURCCA("RIFF", 0) == mmioFOURCC('R', 'I', 'F', 'F'));
    puts("PASS: automatic loader, seven WINMM functions and the rest of WINMM forwarded"); return 0;
}
