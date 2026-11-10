/* Uses only Windows APIs: prove automatic loading and forwarding without Konami files. */
#include <windows.h>
#include <mmsystem.h>
#include <stdio.h>
#include <stdlib.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "FAIL loader line %d: %s\n", __LINE__, #x); return 1; } } while (0)
int main(int argc, char **argv) {
    HMODULE loader = GetModuleHandleW(L"winmm.dll"); CHECK(loader != NULL);
    unsigned (__cdecl *abi)(void) = (void *)GetProcAddress(loader, "Yo3LoaderAbi"); CHECK(abi != NULL && abi() == 2);
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
    puts("PASS: automatic loader and seven WINMM forwards"); return 0;
}
