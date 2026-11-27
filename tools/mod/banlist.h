#ifndef YO3_BANLIST_H
#define YO3_BANLIST_H
/* A player's own banlist for their NPC duels (#77). The game knows one banlist, the login's (0x2201 regulation_id, kept
 * at +0xC8 of the object the NPC screen keeps at +0x64): its deck checks, Auto Duel and Create Duel's "Latest Regulation" use it, and the server
 * keeps that the host's. A player's own choice is for duels against the computer, and changes without logging in again:
 *  - the server pushes REVIVAL_BANLIST {revival_regulation} (0: the server's) at login and when it changes
 *    (crates/yo3-server/src/front.rs); this keeps it, and points Edit Deck's saved choice (deckInfo `cursor_regulation`,
 *    read when Edit Deck opens, 0x43EAEF) and Create Duel's (duelMaking `regulation`, 0x49E375) at it, so Edit Deck
 *    shows the player's banlist and a room they host starts on it;
 *  - the NPC screen's duel start (0x4BE390) takes the duel's banlist from the login one (`mov eax,[edx+0xC8]` at
 *    0x4BE4AB; a mastered NPC's is 3, unlimited, and stays so): the instruction becomes a call to the player's own when
 *    one is set. The duel request (0x17AC) then names it, and the server checks the deck against it;
 *  - Edit Deck's Change Regulations, confirmed, loads the chosen banlist (call 0x41A320 at 0x440900, the id pushed): the
 *    call is wrapped to keep the choice and send REVIVAL_BANLIST {regulation_id} on the front connection (+0x74's sender,
 *    as rental-return.h sends), and the server keeps it for the account.
 * Uses starter.h's message helpers (included from it). */
#define YO3_REVIVAL_BANLIST 0x7a15u
volatile unsigned yo3_personal_regulation __attribute__((used));
static void *banlist_connection;      /* the front connection the dispatcher last saw: where a choice is sent from */
static void *banlist_load_original __attribute__((used));

static void banlist_log(const char *text) { DWORD n; WriteFile(GetStdHandle(STD_ERROR_HANDLE),text,(DWORD)strlen(text),&n,NULL); }

/* A record of the selected player's local profile by its name (deckInfo, duelMaking), as game_deck_info finds one. */
static void *banlist_record(const char *wanted) {
    void *config=*(void **)game_address(0x82ec10);
    if(!config) return NULL;
    void **items=NULL; unsigned count=game_items(config,&items);
    for(unsigned i=0;i<count;++i) {
        char name[65];
        if(game_text(game_get(items[i],0x68),"name",name,sizeof name) && !strcmp(name,wanted)) return items[i];
    }
    return NULL;
}
static void banlist_set(const char *record,const char *field,unsigned id) {
    void *item=banlist_record(record);
    void *map=item?game_get(item,0x68):NULL;
    if(!map) return;
    GameString key; game_string(&key,field);
    ((void (__attribute__((thiscall)) *)(void *,GameString,unsigned))(*(DWORD **)map)[0x14/4])(map,key,id);
}
/* Create Duel's default banlist: duelMaking's `regulation`, under Detailed Rules' Customize (`detail` 1: Standard, 0,
 * always plays the login banlist; live, a Standard room's 0x17AC named 20 with Test Format chosen). Customize keeps the
 * standard LP and times unless the player changed them. None of the player's own leaves Detailed Rules as they were. */
static void banlist_room(unsigned id) {
    banlist_set("duelMaking","regulation",id);
    if(id) banlist_set("duelMaking","detail",1);
}
/* The player's banlist as the screens' default (0: the login one): Edit Deck opens with deckInfo's
 * `cursor_regulation` (0x43EAEF), Create Duel with duelMaking's `regulation` (0x49E375; one that won't load falls back
 * to the login one there). */
static void banlist_editor_choice(unsigned id) {
    banlist_set("deckInfo","cursor_regulation",id);
    banlist_room(id);
}

static void banlist_reply(void *message,void *connection) {
    banlist_connection=connection;
    void *map=game_get(message,0x68);
    if(game_uint(game_field(map,"request"))!=YO3_REVIVAL_BANLIST) return;
    void *value=game_field(map,"revival_regulation");
    if(!value) return;
    yo3_personal_regulation=game_uint(value);
    banlist_editor_choice(yo3_personal_regulation);
}

static void banlist_request(unsigned id,const char *field,unsigned value);
/* Change Regulations confirmed with banlist `id`: the player's own from now, and the server told. */
static __attribute__((used)) void __attribute__((cdecl)) banlist_chosen(unsigned id) {
    if(!id || id>0xffff) return;
    yo3_personal_regulation=id;
    banlist_room(id);
    banlist_request(YO3_REVIVAL_BANLIST,"regulation_id",id);
}
void banlist_confirm_stub(void);
__asm__(".text\n.globl _banlist_confirm_stub\n_banlist_confirm_stub:\n push %eax\n push %ecx\n push %edx\n"
        " pushl 16(%esp)\n call _banlist_chosen\n add $4,%esp\n pop %edx\n pop %ecx\n pop %eax\n jmp *_banlist_load_original\n");
/* The NPC duel's banlist: the player's own when one is set, else the login one (EDX the NPC screen's +0x64 object). */
void banlist_npc_stub(void);
__asm__(".text\n.globl _banlist_npc_stub\n_banlist_npc_stub:\n mov 0xc8(%edx),%eax\n cmpl $0,_yo3_personal_regulation\n"
        " je 1f\n mov _yo3_personal_regulation,%eax\n1:\n ret\n");

