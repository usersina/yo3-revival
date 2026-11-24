#ifndef YO3_UI_PROBE_H
#define YO3_UI_PROBE_H
/* The UI probe: what the client shows, for debugging without screenshots. Off unless YO3_UI_PROBE names a folder
 * (tools/client/vclient.sh sets it for its private clients); players never run any of this. A request file in that
 * folder (`request`) makes the probe write `ui.json` there:
 *   components  the screens, windows and dialogs open now, by class, nested (ui-state.h)
 *   texts       the last UI texts the client looked up (0x51E6C0, table and id → text), newest last: the dialogs,
 *               labels and messages that just appeared, by text id
 *   flash       the last calls the game's code made into its Flash screens (0x5CB1B0), newest last, repeats folded:
 *               the text and state it handed them: an NPC's lines (setSpeech), dialogs opening (showDialog), button
 *               labels (setButtonLabel), list data (respond); MousePos, sent every frame, is left out
 *   duel        in a duel, each side's field slots as the client's card manager holds them (face up, defense)
 * It reads game memory through checked reads, from its own thread; its two hooks (detours of the text lookup and the
 * Flash invoke, each checked byte for byte first) are installed only when the probe is on, and YO3_UI_PROBE_HOOKS=0
 * leaves both out. The invoke hook reads the call's own arguments, which the game is about to hand to Flash. */
#include "ui-state.h"
#include "detour.h"
static wchar_t ui_folder[MAX_PATH];
#define UI_TEXTS 40
static struct { DWORD at; uint32_t table,id; char text[160]; } ui_texts[UI_TEXTS];
static volatile LONG ui_text_next;
static wchar_t *(__attribute__((stdcall)) *ui_lookup)(void *,unsigned);
static int ui_hooked,ui_flash_hooked;
#define UI_CALLS 60
static struct { DWORD at; unsigned repeat; char text[240]; } ui_calls[UI_CALLS];
static volatile LONG ui_call_next;

static int ui_read(void *ctx,uint32_t at,void *out,size_t n) {
    (void)ctx; SIZE_T got=0;
    return at && ReadProcessMemory(GetCurrentProcess(),(const void *)(uintptr_t)at,out,n,&got) && got==n;
}

/* The text lookup's stand-in: the real lookup, then the text kept in the ring (only the first ~160 bytes). */
static wchar_t *__attribute__((stdcall,force_align_arg_pointer)) ui_text_hook(void *table, unsigned id) {
    wchar_t *text=ui_lookup(table,id);
    LONG slot=(InterlockedIncrement(&ui_text_next)-1)%UI_TEXTS;
    ui_texts[slot].at=GetTickCount(); ui_texts[slot].table=(uint32_t)(uintptr_t)table; ui_texts[slot].id=id;
    ui_texts[slot].text[0]=0;
    if(text) WideCharToMultiByte(CP_UTF8,0,text,-1,ui_texts[slot].text,sizeof ui_texts[slot].text,NULL,NULL);
    ui_texts[slot].text[sizeof ui_texts[slot].text-1]=0;
    return text;
}

/* The game reads these arguments itself as soon as this returns, so they are read in place. */
static int ui_direct_read(void *ctx,uint32_t at,void *out,size_t n) { (void)ctx; if(at<0x10000) return 0; memcpy(out,(const void *)(uintptr_t)at,n); return 1; }

/* Each Flash invoke kept as text (the same call again only counts), as duel-numbers.h's detour of the invoke reports
 * it, with the arguments the movie gets. */
