#ifndef YO3_CARD_PROFILE_H
#define YO3_CARD_PROFILE_H
/* Issue #60: the card profile. The server pins which card versions a session plays (the anime setting); this makes the
 * client SHOW the same versions: kind, type, level, attribute, race, ATK/DEF, effect text and artwork, from a profile
 * (card-profile-logic.h has the format). Evidence: docs/client-internals.md "Card profiles (issue #60)".
 *   - the card table. The game loads CARD_Prop/CARD_Genre/CARD_Indx/... into one object (`env+8`, env = 0x406680()) in
 *     0x40DEE0 (fastcall, ECX env), called once from 0x4AC30E. That call is redirected to card_profile_load, which runs
 *     the load and then patches the rows of the profile in place, so every reader (deck editor, card info, duel panels,
 *     search, the Extra Deck rule) sees the profile's cards. Rows: 8 bytes at DB+0x24 (begin) / +0x28 (end); CARD_Genre at
 *     DB+0x3C / +0x40.
 *   - exact ATK/DEF (an a word holds tens only: 1 ATK is not representable): 0x5373C0/0x5373F0 and the info panel's
 *     0x418600/0x418660 are wrapped; they answer the profile's number, or 0xFFFF for "?". The card panels that do the math
 *     themselves (yo3_card_mul_sites: 0x4565C6, 0x4C7955, 0x4C876D and their DEF twins, and the deck editor's ATK/DEF range filters
 *     0x418AB7, 0x418ADE, 0x418CBF, 0x418CE1) have their `ten times` replaced (a profile's "?" is 0 there, as the game's own).
 *   - sorting by ATK/DEF (0x419050) reads the wrapped getters, so its keys are the exact numbers already; nothing more is hooked
 *     there. The search fields' blank maximum (5000, four immediates) is opened when the profile has a stat above it
 *     (card_open_max). Evidence and the key formula: yo3_card_sort_key and docs/client-internals.md "Sorting and range search".
 *   - the effect text: 0x537510 (EAX the Konami id, ESI the table, returns the text's address) is wrapped.
 *   - artwork: see card_profile_art_* below.
 * Where the profile comes from: YO3_CARD_PROFILE (a file path) else the helper's /session/<cookie>/web/card-profile, else
 * none. A game whose bytes differ, or no profile, changes nothing. */
#include "card-profile-logic.h"

#define YO3_CARD_LOAD_CALL 0x4AC30E   /* CALL 0x40DEE0 (E8 CD 1B F6 FF) */
#define YO3_CARD_LOAD_FUNC 0x40DEE0

static Yo3Profile card_profile;
static int card_profile_on, card_profile_loaded;
static uint32_t *card_rows, *card_genre;
static unsigned card_nrows, card_ngenre;

static void card_profile_log(const char *format, ...) {
    char line[256]; va_list args; va_start(args, format);
    int n = vsnprintf(line, sizeof line - 2, format, args); va_end(args);
    if (n < 0) return;
    if ((size_t)n > sizeof line - 3) n = sizeof line - 3;
    memcpy(line + n, "\r\n", 3);
    starter_log(line);
}

/* The profile text: the file named by YO3_CARD_PROFILE, else the helper's. Heap, NUL ended; NULL when there is none. */
static char *card_profile_fetch(size_t *length) {
    char path[MAX_PATH * 2];
    if (GetEnvironmentVariableA("YO3_CARD_PROFILE", path, sizeof path) && path[0]) {
        HANDLE f = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
        if (f == INVALID_HANDLE_VALUE) { card_profile_log("[YO3 MOD] card profile: cannot open YO3_CARD_PROFILE"); return NULL; }
        DWORD size = GetFileSize(f, NULL), got = 0; char *data = NULL;
        if (size != INVALID_FILE_SIZE && size <= YO3_PROFILE_MAX_BYTES && (data = (char *)malloc(size + 1))) {
            if (!ReadFile(f, data, size, &got, NULL) || got != size) { free(data); data = NULL; } else { data[size] = 0; *length = size; }
        }
        CloseHandle(f); return data;
    }
    enum { CAPACITY = 3 << 20 };
    char request[160], *reply = (char *)malloc(CAPACITY + 1);
    if (!reply) return NULL;
    int n = snprintf(request, sizeof request, "GET /session/%s/web/card-profile HTTP/1.0\r\nConnection: close\r\n\r\n", cookie_hex);
    int got = n > 0 ? helper_request(request, n, reply, CAPACITY, 10000) : 0;
    long length_of_body = got > 0 ? yo3_profile_http_body(reply, (size_t)got) : -1;
    if (length_of_body < 0) { free(reply); return NULL; }
    *length = (size_t)length_of_body;
    return reply;
}

