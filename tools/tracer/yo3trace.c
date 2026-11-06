/* yo3trace.dll: logs every field the Yu-Gi-Oh! ONLINE 3 client looks up in a message.
 *
 * Every typed field getter (int 0x409020, uint 0x408F50, string 0x4090F0, raw 0x40F1D0)
 * goes through one helper, std::map<std::string, Value*>::find at 0x41A980:
 *     ESI = tree (map + 4), [esp+4] = iterator out, [esp+8] = std::string *key, ret 8
 * The end node is *(tree + 0x18); a node's value pointer is at node + 0x28.
 * List items are read by 0x410D60: ESI = list, EDI = index, NULL when out of range.
 *
 * Both entry points get a 5-byte JMP to a stub that logs and then runs the original
 * prologue from a trampoline. Nothing is patched unless the original bytes match.
 *
 * Log lines (tab separated), to %YO3_TRACE_FILE% (default yo3_fields.tsv):
 *   F  ms tid getter key found(0/1) value-type request-value field-caller getter-caller tree
 *   (tree identifies the dataset: the same tree as a message's "request" = a top-level field)
 *   I  ms tid index count caller
 * Build: i686-w64-mingw32-gcc -O2 -shared -s -o yo3trace.dll yo3trace.c
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

#define FIND_ADDR 0x41A980u
#define ITEM_ADDR 0x410D60u
#define IMAGE_LO 0x400000u
#define IMAGE_HI 0x894000u
#define TEXT_LO 0x401000u
#define TEXT_HI 0x758000u

static const unsigned char FIND_PROLOGUE[5] = {0x83, 0xEC, 0x18, 0x53, 0x55}; /* sub esp,18; push ebx; push ebp */
static const unsigned char ITEM_PROLOGUE[6] = {0x8B, 0x46, 0x48, 0x2B, 0x46, 0x44}; /* mov eax,[esi+48]; sub eax,[esi+44] */

DWORD find_resume = FIND_ADDR + 5;
DWORD item_resume = ITEM_ADDR + 6;

static CRITICAL_SECTION log_lock;
static HANDLE log_file = INVALID_HANDLE_VALUE;
static DWORD t0;
static volatile LONG in_hook; /* our own lookups must not be logged */

/* ---- stubs -------------------------------------------------------------------------- */

void __cdecl on_find(DWORD tree, DWORD key, DWORD ret);
void __cdecl on_item(DWORD list, DWORD index, DWORD ret);

__asm__(
    ".globl _hook_find\n"
    "_hook_find:\n"
    "  pushal\n"
    "  pushfl\n"
    "  movl %esp, %eax\n"
    "  pushl 36(%eax)\n"          /* return address (into the getter) */
    "  pushl 44(%eax)\n"          /* key */
    "  pushl %esi\n"              /* tree */
    "  call _on_find\n"
    "  addl $12, %esp\n"
    "  popfl\n"
    "  popal\n"
    "_find_tramp:\n"
    "  subl $0x18, %esp\n"        /* relocated prologue */
    "  pushl %ebx\n"
    "  pushl %ebp\n"
    "  jmp *_find_resume\n"

    /* orig_find(tree, iter_out, key): run the real lookup without logging */
    ".globl _orig_find\n"
    "_orig_find:\n"
    "  pushl %esi\n"
    "  movl 8(%esp), %esi\n"
    "  pushl 16(%esp)\n"          /* key */
    "  pushl 16(%esp)\n"          /* iter_out */
    "  call _find_tramp\n"        /* callee pops both (ret 8) */
    "  popl %esi\n"
    "  ret\n"

    ".globl _hook_item\n"
    "_hook_item:\n"
    "  pushal\n"
    "  pushfl\n"
    "  movl %esp, %eax\n"
    "  pushl 36(%eax)\n"          /* caller */
    "  pushl %edi\n"              /* index */
    "  pushl %esi\n"              /* list */
    "  call _on_item\n"
    "  addl $12, %esp\n"
    "  popfl\n"
    "  popal\n"
    "  movl 0x48(%esi), %eax\n"   /* relocated prologue */
    "  subl 0x44(%esi), %eax\n"
    "  jmp *_item_resume\n");

