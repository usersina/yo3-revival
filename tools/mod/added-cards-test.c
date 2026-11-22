/* The added cards' redirect (added-cards-logic.h), without the game. */
#include <assert.h>
#include <stdio.h>
#include "added-cards-logic.h"

int main(void) {
    char folder[64], out[96];
    assert(yo3_added_folder("C:\\Users\\a\\added-cards\\9f\r\n", 27, folder, sizeof folder) && !strcmp(folder, "C:/Users/a/added-cards/9f"));
    assert(!yo3_added_folder("", 0, folder, sizeof folder));                 /* the launcher's empty pointer: nothing added */
    assert(!yo3_added_folder(" \r\n", 3, folder, sizeof folder));
    assert(!yo3_added_folder("C:\\a-very-long-folder-name-that-does-not-fit-in-the-buffer-at-all\\x", 66, folder, sizeof folder));

    assert(yo3_added_path("C:/x", "data/bin#/CARD_PROP.bin", out, sizeof out) && !strcmp(out, "C:/x/CARD_PROP.bin"));
    assert(yo3_added_path("C:/x", "data/bin#/CARD_Sort2_E.bin", out, sizeof out) && !strcmp(out, "C:/x/CARD_Sort2_E.bin"));
    assert(!yo3_added_path("C:/x", "data/bin#/DLG_Text_E.bin", out, sizeof out));   /* not a card file */
    assert(!yo3_added_path("C:/x", "data/card/9709_1.png", out, sizeof out));
    assert(!yo3_added_path("C:/x", "data/bin#/CARD_/../server.crt", out, sizeof out));
    assert(!yo3_added_path("", "data/bin#/CARD_PROP.bin", out, sizeof out));        /* no folder: the game's own */
    assert(!yo3_added_path("C:/x", "data/bin#/CARD_PROP.bin", out, 8));               /* too long for the buffer */
    assert(yo3_added_art_path("C:/x", "data/card/9709_1.png", out, sizeof out) && !strcmp(out, "C:/x/card/9709.png"));
    assert(yo3_added_art_path("C:/x", "data/mini/9709_1.png", out, sizeof out) && !strcmp(out, "C:/x/mini/9709.png"));
    assert(!yo3_added_art_path("C:/x", "data/card/9709.png", out, sizeof out));       /* not the game's name of a face */
    assert(!yo3_added_art_path("C:/x", "data/card/9709_1.jpg", out, sizeof out));
    assert(!yo3_added_art_path("C:/x", "data/card/../9709_1.png", out, sizeof out));
    assert(!yo3_added_art_path("C:/x", "data/back/9709_1.png", out, sizeof out));
    assert(!yo3_added_art_path("", "data/card/9709_1.png", out, sizeof out));
    puts("added cards: ok");
    return 0;
}
