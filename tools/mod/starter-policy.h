#ifndef YO3_STARTER_POLICY_H
#define YO3_STARTER_POLICY_H
#include <stdint.h>
#include <string.h>
typedef struct { uintptr_t connection; unsigned player; int complete; } Yo3StarterState;
typedef struct {
    uintptr_t connection; unsigned player, count;
    int ready, existing_decks, initialized, selection_empty;
} Yo3StarterContext;
typedef struct {
    void *data;
    int (*valid)(void *, unsigned);
    int (*exists)(void *, unsigned); /* negative means the path cannot safely be checked */
    int (*save)(void *, unsigned);   /* publish without replacing an existing file */
    int (*select)(void *, unsigned);
    int (*remember)(void *); /* durable first-run marker; later manual edits/deletions win */
} Yo3StarterOps;
enum { YO3_STARTER_WAIT, YO3_STARTER_PRESERVED, YO3_STARTER_CREATED, YO3_STARTER_UNAVAILABLE, YO3_STARTER_FAILED };
static int yo3_starter_dispatch_call_ok(const unsigned char *bytes, size_t length) {
    static const unsigned char expected[]={0xe8,0x3e,0x05,0x00,0x00};
    return length==sizeof expected && !memcmp(bytes,expected,sizeof expected);
}
static void yo3_starter_reset(Yo3StarterState *s) { memset(s,0,sizeof *s); }
static int yo3_starter_apply(Yo3StarterState *s, const Yo3StarterContext *c, const Yo3StarterOps *ops) {
    if(!c->ready || !c->connection || !c->player) return YO3_STARTER_WAIT;
    if(s->complete && s->connection==c->connection && s->player==c->player) return YO3_STARTER_PRESERVED;
    s->connection=c->connection; s->player=c->player; s->complete=0;
    if(c->initialized || c->existing_decks || !c->selection_empty) {
        if(!c->initialized && !ops->remember(ops->data)) return YO3_STARTER_FAILED;
        s->complete=1; return YO3_STARTER_PRESERVED;
    }
    unsigned count=c->count<10?c->count:10;
    for(unsigned i=0;i<count;++i) {
        if(!ops->valid(ops->data,i) || ops->exists(ops->data,i)!=0) continue;
        if(!ops->save(ops->data,i)) return YO3_STARTER_FAILED;
        if(c->selection_empty) ops->select(ops->data,i);
        if(!ops->remember(ops->data)) return YO3_STARTER_FAILED;
        s->complete=1; return YO3_STARTER_CREATED;
    }
    s->complete=1; return YO3_STARTER_UNAVAILABLE;
}
static int yo3_starter_safe_name(const char *s) {
    size_t n=strlen(s);
    if(!n || n>64 || s[n-1]=='.' || s[n-1]==' ') return 0;
    char base[5]={0}; unsigned i=0;
    for(const unsigned char *p=(const unsigned char *)s;*p;++p) {
        if(*p<32 || *p==127 || strchr("\\/:*?\"<>|",*p)) return 0;
        if(*p=='.') break;
        if(i<sizeof base-1) base[i]=(*p>='a' && *p<='z')?(char)(*p-'a'+'A'):(char)*p;
        ++i;
    }
    if(i==3 && (!strcmp(base,"CON") || !strcmp(base,"PRN") || !strcmp(base,"AUX") || !strcmp(base,"NUL"))) return 0;
    if(i==4 && (!memcmp(base,"COM",3) || !memcmp(base,"LPT",3)) && base[3]>='1' && base[3]<='9') return 0;
    /* Check the extension too: a dot must not hide a path separator/control character. */
    for(const unsigned char *p=(const unsigned char *)s;*p;++p) if(*p<32 || *p==127 || strchr("\\/:*?\"<>|",*p)) return 0;
    return 1;
}
#endif