void hook_find(void);
void hook_item(void);
void __cdecl orig_find(DWORD tree, DWORD *iter_out, DWORD key);

/* ---- helpers ------------------------------------------------------------------------ */

static int in_image(DWORD p, DWORD size) { return p >= IMAGE_LO && p + size <= IMAGE_HI; }

static int readable(DWORD p, DWORD size) { return p && !IsBadReadPtr((void *)p, size); }

/* MSVC 2008 std::string: +4 inline buffer (16) or pointer, +0x14 size, +0x18 capacity */
static int read_key(DWORD key, char *out, int cap) {
    if (!readable(key, 0x1C)) return 0;
    DWORD size = *(DWORD *)(key + 0x14), res = *(DWORD *)(key + 0x18);
    if (size == 0 || size > 200 || res < size) return 0;
    DWORD data = res < 16 ? key + 4 : *(DWORD *)(key + 4);
    if (!readable(data, size)) return 0;
    int n = (int)size < cap - 1 ? (int)size : cap - 1;
    memcpy(out, (void *)data, n);
    out[n] = 0;
    for (int i = 0; i < n; i++)
        if ((unsigned char)out[i] < 0x20 || (unsigned char)out[i] > 0x7E) return 0;
    return 1;
}

/* Class name of a game object from its MSVC RTTI (".?AVEasyMultiPlayDataInt@@" -> "EasyMultiPlayDataInt"). */
static void type_name(DWORD obj, char *out, int cap) {
    strcpy(out, "?");
    if (!readable(obj, 4)) return;
    DWORD vtbl = *(DWORD *)obj;
    if (!in_image(vtbl - 4, 4)) return;
    DWORD col = *(DWORD *)(vtbl - 4);
    if (!in_image(col, 16)) return;
    DWORD td = *(DWORD *)(col + 12);
    if (!in_image(td + 8, 8)) return;
    const char *name = (const char *)(td + 8);
    if (strncmp(name, ".?AV", 4) == 0) name += 4;
    int n = 0;
    while (n < cap - 1 && name[n] && name[n] != '@') { out[n] = name[n]; n++; }
    out[n] = 0;
}

/* value->vtable[0x14] (thiscall): the uint getter used for "request" (see 0x40F290). */
static DWORD value_u32(DWORD obj) {
    if (!readable(obj, 4)) return 0;
    DWORD vtbl = *(DWORD *)obj;
    if (!in_image(vtbl + 0x14, 4)) return 0;
    DWORD fn = *(DWORD *)(vtbl + 0x14), r;
    if (fn < TEXT_LO || fn >= TEXT_HI) return 0;
    DWORD self = obj;
    __asm__ volatile("call *%[fn]" : "=a"(r), "+c"(self) : [fn] "r"(fn) : "edx", "memory", "cc");
    return r;
}

static const char *getter_name(DWORD ret) {
    if (ret >= 0x408F50 && ret < 0x409020) return "uint";
    if (ret >= 0x409020 && ret < 0x4090F0) return "int";
    if (ret >= 0x4090F0 && ret < 0x409200) return "string";
    if (ret >= 0x40F1D0 && ret < 0x40F290) return "raw";
    return "find";
}

static void log_line(const char *s) {
    DWORD n;
    EnterCriticalSection(&log_lock);
    WriteFile(log_file, s, (DWORD)strlen(s), &n, NULL);
    LeaveCriticalSection(&log_lock);
}

/* ---- hooks -------------------------------------------------------------------------- */

