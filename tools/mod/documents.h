#ifndef YO3_DOCUMENTS_H
#define YO3_DOCUMENTS_H
/* A game-data folder per server. The client keeps decks, the deck choice (.y3p) and our starter marker under
 * Documents\Yu-Gi-Oh! ONLINE 3\players\<player name>\, and every server's players start as "Duelist", so a second
 * server inherited the first one's decks (and skipped its starter deck). The client asks for Documents in two places
 * only, both SHGetFolderPath(CSIDL_PERSONAL | CSIDL_FLAG_CREATE) followed by PathAppend "Yu-Gi-Oh! ONLINE 3"
 * (0x52B93F wide, and the narrow one in the function before 0x52BFD0). This answers those with a folder of the server's own:
 *   - The first server played on after this change keeps the original Documents folder, so existing decks stay where
 *     they were (claimed by `Yu-Gi-Oh! ONLINE 3\revival-server.txt` holding its id).
 *   - Every other server gets Documents\YO3 Revival\servers\<id>, created on first use with the original folder's
 *     display settings (options.y3o) copied in.
 * The id (YO3_SERVER_ID, 16 hex digits of the host's key) comes from the launcher; without one, nothing changes. */
#include <shlobj.h>
static HRESULT (WINAPI *documents_w)(HWND,int,HANDLE,DWORD,LPWSTR);
static HRESULT (WINAPI *documents_a)(HWND,int,HANDLE,DWORD,LPSTR);
static wchar_t documents_server[24];
static wchar_t documents_folder[MAX_PATH];
static int documents_ready;

/* The id must be exactly 16 hex digits: it becomes a folder name. */
static int documents_valid_id(const wchar_t *id) {
    if(wcslen(id)!=16) return 0;
    for(const wchar_t *c=id;*c;++c) if(!((*c>='0'&&*c<='9')||(*c>='a'&&*c<='f'))) return 0;
    return 1;
}

/* Reads the claim file's id (empty when none). */
static void documents_claim(const wchar_t *file, wchar_t *id, size_t capacity) {
    id[0]=0; char text[32]={0}; DWORD got=0;
    HANDLE f=CreateFileW(file,GENERIC_READ,FILE_SHARE_READ,NULL,OPEN_EXISTING,0,NULL);
    if(f==INVALID_HANDLE_VALUE) return;
    if(ReadFile(f,text,sizeof text-1,&got,NULL)) {
        while(got && (text[got-1]=='\n'||text[got-1]=='\r'||text[got-1]==' ')) text[--got]=0;
        MultiByteToWideChar(CP_UTF8,0,text,-1,id,(int)capacity);
    }
    CloseHandle(f);
}

/* Decides once, at the first lookup, which Documents folder this server's game data lives under. */
static int documents_decide(void) {
    if(documents_ready) return documents_ready>0;
    documents_ready=-1;
    wchar_t documents[MAX_PATH];
    if(!documents_valid_id(documents_server) || documents_w(NULL,CSIDL_PERSONAL,NULL,SHGFP_TYPE_CURRENT,documents)!=S_OK) return 0;
    wchar_t original[MAX_PATH], claim[MAX_PATH], owner[24];
    if(swprintf(original,MAX_PATH,L"%ls\\Yu-Gi-Oh! ONLINE 3",documents)<0
        || swprintf(claim,MAX_PATH,L"%ls\\revival-server.txt",original)<0) return 0;
    documents_claim(claim,owner,24);
    if(!owner[0]) {
        /* Unclaimed: this server takes the original folder (made if missing), and every other server goes elsewhere. */
        CreateDirectoryW(original,NULL);
        HANDLE f=CreateFileW(claim,GENERIC_WRITE,0,NULL,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,NULL);
        if(f!=INVALID_HANDLE_VALUE) {
            char text[24]; int n=WideCharToMultiByte(CP_UTF8,0,documents_server,-1,text,sizeof text,NULL,NULL);
            DWORD written; WriteFile(f,text,n>0?(DWORD)n-1:0,&written,NULL); CloseHandle(f);
        }
        documents_claim(claim,owner,24);
    }
    if(!wcscmp(owner,documents_server)) { documents_ready=0; return 0; } /* the original folder: answer as usual */
    wchar_t revival[MAX_PATH], servers[MAX_PATH], data[MAX_PATH];
    if(swprintf(revival,MAX_PATH,L"%ls\\YO3 Revival",documents)<0 || swprintf(servers,MAX_PATH,L"%ls\\servers",revival)<0
        || swprintf(documents_folder,MAX_PATH,L"%ls\\%ls",servers,documents_server)<0
        || swprintf(data,MAX_PATH,L"%ls\\Yu-Gi-Oh! ONLINE 3",documents_folder)<0) return 0;
    CreateDirectoryW(revival,NULL); CreateDirectoryW(servers,NULL); CreateDirectoryW(documents_folder,NULL);
    if(CreateDirectoryW(data,NULL)) {
        /* A new server's folder starts with the player's display settings, nothing else. */
        wchar_t from[MAX_PATH], to[MAX_PATH];
        if(swprintf(from,MAX_PATH,L"%ls\\options.y3o",original)>0 && swprintf(to,MAX_PATH,L"%ls\\options.y3o",data)>0)
            CopyFileW(from,to,TRUE);
    }
    DWORD attributes=GetFileAttributesW(data);
    if(attributes==INVALID_FILE_ATTRIBUTES || !(attributes&FILE_ATTRIBUTE_DIRECTORY)) return 0;
    documents_ready=1;
    return 1;
}

static HRESULT WINAPI documents_hook_w(HWND window, int csidl, HANDLE token, DWORD flags, LPWSTR path) {
    if((csidl&0xff)==CSIDL_PERSONAL && path && documents_decide()) { wcscpy(path,documents_folder); return S_OK; }
    return documents_w(window,csidl,token,flags,path);
}

static HRESULT WINAPI documents_hook_a(HWND window, int csidl, HANDLE token, DWORD flags, LPSTR path) {
    if((csidl&0xff)==CSIDL_PERSONAL && path && documents_decide()) {
        /* The narrow lookup gets the same folder; one the ANSI code page can't spell keeps the usual answer. */
        BOOL lossy=FALSE;
        int n=WideCharToMultiByte(CP_ACP,WC_NO_BEST_FIT_CHARS,documents_folder,-1,path,MAX_PATH,NULL,&lossy);
        if(n>0 && !lossy) return S_OK;
    }
    return documents_a(window,csidl,token,flags,path);
}

static int documents_install(void) {
    DWORD n=GetEnvironmentVariableW(L"YO3_SERVER_ID",documents_server,24);
    if(!n || n>=24 || !documents_valid_id(documents_server)) { documents_server[0]=0; return 1; } /* nothing to keep apart */
    return hook("SHGetFolderPathW",(void *)documents_hook_w,(void **)&documents_w)
        && hook("SHGetFolderPathA",(void *)documents_hook_a,(void **)&documents_a);
}
#endif
