#ifndef YO3_IMPORT_HOOKS_H
#define YO3_IMPORT_HOOKS_H
#include <windows.h>
#include <string.h>
static int hook(const char *name, void *replacement, void **original) {
    unsigned char *base = (unsigned char *)GetModuleHandleA(NULL);
    IMAGE_DOS_HEADER *dos = (IMAGE_DOS_HEADER *)base;
    if (dos->e_magic != IMAGE_DOS_SIGNATURE) return 0;
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + dos->e_lfanew);
    if (nt->Signature != IMAGE_NT_SIGNATURE || nt->FileHeader.Machine != IMAGE_FILE_MACHINE_I386) return 0;
    DWORD rva = nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress;
    if (!rva) return 0;
    IMAGE_IMPORT_DESCRIPTOR *imports = (IMAGE_IMPORT_DESCRIPTOR *)(base + rva);
    for (; imports->Name; ++imports) {
        if (!imports->OriginalFirstThunk) continue;
        IMAGE_THUNK_DATA32 *names = (IMAGE_THUNK_DATA32 *)(base + imports->OriginalFirstThunk);
        IMAGE_THUNK_DATA32 *iat = (IMAGE_THUNK_DATA32 *)(base + imports->FirstThunk);
        for (; names->u1.AddressOfData; ++names, ++iat) {
            if (IMAGE_SNAP_BY_ORDINAL32(names->u1.Ordinal)) {
                /* The real game's WSOCK32 imports: IAT connect 0x7587D0 (4), send 0x7587F4 (19), close 0x7587EC (3). */
                if (_stricmp((const char *)(base + imports->Name), "WSOCK32.dll")) continue;
                unsigned ordinal = IMAGE_ORDINAL32(names->u1.Ordinal);
                if (!((!strcmp(name, "connect") && ordinal == 4) || (!strcmp(name, "send") && ordinal == 19)
                    || (!strcmp(name, "closesocket") && ordinal == 3))) continue;
            } else {
                IMAGE_IMPORT_BY_NAME *entry = (IMAGE_IMPORT_BY_NAME *)(base + names->u1.AddressOfData);
                if (strcmp((const char *)entry->Name, name)) continue;
            }
            DWORD old;
            if (!VirtualProtect(&iat->u1.Function, sizeof(DWORD), PAGE_READWRITE, &old)) return 0;
            *original = (void *)iat->u1.Function;
            InterlockedExchangePointer((void *volatile *)&iat->u1.Function, replacement);
            VirtualProtect(&iat->u1.Function, sizeof(DWORD), old, &old);
            return 1;
        }
    }
    return 0;
}

#endif
