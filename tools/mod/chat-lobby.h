#ifndef YO3_CHAT_LOBBY_H
#define YO3_CHAT_LOBBY_H
/* Installs chat-lobby-logic.h: two of ChatWindow's vtable slots (its reset and its event handler) go through this
 * file, and the global dispatch hook (starter.h) passes each reply's id. Another build of the game keeps its own chat:
 * nothing is patched, never a reason to refuse it. */
#include "chat-lobby-logic.h"

static Yo3ChatLobby chat_lobby;
static void (__attribute__((thiscall)) *chat_reset_original)(void *);
static void (__attribute__((thiscall)) *chat_event_original)(void *, void *);

static Yo3ChatRoomState chat_room_state(const BYTE *window) {
    Yo3ChatRoomState s = {window[0x55], *(const int32_t *)(window + 0x50), *(const int32_t *)(window + 0x58)};
    return s;
}

/* 0x4F3C10 / 0x4F3CF0: the chat agent in EDI (which they keep), as the room list's code calls them. */
static void chat_request_list(void *agent) {
    __asm__ volatile("call *%[fn]" : : "D"(agent), [fn]"r"(game_address(YO3_CHAT_LIST)) : "eax","ecx","edx","memory","cc");
}
static void chat_enter(void *agent, int room) {
    __asm__ volatile("pushl %[room]; call *%[fn]" /* ret 4 takes the id */
        : : "D"(agent), [room]"r"(room), [fn]"r"(game_address(YO3_CHAT_ENTER)) : "eax","ecx","edx","memory","cc");
}

static void chat_lobby_reply(void *message) {
    yo3_chat_lobby_reply(&chat_lobby, game_uint(game_field(game_get(message, 0x68), "request")));
}

static void __attribute__((thiscall, force_align_arg_pointer)) chat_reset(void *window) {
    chat_reset_original(window);
    BYTE *w = window;
    void *agent = *(void **)(w + 0x12c);
    Yo3ChatRoomState room = chat_room_state(w);
    if (yo3_chat_lobby_reset(&chat_lobby, (uintptr_t)window, agent != NULL, &room)) chat_request_list(agent);
}

static void __attribute__((thiscall, force_align_arg_pointer)) chat_event(void *window, void *event) {
    chat_event_original(window, event);
    BYTE *w = window;
    Yo3ChatRoomState room = chat_room_state(w);
    const unsigned char *begin = *(const unsigned char **)(w + 0x13c), *end = *(const unsigned char **)(w + 0x140);
    int index = yo3_chat_lobby_event(&chat_lobby, (uintptr_t)window, &room, begin, end);
    void *agent = *(void **)(w + 0x12c);
    if (index < 0 || !agent) return;
    *(int32_t *)(w + 0x58) = index; /* the pending choice, as the click sets it */
    chat_enter(agent, YO3_CHAT_LOBBY);
    starter_log("[YO3 MOD] Chat: entering the Lobby.\r\n");
}

static void chat_lobby_install(void) {
    BYTE *base = (BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if (nt->OptionalHeader.SizeOfImage < YO3_CHAT_VTABLE - 0x400000u + 0x100u) return;
    DWORD *vtable = game_address(YO3_CHAT_VTABLE);
    DWORD *reset = vtable + YO3_CHAT_RESET_SLOT, *event = vtable + YO3_CHAT_EVENT_SLOT;
    if (*reset != (DWORD)(uintptr_t)game_address(YO3_CHAT_RESET) || *event != (DWORD)(uintptr_t)game_address(YO3_CHAT_EVENT)) return;
    DWORD previous;
    if (!VirtualProtect(vtable, (YO3_CHAT_RESET_SLOT + 1) * 4, PAGE_READWRITE, &previous)) return;
    chat_reset_original = (void *)(uintptr_t)*reset;
    chat_event_original = (void *)(uintptr_t)*event;
    *reset = (DWORD)(uintptr_t)chat_reset;
    *event = (DWORD)(uintptr_t)chat_event;
    DWORD ignored; VirtualProtect(vtable, (YO3_CHAT_RESET_SLOT + 1) * 4, previous, &ignored);
}
#endif
