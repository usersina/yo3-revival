#ifndef YO3_SIGNOUT_LOGIC_H
#define YO3_SIGNOUT_LOGIC_H
/* The title screen's Delete Login Data (0x4CA9E0 case 3 opens dialog 0xFFFFD912, text 10217). Its Yes (0x4CA435,
 * the dialog's answer 1) blanks the user settings' "lastLogin" and saves them: `mov ecx,esi; call 0x51D1C0` at
 * 0x4CA486. Nothing else reaches that call site, so it is where the device's saved sign-in for the server goes too
 * (signout.h): the helper revokes it on the server and forgets it (#33). The game deletes no file of its own: the saved
 * login (a `.y3u` file in users) stays, and with no login saved yet it touches nothing but options.y3o. Tested without
 * the game by signout-test.c. */
#include <stdio.h>
#include <string.h>

#define YO3_SIGNOUT_SITE 0x4ca486u
#define YO3_SIGNOUT_SAVE 0x51d1c0u
/* mov ecx,esi; call 0x51D1C0 (rel32 from 0x4CA48D) */
static const unsigned char yo3_signout_site[] = {0x8b, 0xce, 0xe8, 0x33, 0x2d, 0x05, 0x00};

/* Whether `code` is v1.153's Yes branch of Delete Login Data. */
static int yo3_signout_site_ok(const unsigned char *code) {
    return code && !memcmp(code, yo3_signout_site, sizeof yo3_signout_site);
}

/* The helper's local HTTP request (yo3-launcher SIGN_OUT_PATH), under this launch's cookie (64 hex digits). Returns its
 * length, or 0 when the cookie is not one. */
static int yo3_signout_request(char *out, size_t capacity, const char *cookie) {
    if (!cookie || strlen(cookie) != 64) return 0;
    for (const char *c = cookie; *c; ++c) if (!((*c >= '0' && *c <= '9') || (*c >= 'a' && *c <= 'f'))) return 0;
    int n = snprintf(out, capacity, "GET /session/%s/revival/sign-out HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", cookie);
    return n > 0 && (size_t)n < capacity ? n : 0;
}
#endif