static int card_readable(const void *p, size_t n) {
    MEMORY_BASIC_INFORMATION info;
    return p && VirtualQuery(p, &info, sizeof info) && info.State == MEM_COMMIT && !(info.Protect & (PAGE_NOACCESS | PAGE_GUARD))
        && (info.Protect & (PAGE_READWRITE | PAGE_EXECUTE_READWRITE | PAGE_WRITECOPY)) && (const char *)p + n <= (const char *)info.BaseAddress + info.RegionSize;
}

static void *card_detour(uintptr_t preferred, unsigned n, const unsigned char *expect, int call_at, void *wrapper);
static int card_art_install(void);
static void card_open_max(void);

/* The deck editor's search fields: a blank maximum is 5000, which would hide a card whose ATK or DEF is above it (5100, 6000). With such
 * a card in the profile the four immediates are raised (yo3_card_default_max_sites); the bytes are checked first, and nothing else
 * changes: without such a card, or without a profile, the client's own 5000 stays. */
static void card_open_max(void) {
    if (!card_profile_on || !yo3_profile_needs_open_max(&card_profile)) return;
    for (size_t i = 0; i < 4; ++i) if (!yo3_card_default_max_ok((const unsigned char *)game_address(yo3_card_default_max_sites[i]))) {
        card_profile_log("[YO3 MOD] card profile: the search's blank maximum is not as expected, left at 5000"); return;
    }
    for (size_t i = 0; i < 4; ++i) {
        unsigned char *imm = (unsigned char *)game_address(yo3_card_default_max_sites[i]) + 1; DWORD previous, ignored; int32_t v = YO3_STAT_NO_LIMIT;
        if (!VirtualProtect(imm, 4, PAGE_EXECUTE_READWRITE, &previous)) continue;
        memcpy(imm, &v, 4); FlushInstructionCache(GetCurrentProcess(), imm, 4); VirtualProtect(imm, 4, previous, &ignored);
    }
    card_profile_log("[YO3 MOD] card profile: the search's blank maximum is open (the profile has an ATK or DEF above 5000)");
}

/* After the game has loaded its card data: the profile, fetched and applied to the table of `env`'s card object. */
static void card_profile_patch(void *env) {
    if (!env || !card_readable(env, 0x10)) return;
    char *db = *(char **)((char *)env + 8);
    if (!db || !card_readable(db, 0xe0)) { card_profile_log("[YO3 MOD] card profile: no card table"); return; }
    uint32_t *rows = *(uint32_t **)(db + 0x24), *rows_end = *(uint32_t **)(db + 0x28);
    uint32_t *genre = *(uint32_t **)(db + 0x3c), *genre_end = *(uint32_t **)(db + 0x40);
    if (!rows || rows_end <= rows || (size_t)(rows_end - rows) % 2 || (size_t)(rows_end - rows) / 2 < 100 || (size_t)(rows_end - rows) / 2 > 60000
        || !card_readable(rows, (size_t)((char *)rows_end - (char *)rows))) { card_profile_log("[YO3 MOD] card profile: the card table is not as expected"); return; }
    unsigned nrows = (unsigned)((rows_end - rows) / 2), ngenre = 0;
    if (genre && genre_end > genre && card_readable(genre, (size_t)((char *)genre_end - (char *)genre))) ngenre = (unsigned)((genre_end - genre) / 2); else genre = NULL;
    size_t length = 0; char *text = card_profile_fetch(&length);
    if (!text) { card_profile_log("[YO3 MOD] card profile: none (the game's own cards)"); return; }
    char text_copy[40]; memcpy(text_copy, text, length < sizeof text_copy ? length : sizeof text_copy);
    int ok = yo3_profile_parse(text, length, &card_profile); free(text);
    if (!ok) {
        char head[40]; size_t n = length < sizeof head - 1 ? length : sizeof head - 1;
        for (size_t i = 0; i < n; ++i) head[i] = (unsigned char)text_copy[i] >= 32 && (unsigned char)text_copy[i] < 127 ? text_copy[i] : '.';
        head[n] = 0;
        card_profile_log("[YO3 MOD] card profile: not a profile (%u bytes, starts \"%s\"), ignored", (unsigned)length, head); return;
    }
    unsigned refused = 0, done = yo3_profile_apply(&card_profile, rows, nrows, genre, ngenre, &refused);
    card_rows = rows; card_genre = genre; card_nrows = nrows; card_ngenre = ngenre; card_profile_on = 1;
    card_profile_log("[YO3 MOD] card profile %s (%s): %u cards patched, %u refused, %u texts, %u artworks, %u lines skipped",
        card_profile.name, card_profile.hash, done, refused, card_profile.ntexts, card_profile.narts, card_profile.skipped);
    card_art_install();
    card_open_max();
}


