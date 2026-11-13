#ifndef YO3_UI_STATE_H
#define YO3_UI_STATE_H
/* The UI probe's read-only views of the client (ver.1.153), for debugging without screenshots (`vclient.sh ui`).
 * Everything goes through an injected checked reader, so this also runs against synthetic memory (ui-state-test.c).
 * Evidence: docs/client-internals.md, "UI probe".
 *  - Components: every screen, window and dialog is a component made by the factory 0x4EEB90 and added to a parent
 *    with parent->vtable[0x38] (0x403C00): the child's parent goes to child+4 and the child into the parent's
 *    std::map (head node pointer at +0x44; nodes left +0, parent +4, right +8, key +0xC, child +0x10, nil +0x15;
 *    iterated by 0x404570). The root is Yo3App's component part, 0x82F7C0 (Yo3App is the static 0x82F760).
 *  - A component's class is its MSVC RTTI name: vtable-4 → complete object locator, +0xC → type descriptor, +8 → name.
 *  - In a duel the card manager (DuelScene +0xBC) holds each slot's card at +0x4C + (player*17 + zone)*4, as the
 *    duel snapshot reads it (duel-snapshot.h): uid +0x30, place word +0x82 (0x4000 face up, 0x8000 defense).
 *  - Flash calls: the screens are Flash movies, and the game's code talks to them through one invoke, 0x5CB1B0
 *    (thiscall movie, name, args, count): "call" with the ActionScript method first (0x539760's 234 callers, e.g.
 *    "setButtonLabel" from 0x4C35C0), "respond" answering the movie. Each argument is a 16-byte GFx value (copied
 *    by 0x40F0F0): +4 type (0x40 managed), +8 value; 2 bool, 3 number (double), 4 char *, 5 wchar_t *, 0x44 char **. */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
typedef int (*Yo3UiRead)(void *,uint32_t,void *,size_t);
static int ui_word(Yo3UiRead read,void *ctx,uint32_t at,uint32_t *v) { return at && read(ctx,at,v,4); }

/* "DuelScene" from the object's RTTI, or 0 when it has none readable. */
static int ui_class(Yo3UiRead read,void *ctx,uint32_t object,char *out,size_t cap) {
    uint32_t vtable,locator,type; char name[96]={0};
    out[0]=0;
    if(!ui_word(read,ctx,object,&vtable) || !ui_word(read,ctx,vtable-4,&locator) || !ui_word(read,ctx,locator+12,&type)
        || !read(ctx,type+8,name,sizeof name-1)) return 0;
    if(strncmp(name,".?AV",4)) return 0;
    char *end=strstr(name,"@@"); if(!end) return 0; *end=0;
    size_t n=strlen(name+4); if(!n || n+1>cap) return 0;
    for(size_t i=0;i<n;++i) { char c=name[4+i]; if(c<32 || c>126 || c=='"' || c=='\\') return 0; }
    memcpy(out,name+4,n+1); return 1;
}

typedef struct { uint32_t object,key; unsigned depth; char name[64]; } Yo3UiComponent;

/* The component tree under `root`, depth first, children in map order, at most `cap` entries. Only containers are
 * opened: components whose vtable slot 14 (add child, 0x38) is `add_child` (BroadcastComponent's 0x403C00); others
 * keep other data at +0x44. A subtree that can't be read is left closed rather than failing the whole walk. */
