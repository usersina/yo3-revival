/* The UI probe's readers against synthetic memory: a component tree with MSVC RTTI and the client's child maps,
 * a duel's slots, and JSON escaping. */
#include <assert.h>
#include <stdio.h>
#include "ui-state.h"
static unsigned char memory[65536];
static int read_memory(void *ctx,uint32_t at,void *out,size_t n) {
    (void)ctx; if(!at || at>=sizeof memory || n>sizeof memory-at) return 0;
    memcpy(out,memory+at,n); return 1;
}
static void put(uint32_t at,uint32_t value) { memcpy(memory+at,&value,4); }
/* An object at `at` whose vtable (at `vtable`) has RTTI naming `name`. */
static void object(uint32_t at,uint32_t vtable,const char *name) {
    uint32_t locator=vtable+0x40,type=vtable+0x80;
    put(at,vtable);put(vtable-4,locator);put(locator+12,type);put(vtable+14*4,0x403c00); /* a container */
    char mangled[64]; snprintf(mangled,sizeof mangled,".?AV%s@@",name); memcpy(memory+type+8,mangled,strlen(mangled)+1);
}
/* The child map of `parent` (head node at `head`) holding `n` children by key, as a right-leaning chain. */
static void children(uint32_t parent,uint32_t head,const uint32_t *keys,const uint32_t *kids,unsigned n) {
    put(parent+0x44,head); memory[head+0x15]=1;
    put(head+4,n?head+0x20:head);
    for(unsigned i=0;i<n;++i) {
        uint32_t node=head+0x20*(i+1);
        put(node,head);put(node+8,i+1<n?node+0x20:head);put(node+0xc,keys[i]);put(node+0x10,kids[i]);memory[node+0x15]=0;
    }
}
int main(void) {
    const uint32_t root=0x100,menu=0x300,dialog=0x500,scene=0x700;
    object(root,0x2000,"Yo3App");object(menu,0x2200,"DuelMenu");object(dialog,0x2400,"MessageDialog");
    object(scene,0x2600,"DuelScene");
    uint32_t keys[]={(uint32_t)-9970,(uint32_t)-9999},kids[]={dialog,menu};
    children(root,0x4000,keys,kids,2);
    uint32_t inner[]={5},inner_kids[]={scene};
    children(menu,0x4400,inner,inner_kids,1);
    Yo3UiComponent c[16];
    int n=ui_components(read_memory,NULL,root,0x403c00,c,16);
    assert(n==4);
    assert(!strcmp(c[0].name,"Yo3App") && c[0].depth==0);
    /* Children in key order: -9999 (the menu, with its scene) before -9970 (the dialog). */
    assert(!strcmp(c[1].name,"DuelMenu") && c[1].depth==1 && (int32_t)c[1].key==-9999);
    assert(!strcmp(c[2].name,"DuelScene") && c[2].depth==2);
    assert(!strcmp(c[3].name,"MessageDialog") && c[3].depth==1);
    /* A component that isn't a container (another add-child method) isn't opened, whatever its +0x44 holds. */
    put(0x2200+14*4,0x1234);
    n=ui_components(read_memory,NULL,root,0x403c00,c,16);
    assert(n==3 && !strcmp(c[1].name,"DuelMenu") && !strcmp(c[2].name,"MessageDialog"));
    put(0x2200+14*4,0x403c00);
    /* A container whose map can't be read stays closed and is flagged; the rest of the tree still comes. */
    put(0x4400+4,0xfffff0);
    n=ui_components(read_memory,NULL,root,0x403c00,c,16);
    assert(n==3 && (c[1].depth&0x100) && !strcmp(c[2].name,"MessageDialog"));
    children(menu,0x4400,inner,inner_kids,1);
    /* An object without readable RTTI is named "?", not trusted. */
    put(dialog,0xfff0);
    n=ui_components(read_memory,NULL,root,0x403c00,c,16);
    assert(n==4 && !strcmp(c[3].name,"?"));
    /* A duel's slots: a face-down defense monster in zone 1 of player 0. */
    const uint32_t manager=0x6000,card=0x6400;
    put(scene+0xbc,manager);put(manager+0x4c+1*4,card);put(card+0x30,42);memory[card+0x82]=0x00;memory[card+0x83]=0x80;
    Yo3UiSlot slots[11];
    assert(ui_duel_slots(read_memory,NULL,scene,0,slots));
    assert(slots[1].uid==42 && slots[1].place==0x8000 && !slots[0].uid);
    /* JSON escaping. */
    char out[64]; ui_json_text(out,sizeof out,"a \"b\"\\c\n");
    assert(!strcmp(out,"a \\\"b\\\"\\\\c\\u000a"));
    puts("PASS: UI probe readers");
    return 0;
}