/* Artwork. Both ways the game shows a picture by name (the Flash screens' images and the Irrlicht textures of the duel's
 * cards) end in the file system's createAndOpenFile, 0x5505E0 (thiscall, ECX the file system, one stack argument: the
 * name as a char *): it asks its archives (the .ypk files) first, and opens a name they don't hold as a file on disk. The
 * function is wrapped: for "data/card/<cid>_<art>.png" (the face) and "data/mini/<cid>_<art>.png" (the thumbnail) of a
 * card the profile lists with `art`, whose picture is in the art folder (card/<cid>.png, mini/<cid>.png; YO3_ANIME_ART,
 * else <game folder>\anime-art), the name becomes that file's path, which no archive holds. Anything else is untouched. */
static char card_art_folder[MAX_PATH * 2];
static void *(__attribute__((thiscall)) *card_open_original)(void *, const char *) __attribute__((used));
static unsigned card_art_hits, card_art_asked;
static void *__attribute__((thiscall, force_align_arg_pointer)) card_open_hook(void *self, const char *path) {
    char file[MAX_PATH * 2 + 32]; int mini;
    if (card_profile_on && card_art_folder[0] && yo3_profile_art_path(&card_profile, path, card_art_folder, file, sizeof file, &mini)) {
        ++card_art_asked;
        if (GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES) {
            if (card_art_hits++ < 8) card_profile_log("[YO3 MOD] card art: %s -> %s", path, file);
            return card_open_original(self, file);
        }
    }
    return card_open_original(self, path);
}
static int card_art_install(void) {
    if (!card_profile.narts) return 1;
    if (!GetEnvironmentVariableA("YO3_ANIME_ART", card_art_folder, sizeof card_art_folder - 8) || !card_art_folder[0]) {
        DWORD n = GetModuleFileNameA(NULL, card_art_folder, sizeof card_art_folder - 16);
        char *slash = n ? strrchr(card_art_folder, '\\') : NULL;
        if (!slash) { card_art_folder[0] = 0; return 1; }
        strcpy(slash, "\\anime-art");
    }
    for (char *c = card_art_folder; *c; ++c) if (*c == '\\') *c = '/';
    if (!card_open_original) { card_profile_log("[YO3 MOD] card art: the game's file opener is not hooked, the original art stays"); card_art_folder[0] = 0; return 0; }
    card_profile_log("[YO3 MOD] card art: folder %s", card_art_folder);
    return 1;
}

/* 0x40DEE0, fastcall: the game's card load, then the profile. */
static int __attribute__((fastcall, force_align_arg_pointer)) card_profile_load(void *env) {
    int result = ((int (__attribute__((fastcall)) *)(void *))game_address(YO3_CARD_LOAD_FUNC))(env);
    if (!card_profile_loaded) { card_profile_loaded = 1; card_profile_patch(env); }
    return result;
}

/* A detour: the first `n` bytes at `address` (checked against `expect`) are copied into a trampoline (a CALL at offset
 * `call_at` is re-aimed) that continues after them; the function starts with a JMP to `wrapper`. Returns the
 * trampoline, NULL when the bytes are not the expected ones. */
