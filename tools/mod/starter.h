#ifndef YO3_STARTER_H
#define YO3_STARTER_H
/* Original code calling ver.1.153's native APIs. Addresses/ABI evidence: docs/client-internals.md.
 * The disk executable stays unchanged. Only the verified global-dispatch CALL is redirected in memory.
 * Run on the game's dispatch thread, after the original handler populated collection/rental datasets. */
#include "starter-policy.h"
typedef struct { DWORD proxy; union { char text[16]; char *pointer; } data; DWORD size, capacity; } GameString;
_Static_assert(sizeof(GameString)==28,"MSVC 2008 string ABI");
typedef struct { void *cards, *regulation, *collection; } DeckCheck;
typedef void *(__attribute__((thiscall)) *ObjectGet)(void *);
typedef void (__attribute__((thiscall)) *StringCtor)(GameString *, const char *);
typedef void (__attribute__((thiscall)) *StringDtor)(GameString *);
static Yo3StarterState starter_state;
static void starter_log(const char *text) {
    DWORD n; WriteFile(GetStdHandle(STD_ERROR_HANDLE),text,(DWORD)strlen(text),&n,NULL);
}
static void game_string(GameString *out, const char *text) { ((StringCtor)*(DWORD *)game_address(0x7583cc))(out,text); }
static void game_string_free(GameString *s) { ((StringDtor)*(DWORD *)game_address(0x7583d4))(s); }
static void *game_get(void *object, unsigned slot) { return ((ObjectGet)(*(DWORD **)object)[slot/4])(object); }
static void game_delete(void *object) {
    if(object) ((void (__attribute__((thiscall)) *)(void *,unsigned))(*(DWORD **)object)[0])(object,1);
}
static int game_string_read(const GameString *s, char *out, size_t size) {
    if(s->size>=size || s->capacity<s->size) return 0;
    const char *p=s->capacity<16?s->data.text:s->data.pointer;
    if(!p || memchr(p,0,s->size)) return 0;
    memcpy(out,p,s->size); out[s->size]=0; return 1;
}
/* std::map::find, same checked iterator used by the game's typed getters (0x409020/0x4090F0). */
static void *game_field(void *map, const char *key) {
    if(!map) return NULL;
    GameString name; game_string(&name,key);
    DWORD iter[2]={0,0}; void *tree=(BYTE *)map+4;
    __asm__ volatile("pushl %%esi; pushl %[key]; pushl %[out]; movl %[tree],%%esi; movl _yo3_game_image_base,%%eax; addl $0x1a980,%%eax; call *%%eax; popl %%esi"
        : : [tree]"r"(tree),[key]"r"(&name),[out]"r"(iter) : "eax","ecx","edx","memory","cc");
    void *value=iter[1] && iter[1]!=*(DWORD *)((BYTE *)tree+0x18)?*(void **)(iter[1]+0x28):NULL;
    game_string_free(&name); return value;
}
static unsigned game_uint(void *object) {
    return object?((unsigned (__attribute__((thiscall)) *)(void *))(*(DWORD **)object)[0x14/4])(object):0;
}
static int game_text(void *map, const char *key, char *out, size_t size) {
    void *value=game_field(map,key);
    if(!value || *(DWORD *)value!=(DWORD)(uintptr_t)game_address(0x793e94)) return 0; /* EasyMultiPlayDataString */
    GameString text;
    ((void (__attribute__((thiscall)) *)(void *,GameString *))(*(DWORD **)value)[0x1c/4])(value,&text);
    int ok=game_string_read(&text,out,size); game_string_free(&text); return ok;
}
static unsigned game_items(void *object, void ***items) {
    void *list=game_get(object,0x94); DWORD begin=*(DWORD *)((BYTE *)list+0x44), end=*(DWORD *)((BYTE *)list+0x48);
    if(end<begin || (end-begin)%4 || (end-begin)/4>1000) return 0;
    *items=(void **)begin; return (end-begin)/4;
}
static void *game_deck_info(void) {
    void *config=*(void **)game_address(0x82ec10); /* application (0x40A420) +0x50: selected player's local profile */
    if(!config) return NULL;
    void **items=NULL; unsigned count=game_items(config,&items);
    for(unsigned i=0;i<count;++i) {
        char name[65];
        if(game_text(game_get(items[i],0x68),"name",name,sizeof name) && !strcmp(name,"deckInfo")) return items[i];
    }
    return NULL;
}
static int game_check(DeckCheck *check, void *deck) {
    int result;
    __asm__ volatile("pushl %%ebx; pushl %[deck]; movl %[check],%%ebx; popl %%eax; movl _yo3_game_image_base,%%ecx; addl $0x11aa80,%%ecx; call *%%ecx; popl %%ebx"
        : "=a"(result) : [check]"r"(check),[deck]"r"(deck) : "ecx","edx","memory","cc");
    return result;
}
static int game_check_all(DeckCheck *check, void *manager) {
    return ((int (__attribute__((stdcall)) *)(DeckCheck *,void *))game_address(0x51ab00))(check,(BYTE *)manager+0x1000d8);
}
typedef struct { void *manager, *connection; void **decks; DeckCheck check; char player[65]; } StarterJob;
static int starter_local_path(StarterJob *job, const char *suffix, wchar_t *out) {
    char root[MAX_PATH], full[MAX_PATH];
    if(!yo3_starter_safe_name(job->player) || !game_string_read((GameString *)((BYTE *)job->manager+0x50),root,sizeof root)) return 0;
    int n=snprintf(full,sizeof full,"%s\\players\\%s\\%s",root,job->player,suffix);
    return n>0 && n<(int)sizeof full && MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,full,-1,out,MAX_PATH)>0;
}
static int starter_path(StarterJob *job, unsigned index, wchar_t *path, wchar_t *directory) {
    char name[65], suffix[96];
    if(!game_text(game_get(job->decks[index],0x68),"deck_name",name,sizeof name) || !yo3_starter_safe_name(name)) return 0;
    snprintf(suffix,sizeof suffix,"decks\\%s.y3d",name);
    return starter_local_path(job,suffix,path) && starter_local_path(job,"decks",directory);
}
static int starter_initialized(StarterJob *job) {
    wchar_t file[MAX_PATH];
    if(!starter_local_path(job,"revival-starter.initialized",file)) return 1;
    if(GetFileAttributesW(file)!=INVALID_FILE_ATTRIBUTES) return 1;
    DWORD error=GetLastError(); return error!=ERROR_FILE_NOT_FOUND && error!=ERROR_PATH_NOT_FOUND;
}
static int starter_has_decks(StarterJob *job) {
    wchar_t pattern[MAX_PATH]; WIN32_FIND_DATAW found;
    if(!starter_local_path(job,"decks\\*.y3d",pattern)) return 1;
    HANDLE search=FindFirstFileW(pattern,&found);
    if(search==INVALID_HANDLE_VALUE) {
        DWORD error=GetLastError(); return error!=ERROR_FILE_NOT_FOUND && error!=ERROR_PATH_NOT_FOUND;
    }
    do { if(!(found.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY)) { FindClose(search); return 1; } }
    while(FindNextFileW(search,&found));
    DWORD error=GetLastError(); FindClose(search); return error!=ERROR_NO_MORE_FILES;
}
static int starter_remember(void *data) {
    wchar_t path[MAX_PATH];
    if(!starter_local_path(data,"revival-starter.initialized",path)) return 0;
    HANDLE file=CreateFileW(path,GENERIC_WRITE,0,NULL,CREATE_NEW,FILE_ATTRIBUTE_NORMAL|FILE_FLAG_WRITE_THROUGH,NULL);
    if(file==INVALID_HANDLE_VALUE) return GetLastError()==ERROR_FILE_EXISTS || GetLastError()==ERROR_ALREADY_EXISTS;
    static const char note[]="yo3-revival starter initialized v1\r\n"; DWORD written=0;
    int ok=WriteFile(file,note,sizeof note-1,&written,NULL) && written==sizeof note-1;
    CloseHandle(file); return ok;
}
static int starter_valid(void *data, unsigned index) {
    StarterJob *job=data; void *deck=job->decks[index];
    return *((BYTE *)deck+0x34)==6 && game_check(&job->check,deck)==0;
}
static int starter_exists(void *data, unsigned index) {
    wchar_t file[MAX_PATH], directory[MAX_PATH];
    if(!starter_path(data,index,file,directory)) return -1;
    if(GetFileAttributesW(file)!=INVALID_FILE_ATTRIBUTES) return 1;
    DWORD error=GetLastError(); return error==ERROR_FILE_NOT_FOUND || error==ERROR_PATH_NOT_FOUND?0:-1;
}
static int starter_save(void *data, unsigned index) {
    StarterJob *job=data; wchar_t file[MAX_PATH], directory[MAX_PATH], stage[MAX_PATH]; char encoded[MAX_PATH*3];
    if(!starter_path(job,index,file,directory)) return 0;
    if(!CreateDirectoryW(directory,NULL) && GetLastError()!=ERROR_ALREADY_EXISTS) return 0;
    DWORD attributes=GetFileAttributesW(directory);
    if(attributes==INVALID_FILE_ATTRIBUTES || !(attributes&FILE_ATTRIBUTE_DIRECTORY) || (attributes&FILE_ATTRIBUTE_REPARSE_POINT)) return 0;
    if(!GetTempFileNameW(directory,L"rev",0,stage)) return 0;
    if(!WideCharToMultiByte(CP_UTF8,0,stage,-1,encoded,sizeof encoded,NULL,NULL)) { DeleteFileW(stage); return 0; }
    GameString path; game_string(&path,encoded);
    /* 0x52EE60 ultimately calls this serializer. It consumes the by-value string (0x20 stack bytes),
     * so the C copy transfers ownership. Stage first, then atomically publish without REPLACE_EXISTING. */
    int result=((int (__attribute__((thiscall)) *)(void *,void *,GameString))game_address(0x52c320))(job->manager,job->decks[index],path);
    WIN32_FILE_ATTRIBUTE_DATA written;
    if(result || !GetFileAttributesExW(stage,GetFileExInfoStandard,&written) || written.nFileSizeHigh || written.nFileSizeLow<16
        || !MoveFileExW(stage,file,MOVEFILE_WRITE_THROUGH)) { DeleteFileW(stage); return 0; }
    /* Refresh the same per-player cache loaded at 0x46BDB0. Avoid borrowing the reply's object beyond dispatch. */
    *((BYTE *)job->manager+0x1000f8)=1;
    GameString player; game_string(&player,job->player);
    ((void *(__attribute__((thiscall)) *)(void *,GameString))game_address(0x52f240))(job->manager,player);
    return game_check_all(&job->check,job->manager)==0;
}
static int starter_select(void *data, unsigned index) {
    StarterJob *job=data; void *info=game_deck_info(); char current[65], name[65];
    if(!info) return 0;
    void *map=game_get(info,0x68);
    if(game_text(map,"cursor_deck_name",current,sizeof current) && *current) return 0;
    if(!game_text(game_get(job->decks[index],0x68),"deck_name",name,sizeof name)) return 0;
    GameString key,value; game_string(&key,"cursor_deck_name"); game_string(&value,name);
    ((void (__attribute__((thiscall)) *)(void *,GameString,GameString))(*(DWORD **)map)[0xc/4])(map,key,value);
    void *config=*(void **)game_address(0x82ec10);
    return ((int (__attribute__((thiscall)) *)(void *,void *))game_address(0x52e200))(config,job->manager)==0;
}
static void starter_reply(void *message, void *connection) {
    unsigned id=game_uint(game_field(game_get(message,0x68),"request"));
    if(id==0x2266) { yo3_starter_reset(&starter_state); return; }
    if(id!=0x22c8) return;
    StarterJob job={0}; job.manager=*(void **)game_address(0x82d2c8); job.connection=connection;
    if(!job.manager || !game_text(game_address(0x82eca4),"pname",job.player,sizeof job.player) || !yo3_starter_safe_name(job.player)) return;
    unsigned player=*(DWORD *)((BYTE *)connection+0xd4), regulation=*(DWORD *)((BYTE *)connection+0xc8);
    DWORD begin=*(DWORD *)((BYTE *)connection+0x16c), end=*(DWORD *)((BYTE *)connection+0x170);
    if(!player || !regulation || end<begin || (end-begin)/4<19 || !((void **)begin)[18] || !((void **)begin)[8]) return;
    if(starter_state.complete && starter_state.connection==(uintptr_t)connection && starter_state.player==player) return;
    job.check.cards=*(void **)game_address(0x82eba8); /* 0x406680 data manager +8, as in NPC check 0x4BD8E6 */
    if(!job.check.cards) return;
    struct { DWORD vtable, value; } rule={(DWORD)(uintptr_t)game_address(0x793dac),regulation};
    job.check.regulation=((void *(__attribute__((stdcall)) *)(void *,typeof(rule)))game_address(0x52db30))(job.manager,rule);
    if(!job.check.regulation) return;
    __asm__ volatile("pushl %%esi; movl %[rule],%%esi; movl _yo3_game_image_base,%%eax; addl $0x14630,%%eax; call *%%eax; popl %%esi"
        : : [rule]"r"(job.check.regulation) : "eax","ecx","edx","memory","cc");
    job.check.collection=((void *(__attribute__((stdcall)) *)(void *,unsigned))game_address(0x4f6340))(connection,0x1d);
    if(job.check.collection) {
        void *info=game_deck_info(); char selected[65];
        Yo3StarterContext context={.connection=(uintptr_t)connection,.player=player,.count=game_items(message,&job.decks),.ready=info!=NULL,
            .existing_decks=starter_has_decks(&job),.initialized=starter_initialized(&job),
            .selection_empty=info && (!game_text(game_get(info,0x68),"cursor_deck_name",selected,sizeof selected) || !*selected)};
        Yo3StarterOps ops={&job,starter_valid,starter_exists,starter_save,starter_select,starter_remember};
        int result=yo3_starter_apply(&starter_state,&context,&ops);
        if(result==YO3_STARTER_CREATED) starter_log("[YO3 MOD] Starter deck prepared; Deck Edit is not required.\r\n");
        else if(result==YO3_STARTER_FAILED) starter_log("[YO3 MOD] Automatic deck setup could not finish; existing files were preserved. Check client-profile permissions.\r\n");
    }
    game_delete(job.check.collection); game_delete(job.check.regulation);
}
static void __attribute__((fastcall)) starter_dispatch(void *message, void *unused, void *connection) {
    (void)unused;
    ((void (__attribute__((thiscall)) *)(void *,void *))game_address(0x4f4a00))(message,connection);
    starter_reply(message,connection);
}
static int starter_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* The import-contract fixtures are small standalone executables, with no game dispatcher. */
    if(nt->OptionalHeader.SizeOfImage<0x0f44c2) return 1;
    BYTE *site=base+0x0f44bd;
    if(!yo3_starter_dispatch_call_ok(site,5)) return 0;
    DWORD previous; if(!VirtualProtect(site,5,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    LONG offset=(BYTE *)starter_dispatch-(site+5); memcpy(site+1,&offset,4);
    FlushInstructionCache(GetCurrentProcess(),site,5);
    DWORD ignored; return VirtualProtect(site,5,previous,&ignored)!=0;
}
#endif
