#ifndef YO3_DIALOG_TEXT_H
#define YO3_DIALOG_TEXT_H
/* Duel dialogs show the server's own text. A dialog takes only ids of the client's duel prompt table: a message
 * (0x519870: an id, or entries of text ids, card names and numbers), a Select dialog's labels (0x4C5880, 14 bits of
 * an id) and every formatted text look an id up with 0x40BB20, that table's vector::at (EDI the vector at data manager
 * +8 +0x84, EBX the id; past the end it throws, 0x537CF0). The server sends its texts on the duel connection as
 * duelRequestName 0x7A12 {slot, text} (crates/yo3-server/src/duel.rs, text_message), which the stock pump drops
 * (0x52341C: only 0x10-0x15), and names them as ids 0x3F00 + slot. Two byte-guarded hooks:
 *   the pump's call to its duelRequestName reader (0x523411 -> 0x5201A0, ESI the message) also keeps a 0x7A12 text;
 *   0x40BB20 answers ids 0x3F00-0x3FFF past the end of that one table with the kept text (an offset from the table's
 *   string base, +0x9C, as the table's own entries are), and leaves every other lookup to the original.
 * The client's markup applies to them as to its own texts (0x519F80: `@n` colours, HTML). Uses starter.h's message
 * helpers and ui-probe.h's detour. Evidence: docs/protocol.md, "Server texts". */
#define YO3_REVIVAL_TEXT 0x7a12u
#define YO3_TEXT_IDS 0x3f00u
#define YO3_TEXT_SLOTS 0x100u
#define YO3_TEXT_UNITS 512
static wchar_t dialog_texts[YO3_TEXT_SLOTS][YO3_TEXT_UNITS];
static DWORD dialog_offset;
void *dialog_lookup_original;
static const unsigned char yo3_lookup_prologue[]={0x55,0x8b,0xec,0x83,0xe4,0xf8}; /* push ebp; mov ebp,esp; and esp,-8 */
static int yo3_dialog_text_ok(const unsigned char *pump_call, const unsigned char *lookup) {
    static const unsigned char call[]={0xe8,0x8a,0xcd,0xff,0xff}; /* 0x523411: call 0x5201A0 */
    return !memcmp(pump_call,call,sizeof call) && !memcmp(lookup,yo3_lookup_prologue,sizeof yo3_lookup_prologue);
}

/* A 0x7A12 message: its text (UTF-8) kept for its slot. On the game's thread, as the pump pops it. */
void __attribute__((cdecl,force_align_arg_pointer)) dialog_text_message(void *message) {
    void *map=game_get(message,0x68);
    void *slot=game_field(map,"slot");
    if(!slot) return;
    unsigned n=game_uint(slot);
    if(n>=YO3_TEXT_SLOTS) return;
    static char text[YO3_TEXT_UNITS*3];
    if(!game_text(map,"text",text,sizeof text)
        || !MultiByteToWideChar(CP_UTF8,0,text,-1,dialog_texts[n],YO3_TEXT_UNITS)) dialog_texts[n][0]=0;
    dialog_texts[n][YO3_TEXT_UNITS-1]=0;
}

/* 0x40BB20's answer for a server text: a pointer to the entry the caller adds to the table's base, or NULL for the
 * original lookup (another vector, an id of the table's own, or outside the server's ids). */
DWORD *__attribute__((cdecl,force_align_arg_pointer)) dialog_text_lookup(BYTE *vector, unsigned id) {
    BYTE *prompts=*(BYTE **)game_address(0x82eba8); /* 0x406680's data manager (0x82EBA0) +8 */
    if(!prompts || vector!=prompts+0x84) return NULL;
    DWORD begin=*(DWORD *)(vector+0xc),end=*(DWORD *)(vector+0x10);
    if(end<begin || id<(end-begin)/4 || id<YO3_TEXT_IDS || id>=YO3_TEXT_IDS+YO3_TEXT_SLOTS) return NULL;
    dialog_offset=(DWORD)(uintptr_t)dialog_texts[id-YO3_TEXT_IDS]-*(DWORD *)(prompts+0x9c);
    return &dialog_offset;
}

void dialog_lookup_thunk(void);
void dialog_kind_thunk(void);
__asm__(
    ".text\n"
    ".globl _dialog_lookup_thunk\n"
    "_dialog_lookup_thunk:\n"
    "    cmpl $0x3f00,%ebx\n"
    "    jb 1f\n"
    "    cmpl $0x4000,%ebx\n"
    "    jae 1f\n"
    "    pushl %ecx\n"
    "    pushl %edx\n"
    "    pushl %ebx\n"
    "    pushl %edi\n"
    "    call _dialog_text_lookup\n"
    "    addl $8,%esp\n"
    "    popl %edx\n"
    "    popl %ecx\n"
    "    testl %eax,%eax\n"
    "    jz 1f\n"
    "    ret\n"
    "1:  jmp *_dialog_lookup_original\n"
    ".globl _dialog_kind_thunk\n"
    "_dialog_kind_thunk:\n"
    "    movl _yo3_game_image_base,%eax\n"
    "    addl $0x1201a0,%eax\n"
    "    call *%eax\n"
    "    cmpl $0x7a12,%eax\n"
    "    jne 2f\n"
    "    pushl %eax\n"
    "    pushl %ecx\n"
    "    pushl %edx\n"
    "    pushl %esi\n"
    "    call _dialog_text_message\n"
    "    addl $4,%esp\n"
    "    popl %edx\n"
    "    popl %ecx\n"
    "    popl %eax\n"
    "2:  ret\n");

static int dialog_text_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As deck-upload.h: the contract fixtures are small executables; the other guards have identified v1.153. */
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return 1;
    BYTE *call=base+0x123411;
    if(!yo3_dialog_text_ok(call,base+0xbb20)) return 0;
    if(!ui_detour(0x40bb20,yo3_lookup_prologue,sizeof yo3_lookup_prologue,(void *)dialog_lookup_thunk,&dialog_lookup_original)) return 0;
    DWORD previous;
    if(!VirtualProtect(call,5,PAGE_EXECUTE_READWRITE,&previous)) return 0;
    LONG offset=(LONG)((BYTE *)dialog_kind_thunk-(call+5)); memcpy(call+1,&offset,4);
    FlushInstructionCache(GetCurrentProcess(),call,5);
    DWORD ignored; return VirtualProtect(call,5,previous,&ignored)!=0;
}
#endif
