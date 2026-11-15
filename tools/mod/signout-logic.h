#ifndef YO3_SIGNOUT_LOGIC_H
#define YO3_SIGNOUT_LOGIC_H
/* The title screen's Delete Login Data (0x4CA9E0 case 3 opens dialog 0xFFFFD912, text 10217). Its Yes (0x4CA435,
 * the dialog's answer 1) blanks the user settings' "lastLogin" and saves them: `mov ecx,esi; call 0x51D1C0` at
 * 0x4CA486. Nothing else reaches that call site, so it is where direct start's saved sign-in goes too (signout.h).
 * It deletes no file of its own: the saved login (a `.y3u` file in users) stays, and with no login saved yet the game touches
 * nothing but options.y3o. Tested without the game by signout-test.c. */
#include <string.h>

#define YO3_SIGNOUT_SITE 0x4ca486u
#define YO3_SIGNOUT_SAVE 0x51d1c0u
/* mov ecx,esi; call 0x51D1C0 (rel32 from 0x4CA48D) */
static const unsigned char yo3_signout_site[] = {0x8b, 0xce, 0xe8, 0x33, 0x2d, 0x05, 0x00};

/* Whether `code` is v1.153's Yes branch of Delete Login Data. */
static int yo3_signout_site_ok(const unsigned char *code) {
    return code && !memcmp(code, yo3_signout_site, sizeof yo3_signout_site);
}
#endif
