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
#include "added-cards-logic.h"

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

/* The added cards (#75, added-cards-logic.h): the launcher's folder of extended card files, read from the pointer file
 * (YO3_ADDED_CARDS) at the first card file the game opens, which is after the launcher filled it in (the game is
 * suspended until then). */
static char card_added_folder[MAX_PATH * 2];
static int card_added_read;
/* Kaiba Corporation's slots the DLL makes room for: the game's 25 (0-24), then custom NPCs in 25 to NPC_SLOTS - 1.
 * 99 is the last custom slot: slot 100's name (text 0x5208 + 2 * slot) is another text of the game's. Past slot 42 the
 * deck titles move (npc_titles_install) and the players' NPC experience reaches further (NPC_MAX_ID) (docs/custom-npcs.md). */
#define NPC_SLOTS 100
/* The highest NPC id the experience blob (npcbin, 0x51C680) covers: the game's bound is 256 (`cmp esi,0x400` at 0x51C68F,
 * esi = 4 * (id - 1)); slot 99's last level is id 419. The server sends 8 * NPC_MAX_ID bytes (front.rs NPCBIN_LEN). */
#define NPC_MAX_ID 512
/* An NPC deck's title is text 0x5FB4 + 2 * id (read at 0x4BB299, 0x4BFE05, 0x5174A8); from id 250 those are player
 * titles, so higher ids' titles are NPC_HIGH_TITLE + 2 * id (yo3_data::added::deck_title_text). */
#define NPC_FIRST_HIGH_TITLE_ID 250
#define NPC_HIGH_TITLE 200000
#define NPC_CUSTOM (NPC_SLOTS - 25)
/* The NPC bodies' table (npc_models_install): the game's 25 paths, then the custom slots. A custom NPC's body is a file
 * the launcher wrote, which the resource manager's load (vtable +0x58, which looks in the archives before any open) can
 * only find by its full path on disk: once the folder is known, each slot whose model is there gets that path. */
static const char *npc_model_paths[NPC_SLOTS];
static char npc_model_names[NPC_CUSTOM][MAX_PATH * 2 + 32];
/* The cut-in's sound timing choices, ten per NPC (0x78BBF8, read once at 0x4E3EB3 by the NPC index at +0xE4): the game's
 * 25 rows, then a custom slot's donor's row (the animations it plays are the donor's), from `model/npNNN/donor.txt`. */
static int32_t npc_cutin_timing[NPC_SLOTS * 10];
/* The duel cut-in's background, `data/gfx/cutin_bg_npc%02d.swf` (01-10), by slot + 1 (0x788C40, read at 0x50627E and
 * 0x51789E): the game's 25 slots' entries, then each custom slot's, the second number of its `donor.txt` (1-10), else
 * its donor's. The game's table has 26 entries; past them sit other tables' numbers (0x788CA8 on). */
