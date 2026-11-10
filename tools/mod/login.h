#ifndef YO3_LOGIN_H
#define YO3_LOGIN_H
#include "login-flow.h"
/* Replace only LoginWindow's one-shot StartLogin call. The movie stays at frame 1 (form alpha 0)
 * until the normal reply handler invokes StartPSelect. No Flash/game files or credentials change.
 * Native connection/error/player handlers still run. Evidence: docs/client-internals.md. */
static void login_copy_string(void *to, const void *from) {
    ((void (__attribute__((thiscall)) *)(void *,const void *))*(DWORD *)0x7583b4)(to,from);
}
static void login_connect_gate(void *gate) {
    ((void (__attribute__((thiscall)) *)(void *))(*(DWORD **)gate)[0x38/4])(gate);
}
static void __attribute__((used,noinline,force_align_arg_pointer)) login_begin(void *window) {
    yo3_login_connect(window,login_copy_string,login_connect_gate);
}
/* 0x46964B passes the LoginWindow in ESI and a discarded FxDelegate argument on the stack.
 * Match 0x4F02A0's ret 4; the C helper preserves the caller's callee-saved registers. */
static void __attribute__((naked)) login_start(void) {
    __asm__ volatile("pushl %esi; call _login_begin; addl $4,%esp; ret $4");
}
static int login_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As with the starter hook, small import-contract fixtures contain no game code. */
    if(nt->OptionalHeader.SizeOfImage<0x0f44c2) return 1;
    BYTE *site=base+0x06964b;
    if(base!=(BYTE *)0x400000 || !yo3_login_call_ok(site,5)) return 0;
    DWORD previous; if(!VirtualProtect(site,5,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    LONG offset=(BYTE *)login_start-(site+5); memcpy(site+1,&offset,4);
    FlushInstructionCache(GetCurrentProcess(),site,5);
    DWORD ignored; return VirtualProtect(site,5,previous,&ignored)!=0;
}
#endif