/* The duel's deck check found no valid deck (DuelMatchingAgent 0x4A0E90's failure, 0x4A1031: the game says only "There
 * are no valid Decks available."): the banlist it checked against, as it chose it (the opponent's item's regulation_id
 * or duel_regulation_id, else the login one at the front connection +0xC8), goes to the server as REVIVAL_DECKCHECK
 * {regulation_id}, which explains each deck in the chat. EDI the agent: +0x34 the front connection, +0x38 the item. */
#define YO3_REVIVAL_DECKCHECK 0x7a16u
static void *banlist_refusal_original __attribute__((used));
static void banlist_request(unsigned id,const char *field,unsigned value) {
    BYTE *front=(BYTE *)banlist_connection;
    void *sender=front?*(void **)(front+0x74):NULL;
    if(!sender) return;
    __attribute__((aligned(8))) BYTE request[0x80];
    memset(request,0,sizeof request);
    ((void *(__attribute__((stdcall)) *)(void *))game_address(0x40a190))(request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4);
    request[0x34]=4;
    ((void (__attribute__((thiscall)) *)(void *,unsigned))game_address(0x41bca0))(request,id);
    void *map=*(void **)(request+0x60);
    GameString key; game_string(&key,field);
    ((void (__attribute__((thiscall)) *)(void *,GameString,unsigned))(*(DWORD **)map)[0x14/4])(map,key,value);
    ((void (__attribute__((thiscall)) *)(void *,void *))(*(DWORD **)sender)[0x50/4])(sender,request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4);
    ((void (__attribute__((thiscall)) *)(void *))game_address(0x40a210))(request);
}
static __attribute__((used)) void __attribute__((cdecl)) banlist_no_deck(BYTE *agent) {
    if(!agent) return;
    unsigned regulation=0;
    void *item=*(void **)(agent+0x38);
    if(item) {
        void *map=game_get(item,0x68);
        regulation=game_uint(game_field(map,"regulation_id"));
        if(!regulation) regulation=game_uint(game_field(map,"duel_regulation_id"));
    }
    BYTE *front=*(BYTE **)(agent+0x34);
    if(!regulation && front) regulation=*(unsigned *)(front+0xc8);
    banlist_request(YO3_REVIVAL_DECKCHECK,"regulation_id",regulation);
}
void banlist_refusal_stub(void);
__asm__(".text\n.globl _banlist_refusal_stub\n_banlist_refusal_stub:\n push %eax\n push %ecx\n push %edx\n push %edi\n"
        " call _banlist_no_deck\n add $4,%esp\n pop %edx\n pop %ecx\n pop %eax\n jmp *_banlist_refusal_original\n");

static int banlist_write(unsigned char *site,const unsigned char *bytes,size_t n) {
    DWORD previous,ignored;
    if(!VirtualProtect(site,n,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    memcpy(site,bytes,n);
    FlushInstructionCache(GetCurrentProcess(),site,n); VirtualProtect(site,n,previous,&ignored);
    return 1;
}
static void banlist_call_bytes(unsigned char out[5],unsigned char *site,void *target) {
    int32_t rel=(int32_t)((intptr_t)target-(intptr_t)(site+5));
    out[0]=0xE8; memcpy(out+1,&rel,4);
}
/* Both patches, each only over the game's own bytes; a game that differs keeps its own behavior. */
static void banlist_install(void) {
    unsigned char *npc=(unsigned char *)game_address(0x4be4ab);
    static const unsigned char npc_code[]={0x8b,0x82,0xc8,0x00,0x00,0x00}; /* mov eax,[edx+0xC8] */
    unsigned char call[6];
    if(!memcmp(npc,npc_code,sizeof npc_code)) {
        banlist_call_bytes(call,npc,(void *)banlist_npc_stub); call[5]=0x90;
        if(!banlist_write(npc,call,6)) banlist_log("[YO3 MOD] banlist: NPC duel patch not written\r\n");
    } else banlist_log("[YO3 MOD] banlist: the NPC duel's banlist code is not the expected one, left alone\r\n");
    unsigned char *confirm=(unsigned char *)game_address(0x440900);
    int32_t rel; memcpy(&rel,confirm+1,4);
    if(confirm[0]==0xE8 && (intptr_t)(confirm+5)+rel==(intptr_t)game_address(0x41a320)) {
        banlist_load_original=game_address(0x41a320);
        banlist_call_bytes(call,confirm,(void *)banlist_confirm_stub);
        if(!banlist_write(confirm,call,5)) banlist_log("[YO3 MOD] banlist: Change Regulations hook not written\r\n");
    } else banlist_log("[YO3 MOD] banlist: Change Regulations' confirm is not the expected code, left alone\r\n");
    unsigned char *refusal=(unsigned char *)game_address(0x4a103d);
    memcpy(&rel,refusal+1,4);
    if(refusal[0]==0xE8 && (intptr_t)(refusal+5)+rel==(intptr_t)game_address(0x4a0c30)) {
        banlist_refusal_original=game_address(0x4a0c30);
        banlist_call_bytes(call,refusal,(void *)banlist_refusal_stub);
        if(!banlist_write(refusal,call,5)) banlist_log("[YO3 MOD] banlist: no-valid-deck hook not written\r\n");
    } else banlist_log("[YO3 MOD] banlist: the deck check's refusal is not the expected code, left alone\r\n");
}
#endif