static void ui_flash_observe(const char *name,const void *args,unsigned count) {
    char text[sizeof ui_calls[0].text];
    ui_flash_call(ui_direct_read,NULL,(uint32_t)(uintptr_t)name,(uint32_t)(uintptr_t)args,count,text,sizeof text);
    if(!strncmp(text,"MousePos(",9)) return; /* every frame: it would bury the rest */
    LONG last=ui_call_next-1; unsigned slot=(unsigned)last%UI_CALLS;
    if(last>=0 && !strcmp(ui_calls[slot].text,text)) { ++ui_calls[slot].repeat; ui_calls[slot].at=GetTickCount(); }
    else {
        slot=(unsigned)ui_call_next%UI_CALLS;
        ui_calls[slot].at=0; memcpy(ui_calls[slot].text,text,sizeof text); ui_calls[slot].repeat=1; ui_calls[slot].at=GetTickCount();
        InterlockedIncrement(&ui_call_next);
    }
}

/* The detour helper is shared (detour.h): the infinite ATK display (duel-numbers.h) detours the Flash invoke too. */
#define ui_detour yo3_detour

static void ui_append(char **at,size_t *left,const char *format,...) {
    va_list args; va_start(args,format); int n=vsnprintf(*at,*left,format,args); va_end(args);
    if(n<0 || (size_t)n>=*left) { *left=0; return; }
    *at+=n; *left-=(size_t)n;
}

static void ui_write(void) {
    static char json[262144]; char *at=json; size_t left=sizeof json; char text[400];
    static Yo3UiComponent components[400];
    int count=ui_components(ui_read,NULL,(uint32_t)(uintptr_t)game_address(0x82f7c0),(uint32_t)(uintptr_t)game_address(0x403c00),components,400);
    /* text_hook false: the lookup's bytes weren't as expected (or YO3_UI_PROBE_HOOKS=0), so `texts` stays empty; flash_hook likewise. */
    ui_append(&at,&left,"{\"time\":%lu,\"text_hook\":%s,\"flash_hook\":%s,\"components\":[",(unsigned long)GetTickCount(),
        ui_hooked?"true":"false",ui_flash_hooked?"true":"false");
    uint32_t duel_scene=0;
    for(int i=0;i<count;++i) {
        ui_append(&at,&left,"%s{\"depth\":%u,\"class\":\"%s\",\"key\":%ld,\"at\":\"0x%08x\"%s}",i?",":"",components[i].depth&0xff,
            components[i].name,(long)(int32_t)components[i].key,components[i].object,(components[i].depth&0x100)?",\"unreadable_children\":true":"");
        if(!duel_scene && !strcmp(components[i].name,"DuelScene")) duel_scene=components[i].object;
    }
    ui_append(&at,&left,"],\"components_readable\":%s,\"texts\":[",count<0?"false":"true");
    LONG next=ui_text_next; DWORD now=GetTickCount(); int first=1;
    for(LONG k=next-UI_TEXTS;k<next;++k) {
        if(k<0) continue;
        unsigned slot=(unsigned)k%UI_TEXTS; if(!ui_texts[slot].at) continue;
        ui_json_text(text,sizeof text,ui_texts[slot].text);
        ui_append(&at,&left,"%s{\"ago_ms\":%lu,\"id\":%u,\"text\":\"%s\"}",first?"":",",
            (unsigned long)(now-ui_texts[slot].at),ui_texts[slot].id,text);
        first=0;
    }
    ui_append(&at,&left,"],\"flash\":[");
    next=ui_call_next; first=1;
    for(LONG k=next-UI_CALLS;k<next;++k) {
        if(k<0) continue;
        unsigned slot=(unsigned)k%UI_CALLS; if(!ui_calls[slot].at) continue;
        char call[2*sizeof ui_calls[0].text]; ui_json_text(call,sizeof call,ui_calls[slot].text);
        ui_append(&at,&left,"%s{\"ago_ms\":%lu,\"repeat\":%u,\"call\":\"%s\"}",first?"":",",
            (unsigned long)(now-ui_calls[slot].at),ui_calls[slot].repeat,call);
        first=0;
    }
    ui_append(&at,&left,"],\"duel\":");
    uint32_t data,local;
    if(duel_scene && ui_word(ui_read,NULL,(uint32_t)(uintptr_t)game_address(0x82d2c0),&data)
        && ui_word(ui_read,NULL,data+0x1b0,&local) && local<2) {
        ui_append(&at,&left,"{\"scene\":\"0x%08x\",\"sides\":[",duel_scene);
        for(unsigned side=0;side<2;++side) {
            Yo3UiSlot slots[11]; unsigned physical=side^local; /* side 0: this client's */
            ui_append(&at,&left,"%s{\"side\":\"%s\",\"slots\":[",side?",":"",side?"opponent":"you");
            if(ui_duel_slots(ui_read,NULL,duel_scene,physical,slots)) for(unsigned z=0,shown=0;z<11;++z) {
                if(!slots[z].uid) continue;
                ui_append(&at,&left,"%s{\"zone\":%u,\"uid\":%u,\"place\":\"0x%04x\",\"face_up\":%s,\"defense\":%s}",shown++?",":"",
                    z,slots[z].uid,slots[z].place,(slots[z].place&0x4000)?"true":"false",(slots[z].place&0x8000)?"true":"false");
            }
            ui_append(&at,&left,"]}");
        }
        ui_append(&at,&left,"]}");
    } else ui_append(&at,&left,"null");
    ui_append(&at,&left,"}\n");
    if(!left) return;
    wchar_t temporary[MAX_PATH],target[MAX_PATH];
    if(swprintf(temporary,MAX_PATH,L"%ls\\ui.json.tmp",ui_folder)<0 || swprintf(target,MAX_PATH,L"%ls\\ui.json",ui_folder)<0) return;
    HANDLE f=CreateFileW(temporary,GENERIC_WRITE,0,NULL,CREATE_ALWAYS,FILE_ATTRIBUTE_NORMAL,NULL);
    if(f==INVALID_HANDLE_VALUE) return;
    DWORD written; WriteFile(f,json,(DWORD)(at-json),&written,NULL); CloseHandle(f);
    MoveFileExW(temporary,target,MOVEFILE_REPLACE_EXISTING);
}

