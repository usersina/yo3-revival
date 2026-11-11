#ifndef YO3_GAME_IMAGE_H
#define YO3_GAME_IMAGE_H
#include <stdint.h>
/* Evidence addresses use the executable's preferred base. Windows may map it elsewhere under ASLR. */
static uintptr_t yo3_game_image_base __attribute__((used));
static void *game_address(uintptr_t preferred) { return (void *)(yo3_game_image_base + preferred - 0x400000u); }
#endif
