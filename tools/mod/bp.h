#ifndef YO3_BP_H
#define YO3_BP_H
/* Revival's BP push. The original client changes its shown BP (UserData +0x76C, UserData from the singleton getter
 * 0x40A500) only on the login, a purchase (0x1C24), a rental (0x2D77) and the BOOSTERPASS window (0x40F7B0), so BP
 * the server pays for duels or the host gives would show only later. The server pushes REVIVAL_BP {revival_bp} to
 * the account's online players (crates/yo3-server/src/front.rs, Hub::push_bp); the client's own dispatcher ignores
 * the id (default case 0x4F6057) and this applies it. Uses starter.h's message helpers. */
#define YO3_REVIVAL_BP 0x7a11u
static void bp_reply(void *message) {
    void *map=game_get(message,0x68);
    if(game_uint(game_field(map,"request"))!=YO3_REVIVAL_BP) return;
    void *value=game_field(map,"revival_bp");
    if(!value) return;
    BYTE *user=((BYTE *(*)(void))game_address(0x40a500))();
    if(user) *(DWORD *)(user+0x76c)=game_uint(value);
}
#endif
