#ifndef YO3_DUEL_OBSERVER_H
#define YO3_DUEL_OBSERVER_H
#include "duel-snapshot.h"
static struct { uint32_t schema,sequence,inventory,stats,scene; DWORD started; int active; } duel_pending;
/* Issue #11: the server ends the batch of the player it waits on with engine command 0x5F1 {seq}
 * (crates/yo3-server/src/clock.rs, SETTLE). The scene pops a command only when the ones before it have played
 * (0x522A50); once cards, battles and chains have settled as well (the snapshot's own test), the client answers
 * duelRequestName 0x7A13 {seq} and the server counts the player's clocks from then. Unanswered within 30 s, the
 * server's allowance stands. */
#define YO3_SETTLE 0x5f1u
#define YO3_SETTLED 0x7a13u
static struct { uint32_t seq,scene; DWORD started; int active; } duel_settle;
static int duel_read(void *ctx,uint32_t at,void *out,size_t n) {
    (void)ctx; SIZE_T got=0;
    return ReadProcessMemory(GetCurrentProcess(),(const void *)(uintptr_t)at,out,n,&got) && got==n;
}
static void duel_observer_log(const char *status,const Yo3Snapshot *snapshot) {
    char line[14000];
    int n=snprintf(line,sizeof line,"[YO3 CHECK] {\"status\":\"%s\",\"schema\":%u,\"sequence\":%u,\"expected\":[%u,%u]",
        status,duel_pending.schema,duel_pending.sequence,duel_pending.inventory,duel_pending.stats);
    if(snapshot) {
        n+=snprintf(line+n,sizeof line-n,",\"actual\":[%u,%u],\"inventory\":[",
            yo3_snapshot_hash(snapshot->inventory,snapshot->used),yo3_snapshot_hash(snapshot->stats,snapshot->stat_used));
        for(unsigned i=0;i<snapshot->used && n<(int)sizeof line-100;++i)
            n+=snprintf(line+n,sizeof line-n,"%s%u",i?",":"",snapshot->inventory[i]);
        n+=snprintf(line+n,sizeof line-n,"],\"stats\":[");
        for(unsigned i=0;i<snapshot->stat_used && n<(int)sizeof line-100;++i)
            n+=snprintf(line+n,sizeof line-n,"%s%u",i?",":"",snapshot->stats[i]);
        n+=snprintf(line+n,sizeof line-n,"]");
    }
    n+=snprintf(line+n,sizeof line-n,"}\r\n");
    DWORD written; WriteFile(GetStdHandle(STD_ERROR_HANDLE),line,(DWORD)n,&written,NULL);
}
/* The answer, built as the client's own duel requests are (the phase button 0x521180: a YO3Request 0x200C with its
 * duelRequestName, sent through the duel connection 0x82D2C4's vtable +0x50), but not in a replay (the engine
 * 0x82D2C0's +0 is 1). Unlike the player's requests it doesn't wait for the input flag (+0x284, set when the input
 * wait 4 or a card list 0x37 pops, 0x523062): the marker comes before those. */