static int ui_components(Yo3UiRead read,void *ctx,uint32_t root,uint32_t add_child,Yo3UiComponent *out,unsigned cap) {
    struct { uint32_t object,key; unsigned depth; } stack[256]; unsigned top=0,count=0,visited=0;
    stack[top].object=root; stack[top].key=0; stack[top++].depth=0;
    while(top) {
        --top; uint32_t object=stack[top].object,key=stack[top].key; unsigned depth=stack[top].depth;
        if(count==cap) return (int)count;
        out[count].object=object; out[count].depth=depth; out[count].key=key;
        if(!ui_class(read,ctx,object,out[count].name,sizeof out[count].name)) strcpy(out[count].name,"?");
        ++count;
        if(depth>=12) continue;
        uint32_t vtable,adds,head,tree;
        if(!ui_word(read,ctx,object,&vtable) || !ui_word(read,ctx,vtable+14*4,&adds) || adds!=add_child) continue;
        if(!ui_word(read,ctx,object+0x44,&head) || !head || !ui_word(read,ctx,head+4,&tree) || tree==head) continue;
        /* The map's nodes (root = head->parent), any order, then sorted by key: the children in map order. */
        uint32_t nodes[64],children[64],keys[64]; unsigned pending=0,n=0; int broken=0;
        nodes[pending++]=tree;
        while(pending && !broken) {
            uint32_t node=nodes[--pending],left,right; uint8_t nil;
            if(++visited>4096 || !read(ctx,node+0x15,&nil,1)) { broken=1; break; }
            if(nil) continue;
            if(n==64 || !ui_word(read,ctx,node,&left) || !ui_word(read,ctx,node+8,&right)
                || !ui_word(read,ctx,node+0xc,&keys[n]) || !ui_word(read,ctx,node+0x10,&children[n])) { broken=1; break; }
            ++n;
            if(left!=head) { if(pending==64) { broken=1; break; } nodes[pending++]=left; }
            if(right!=head) { if(pending==64) { broken=1; break; } nodes[pending++]=right; }
        }
        if(broken) { out[count-1].depth|=0x100; continue; } /* flagged: its children couldn't be read */
        for(unsigned i=1;i<n;++i) for(unsigned j=i;j>0 && keys[j-1]>keys[j];--j) {
            uint32_t k=keys[j]; keys[j]=keys[j-1]; keys[j-1]=k; k=children[j]; children[j]=children[j-1]; children[j-1]=k;
        }
        for(unsigned i=n;i-- > 0;) {
            if(!children[i] || top==256) continue;
            stack[top].object=children[i]; stack[top].key=keys[i]; stack[top++].depth=depth+1;
        }
    }
    return (int)count;
}

typedef struct { uint32_t uid; uint16_t place; } Yo3UiSlot;

/* Physical player `p`'s 11 field slots (5 monster, 5 Spell/Trap, field) as the card manager holds them. */
static int ui_duel_slots(Yo3UiRead read,void *ctx,uint32_t scene,unsigned p,Yo3UiSlot slots[11]) {
    uint32_t manager;
    if(p>1 || !ui_word(read,ctx,scene+0xbc,&manager) || !manager) return 0;
    for(unsigned z=0;z<11;++z) {
        uint32_t card; slots[z].uid=0; slots[z].place=0;
        if(!ui_word(read,ctx,manager+0x4c+(p*17+z)*4,&card)) return 0;
        if(!card) continue;
        if(!ui_word(read,ctx,card+0x30,&slots[z].uid) || !read(ctx,card+0x82,&slots[z].place,2)) return 0;
    }
    return 1;
}

typedef struct { char *at; size_t left; } Yo3UiOut;
/* Appends all of `text` or, when it doesn't fit, nothing more at all (so a UTF-8 sequence is never cut). */
static void ui_put(Yo3UiOut *o,const char *text) {
    size_t n=strlen(text);
    if(n+1>o->left) { o->left=o->left?1:0; return; }
    memcpy(o->at,text,n+1); o->at+=n; o->left-=n;
}

