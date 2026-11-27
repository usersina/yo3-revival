/* The cards the game never had (#75): which of the game's file opens get the launcher's extended copy instead.
 *
 * The launcher writes the game's card files with the server's added rows into a folder (crates/yo3-launcher/src/added.rs,
 * yo3_data::added::extend_files) and names that folder in a small pointer file whose path is in YO3_ADDED_CARDS (the
 * folder is filled in once the launcher has signed in, before the game is released; empty: nothing is added). The game
 * loads its card files by name through the file system's createAndOpenFile (0x5505E0; `data/bin#/CARD_PROP.bin` and the
 * rest, live): an open of a `data/bin#/CARD_` file becomes the folder's file of the same last part when the folder has
 * one. The game's loader reads each file to its end into growable arrays and computes CARD_IntID's bounds from the rows
 * (0x40DEE0, 0x537770, 0x537890, 0x537970, 0x536FF0), so longer files need nothing else. Pure logic, tested alone. */
#ifndef YO3_ADDED_CARDS_LOGIC_H
#define YO3_ADDED_CARDS_LOGIC_H
#include <stddef.h>
#include <string.h>

#define YO3_ADDED_PREFIX "data/bin#/CARD_"

/* The folder from the pointer file's text (`text`, `n` bytes): trimmed of spaces and line ends, its `\` made `/` (the
 * game's file system takes forward slashes, as the anime artwork's paths). 0 when there is none or it is too long. */
static int yo3_added_folder(const char *text, size_t n, char *folder, size_t size) {
    while (n && (text[n - 1] == '\r' || text[n - 1] == '\n' || text[n - 1] == ' ')) --n;
    while (n && (*text == ' ' || *text == '\r' || *text == '\n')) { ++text; --n; }
    if (!n || n + 1 > size) return 0;
    for (size_t i = 0; i < n; ++i) folder[i] = text[i] == '\\' ? '/' : text[i];
    folder[n] = 0;
    return 1;
}

/* The extended copy's path for the game's open of `path`: `folder/<last part>` for a `data/bin#/CARD_...` file, else 0.
 * A path with another folder in its name is never redirected. */
static int yo3_added_path(const char *folder, const char *path, char *out, size_t size) {
    if (!folder || !folder[0] || !path || strncmp(path, YO3_ADDED_PREFIX, sizeof YO3_ADDED_PREFIX - 1)) return 0;
    const char *leaf = path + sizeof YO3_ADDED_PREFIX - 1 - 5; /* "CARD_..." */
    if (strchr(leaf, '/') || strchr(leaf, '\\') || strstr(leaf, "..")) return 0;
    size_t a = strlen(folder), b = strlen(leaf);
    if (a + 1 + b + 1 > size) return 0;
    memcpy(out, folder, a); out[a] = '/'; memcpy(out + a + 1, leaf, b + 1);
    return 1;
}
/* A banlist with the anime-only cards Forbidden (the server has anime cards off): the game's open of
 * `data/regulation/<id>.y3r` (0x52DB30 builds the name) becomes `folder/regulation/<id>.y3r` when the launcher wrote one. */
#define YO3_REGULATION_PREFIX "data/regulation/"
static int yo3_added_regulation_path(const char *folder, const char *path, char *out, size_t size) {
    if (!folder || !folder[0] || !path || strncmp(path, YO3_REGULATION_PREFIX, sizeof YO3_REGULATION_PREFIX - 1)) return 0;
    const char *leaf = path + sizeof YO3_REGULATION_PREFIX - 1, *p = leaf;
    while (*p >= '0' && *p <= '9') ++p;
    if (p == leaf || p - leaf > 5 || strcmp(p, ".y3r")) return 0;
    size_t a = strlen(folder), b = strlen(leaf);
    if (a + 12 + b + 1 > size) return 0;
    memcpy(out, folder, a); memcpy(out + a, "/regulation/", 12); memcpy(out + a + 12, leaf, b + 1);
    return 1;
}
/* The Anime Collection pack (yo3_data::added::anime_pack_files): the game's open of `data/pack/<id>.y3a` (0x52DCF0),
 * `data/text/txttbl_<lang>.bin` (0x51E5B0) or a pack picture (Flash's loadClip of "../img/pack/<name>", which may come
 * as `data/img/pack/<name>` or with the Flash folder before it, `data/gfx/../img/pack/<name>`) becomes `folder/pack/...`,
 * `folder/text/...` or `folder/img/pack/<name>`. The caller redirects only when that file exists, so the game's own
 * packs, pictures and texts stay its own unless the launcher wrote a copy. */
