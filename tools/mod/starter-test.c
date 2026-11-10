/* Portable regression proof for the DLL's login policy; no game files or Wine required. */
#include <assert.h>
#include <stdio.h>
#include "starter-policy.h"
typedef struct { int valid[12], exists[12], saves, selects, remembers, fail; } Fixture;
static int valid(void *p, unsigned i) { return ((Fixture *)p)->valid[i]; }
static int exists(void *p, unsigned i) { return ((Fixture *)p)->exists[i]; }
static int save(void *p, unsigned i) { Fixture *f=p; if(f->fail)return 0; assert(!f->exists[i]); f->exists[i]=1; ++f->saves; return 1; }
static int select_deck(void *p, unsigned i) { (void)i; ++((Fixture *)p)->selects; return 1; }
static int remember(void *p) { ++((Fixture *)p)->remembers; return 1; }
int main(void) {
    unsigned char call[]={0xe8,0x3e,0x05,0,0};
    assert(yo3_starter_dispatch_call_ok(call,sizeof call));
    assert(!yo3_starter_dispatch_call_ok(call,4));
    call[1]^=1; assert(!yo3_starter_dispatch_call_ok(call,sizeof call));
    Yo3StarterState state={0}; Fixture f={.valid={1}};
    Yo3StarterOps ops={&f,valid,exists,save,select_deck,remember};
    Yo3StarterContext c={.connection=1,.player=1001,.count=1,.selection_empty=1};
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_WAIT);
    assert(!f.saves && !f.selects);
    c.ready=1; c.existing_decks=1; f.valid[0]=0; /* manual unfinished decks also stop initialization */
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_PRESERVED);
    assert(!f.saves && !f.selects);
    /* A prior setup still wins after intentional deletion, even across process/state resets. */
    yo3_starter_reset(&state); c.existing_decks=0; c.initialized=1;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_PRESERVED);
    assert(!f.saves && !f.selects && f.remembers==1);
    yo3_starter_reset(&state); c.initialized=0; f.valid[0]=1;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_CREATED);
    assert(f.saves==1 && f.selects==1);
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_PRESERVED);
    assert(f.saves==1 && f.selects==1);
    c.player=1002; f.exists[0]=0; c.selection_empty=0;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_PRESERVED);
    assert(f.saves==1 && f.selects==1); /* a saved selection also proves prior manual setup */
    yo3_starter_reset(&state); c.player=1003; c.selection_empty=1;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_CREATED);
    assert(f.saves==2 && f.selects==2);
    yo3_starter_reset(&state);
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_UNAVAILABLE);
    assert(f.saves==2); /* an existing file must never be replaced, even if invalid */
    yo3_starter_reset(&state); f.exists[0]=-1;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_UNAVAILABLE); /* failed path check */
    yo3_starter_reset(&state); f.exists[0]=0; f.valid[0]=0;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_UNAVAILABLE); /* native size/ownership/banlist refusal */
    yo3_starter_reset(&state); f.valid[0]=1; f.fail=1;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_FAILED);
    assert(f.saves==2 && f.selects==2);
    f.fail=0; c.selection_empty=1;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_CREATED); /* failed writes may retry */
    assert(f.saves==3 && f.selects==3);
    yo3_starter_reset(&state); c.connection=2; c.player=0;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_WAIT);
    for (unsigned i=0;i<12;++i) { f.valid[i]=i==10; f.exists[i]=0; }
    c.player=1003; c.count=12;
    assert(yo3_starter_apply(&state,&c,&ops)==YO3_STARTER_UNAVAILABLE); /* client's ten-deck bound */
    assert(f.saves==3);
    const char *bad[]={"",".","..","../outside","A/B","A\\B","A:stream","CON","nul.y3d","COM1","lpt9.txt","bad.","bad ","bad\nname"};
    for(unsigned i=0;i<sizeof bad/sizeof *bad;++i) assert(!yo3_starter_safe_name(bad[i]));
    assert(yo3_starter_safe_name("Starter Deck") && yo3_starter_safe_name("Existing custom deck"));
    assert(yo3_starter_safe_name("日本語"));
    puts("PASS: readiness, manual decks/deletion, selection, retries, player switches, paths and one-time initialization");
    return 0;
}
