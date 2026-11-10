#include <assert.h>
#include <stdio.h>
#include "duel-snapshot.h"
static unsigned char memory[65536],before[65536];
static int read_memory(void *ctx,uint32_t at,void *out,size_t n) {
    (void)ctx; if(at>=sizeof memory || n>sizeof memory-at) return 0;
    memcpy(out,memory+at,n); return 1;
}
static void put(uint32_t at,uint32_t value) { memcpy(memory+at,&value,4); }
int main(void) {
    const uint32_t scene=0x100,manager=0x400,cockpit=0x800,battle=0x900,chain=0xa00,head=0xb00,card=0xc00;
    put(scene+0xbc,manager);put(scene+0x8c,cockpit);put(scene+0xa8,battle);put(scene+0xc0,chain);
    put(manager+0x44,head);put(head+4,head);put(cockpit+0x50,8000);put(cockpit+0x54,8000);
    for(unsigned p=0;p<2;++p) for(unsigned z=12;z<16;++z) {
        unsigned pile=0x1000+(p*4+z-12)*0x100;put(manager+0x4c+(p*17+z)*4,pile);put(pile+(z==15?0xc:0x14),z==13?40:0);
        if(z==15) put(pile+0x14,0xdeadbeef); /* Banished pile is a different native class. */
    }
    put(manager+0x4c,card);put(card+0x30,7);put(card+0x44,1);put(card+0x82,0x4000);
    put(card+0x130,0xd80);put(0xd84,0xd80);
    for(unsigned i=0;i<5;++i) put(card+0xb8+i*4,i==0?4:1800);
    Yo3Snapshot s; memcpy(before,memory,sizeof memory);
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==1);
    assert(!memcmp(before,memory,sizeof memory));
    uint32_t baseline=yo3_snapshot_hash(s.inventory,s.used),stats=yo3_snapshot_hash(s.stats,s.stat_used);
    assert(s.used==37 && baseline==0x5746458eu && stats==0xc79d18c0u); /* Shared Rust/C schema vector. */
    put(card+0x44,0); /* Original bug: occupied slot, invisible card. */
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==1);
    assert(yo3_snapshot_hash(s.inventory,s.used)!=baseline);
    put(card+0x44,1);put(card+0x114,1);
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==0);
    put(card+0x114,0);put(card+0xbc,2200);
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==1);
    assert(yo3_snapshot_hash(s.stats,s.stat_used)!=stats);
    put(card+0xbc,1800);put(0xd84,0x3000);put(0x3000,0xd80);put(0x3008,0xd80);put(0x3010,2);
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==1);
    assert(yo3_snapshot_hash(s.stats,s.stat_used)!=stats); /* Actual per-card counter map. */
    put(cockpit+0x50,7600);
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==1);
    assert(yo3_snapshot_hash(s.inventory,s.used)!=baseline);
    put(manager+0xe0,0x2000);put(manager+0xe4,0x3000); /* Bound corrupt vectors. */
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==-1);
    put(scene+0xbc,0xfffffff0);
    assert(yo3_duel_snapshot(read_memory,NULL,scene,0,&s)==-1);
    unsigned char call[]={0xe8,0x79,0,0,0},update[]={0xb8,0x30,0x5a,0x51,0};
    assert(yo3_snapshot_call_ok(call,5) && yo3_snapshot_update_ok(update,5));
    call[1]^=1;update[2]^=1;
    assert(!yo3_snapshot_call_ok(call,5) && !yo3_snapshot_update_ok(update,5));
    puts("PASS: read-only native snapshots, hidden survivors, LP/stats, animation gating, bounds and hook guards");
    return 0;
}