static void *card_detour(uintptr_t preferred, unsigned n, const unsigned char *expect, int call_at, void *wrapper) {
    unsigned char *site = (unsigned char *)game_address(preferred);
    if (memcmp(site, expect, n)) return NULL;
    unsigned char *tramp = (unsigned char *)VirtualAlloc(NULL, 64, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if (!tramp) return NULL;
    memcpy(tramp, site, n);
    if (call_at >= 0) { int32_t rel; memcpy(&rel, site + call_at + 1, 4); intptr_t target = (intptr_t)(site + call_at + 5) + rel; rel = (int32_t)(target - (intptr_t)(tramp + call_at + 5)); memcpy(tramp + call_at + 1, &rel, 4); }
    tramp[n] = 0xE9; { int32_t rel = (int32_t)((intptr_t)(site + n) - (intptr_t)(tramp + n + 5)); memcpy(tramp + n + 1, &rel, 4); }
    DWORD previous, ignored;
    if (!VirtualProtect(site, n, PAGE_EXECUTE_READWRITE, &previous)) return NULL;
    site[0] = 0xE9; { int32_t rel = (int32_t)((intptr_t)wrapper - (intptr_t)(site + 5)); memcpy(site + 1, &rel, 4); }
    for (unsigned i = 5; i < n; ++i) site[i] = 0x90;
    FlushInstructionCache(GetCurrentProcess(), site, n); VirtualProtect(site, n, previous, &ignored);
    return tramp;
}

/* ATK/DEF and the text. The game's getters take the Konami id in EAX (and the card table in ESI) and answer in EAX; the
 * wrappers run the original and let the profile change the answer: cdecl callbacks (id, the original answer). The info
 * panel's 0x418600/0x418660 get the id through a stub on their CALL 0x537230 (EAX the id there). */
static void *card_tramp_atk __attribute__((used)), *card_tramp_def __attribute__((used)), *card_tramp_text __attribute__((used)), *card_tramp_info_atk __attribute__((used)), *card_tramp_info_def __attribute__((used));
static unsigned card_last_cid __attribute__((used));
static __attribute__((used)) int __attribute__((cdecl)) card_cb_atk(unsigned cid, int original) { int v; return card_profile_on && yo3_profile_stat_of(&card_profile, cid, 0, &v) ? v : original; }
static __attribute__((used)) int __attribute__((cdecl)) card_cb_def(unsigned cid, int original) { int v; return card_profile_on && yo3_profile_stat_of(&card_profile, cid, 1, &v) ? v : original; }
static __attribute__((used)) const wchar_t *__attribute__((cdecl)) card_cb_text(unsigned cid, const wchar_t *original) {
    const wchar_t *t = card_profile_on ? yo3_profile_text_of(&card_profile, card_rows, card_nrows, cid) : NULL; return t ? t : original;
}
static __attribute__((used)) int __attribute__((cdecl)) card_cb_last_atk(unsigned unused, int original) { (void)unused; return card_cb_atk(card_last_cid, original); }
static __attribute__((used)) int __attribute__((cdecl)) card_cb_last_def(unsigned unused, int original) { (void)unused; return card_cb_def(card_last_cid, original); }
static __attribute__((used)) void __attribute__((cdecl)) card_forget_cid(void) { card_last_cid = 0; }
static void *card_orig_rows_by_cid __attribute__((used)); /* 0x537230 */
/* The panels' own `ten times the nine bits` (yo3_card_mul_sites): the profile's number when it has one. */
static __attribute__((used)) int __attribute__((cdecl)) card_cb_mul_atk(unsigned nine) { int v; return card_profile_on && yo3_profile_stat_of(&card_profile, card_last_cid, 0, &v) ? v : yo3_card_times_ten(nine); }
static __attribute__((used)) int __attribute__((cdecl)) card_cb_mul_def(unsigned nine) { int v; return card_profile_on && yo3_profile_stat_of(&card_profile, card_last_cid, 1, &v) ? v : yo3_card_times_ten(nine); }
/* The range filters' (yo3_card_filter_stat): the same, but a profile's "?" is 0 there, as the game's own "?" is. */
static __attribute__((used)) int __attribute__((cdecl)) card_cb_filter_atk(unsigned nine) { return card_profile_on ? yo3_card_filter_stat(&card_profile, card_last_cid, 0, nine) : yo3_card_times_ten(nine); }
static __attribute__((used)) int __attribute__((cdecl)) card_cb_filter_def(unsigned nine) { return card_profile_on ? yo3_card_filter_stat(&card_profile, card_last_cid, 1, nine) : yo3_card_times_ten(nine); }

#define YO3_CARD_WRAPPER(name, tramp, callback) \
    __asm__(".text\n.globl _" #name "\n_" #name ":\n push %eax\n call *_" #tramp "\n pop %ecx\n push %eax\n push %ecx\n call _" #callback "\n add $8,%esp\n ret\n")
#define YO3_CARD_WRAPPER_FORGET(name, tramp, callback) \
    __asm__(".text\n.globl _" #name "\n_" #name ":\n push %eax\n call _card_forget_cid\n pop %eax\n push %eax\n call *_" #tramp "\n pop %ecx\n push %eax\n push %ecx\n call _" #callback "\n add $8,%esp\n ret\n")
void card_wrap_atk(void); void card_wrap_def(void); void card_wrap_text(void); void card_wrap_info_atk(void); void card_wrap_info_def(void);
YO3_CARD_WRAPPER(card_wrap_atk, card_tramp_atk, card_cb_atk);
YO3_CARD_WRAPPER(card_wrap_def, card_tramp_def, card_cb_def);
YO3_CARD_WRAPPER(card_wrap_text, card_tramp_text, card_cb_text);
YO3_CARD_WRAPPER_FORGET(card_wrap_info_atk, card_tramp_info_atk, card_cb_last_atk);
YO3_CARD_WRAPPER_FORGET(card_wrap_info_def, card_tramp_info_def, card_cb_last_def);

/* The CALL 0x537230 inside 0x418600 / 0x418660 (EAX the Konami id): note the id, then do the call. */
void card_stub_note_cid(void);
__asm__(".text\n.globl _card_stub_note_cid\n_card_stub_note_cid:\n mov %eax,_card_last_cid\n jmp *_card_orig_rows_by_cid\n");


/* The five bytes `lea R,[eax+eax*4]; add R,R` become CALL stub_R_kind: the nine bits are in eax; the answer goes to R and
 * the other registers stay as they were. */
#define YO3_CARD_MUL_EAX(name, cb) __asm__(".text\n.globl _" #name "\n_" #name ":\n push %ecx\n push %edx\n push %eax\n call _" #cb "\n add $4,%esp\n pop %edx\n pop %ecx\n ret\n")
#define YO3_CARD_MUL_ECX(name, cb) __asm__(".text\n.globl _" #name "\n_" #name ":\n push %eax\n push %edx\n push %eax\n call _" #cb "\n add $4,%esp\n mov %eax,%ecx\n pop %edx\n pop %eax\n ret\n")
#define YO3_CARD_MUL_EDX(name, cb) __asm__(".text\n.globl _" #name "\n_" #name ":\n push %eax\n push %ecx\n push %eax\n call _" #cb "\n add $4,%esp\n mov %eax,%edx\n pop %ecx\n pop %eax\n ret\n")
void card_mul_atk_eax(void); void card_mul_atk_edx(void); void card_mul_def_eax(void); void card_mul_def_ecx(void);
void card_filter_atk_eax(void); void card_filter_def_eax(void);
YO3_CARD_MUL_EAX(card_filter_atk_eax, card_cb_filter_atk); YO3_CARD_MUL_EAX(card_filter_def_eax, card_cb_filter_def);
YO3_CARD_MUL_EAX(card_mul_atk_eax, card_cb_mul_atk); YO3_CARD_MUL_EDX(card_mul_atk_edx, card_cb_mul_atk);
YO3_CARD_MUL_EAX(card_mul_def_eax, card_cb_mul_def); YO3_CARD_MUL_ECX(card_mul_def_ecx, card_cb_mul_def);
static void *card_mul_stub(const Yo3CardMulSite *site) {
    if (site->filter) return site->reg == 0 ? (void *)(site->def ? card_filter_def_eax : card_filter_atk_eax) : NULL;
    if (site->def) return site->reg == 0 ? (void *)card_mul_def_eax : site->reg == 1 ? (void *)card_mul_def_ecx : NULL;
    return site->reg == 0 ? (void *)card_mul_atk_eax : site->reg == 2 ? (void *)card_mul_atk_edx : NULL;
}

static int card_patch_call(uintptr_t preferred, uintptr_t target, void *replacement) {
    unsigned char *site = (unsigned char *)game_address(preferred);
    int32_t rel; memcpy(&rel, site + 1, 4);
    if (site[0] != 0xE8 || (intptr_t)(site + 5) + rel != (intptr_t)game_address(target)) return 0;
    DWORD previous, ignored;
    if (!VirtualProtect(site, 5, PAGE_EXECUTE_READWRITE, &previous)) return 0;
    rel = (int32_t)((intptr_t)replacement - (intptr_t)(site + 5)); memcpy(site + 1, &rel, 4);
    FlushInstructionCache(GetCurrentProcess(), site, 5); VirtualProtect(site, 5, previous, &ignored);
    return 1;
}

static int card_profile_install(void) {
    BYTE *base = (BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As deck-upload.h: the contract fixtures keep their own code at these addresses. */
    if (nt->OptionalHeader.SizeOfImage < 0x42e000) return 1;
    if (!card_patch_call(YO3_CARD_LOAD_CALL, YO3_CARD_LOAD_FUNC, (void *)card_profile_load)) return 0;
    card_orig_rows_by_cid = game_address(0x537230);
    static const unsigned char text[] = { 0x51, 0x8b, 0x8e, 0xd0, 0x00, 0x00, 0x00 };       /* push ecx; mov ecx,[esi+0xd0] */
    static const unsigned char atk[] = { 0x83, 0xec, 0x0c, 0xe8, 0x68, 0xfe, 0xff, 0xff };  /* sub esp,0xc; call 0x537230 */
    static const unsigned char def[] = { 0x83, 0xec, 0x0c, 0xe8, 0x38, 0xfe, 0xff, 0xff };
    static const unsigned char info[] = { 0x83, 0xec, 0x08, 0x56, 0x8b, 0xf0 };              /* sub esp,8; push esi; mov esi,eax */
    if (!(card_tramp_text = card_detour(0x537510, sizeof text, text, -1, (void *)card_wrap_text))) return 0;
    if (!(card_tramp_atk = card_detour(0x5373c0, sizeof atk, atk, 3, (void *)card_wrap_atk))) return 0;
    if (!(card_tramp_def = card_detour(0x5373f0, sizeof def, def, 3, (void *)card_wrap_def))) return 0;
    if (!card_patch_call(0x41862c, 0x537230, (void *)card_stub_note_cid) || !card_patch_call(0x41868c, 0x537230, (void *)card_stub_note_cid)) return 0;
    if (!(card_tramp_info_atk = card_detour(0x418600, sizeof info, info, -1, (void *)card_wrap_info_atk))) return 0;
    if (!(card_tramp_info_def = card_detour(0x418660, sizeof info, info, -1, (void *)card_wrap_info_def))) return 0;
    /* The other panels (the card panel 0x4565C6, 0x4C7955 and 0x4C876D families) do the math inline: check every site,
     * then patch them all or none. */
    for (size_t i = 0; i < sizeof yo3_card_mul_sites / sizeof *yo3_card_mul_sites; ++i) {
        const Yo3CardMulSite *site = &yo3_card_mul_sites[i];
        unsigned char *call = (unsigned char *)game_address(site->call); int32_t rel; memcpy(&rel, call + 1, 4);
        if (call[0] != 0xE8 || (intptr_t)(call + 5) + rel != (intptr_t)game_address(0x537230)) return 0;
        if (!yo3_card_mul_bytes_ok((const unsigned char *)game_address(site->mul), site->reg) || !card_mul_stub(site)) return 0;
    }
    for (size_t i = 0; i < sizeof yo3_card_mul_sites / sizeof *yo3_card_mul_sites; ++i) {
        const Yo3CardMulSite *site = &yo3_card_mul_sites[i];
        if (!card_patch_call(site->call, 0x537230, (void *)card_stub_note_cid)) return 0;
        unsigned char *mul = (unsigned char *)game_address(site->mul); DWORD previous, ignored;
        if (!VirtualProtect(mul, 5, PAGE_EXECUTE_READWRITE, &previous)) return 0;
        yo3_card_write_call(mul, (unsigned long)(uintptr_t)card_mul_stub(site));
        FlushInstructionCache(GetCurrentProcess(), mul, 5); VirtualProtect(mul, 5, previous, &ignored);
    }
    static const unsigned char open_prologue[] = { 0x55, 0x8b, 0xec, 0x83, 0xe4, 0xf8, 0x83, 0xec, 0x24 }; /* push ebp; mov ebp,esp; and esp,-8; sub esp,0x24 */
    void *open_tramp = card_detour(0x5505E0, sizeof open_prologue, open_prologue, -1, (void *)card_open_hook);
    if (!open_tramp) return 0;
    card_open_original = (void *(__attribute__((thiscall)) *)(void *, const char *))open_tramp;
    return 1;
}
#endif