static void duel_settle_send(uint32_t seq) {
    BYTE *engine=*(BYTE **)game_address(0x82d2c0);
    void *sender=*(void **)game_address(0x82d2c4);
    if(!engine || !sender || engine[0]==1) return;
    __attribute__((aligned(8))) BYTE request[0x80]; /* YO3Request is 0x6C bytes (0x52FCC0 allocates that much) */
    memset(request,0,sizeof request);
    ((void *(__attribute__((stdcall)) *)(void *))game_address(0x40a190))(request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4); /* YO3Request's vtable */
    request[0x34]=4;
    ((void (__attribute__((thiscall)) *)(void *,unsigned))game_address(0x41bca0))(request,0x200c);
    void *map=*(void **)(request+0x60);
    typedef void (__attribute__((thiscall)) *SetUint)(void *,GameString,unsigned);
    GameString key; game_string(&key,"duelRequestName");
    ((SetUint)(*(DWORD **)map)[0x14/4])(map,key,YO3_SETTLED);
    game_string(&key,"seq");
    ((SetUint)(*(DWORD **)map)[0x14/4])(map,key,seq);
    ((void (__attribute__((thiscall)) *)(void *,void *))(*(DWORD **)sender)[0x50/4])(sender,request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4);
    ((void (__attribute__((thiscall)) *)(void *))game_address(0x40a210))(request);
}
static void duel_settle_poll(void *scene) {
    if(!duel_settle.active) return;
    if(duel_settle.scene!=(uint32_t)(uintptr_t)scene || GetTickCount()-duel_settle.started>=30000) { duel_settle.active=0; return; }
    uint32_t data,local;
    if(!yo3_word(duel_read,NULL,(uint32_t)(uintptr_t)game_address(0x82d2c0),&data) || !yo3_word(duel_read,NULL,data+0x1b0,&local)) return;
    static Yo3Snapshot snapshot;
    int state=yo3_duel_snapshot(duel_read,NULL,duel_settle.scene,local,&snapshot);
    if(state==0) return;
    duel_settle.active=0;
    if(state==1) duel_settle_send(duel_settle.seq);
}
static void duel_observer_poll(void *scene) {
    if(!duel_pending.active) return;
    if(duel_pending.scene!=(uint32_t)(uintptr_t)scene) { duel_observer_log("scene-changed",NULL);duel_pending.active=0;return; }
    uint32_t data,local;
    if(!yo3_word(duel_read,NULL,(uint32_t)(uintptr_t)game_address(0x82d2c0),&data) || !yo3_word(duel_read,NULL,data+0x1b0,&local)) {
        duel_observer_log("unreadable",NULL);duel_pending.active=0;return;
    }
    Yo3Snapshot snapshot; int state=yo3_duel_snapshot(duel_read,NULL,duel_pending.scene,local,&snapshot);
    if(state==0 && GetTickCount()-duel_pending.started<15000) return;
    if(state!=1) duel_observer_log(state==0?"timeout":"unreadable",NULL);
    else duel_observer_log(yo3_snapshot_hash(snapshot.inventory,snapshot.used)==duel_pending.inventory
        && yo3_snapshot_hash(snapshot.stats,snapshot.stat_used)==duel_pending.stats?"pass":"mismatch",&snapshot);
    duel_pending.active=0;
}
static void __attribute__((fastcall,force_align_arg_pointer)) duel_observer_dispatch(unsigned id,uint32_t *params,void *scene) {
    if(id==YO3_CHECKPOINT) {
        if(duel_pending.active) duel_observer_log("superseded",NULL);
        duel_pending.schema=params[0];duel_pending.sequence=params[1];duel_pending.inventory=params[2];duel_pending.stats=params[3];
        duel_pending.scene=(uint32_t)(uintptr_t)scene;duel_pending.started=GetTickCount();duel_pending.active=1;
        if(params[0]!=YO3_SNAPSHOT_SCHEMA) { duel_observer_log("unsupported-schema",NULL);duel_pending.active=0; }
        return;
    }
    if(id==YO3_SETTLE) {
        duel_settle.seq=params[0];duel_settle.scene=(uint32_t)(uintptr_t)scene;duel_settle.started=GetTickCount();duel_settle.active=1;
        return;
    }
    ((void (__attribute__((fastcall)) *)(unsigned,uint32_t *,void *))game_address(0x514290))(id,params,scene);
}
static void __attribute__((fastcall,force_align_arg_pointer)) duel_observer_update(void *scene,void *unused) {
    (void)unused;
    ((void (__attribute__((thiscall)) *)(void *))game_address(0x515a30))(scene);
    duel_observer_poll(scene);
    duel_settle_poll(scene);
}
static int duel_observer_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if(nt->OptionalHeader.SizeOfImage<0x0f44c2) return 1; /* Standalone import fixtures. */
    if(nt->OptionalHeader.SizeOfImage<0x1159ef) return 0;
    BYTE *call=base+0x114212,*update=base+0x1159ea;
    if(!yo3_snapshot_call_ok(call,5) || !yo3_snapshot_update_ok(update,5,(uint32_t)(uintptr_t)game_address(0x515a30))) return 0;
    DWORD old;
    if(!VirtualProtect(call,5,PAGE_EXECUTE_READWRITE,&old)) return 0;
    LONG offset=(BYTE *)duel_observer_dispatch-(call+5);memcpy(call+1,&offset,4);
    FlushInstructionCache(GetCurrentProcess(),call,5);
    DWORD ignored;if(!VirtualProtect(call,5,old,&ignored)) return 0;
    if(!VirtualProtect(update,5,PAGE_EXECUTE_READWRITE,&old)) return 0;
    DWORD target=(DWORD)(uintptr_t)duel_observer_update;memcpy(update+1,&target,4);
    FlushInstructionCache(GetCurrentProcess(),update,5);
    return VirtualProtect(update,5,old,&ignored)!=0;
}
#endif
