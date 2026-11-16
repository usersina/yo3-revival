/* Delete Login Data (signout-logic.h), without the game: the guarded call site and where its call goes. */
#include <assert.h>
#include <stdio.h>
#include "signout-logic.h"

int main(void) {
    unsigned char code[] = {0x8b, 0xce, 0xe8, 0x33, 0x2d, 0x05, 0x00};
    assert(yo3_signout_site_ok(code));
    int rel; memcpy(&rel, code + 3, 4);
    assert(YO3_SIGNOUT_SITE + 7 + rel == YO3_SIGNOUT_SAVE);
    code[6] = 0x01; assert(!yo3_signout_site_ok(code)); /* another build: left alone */
    assert(!yo3_signout_site_ok(NULL));
    char request[256], cookie[65]; memset(cookie, 'a', 64); cookie[64] = 0;
    int n = yo3_signout_request(request, sizeof request, cookie);
    assert(n > 0 && !strncmp(request, "GET /session/aaaa", 17) && strstr(request, "/revival/sign-out HTTP/1.1\r\n"));
    assert(!strcmp(request + n - 4, "\r\n\r\n"));
    cookie[3] = 'G'; assert(!yo3_signout_request(request, sizeof request, cookie)); /* never a path of anything else */
    assert(!yo3_signout_request(request, sizeof request, "abc"));
    assert(!yo3_signout_request(request, 40, "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
    puts("PASS: Delete Login Data's Yes is found only at v1.153's call site, and asks the helper to forget the sign-in");
    return 0;
}
