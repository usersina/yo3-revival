/* The website links (web-links-logic.h), without the game: Konami's old sites go to the host's server through the
 * launch's local relay; anything else is left alone. */
#include <assert.h>
#include <stdio.h>
#include "web-links-logic.h"

int main(void) {
    char out[1024];
    const char *cookie = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
    assert(yo3_web_link("http://www.yugioh-online.net/v3/events/rank_info?country=GB", 41234, cookie, out, sizeof out));
    assert(!strcmp(out, "http://127.0.0.1:41234/session/00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff/web/v3/events/rank_info?country=GB"));
    assert(yo3_web_link("http://www.yugioh-online.net/", 1, cookie, out, sizeof out) && strstr(out, "/web/") && out[strlen(out) - 1] == '/');
    assert(yo3_web_link("http://www.yugioh-online.ne.kr/v3/bpass/index.html", 1, cookie, out, sizeof out) && strstr(out, "/web/v3/bpass/index.html"));
    assert(yo3_web_link("HTTP://WWW.YUGIOH-ONLINE.NET/v3/bpass/index/", 1, cookie, out, sizeof out));
    /* Anything else is the game's business: a local program, another site, a look-alike domain. */
    assert(!yo3_web_link("C:\\Program Files\\Konami\\yo3.exe", 1, cookie, out, sizeof out));
    assert(!yo3_web_link("http://www.yugioh-online.net.example.com/", 1, cookie, out, sizeof out));
    assert(!yo3_web_link("https://example.com/", 1, cookie, out, sizeof out));
    assert(!yo3_web_link(NULL, 1, cookie, out, sizeof out));
    /* No relay yet, a path that could break out of the URL, or too little room: left alone. */
    assert(!yo3_web_link("http://www.yugioh-online.net/", 0, cookie, out, sizeof out));
    assert(!yo3_web_link("http://www.yugioh-online.net/a b", 1, cookie, out, sizeof out));
    assert(!yo3_web_link("http://www.yugioh-online.net/v3/", 1, cookie, out, 20));
    puts("PASS: the old website links go to the host's server, and nothing else is touched");
    return 0;
}
