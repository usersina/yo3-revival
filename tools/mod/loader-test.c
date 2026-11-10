/* Uses only Windows APIs: prove automatic loading and forwarding without Konami files. */
#include <windows.h>
#include <mmsystem.h>
#include <stdio.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "FAIL loader line %d: %s\n", __LINE__, #x); return 1; } } while (0)
int main(void) {
    HMODULE loader = GetModuleHandleW(L"winmm.dll"); CHECK(loader != NULL);
    unsigned (__cdecl *abi)(void) = (void *)GetProcAddress(loader, "Yo3LoaderAbi"); CHECK(abi != NULL && abi() == 2);
    CHECK(timeGetTime() > 0);
    TIMECAPS caps = {0}; CHECK(timeGetDevCaps(&caps, sizeof caps) == TIMERR_NOERROR);
    CHECK(timeBeginPeriod(caps.wPeriodMin) == TIMERR_NOERROR); CHECK(timeEndPeriod(caps.wPeriodMin) == TIMERR_NOERROR);
    (void)joyGetNumDevs(); JOYCAPSA joy = {0}; (void)joyGetDevCapsA(0, &joy, sizeof joy);
    JOYINFOEX pos = {0}; pos.dwSize = sizeof pos; pos.dwFlags = JOY_RETURNALL; (void)joyGetPosEx(0, &pos);
    puts("PASS: automatic loader and seven WINMM forwards"); return 0;
}