static DWORD WINAPI ui_probe_thread(void *unused) {
    (void)unused; wchar_t request[MAX_PATH];
    if(swprintf(request,MAX_PATH,L"%ls\\request",ui_folder)<0) return 0;
    for(;;) {
        Sleep(200);
        if(GetFileAttributesW(request)==INVALID_FILE_ATTRIBUTES) continue;
        DeleteFileW(request);
        ui_write();
    }
}

static int ui_probe_install(void) {
    DWORD n=GetEnvironmentVariableW(L"YO3_UI_PROBE",ui_folder,MAX_PATH);
    if(!n || n>=MAX_PATH) return 1; /* off: nothing installed */
    DWORD attributes=GetFileAttributesW(ui_folder);
    if(attributes==INVALID_FILE_ATTRIBUTES || !(attributes&FILE_ATTRIBUTE_DIRECTORY)) return 1;
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return 1; /* contract fixtures: no game to look at */
    char skip[4]={0};
    if(!GetEnvironmentVariableA("YO3_UI_PROBE_HOOKS",skip,sizeof skip) || skip[0]!='0') {
        /* Without a hook the probe still reports components and the duel. */
        /* 0x51E6C0: push ebp; mov ebp,esp; and esp,-8; sub esp,0xC. */
        static const BYTE lookup[]={0x55,0x8b,0xec,0x83,0xe4,0xf8,0x83,0xec,0x0c};
        ui_hooked=ui_detour(0x51e6c0,lookup,sizeof lookup,(void *)ui_text_hook,(void **)&ui_lookup);
        /* The Flash invoke 0x5CB1B0 is detoured by duel-numbers.h, installed first: the probe watches through it. */
        if(duel_numbers_hooked) { yo3_invoke_observer=ui_flash_observe; ui_flash_hooked=1; }
    }
    HANDLE thread=CreateThread(NULL,0,ui_probe_thread,NULL,0,NULL);
    if(thread) CloseHandle(thread);
    return 1;
}
#endif