static int yo3_added_join(const char *folder, const char *kind, const char *leaf, char *out, size_t size) {
    if (!*leaf || strchr(leaf, '/') || strchr(leaf, '\\') || strstr(leaf, "..")) return 0;
    size_t a = strlen(folder), k = strlen(kind), b = strlen(leaf);
    if (a + 1 + k + 1 + b + 1 > size) return 0;
    memcpy(out, folder, a); out[a] = '/'; memcpy(out + a + 1, kind, k); out[a + 1 + k] = '/'; memcpy(out + a + 2 + k, leaf, b + 1);
    return 1;
}
static int yo3_added_pack_path(const char *folder, const char *path, char *out, size_t size) {
    if (!folder || !folder[0] || !path || strncmp(path, "data/", 5)) return 0;
    if (!strncmp(path, "data/pack/", 10)) {
        const char *leaf = path + 10, *p = leaf;
        while (*p >= '0' && *p <= '9') ++p;
        if (p == leaf || p - leaf > 5 || strcmp(p, ".y3a")) return 0;
        return yo3_added_join(folder, "pack", leaf, out, size);
    }
    if (!strncmp(path, "data/text/txttbl_", 17)) {
        const char *leaf = path + 10;
        size_t n = strlen(leaf);
        if (n != 12 || strcmp(leaf + 8, ".bin")) return 0; /* txttbl_E.bin */
        return yo3_added_join(folder, "text", leaf, out, size);
    }
    const char *img = strstr(path, "img/pack/");
    if (img && (img == path + 5 || img[-1] == '/')) {
        const char *leaf = img + 9;
        size_t n = strlen(leaf);
        if (n < 5 || n > 64 || (strcmp(leaf + n - 4, ".png") && strcmp(leaf + n - 4, ".PNG"))) return 0;
        return yo3_added_join(folder, "img/pack", leaf, out, size);
    }
    return 0;
}
/* A custom NPC's files (slots 25 on, beyond the game's 25): the game's open of `data/model/npNNN...` (its body and the
 * skins it names), `data/npc/npcselNNN.png` (the Kaiba Corporation portrait) or `data/face/npNNNf<k>.png` (the duel's
 * faces) with NNN 025 or more becomes `folder/model/...`, `folder/npc/...` or `folder/face/...`; the caller redirects
 * only when the launcher wrote that file. The game's own 25 NPCs (000-024) are never touched. */
static int yo3_added_npc_path(const char *folder, const char *path, char *out, size_t size) {
    if (!folder || !folder[0] || !path) return 0;
    static const struct { const char *prefix, *kind, *stem; } kinds[] = {
        { "data/model/", "model", "np" }, { "data/npc/", "npc", "npcsel" }, { "data/face/", "face", "np" } };
    for (size_t i = 0; i < sizeof kinds / sizeof kinds[0]; ++i) {
        size_t n = strlen(kinds[i].prefix), s = strlen(kinds[i].stem);
        if (strncmp(path, kinds[i].prefix, n)) continue;
        const char *leaf = path + n;
        if (strncmp(leaf, kinds[i].stem, s)) return 0;
        const char *d = leaf + s;
        if (d[0] < '0' || d[0] > '9' || d[1] < '0' || d[1] > '9' || d[2] < '0' || d[2] > '9') return 0;
        int slot = (d[0] - '0') * 100 + (d[1] - '0') * 10 + (d[2] - '0');
        if (slot < 25) return 0;
        return yo3_added_join(folder, kinds[i].kind, leaf, out, size);
    }
    return 0;
}
/* A custom NPC's `model/npNNN/donor.txt` (yo3_data::added::npc_files): the game NPC (0-24) whose animations and cut-in
 * timing it uses, then, optionally after a space or a line break, its duel cut-in background (`cutin_bg_npc01.swf` to 10).
 * A donor out of range is 0 (Yami Yugi); a missing or out-of-range background is 0, "the donor's". */
static void yo3_npc_donor(const char *text, int *donor, int *bg) {
    int n[2] = {0, 0}, k = 0, digits = 0;
    *donor = 0; *bg = 0;
    for (const char *c = text; c && *c && k < 2; ++c) {
        if (*c >= '0' && *c <= '9') { if (n[k] < 1000) n[k] = n[k] * 10 + (*c - '0'); digits = 1; }
        else if (digits) { ++k; digits = 0; }
    }
    if (n[0] < 25) *donor = n[0];
    if (n[1] >= 1 && n[1] <= 10) *bg = n[1];
}
/* An added card's picture: the game's open of `data/card/<id>_<art>.png` (the face), `data/mini/<id>_<art>.png` (the
 * thumbnail) or `data/field/<id>_1.png` (a Field Spell's duel field) becomes `folder/card/<id>.png`, `folder/mini/<id>.png`
 * or `folder/field/<id>.png`. The launcher puts only added cards' pictures in its folder, so a game card's picture is
 * never replaced here (the anime pictures go through the card profile). */
static int yo3_added_art_path(const char *folder, const char *path, char *out, size_t size) {
    if (!folder || !folder[0] || !path) return 0;
    const char *kind = !strncmp(path, "data/card/", 10) ? "card" : !strncmp(path, "data/mini/", 10) ? "mini"
                     : !strncmp(path, "data/field/", 11) ? "field" : NULL;
    if (!kind) return 0;
    size_t k = strlen(kind);
    const char *id = path + 6 + k, *p = id;
    while (*p >= '0' && *p <= '9') ++p;
    if (p == id || p - id > 5 || *p != '_') return 0;
    const char *art = ++p;
    while (*p >= '0' && *p <= '9') ++p;
    if (p == art || strcmp(p, ".png")) return 0;
    size_t a = strlen(folder), n = (size_t)(art - 1 - id);
    if (a + 2 + k + n + 5 > size) return 0;
    memcpy(out, folder, a); out[a] = '/'; memcpy(out + a + 1, kind, k); out[a + 1 + k] = '/';
    memcpy(out + a + 2 + k, id, n); memcpy(out + a + 2 + k + n, ".png", 5);
    return 1;
}
#endif
