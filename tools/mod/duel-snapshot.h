#ifndef YO3_DUEL_SNAPSHOT_H
#define YO3_DUEL_SNAPSHOT_H
/* Original read-only observer for ver.1.153. Native offsets and ABI evidence: docs/client-internals.md.
 * Address reads are injected, bounded and checked; this file also runs against synthetic memory without Wine. */
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#define YO3_CHECKPOINT 0x5f0u
#define YO3_SNAPSHOT_SCHEMA 1u
#define YO3_SNAPSHOT_WORDS 1024u
typedef int (*Yo3Read)(void *,uint32_t,void *,size_t);
typedef struct { uint32_t inventory[YO3_SNAPSHOT_WORDS],stats[YO3_SNAPSHOT_WORDS]; unsigned used,stat_used; } Yo3Snapshot;
static uint32_t yo3_snapshot_hash(const uint32_t *words,unsigned n) {
    uint32_t hash=0x811c9dc5u;
    for(unsigned i=0;i<n;++i) for(unsigned b=0;b<4;++b) hash=(hash^((words[i]>>(b*8))&255u))*0x1000193u;
    return hash;
}
static int yo3_snapshot_call_ok(const unsigned char *call,size_t n) {
    static const unsigned char expected[]={0xe8,0x79,0,0,0}; /* 0x514212 -> 0x514290 */
    return n==sizeof expected && !memcmp(call,expected,n);
}
static int yo3_snapshot_update_ok(const unsigned char *code,size_t n,uint32_t target) {
    if(n!=5 || code[0]!=0xb8 || !target) return 0; /* 0x5159EA: MOV EAX, relocated update callback. */
    uint32_t actual; memcpy(&actual,code+1,4); return actual==target;
}
static int yo3_word(Yo3Read read,void *ctx,uint32_t at,uint32_t *value) { return at && read(ctx,at,value,4); }
static int yo3_snap_push(Yo3Snapshot *s,uint32_t value) {
    if(s->used==YO3_SNAPSHOT_WORDS) return 0;
    s->inventory[s->used++]=value; return 1;
}
static int yo3_snapshot_counters(Yo3Read read,void *ctx,uint32_t card,Yo3Snapshot *s) {
    uint32_t head,node,stack[64],pairs[128];unsigned top=0,count=0,visited=0;
    if(!yo3_word(read,ctx,card+0x130,&head) || !head || !yo3_word(read,ctx,head+4,&node)) return 0;
    while(node!=head || top) {
        while(node!=head) {
            if(top==64 || ++visited>128) return 0;
            stack[top++]=node;if(!yo3_word(read,ctx,node,&node)) return 0;
        }
        node=stack[--top];uint32_t kind,value;
        if(!yo3_word(read,ctx,node+0xc,&kind) || !yo3_word(read,ctx,node+0x10,&value)) return 0;
        if(value) { if(count==64) return 0;pairs[count*2]=kind;pairs[count*2+1]=value;++count; }
        if(!yo3_word(read,ctx,node+8,&node)) return 0;
    }
    if(s->stat_used+1+count*2>YO3_SNAPSHOT_WORDS) return 0;
    s->stats[s->stat_used++]=count;
    memcpy(s->stats+s->stat_used,pairs,count*8);s->stat_used+=count*2;return 1;
}
/* -1: unreadable/unsupported state; 0: animation in progress; 1: settled snapshot. Never calls a game API. */
static int yo3_duel_snapshot(Yo3Read read,void *ctx,uint32_t scene,unsigned local,Yo3Snapshot *s) {
    uint32_t manager,cockpit,battle,chain,value;
    memset(s,0,sizeof *s);
    if(local>1 || !yo3_word(read,ctx,scene+0xbc,&manager) || !manager
        || !yo3_word(read,ctx,scene+0x8c,&cockpit) || !cockpit
        || !yo3_word(read,ctx,scene+0xa8,&battle) || !battle
        || !yo3_word(read,ctx,scene+0xc0,&chain) || !chain) return -1;
    if(!yo3_word(read,ctx,battle+0x74,&value)) return -1;
    if(value) return 0;
    if(!yo3_word(read,ctx,chain+0x58,&value)) return -1;
    if(value) return 0;
    /* Include cards travelling to a pile, which are no longer in a field/hand slot. MSVC map sentinel/root. */
    uint32_t head,root,stack[64]; unsigned top=0,visited=0;
    if(!yo3_word(read,ctx,manager+0x44,&head) || !head || !yo3_word(read,ctx,head+4,&root)) return -1;
    if(root!=head) stack[top++]=root;
    while(top) {
        uint32_t node=stack[--top],left,right,card;
        if(++visited>512 || !yo3_word(read,ctx,node,&left) || !yo3_word(read,ctx,node+8,&right)
            || !yo3_word(read,ctx,node+0x10,&card) || !card || !yo3_word(read,ctx,card+0x114,&value)) return -1;
        if(value) return 0;
        if(left!=head) { if(top==64) return -1; stack[top++]=left; }
        if(right!=head) { if(top==64) return -1; stack[top++]=right; }
    }
    s->inventory[s->used++]=YO3_SNAPSHOT_SCHEMA; s->stats[s->stat_used++]=YO3_SNAPSHOT_SCHEMA;
    for(unsigned p=0;p<2;++p) {
        unsigned physical=p^local;
        if(!yo3_word(read,ctx,cockpit+0x50+physical*4,&value) || !yo3_snap_push(s,value)) return -1;
        for(unsigned z=0;z<11;++z) {
            uint32_t card,uid,mode; uint16_t place;
            if(!yo3_word(read,ctx,manager+0x4c+(physical*17+z)*4,&card)) return -1;
            if(!card) { if(!yo3_snap_push(s,0)) return -1; continue; }
            if(!yo3_word(read,ctx,card+0x114,&value)) return -1;
            if(value) return 0;
            if(!yo3_word(read,ctx,card+0x30,&uid) || !yo3_word(read,ctx,card+0x44,&mode)
                || !read(ctx,card+0x82,&place,2)) return -1;
            if(!yo3_snap_push(s,uid) || !yo3_snap_push(s,(place^local)&0xc03f) || !yo3_snap_push(s,mode!=0)) return -1;
            if(z<5 && (place&0x4000)) {
                if(s->stat_used+5>YO3_SNAPSHOT_WORDS) return -1;
                if(!read(ctx,card+0xb8,s->stats+s->stat_used,20)) return -1;
                s->stat_used+=5;
            }
            if(!yo3_snapshot_counters(read,ctx,card,s)) return -1;
        }
        uint32_t begin,end;
        if(!yo3_word(read,ctx,manager+0xe0+physical*24,&begin) || !yo3_word(read,ctx,manager+0xe4+physical*24,&end)
            || end<begin || (end-begin)%4 || (end-begin)/4>200) return -1;
        unsigned count=(end-begin)/4;
        if(!yo3_snap_push(s,count)) return -1;
        for(unsigned i=0;i<count;++i) {
            uint32_t card,uid;
            if(!yo3_word(read,ctx,begin+i*4,&card) || !yo3_word(read,ctx,card+0x30,&uid) || !yo3_snap_push(s,uid)) return -1;
        }
        for(unsigned z=12;z<16;++z) {
            uint32_t pile;
            if(!yo3_word(read,ctx,manager+0x4c+(physical*17+z)*4,&pile) || !pile
                || !yo3_word(read,ctx,pile+(z==15?0xc:0x14),&value) || !yo3_snap_push(s,value)) return -1;
        }
    }
    return 1;
}
#endif