void __cdecl on_find(DWORD tree, DWORD key, DWORD ret) {
    char name[208], type[64] = "-", line[512];
    if (in_hook || !read_key(key, name, sizeof name)) return;
    InterlockedIncrement(&in_hook);
    DWORD iter[2] = {0, 0};
    orig_find(tree, iter, key);
    DWORD end = readable(tree + 0x18, 4) ? *(DWORD *)(tree + 0x18) : 0;
    int found = iter[1] && iter[1] != end;
    DWORD value = found && readable(iter[1] + 0x28, 4) ? *(DWORD *)(iter[1] + 0x28) : 0;
    if (found) type_name(value, type, sizeof type);
    char req[16] = "-";
    if (found && strcmp(name, "request") == 0) sprintf(req, "%#lx", (unsigned long)value_u32(value));
    /* For calls through a getter the key is the getter's by-value argument at entry esp + 8,
       so the game code that asked for the field is the return address at key - 8. */
    const char *getter = getter_name(ret);
    DWORD caller = ret;
    if (strcmp(getter, "find") != 0 && readable(key - 8, 4)) {
        DWORD r = *(DWORD *)(key - 8);
        caller = r >= TEXT_LO && r < TEXT_HI ? r : 0;
    }
    sprintf(line, "F\t%lu\t%lu\t%s\t%s\t%d\t%s\t%s\t%#lx\t%#lx\t%#lx\n", (unsigned long)(GetTickCount() - t0),
            (unsigned long)GetCurrentThreadId(), getter, name, found, type, req, (unsigned long)caller,
            (unsigned long)ret, (unsigned long)tree);
    log_line(line);
    InterlockedDecrement(&in_hook);
}

void __cdecl on_item(DWORD list, DWORD index, DWORD ret) {
    char line[160];
    if (in_hook || !readable(list + 0x44, 8)) return;
    DWORD count = (*(DWORD *)(list + 0x48) - *(DWORD *)(list + 0x44)) >> 2;
    sprintf(line, "I\t%lu\t%lu\t%lu\t%lu\t%#lx\n", (unsigned long)(GetTickCount() - t0),
            (unsigned long)GetCurrentThreadId(), (unsigned long)index, (unsigned long)count, (unsigned long)ret);
    log_line(line);
}

/* ---- install ------------------------------------------------------------------------ */

static int patch(DWORD at, const unsigned char *expect, int len, void *target) {
    if (memcmp((void *)at, expect, len) != 0) return 0;
    DWORD old;
    if (!VirtualProtect((void *)at, len, PAGE_EXECUTE_READWRITE, &old)) return 0;
    unsigned char jmp[8];
    memset(jmp, 0x90, sizeof jmp);
    jmp[0] = 0xE9;
    *(DWORD *)(jmp + 1) = (DWORD)target - (at + 5);
    memcpy((void *)at, jmp, len);
    VirtualProtect((void *)at, len, old, &old);
    FlushInstructionCache(GetCurrentProcess(), (void *)at, len);
    return 1;
}

BOOL WINAPI DllMain(HINSTANCE dll, DWORD reason, LPVOID reserved) {
    (void)dll;
    (void)reserved;
    if (reason != DLL_PROCESS_ATTACH) return TRUE;
    InitializeCriticalSection(&log_lock);
    t0 = GetTickCount();
    char path[MAX_PATH];
    if (!GetEnvironmentVariableA("YO3_TRACE_FILE", path, sizeof path)) strcpy(path, "yo3_fields.tsv");
    log_file = CreateFileA(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (log_file == INVALID_HANDLE_VALUE) return TRUE;
    char line[160];
    int ok_find = patch(FIND_ADDR, FIND_PROLOGUE, 5, (void *)hook_find);
    int ok_item = patch(ITEM_ADDR, ITEM_PROLOGUE, 6, (void *)hook_item);
    sprintf(line, "#\tyo3trace\tfind=%s\titem=%s\n", ok_find ? "hooked" : "SKIPPED (bytes differ)",
            ok_item ? "hooked" : "SKIPPED (bytes differ)");
    log_line(line);
    return TRUE;
}
