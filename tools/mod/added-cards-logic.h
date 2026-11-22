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
/* An added card's picture: the game's open of `data/card/<id>_<art>.png` (the face) or `data/mini/<id>_<art>.png` (the
 * thumbnail) becomes `folder/card/<id>.png` or `folder/mini/<id>.png`. The launcher puts only added cards' pictures in
 * its folder, so a game card's picture is never replaced here (the anime pictures go through the card profile). */
static int yo3_added_art_path(const char *folder, const char *path, char *out, size_t size) {
    if (!folder || !folder[0] || !path) return 0;
    const char *kind = !strncmp(path, "data/card/", 10) ? "card" : !strncmp(path, "data/mini/", 10) ? "mini" : NULL;
    if (!kind) return 0;
    const char *id = path + 10, *p = id;
    while (*p >= '0' && *p <= '9') ++p;
    if (p == id || p - id > 5 || *p != '_') return 0;
    const char *art = ++p;
    while (*p >= '0' && *p <= '9') ++p;
    if (p == art || strcmp(p, ".png")) return 0;
    size_t a = strlen(folder), n = (size_t)(art - 1 - id);
    if (a + 6 + n + 5 > size) return 0;
    memcpy(out, folder, a); memcpy(out + a, "/", 1); memcpy(out + a + 1, kind, 4); out[a + 5] = '/';
    memcpy(out + a + 6, id, n); memcpy(out + a + 6 + n, ".png", 5);
    return 1;
}
#endif
