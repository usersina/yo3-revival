#ifndef YO3_SIGNOUT_H
#define YO3_SIGNOUT_H
/* Delete Login Data signs this PC out of direct start too (signout-logic.h): the Yes branch's save call (0x4CA488)
 * goes through signout_save, which first deletes the saved sign-in the direct-start loader uses (the `profile` of
 * the loader configuration that HKCU\Software\YO3 Revival\LoaderConfig names), then saves as the game would. The
 * loader then tells the player to press Play once in YO3 Revival (loader-start.h), which signs this PC in again. A
 * game started from the app is unaffected until then: it uses its own session. */
#include "signout-logic.h"

static void signout_direct_start(void) {
    wchar_t ini[4096], profile[4096];
    DWORD bytes = sizeof ini;
    if (RegGetValueW(HKEY_CURRENT_USER, L"Software\\YO3 Revival", L"LoaderConfig", RRF_RT_REG_SZ, NULL, ini, &bytes) != ERROR_SUCCESS) return;
    DWORD n = GetPrivateProfileStringW(L"revival", L"profile", L"", profile, sizeof profile / sizeof *profile, ini);
    if (!n || n >= sizeof profile / sizeof *profile - 1) return;
    if (DeleteFileW(profile)) starter_log("[YO3 MOD] Delete Login Data: direct start is signed out on this PC.\r\n");
}

/* 0x51D1C0, UserSettings' save (thiscall, ECX the settings), after the sign-out. */
static char __attribute__((thiscall, force_align_arg_pointer)) signout_save(void *settings) {
    signout_direct_start();
    return ((char (__attribute__((thiscall)) *)(void *))game_address(YO3_SIGNOUT_SAVE))(settings);
}

/* Another build of the game keeps its own Delete Login Data: nothing is patched, never a reason to refuse it. */
static void signout_install(void) {
    BYTE *base = (BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if (nt->OptionalHeader.SizeOfImage < 0x42e000) return;
    BYTE *site = (BYTE *)game_address(YO3_SIGNOUT_SITE);
    if (!yo3_signout_site_ok(site)) return;
    BYTE *call = site + 2;
    LONG to = (LONG)((BYTE *)signout_save - (call + 5));
    DWORD previous;
    if (!VirtualProtect(call + 1, 4, PAGE_EXECUTE_READWRITE, &previous)) return;
    memcpy(call + 1, &to, 4);
    FlushInstructionCache(GetCurrentProcess(), call, 5);
    DWORD ignored; VirtualProtect(call + 1, 4, previous, &ignored);
}
#endif
