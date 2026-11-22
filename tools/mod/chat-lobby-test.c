/* The chat's automatic Lobby (chat-lobby-logic.h), without the game: once per login, only into a free window, room 1. */
#include <assert.h>
#include <stdio.h>
#include "chat-lobby-logic.h"

static void room(unsigned char *at, int32_t id, int32_t type) { memcpy(at, &id, 4); memcpy(at + 0x30, &type, 4); }

int main(void) {
    unsigned char rooms[3 * YO3_CHAT_ROOM_SIZE] = {0};
    room(rooms, 2, 0); room(rooms + YO3_CHAT_ROOM_SIZE, 1, 1); room(rooms + 2 * YO3_CHAT_ROOM_SIZE, 1, 0);
    assert(yo3_chat_lobby_index(rooms, rooms + sizeof rooms) == 2);       /* the listed room 1, wherever it sorts */
    assert(yo3_chat_lobby_index(rooms, rooms + YO3_CHAT_ROOM_SIZE) == -1);  /* no Lobby: nothing */
    assert(yo3_chat_lobby_index(rooms, rooms + 5) == -1);                   /* not a room list */
    assert(yo3_chat_lobby_index(rooms + 5, rooms) == -1);

    Yo3ChatLobby s = {0};
    Yo3ChatRoomState free_room = {0, -1, -1}, joined = {1, 0, -1}, picking = {0, -1, 0};
    assert(!yo3_chat_lobby_reset(&s, 0x1000, 1, &free_room));              /* no login yet */
    yo3_chat_lobby_reply(&s, 0x2455); assert(s.step == YO3_CHAT_IDLE);     /* other replies: nothing */
    yo3_chat_lobby_reply(&s, YO3_CHAT_SELECTED);
    assert(!yo3_chat_lobby_reset(&s, 0x1000, 0, &free_room));              /* no chat agent yet: stays armed */
    assert(yo3_chat_lobby_reset(&s, 0x1000, 1, &free_room));               /* asks for the list */
    assert(yo3_chat_lobby_event(&s, 0x1000, &free_room, rooms, rooms) == -1);   /* list not in yet */
    assert(yo3_chat_lobby_event(&s, 0x2000, &free_room, rooms, rooms + sizeof rooms) == -1); /* another window */
    assert(yo3_chat_lobby_event(&s, 0x1000, &free_room, rooms, rooms + sizeof rooms) == 2);
    assert(yo3_chat_lobby_event(&s, 0x1000, &free_room, rooms, rooms + sizeof rooms) == -1); /* once */
    assert(!yo3_chat_lobby_reset(&s, 0x1000, 1, &free_room));              /* left on purpose later: left alone */

    yo3_chat_lobby_reply(&s, YO3_CHAT_SELECTED);
    assert(!yo3_chat_lobby_reset(&s, 0x1000, 1, &joined));                 /* already in a room */
    assert(s.step == YO3_CHAT_IDLE);
    yo3_chat_lobby_reply(&s, YO3_CHAT_SELECTED);
    assert(yo3_chat_lobby_reset(&s, 0x1000, 1, &free_room));
    assert(yo3_chat_lobby_event(&s, 0x1000, &picking, rooms, rooms + sizeof rooms) == -1); /* the player picked first */
    assert(s.step == YO3_CHAT_IDLE);
    puts("PASS: the chat enters room 1 once per login, only when the player is in no room and picking none");
    return 0;
}
