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
    puts("PASS: Delete Login Data's Yes is found only at v1.153's call site");
    return 0;
}