/* A string argument as `"text"`, at most `limit` characters, UTF-16 turned into UTF-8. */
static void ui_put_string(Yo3UiRead read,void *ctx,uint32_t at,int wide,unsigned limit,Yo3UiOut *o) {
    ui_put(o,"\"");
    for(unsigned i=0;i<limit;++i) {
        uint32_t c=0; uint16_t w; uint8_t b;
        if(wide) { if(!read(ctx,at+i*2,&w,2)) break; c=w; }
        else { if(!read(ctx,at+i,&b,1)) break; c=b; }
        if(!c) { ui_put(o,"\""); return; }
        if(wide && c>=0xd800 && c<0xdc00) { /* a surrogate pair */
            if(!read(ctx,at+(i+1)*2,&w,2) || w<0xdc00 || w>=0xe000) break;
            c=0x10000+((c-0xd800)<<10)+(w-0xdc00); ++i;
        }
        char u[5]={0};
        if(c<0x80) u[0]=(char)c;
        else if(!wide) u[0]='?'; /* a byte of an unknown code page */
        else if(c<0x800) { u[0]=(char)(0xc0|c>>6); u[1]=(char)(0x80|(c&0x3f)); }
        else if(c<0x10000) { u[0]=(char)(0xe0|c>>12); u[1]=(char)(0x80|((c>>6)&0x3f)); u[2]=(char)(0x80|(c&0x3f)); }
        else { u[0]=(char)(0xf0|c>>18); u[1]=(char)(0x80|((c>>12)&0x3f)); u[2]=(char)(0x80|((c>>6)&0x3f)); u[3]=(char)(0x80|(c&0x3f)); }
        ui_put(o,u);
    }
    ui_put(o,"…\""); /* cut, or unreadable */
}

/* One GFx value as text. */
static void ui_put_value(Yo3UiRead read,void *ctx,uint32_t value,Yo3UiOut *o) {
    uint32_t type,pointer; char text[40];
    if(!ui_word(read,ctx,value+4,&type)) { ui_put(o,"?"); return; }
    switch(type) {
    case 0: ui_put(o,"undefined"); return;
    case 1: ui_put(o,"null"); return;
    case 2: { uint8_t b; if(!read(ctx,value+8,&b,1)) break; ui_put(o,b?"true":"false"); return; }
    case 3: { double d; if(!read(ctx,value+8,&d,8)) break; snprintf(text,sizeof text,"%.10g",d); ui_put(o,text); return; }
    case 4: case 5: if(!ui_word(read,ctx,value+8,&pointer)) break; ui_put_string(read,ctx,pointer,type==5,80,o); return;
    case 0x44: if(!ui_word(read,ctx,value+8,&pointer) || !ui_word(read,ctx,pointer,&pointer)) break;
        ui_put_string(read,ctx,pointer,0,80,o); return;
    }
    snprintf(text,sizeof text,"<type 0x%x>",type); ui_put(o,text);
}

/* A Flash call as `method(arg, ...)`: "call"'s first argument names the method; other invokes keep their name
 * (`respond(...)`). At most 8 arguments are shown. */
static void ui_flash_call(Yo3UiRead read,void *ctx,uint32_t name,uint32_t args,unsigned count,char *out,size_t cap) {
    Yo3UiOut o={out,cap}; out[0]=0; char invoke[24]={0}; unsigned first=0;
    for(unsigned i=0;i+1<sizeof invoke;++i) if(!read(ctx,name+i,&invoke[i],1) || !invoke[i]) break;
    uint32_t type;
    if(!strcmp(invoke,"call") && count && ui_word(read,ctx,args+4,&type) && (type==4 || type==0x44)) {
        uint32_t method; char m[64]={0};
        if(ui_word(read,ctx,args+8,&method) && (type==4 || ui_word(read,ctx,method,&method)))
            for(unsigned i=0;i+1<sizeof m;++i) if(!read(ctx,method+i,&m[i],1) || !m[i]) break;
        ui_put(&o,m[0]?m:"?"); first=1;
    } else ui_put(&o,invoke[0]?invoke:"?");
    ui_put(&o,"(");
    for(unsigned i=first;i<count && i<first+8;++i) { if(i>first) ui_put(&o,", "); ui_put_value(read,ctx,args+i*16,&o); }
    if(count>first+8) ui_put(&o,", …");
    ui_put(&o,")");
}

/* JSON-safe copy of UTF-8 text (quotes, backslashes and control characters escaped), within `cap`. */
static void ui_json_text(char *out,size_t cap,const char *text) {
    size_t n=0;
    for(const unsigned char *c=(const unsigned char *)text;*c && n+7<cap;++c) {
        if(*c=='"' || *c=='\\') { out[n++]='\\'; out[n++]=(char)*c; }
        else if(*c<32) n+=(size_t)snprintf(out+n,cap-n,"\\u%04x",*c);
        else out[n++]=(char)*c;
    }
    out[n]=0;
}
#endif