static int32_t npc_cutin_bg[NPC_SLOTS + 1];
static void npc_models_point(const char *folder) {
    for (int i = 0; i < NPC_CUSTOM; ++i) {
        char file[MAX_PATH * 2 + 32];
        snprintf(file, sizeof file, "%s/model/np%03d/np%03d.mdl", folder, 25 + i, 25 + i);
        if (GetFileAttributesA(file) == INVALID_FILE_ATTRIBUTES) continue;
        memcpy(npc_model_names[i], file, sizeof file);
        npc_model_paths[25 + i] = npc_model_names[i];
        char donor_file[MAX_PATH * 2 + 32], text[16] = {0}; DWORD n = 0; int donor = 0, bg = 0;
        snprintf(donor_file, sizeof donor_file, "%s/model/np%03d/donor.txt", folder, 25 + i);
        HANDLE f = CreateFileA(donor_file, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
        if (f != INVALID_HANDLE_VALUE) { ReadFile(f, text, sizeof text - 1, &n, NULL); CloseHandle(f); yo3_npc_donor(text, &donor, &bg); }
        memcpy(&npc_cutin_timing[(25 + i) * 10], &npc_cutin_timing[donor * 10], 10 * sizeof npc_cutin_timing[0]);
        npc_cutin_bg[25 + i + 1] = bg ? bg : npc_cutin_bg[donor + 1];
        card_profile_log("[YO3 MOD] custom NPC: slot %d's body %s (moves as NPC %d, cut-in background %d)", 25 + i, file, donor,
                         (int)npc_cutin_bg[25 + i + 1]);
    }
}
static void card_added_learn(void) {
    if (card_added_read) return;
    card_added_read = 1;
    char pointer[MAX_PATH * 2];
    if (!GetEnvironmentVariableA("YO3_ADDED_CARDS", pointer, sizeof pointer) || !pointer[0]) return;
    HANDLE f = CreateFileA(pointer, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (f == INVALID_HANDLE_VALUE) { card_profile_log("[YO3 MOD] added cards: no pointer file, the game's own cards"); return; }
    char text[MAX_PATH * 2]; DWORD n = 0;
    BOOL ok = ReadFile(f, text, sizeof text - 1, &n, NULL); CloseHandle(f);
    if (!ok || !yo3_added_folder(text, n, card_added_folder, sizeof card_added_folder)) { card_added_folder[0] = 0; return; }
    card_profile_log("[YO3 MOD] added cards: files from %s", card_added_folder);
    npc_models_point(card_added_folder);
}
static void *(__attribute__((thiscall)) *card_open_original)(void *, const char *) __attribute__((used));
static unsigned card_art_hits, card_art_asked;
static void *__attribute__((thiscall, force_align_arg_pointer)) card_open_hook(void *self, const char *path) {
    char file[MAX_PATH * 2 + 32]; int mini;
    if (path && !strncmp(path, YO3_ADDED_PREFIX, sizeof YO3_ADDED_PREFIX - 1)) {
        card_added_learn();
        if (yo3_added_path(card_added_folder, path, file, sizeof file) && GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES) {
            card_profile_log("[YO3 MOD] added cards: %s -> %s", path, file);
            return card_open_original(self, file);
        }
    }
    /* A banlist with the anime-only cards Forbidden (anime cards off), read after the card files like the pictures. */
    if (card_added_folder[0] && yo3_added_regulation_path(card_added_folder, path, file, sizeof file) && GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES) {
        card_profile_log("[YO3 MOD] added cards: %s -> %s", path, file);
        return card_open_original(self, file);
    }
    /* The Anime Collection pack: its file, the UI text tables with its name, and its picture (only when written). */
    /* How Flash names a pack picture (seen live: the shop's first few). */
    if (path && strstr(path, "img/pack/")) { static unsigned seen; if (seen++ < 4) card_profile_log("[YO3 MOD] pack picture asked: %s", path); }
    /* The UI text tables load at boot, before any card file: learn the folder here too (the launcher filled the pointer
     * before it released the game). */
    if (!card_added_folder[0] && path && (!strncmp(path, "data/text/txttbl_", 17) || !strncmp(path, "data/pack/", 10) || strstr(path, "img/pack/")))
        card_added_learn();
    if (card_added_folder[0] && yo3_added_pack_path(card_added_folder, path, file, sizeof file)) {
        static unsigned pack_logs;
        if (GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES) {
            if (pack_logs++ < 16) card_profile_log("[YO3 MOD] added cards: %s -> %s", path, file);
            return card_open_original(self, file);
        }
    }
    if (path && strstr(path, "model/np02")) { static unsigned seen; if (seen++ < 8) card_profile_log("[YO3 MOD] custom NPC asked: %s", path); }
    /* A custom NPC's body, skins, portrait and faces (slots 25 on). */
    if (card_added_folder[0] && yo3_added_npc_path(card_added_folder, path, file, sizeof file) && GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES) {
        static unsigned npc_logs;
        if (npc_logs++ < 16) card_profile_log("[YO3 MOD] custom NPC: %s -> %s", path, file);
        return card_open_original(self, file);
    }
    /* An added card's picture (the folder holds only theirs); read after the card files, so the folder is known. */
    if (card_added_folder[0] && yo3_added_art_path(card_added_folder, path, file, sizeof file) && GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES)
        return card_open_original(self, file);
    if (card_profile_on && card_art_folder[0] && yo3_profile_art_path(&card_profile, path, card_art_folder, file, sizeof file, &mini)) {
        ++card_art_asked;
        if (GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES) {
            if (card_art_hits++ < 8) card_profile_log("[YO3 MOD] card art: %s -> %s", path, file);
            return card_open_original(self, file);
        }
    }
    return card_open_original(self, path);
}
/* Edit Deck's banlist table (0x419F30, which `kinsi` reads through 0x41A510) keeps a card of a banlist only when its
 * picture exists: `data/card/%4d_1.png` asked of the resource manager's +0x98 (`mov edx,[edx+0x98]` at 0x41A058, then
 * `call edx`, thiscall, one argument). An added card's picture is in the launcher's folder, which only the open hook knew
 * about, so the anime-only cards a banlist forbids (anime cards off) lost their Forbidden mark. The load becomes
 * `mov edx, card_exists_hook`: the game's answer first, then the added folder's. */
static char __attribute__((thiscall, force_align_arg_pointer)) card_exists_hook(void *self, const char *path) {
    char (__attribute__((thiscall)) *exists)(void *, const char *) = (char (__attribute__((thiscall)) *)(void *, const char *))(*(void ***)self)[0x98 / 4];
    if (exists(self, path)) return 1;
    char file[MAX_PATH * 2 + 32];
    card_added_learn();
    return card_added_folder[0] && yo3_added_art_path(card_added_folder, path, file, sizeof file) && GetFileAttributesA(file) != INVALID_FILE_ATTRIBUTES;
}
static int card_exists_install(void) {
    static const unsigned char load[] = { 0x8b, 0x92, 0x98, 0x00, 0x00, 0x00 };       /* mov edx,[edx+0x98] */
    unsigned char *site = (unsigned char *)game_address(0x41A058);
    if (memcmp(site, load, sizeof load)) { card_profile_log("[YO3 MOD] added cards: Edit Deck's picture check is not the expected code, left alone"); return 1; }
    DWORD previous, ignored; uint32_t to = (uint32_t)(uintptr_t)card_exists_hook;
    if (!VirtualProtect(site, sizeof load, PAGE_EXECUTE_READWRITE, &previous)) return 1;
    site[0] = 0xBA; memcpy(site + 1, &to, 4); site[5] = 0x90;                          /* mov edx, imm32; nop */
    FlushInstructionCache(GetCurrentProcess(), site, sizeof load); VirtualProtect(site, sizeof load, previous, &ignored);
    return 1;
}
/* The NPC bodies: FUN_004E4180 reads the k-th NPC's model path from a 25-entry table (`mov ecx,[eax*4+0x7F5B48]` at
 * 0x4E430F, its only reader), past which sit unrelated pointers ("bad allocation"). The read takes a table of our own:
 * the game's 25 paths, then `data/model/np025.mdl` on for the custom slots. A slot with no body in the launcher's folder
 * loads nothing, as slot 25 always did. */
/* Patches a `[reg*4 + disp32]` read at `at` (its disp32 at `at + 3`, after the 3 opcode bytes `code`) from the game's
 * table `table` to `ours`: 1 when the code was as expected. */
static int npc_patch_table(uintptr_t at, const unsigned char code[3], uintptr_t table, const void *ours) {
    unsigned char *site = (unsigned char *)game_address(at);
    uint32_t was = (uint32_t)(uintptr_t)game_address(table), to = (uint32_t)(uintptr_t)ours;
    if (memcmp(site, code, 3) || memcmp(site + 3, &was, 4)) return 0;
    DWORD previous, ignored;
    if (!VirtualProtect(site + 3, 4, PAGE_EXECUTE_READWRITE, &previous)) return 0;
    memcpy(site + 3, &to, 4);
    FlushInstructionCache(GetCurrentProcess(), site, 7); VirtualProtect(site + 3, 4, previous, &ignored);
    return 1;
}
static int npc_ids_install(void);
static int npc_models_install(void) {
    /* The cut-in (CutinForNPC, constructor 0x4E3B90) loads a body only for an NPC index below 25 (`cmp edi,0x19` at
     * 0x4E3C1D, then a signed jge): NPC_SLOTS, the model table's room. Its sound timing table gets rows for the custom
     * slots too, and its background table (0x788C40) entries for them. */
    static const unsigned char below25[] = { 0x83, 0xff, 0x19 }, timing_read[] = { 0x8b, 0x04, 0x8d };
    static const unsigned char bg_read_eax[] = { 0x8b, 0x04, 0x85 }, bg_read_ecx[] = { 0x8b, 0x0c, 0x85 };
    unsigned char *limit = (unsigned char *)game_address(0x4E3C1D);
    memcpy(npc_cutin_timing, (const void *)game_address(0x78BBF8), 25 * 10 * sizeof npc_cutin_timing[0]);
    memcpy(npc_cutin_bg, (const void *)game_address(0x788C40), 26 * sizeof npc_cutin_bg[0]);
    for (int k = 25; k < NPC_SLOTS; ++k) npc_cutin_bg[k + 1] = npc_cutin_bg[1];
    if (!memcmp(limit, below25, sizeof below25) && npc_patch_table(0x4E3EB3, timing_read, 0x78BBF8, npc_cutin_timing)) {
        DWORD previous, ignored;
        if (VirtualProtect(limit + 2, 1, PAGE_EXECUTE_READWRITE, &previous)) {
            limit[2] = NPC_SLOTS;
            FlushInstructionCache(GetCurrentProcess(), limit, 3); VirtualProtect(limit + 2, 1, previous, &ignored);
        }
    } else card_profile_log("[YO3 MOD] custom NPC: the cut-in's code is not the expected code, custom NPCs get no cut-in");
    if (!npc_patch_table(0x50627E, bg_read_eax, 0x788C40, npc_cutin_bg) | !npc_patch_table(0x51789E, bg_read_ecx, 0x788C40, npc_cutin_bg))
        card_profile_log("[YO3 MOD] custom NPC: a cut-in background read is not the expected code, left alone");
    static const unsigned char read[] = { 0x8b, 0x0c, 0x85 };                     /* mov ecx,[eax*4+disp32] */
    unsigned char *site = (unsigned char *)game_address(0x4E430F);
    uint32_t table = (uint32_t)(uintptr_t)game_address(0x7F5B48);
    if (memcmp(site, read, sizeof read) || memcmp(site + 3, &table, 4)) { card_profile_log("[YO3 MOD] custom NPC: the model read is not the expected code, left alone"); return npc_ids_install(); }
    memcpy(npc_model_paths, (const void *)(uintptr_t)table, 25 * sizeof npc_model_paths[0]);
    for (int i = 0; i < NPC_CUSTOM; ++i) if (!npc_model_paths[25 + i]) {
        snprintf(npc_model_names[i], sizeof npc_model_names[i], "data/model/np%03d.mdl", 25 + i);
        npc_model_paths[25 + i] = npc_model_names[i];
    }
    if (card_added_folder[0]) npc_models_point(card_added_folder);
    DWORD previous, ignored; uint32_t to = (uint32_t)(uintptr_t)npc_model_paths;
    if (!VirtualProtect(site + 3, 4, PAGE_EXECUTE_READWRITE, &previous)) return 1;
    memcpy(site + 3, &to, 4);
    FlushInstructionCache(GetCurrentProcess(), site, 7); VirtualProtect(site + 3, 4, previous, &ignored);
    card_profile_log("[YO3 MOD] custom NPC: room for %d NPC bodies", NPC_SLOTS);
    return npc_ids_install();
}
/* The three deck title lookups: `lea R,[S+S+0x5FB4]` (7 bytes) becomes `call stub; nop; nop`, the stub keeping the
 * game's text below id 250 and NPC_HIGH_TITLE + 2 * id from there. lea leaves the flags alone, the stubs don't: each
 * site's next instruction is a call, which doesn't read them. */
#define YO3_NPC_TITLE(name, in, out) __asm__(".text\n.globl _" #name "\n_" #name ":\n cmp $250,%" #in "\n jge 1f\n" \
    " lea 0x5fb4(%" #in ",%" #in "),%" #out "\n ret\n1:\n lea 200000(%" #in ",%" #in "),%" #out "\n ret\n")
void npc_title_edi_edi(void); void npc_title_ebx_esi(void); void npc_title_eax_esi(void);
YO3_NPC_TITLE(npc_title_edi_edi, edi, edi); YO3_NPC_TITLE(npc_title_ebx_esi, ebx, esi); YO3_NPC_TITLE(npc_title_eax_esi, eax, esi);
_Static_assert(NPC_FIRST_HIGH_TITLE_ID == 250 && NPC_HIGH_TITLE == 200000, "the stubs' numbers");
static int npc_call_patch(uintptr_t at, const unsigned char was[7], void (*stub)(void)) {
    unsigned char *site = (unsigned char *)game_address(at);
    if (memcmp(site, was, 7)) return 0;
    DWORD previous, ignored;
    if (!VirtualProtect(site, 7, PAGE_EXECUTE_READWRITE, &previous)) return 0;
    int32_t rel = (int32_t)((uintptr_t)stub - ((uintptr_t)site + 5));
    site[0] = 0xE8; memcpy(site + 1, &rel, 4); site[5] = 0x90; site[6] = 0x90;
    FlushInstructionCache(GetCurrentProcess(), site, 7); VirtualProtect(site, 7, previous, &ignored);
    return 1;
}
/* Past slot 42: the deck titles' lookups (above) and the experience blob's bound (the imm32 of `cmp esi,0x400` at
 * 0x51C68F) to NPC_MAX_ID. */
static int npc_ids_install(void) {
    static const unsigned char edi_edi[] = { 0x8d, 0xbc, 0x3f, 0xb4, 0x5f, 0x00, 0x00 }, ebx_esi[] = { 0x8d, 0xb4, 0x1b, 0xb4, 0x5f, 0x00, 0x00 },
        eax_esi[] = { 0x8d, 0xb4, 0x00, 0xb4, 0x5f, 0x00, 0x00 }, bound[] = { 0x81, 0xfe, 0x00, 0x04, 0x00, 0x00 };
    int titles = npc_call_patch(0x4BB299, edi_edi, npc_title_edi_edi) + npc_call_patch(0x4BFE05, ebx_esi, npc_title_ebx_esi)
        + npc_call_patch(0x5174A8, eax_esi, npc_title_eax_esi);
    unsigned char *cmp = (unsigned char *)game_address(0x51C68F);
    int experience = 0; DWORD previous, ignored;
    if (!memcmp(cmp, bound, sizeof bound) && VirtualProtect(cmp + 2, 4, PAGE_EXECUTE_READWRITE, &previous)) {
        uint32_t to = 4 * NPC_MAX_ID;
        memcpy(cmp + 2, &to, 4);
        FlushInstructionCache(GetCurrentProcess(), cmp, 6); VirtualProtect(cmp + 2, 4, previous, &ignored);
        experience = 1;
    }
    if (experience) card_profile_log("[YO3 MOD] custom NPC: %d of 3 deck title lookups moved, experience to id %d", titles, NPC_MAX_ID);
    else card_profile_log("[YO3 MOD] custom NPC: %d of 3 deck title lookups moved, experience left at id 256 (not the expected code)", titles);
    return 1;
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

/* The duel field's background (FieldChange, DuelScene 0x50CD60, called once from 0x51464F with EBX's object in EAX and
 * the Field Spell's Konami id in ESI): 0 hides the field picture (the plain board), any other id loads
 * `data/field/<id>_1.png`, and a missing one draws the field white. An added Field Spell's picture is downloaded with the
 * card pictures; until it is, the launcher leaves `field/<id>.none` in its folder, and the id becomes 0 here. */
static void *field_change_original __attribute__((used));
static __attribute__((used)) unsigned __attribute__((cdecl)) field_shown(unsigned cid) {
    char file[MAX_PATH * 2 + 32];
    if (!cid || !card_added_folder[0]) return cid;
    snprintf(file, sizeof file, "%s/field/%u.none", card_added_folder, cid);
    if (GetFileAttributesA(file) == INVALID_FILE_ATTRIBUTES) return cid;
    card_profile_log("[YO3 MOD] added cards: Field Spell %u has no field picture yet, the plain board", cid);
    return 0;
}
void field_change_stub(void);
__asm__(".text\n.globl _field_change_stub\n_field_change_stub:\n push %esi\n push %eax\n push %ecx\n push %edx\n push %esi\n"
        " call _field_shown\n add $4,%esp\n mov %eax,%esi\n pop %edx\n pop %ecx\n pop %eax\n call *_field_change_original\n pop %esi\n ret\n");
static int field_change_install(void) {
    field_change_original = game_address(0x50CD60);
    if (!card_patch_call(0x51464F, 0x50CD60, (void *)field_change_stub)) {
        card_profile_log("[YO3 MOD] added cards: the field change is not the expected code, left alone");
        return 0;
    }
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
    card_exists_install();
    npc_models_install();
    field_change_install();
    card_open_original = (void *(__attribute__((thiscall)) *)(void *, const char *))open_tramp;
    return 1;
}
#endif
