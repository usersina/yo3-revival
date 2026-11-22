#ifndef YO3_CHAT_LOBBY_LOGIC_H
#define YO3_CHAT_LOBBY_LOGIC_H
/* The chat joins the Lobby by itself once per login (chat-lobby.h). Without it the chat window's first tab reads
 * "----" until the player opens the room list and picks a room, and a line typed before that goes nowhere.
 *
 * The server can't push the entry: ChatWindow's reply handler (0x435950) acts on 0x245A {room_id} only while its
 * pending choice (+0x58, an index into its room list) is >= 0. The room list's click callback (0x437300) sets it and
 * sends the enter request through 0x4F3CF0 (0x2136 {room_id}; EDI = the chat agent at +0x12C, the id on the stack,
 * `ret 4`), but only when the window is in no room (+0x55 == 0 or +0x50 < 0) and the index is in the room list
 * (+0x13C..+0x140, 0x34-byte entries: id +0, name +4, type +0x30), filled by the 0x2458 reply of the list request
 * 0x4F3C10 (0x2135, EDI = the agent, `ret`). So the mod does the same as a click, in steps:
 *   - the player-select reply 0x2266 (one per login) arms it;
 *   - ChatWindow's reset (vtable slot 14, 0x435710: clears the tabs and prints the welcome lines) asks for the list;
 *   - after ChatWindow's event handler (slot 6, 0x4358F0) has taken the 0x2458 reply, it picks room 1 (the Lobby)
 *     and enters it as the click would. The game's own 0x245A case then prints "In>>Lobby" and names the tab.
 * It is done at most once per login: a player who leaves a room or picks another one is left alone. Tested without
 * the game by chat-lobby-test.c. */
#include <stdint.h>
#include <string.h>

#define YO3_CHAT_VTABLE 0x7a48dcu
#define YO3_CHAT_EVENT_SLOT 6u
#define YO3_CHAT_EVENT 0x4358f0u
#define YO3_CHAT_RESET_SLOT 14u
#define YO3_CHAT_RESET 0x435710u
#define YO3_CHAT_LIST 0x4f3c10u
#define YO3_CHAT_ENTER 0x4f3cf0u
#define YO3_CHAT_SELECTED 0x2266u
#define YO3_CHAT_LOBBY 1
#define YO3_CHAT_ROOM_SIZE 0x34u

enum { YO3_CHAT_IDLE, YO3_CHAT_ARMED, YO3_CHAT_LISTING };
typedef struct { int step; uintptr_t window; } Yo3ChatLobby;

/* The window's room state: in a room (or entering one), the game's own fields. */
typedef struct { unsigned char joined; int32_t current, pending; } Yo3ChatRoomState;

static int yo3_chat_lobby_free(const Yo3ChatRoomState *s) {
    return (!s->joined || s->current < 0) && s->pending < 0;
}

/* The room list's index of the Lobby (id 1, listed: type 0), or -1. */
static int yo3_chat_lobby_index(const unsigned char *begin, const unsigned char *end) {
    if (!begin || end < begin || (size_t)(end - begin) % YO3_CHAT_ROOM_SIZE || (size_t)(end - begin) / YO3_CHAT_ROOM_SIZE > 1000)
        return -1;
    for (size_t i = 0; begin + (i + 1) * YO3_CHAT_ROOM_SIZE <= end; ++i) {
        int32_t id, type;
        memcpy(&id, begin + i * YO3_CHAT_ROOM_SIZE, 4);
        memcpy(&type, begin + i * YO3_CHAT_ROOM_SIZE + 0x30, 4);
        if (id == YO3_CHAT_LOBBY && type == 0) return (int)i;
    }
    return -1;
}

/* A reply passed the game's dispatcher: a player was selected, so the next chat reset joins the Lobby. */
static void yo3_chat_lobby_reply(Yo3ChatLobby *s, unsigned id) {
    if (id == YO3_CHAT_SELECTED) { s->step = YO3_CHAT_ARMED; s->window = 0; }
}

/* ChatWindow reset: whether to ask for the room list now (then the list's reply is awaited for this window). */
static int yo3_chat_lobby_reset(Yo3ChatLobby *s, uintptr_t window, int has_agent, const Yo3ChatRoomState *room) {
    if (s->step != YO3_CHAT_ARMED || !has_agent) return 0;
    if (!yo3_chat_lobby_free(room)) { s->step = YO3_CHAT_IDLE; return 0; }
    s->step = YO3_CHAT_LISTING; s->window = window;
    return 1;
}

/* After ChatWindow took an event: the index to enter, or -1. Gives up once the list is in, whatever it holds. */
static int yo3_chat_lobby_event(Yo3ChatLobby *s, uintptr_t window, const Yo3ChatRoomState *room,
                                const unsigned char *begin, const unsigned char *end) {
    if (s->step != YO3_CHAT_LISTING || s->window != window || begin == end) return -1;
    s->step = YO3_CHAT_IDLE;
    return yo3_chat_lobby_free(room) ? yo3_chat_lobby_index(begin, end) : -1;
}
#endif
