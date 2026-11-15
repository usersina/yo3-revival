#ifndef YO3_WEB_LINKS_LOGIC_H
#define YO3_WEB_LINKS_LOGIC_H
/* The game's links to Konami's website (gone since 2012), sent to the host's server instead (web-links.h). The game
 * opens them with ShellExecuteA: Leaderboard (0x46FC14, "http://www.yugioh-online.net/v3/events/rank_info?country="),
 * the team ranking, the BOOSTERPASS page and the title screen's Official Website. Konami's domains no longer resolve;
 * whoever registered one would get every click. Tested without the game by web-links-test.c. */
#include <stdio.h>
#include <string.h>

static const char *const yo3_old_sites[] = {
    "http://www.yugioh-online.net/",
    "http://www.yugioh-online.ne.kr/",
    "http://download.yugioh-online.net/",
};

/* A link to one of Konami's old sites as a page of the host's server, through the launcher's local HTTP relay (the
 * launch cookie's path, as the game's own web requests): http://127.0.0.1:PORT/session/COOKIE/web/<the old path>.
 * 0 for any other URL or program, which the game opens as before, and for a path too long to carry. */
static int yo3_web_link(const char *url, unsigned port, const char *cookie, char *out, size_t size) {
    if (!url || !cookie || !port) return 0;
    for (size_t i = 0; i < sizeof yo3_old_sites / sizeof yo3_old_sites[0]; ++i) {
        size_t n = strlen(yo3_old_sites[i]);
        if (_strnicmp(url, yo3_old_sites[i], n)) continue;
        const char *path = url + n;
        if (strlen(path) > 512 || strpbrk(path, " \"\r\n")) return 0;
        int written = snprintf(out, size, "http://127.0.0.1:%u/session/%s/web/%s", port, cookie, path);
        return written > 0 && (size_t)written < size;
    }
    return 0;
}
#endif
