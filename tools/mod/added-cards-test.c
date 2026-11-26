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
    assert(yo3_added_art_path("C:/x", "data/field/10327_1.png", out, sizeof out) && !strcmp(out, "C:/x/field/10327.png"));
    assert(!yo3_added_art_path("C:/x", "data/field/duel_field_right.png", out, sizeof out)); /* the board itself */
    assert(!yo3_added_art_path("C:/x", "data/fieldx/10327_1.png", out, sizeof out));
    assert(!yo3_added_art_path("C:/x", "data/card/9709.png", out, sizeof out));       /* not the game's name of a face */
    assert(!yo3_added_art_path("C:/x", "data/card/9709_1.jpg", out, sizeof out));
    assert(!yo3_added_art_path("C:/x", "data/card/../9709_1.png", out, sizeof out));
    assert(!yo3_added_art_path("C:/x", "data/back/9709_1.png", out, sizeof out));
    assert(!yo3_added_art_path("", "data/card/9709_1.png", out, sizeof out));
    /* A custom NPC (slot 25 on): its body and skins, portrait and faces; the game's own 25 never. */
    assert(yo3_added_npc_path("C:/x", "data/model/np025.mdl", out, sizeof out) && !strcmp(out, "C:/x/model/np025.mdl"));
    assert(yo3_added_npc_path("C:/x", "data/model/np0250.png", out, sizeof out) && !strcmp(out, "C:/x/model/np0250.png"));
    assert(yo3_added_npc_path("C:/x", "data/npc/npcsel025.png", out, sizeof out) && !strcmp(out, "C:/x/npc/npcsel025.png"));
    assert(yo3_added_npc_path("C:/x", "data/face/np025f3.png", out, sizeof out) && !strcmp(out, "C:/x/face/np025f3.png"));
    assert(!yo3_added_npc_path("C:/x", "data/model/np005.mdl", out, sizeof out));
    assert(!yo3_added_npc_path("C:/x", "data/npc/npcsel024.png", out, sizeof out));
    assert(!yo3_added_npc_path("C:/x", "data/model/np02.mdl", out, sizeof out));
    assert(!yo3_added_npc_path("C:/x", "data/model/../np025.mdl", out, sizeof out));
    /* The Anime Collection pack: its file, the text tables and its picture, however Flash names the folder. */
    assert(yo3_added_pack_path("C:/x", "data/pack/300.y3a", out, sizeof out) && !strcmp(out, "C:/x/pack/300.y3a"));
    assert(!yo3_added_pack_path("C:/x", "data/pack/300.y3d", out, sizeof out));
    assert(!yo3_added_pack_path("C:/x", "data/pack/../300.y3a", out, sizeof out));
    assert(yo3_added_pack_path("C:/x", "data/text/txttbl_E.bin", out, sizeof out) && !strcmp(out, "C:/x/text/txttbl_E.bin"));
    assert(!yo3_added_pack_path("C:/x", "data/text/countrytbl.bin", out, sizeof out));
    assert(yo3_added_pack_path("C:/x", "data/img/pack/YO3_ANIME_1.png", out, sizeof out) && !strcmp(out, "C:/x/img/pack/YO3_ANIME_1.png"));
    assert(yo3_added_pack_path("C:/x", "data/gfx/../img/pack/YO3_ANIME_1.png", out, sizeof out) && !strcmp(out, "C:/x/img/pack/YO3_ANIME_1.png"));
    assert(!yo3_added_pack_path("C:/x", "data/img/pack/../../x.png", out, sizeof out));
    assert(!yo3_added_pack_path("C:/x", "data/img/packs/YO3.png", out, sizeof out));
    assert(!yo3_added_pack_path("", "data/pack/300.y3a", out, sizeof out));
    assert(yo3_added_regulation_path("C:/x", "data/regulation/20.y3r", out, sizeof out) && !strcmp(out, "C:/x/regulation/20.y3r"));
    assert(yo3_added_regulation_path("C:/x", "data/regulation/3.y3r", out, sizeof out) && !strcmp(out, "C:/x/regulation/3.y3r"));
    assert(!yo3_added_regulation_path("C:/x", "data/regulation/../20.y3r", out, sizeof out));
    assert(!yo3_added_regulation_path("C:/x", "data/regulation/20.y3d", out, sizeof out));
    assert(!yo3_added_regulation_path("C:/x", "data/recipe/20.y3r", out, sizeof out));
    assert(!yo3_added_regulation_path("", "data/regulation/20.y3r", out, sizeof out));      /* no folder: the game's own */
    assert(!yo3_added_regulation_path("C:/x", "data/regulation/20.y3r", out, 16));          /* too long for the buffer */
    puts("added cards: ok");
    return 0;
}
