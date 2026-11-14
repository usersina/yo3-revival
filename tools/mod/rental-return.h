#ifndef YO3_RENTAL_RETURN_H
#define YO3_RENTAL_RETURN_H
/* Issue #13: the borrower returns a rental early from the Rental Card List's Borrowed tab.
 *
 * rentalrecord.swf's Borrowed frame (3) already wires `borrow_btn1` to `click_return`, which calls the Flash callback
 * `BorrowReturan` with the selected row (`listBorrowIndex + selectList`, nothing while no row is selected), but the
 * movie never places that button and RentalRecord registers no such callback. The DLL supplies both, writing no game
 * code and no movie bytes:
 *  - the button: the movie's own library symbol `btn_main01` (a SoundButton with a return arrow), attached to `_root`
 *    as `borrow_btn1` through GFx (Invoke `_root.attachMovie`, SetVariable for its place, help text and sounds), its
 *    click added to `_root.click_return` as frame 3 does; shown on the Borrowed frames (3-8) only. RentalRecord's tick
 *    (vtable 0x7A7A14 slot 14, 0x482250) is wrapped for that.
 *  - the callback: RentalRecord's Accept (FxDelegateHandler vtable 0x7A7A54 slot 1, 0x481D70) is wrapped to register
 *    `BorrowReturan` as well. It reads the row's fields from the borrowed list (+0x90/+0x94, as GetBorrowList
 *    0x482A90 does), sends `0x7A14 {rental_id}` on the front connection (+0x34's sender +0x74, as 0x482870 sends
 *    0x2C34) and greys the row through the movie's `BorrowSetFlag`. The server ends the rental as at its term and
 *    pushes the records (crates/yo3-server/src/front.rs, REVIVAL_RETURN).
 *  - those pushes: RentalRecord answers any 0x2D6C by asking for its lists again, and stores the answers past its
 *    lists (rental-return-logic.h, yo3_rental_skip_packet), so its packet handler (slot 6, 0x482030) is wrapped to
 *    leave 0x2D6C alone once the screen asked for its lists. The global dispatcher still keeps them.
 * Evidence: docs/protocol.md "Card rental between players", docs/client-internals.md "Rental Card List". */
#include "rental-return-logic.h"

/* A GFxValue (GFx 3): +4 type (0 undefined, 2 bool, 3 number, 4 char *, 6 object, 8 display object; 0x40 managed),
 * +8 the value. */
typedef struct { void *iface; DWORD type; union { double number; const char *text; void *data; BYTE flag; } v; } GfxValue;
_Static_assert(sizeof(GfxValue)==16,"GFxValue is 16 bytes");
#define YO3_RENTAL_DEPTH 1000      /* _root depth of the attached button: above the timeline's, clear of card_stage */
#define YO3_RENTAL_BUTTON_X 470    /* below the Borrowed list (rows at x 253, list9 at y 429), left of the card view */
#define YO3_RENTAL_BUTTON_Y 470

typedef int (__attribute__((thiscall)) *RentalTick)(void *,void *);
typedef void (__attribute__((thiscall)) *RentalEvents)(void *,void *);
typedef void (__attribute__((thiscall)) *RentalAccept)(void *,void *);
static struct {
    RentalTick tick; RentalEvents events; RentalAccept accept;
    void *movie; int visible; void *failed;
    int flag_row; void *flag_movie; /* BorrowSetFlag waiting for the next tick */
    Yo3RentalReturned returned;
} rental;

/* The movie's GFxMovieView (GFxMovieSprite +0x48 → {def, view, delegate}), once the screen built it (state 3). */
static void *rental_movie(BYTE *rr) {
    if(*(int32_t *)(rr+0x40)<3) return NULL;
    BYTE *sprite=*(BYTE **)(rr+0x5c);
    BYTE *holder=sprite?*(BYTE **)(sprite+0x48):NULL;
    return holder?*(void **)(holder+4):NULL;
}
/* GFxMovie's SetVariable (+0x40, as the game's own `_root.canbtn.disabled`) and GetVariable (+0x44, `_root.xpos`); the
 * movie root's Invoke 0x5CB1B0 (name, args, count), as FxDelegate's `call` and `respond`. */
static int gfx_set(void *movie,const char *path,GfxValue value) {
    return ((char (__attribute__((thiscall)) *)(void *,const char *,const GfxValue *,int))(*(DWORD **)movie)[0x40/4])
        (movie,path,&value,0)!=0;
}
static int gfx_get(void *movie,const char *path,GfxValue *out) {
    memset(out,0,sizeof *out);
    return ((char (__attribute__((thiscall)) *)(void *,GfxValue *,const char *))(*(DWORD **)movie)[0x44/4])(movie,out,path)!=0;
}
static void gfx_release(GfxValue *value) {
    if(value->type & 0x40)
        ((void (__attribute__((thiscall)) *)(void *,GfxValue *,void *))game_address(0x5c98c0))(value->iface,value,value->v.data);
    memset(value,0,sizeof *value);
}
static int gfx_invoke(void *movie,const char *name,const GfxValue *args,unsigned count) {
    return ((char (__attribute__((thiscall)) *)(void *,const char *,const GfxValue *,unsigned))game_address(0x5cb1b0))
        (movie,name,args,count)!=0;
}
static int gfx_number(void *movie,const char *path,double *out) {
    GfxValue value; int ok=gfx_get(movie,path,&value) && (value.type & 0x8f)==3;
    if(ok) *out=value.v.number;
    gfx_release(&value);
    return ok;
}
static GfxValue gfx_text(const char *text) { GfxValue v={0}; v.type=4; v.v.text=text; return v; }
static GfxValue gfx_num(double number) { GfxValue v={0}; v.type=3; v.v.number=number; return v; }
static GfxValue gfx_bool(int flag) { GfxValue v={0}; v.type=2; v.v.flag=flag!=0; return v; }

static int rental_attach(void *movie) {
    GfxValue attach[3]={gfx_text("btn_main01"),gfx_text("borrow_btn1"),gfx_num(YO3_RENTAL_DEPTH)};
    double x;
    if(!gfx_invoke(movie,"_root.attachMovie",attach,3) || !gfx_number(movie,"_root.borrow_btn1._x",&x)) return 0;
    gfx_set(movie,"_root.borrow_btn1._x",gfx_num(YO3_RENTAL_BUTTON_X));
    gfx_set(movie,"_root.borrow_btn1._y",gfx_num(YO3_RENTAL_BUTTON_Y));
    gfx_set(movie,"_root.borrow_btn1.helpText",gfx_text("$14282")); /* "Return these cards now." */
    gfx_set(movie,"_root.borrow_btn1.clickSound",gfx_text("SE_DECIDE"));
    gfx_set(movie,"_root.borrow_btn1.overSound",gfx_text("SE_CURSOR"));
    /* this.borrow_btn1.addEventListener("click", this, "click_return"), as frame 3 does (CLIK keeps one per scope). */
    GfxValue root; int wired=0;
    if(gfx_get(movie,"_root",&root) && (root.type & 0x8f)>=6) {
        GfxValue listen[3]={gfx_text("click"),root,gfx_text("click_return")};
        wired=gfx_invoke(movie,"_root.borrow_btn1.addEventListener",listen,3);
    }
    gfx_release(&root);
    return wired;
}
static void rental_button(BYTE *rr) {
    void *movie=rental_movie(rr);
    if(!movie || movie==rental.failed) return;
    double frame,x;
    if(!gfx_number(movie,"_root._currentframe",&frame)) return;
    int want=yo3_rental_borrowed_frame(frame);
    if(movie!=rental.movie) { rental.movie=movie; rental.visible=-1; }
    if(!gfx_number(movie,"_root.borrow_btn1._x",&x)) {
        if(!want) return;
        if(!rental_attach(movie)) {
            rental.failed=movie;
            starter_log("[YO3 MOD] The Rental Card List's Return button could not be added.\r\n");
            return;
        }
        rental.visible=-1;
    }
    if(want!=rental.visible && gfx_set(movie,"_root.borrow_btn1._visible",gfx_bool(want))) rental.visible=want;
    /* The returned row greys as at its term (the movie's BorrowSetFlag), from the tick: the same invoke made inside the
     * BorrowReturan callback was seen live and changed nothing. */
    if(rental.flag_movie==movie) {
        GfxValue flag[2]={gfx_text("BorrowSetFlag"),gfx_num(rental.flag_row)};
        gfx_invoke(movie,"call",flag,2);
    }
    rental.flag_movie=NULL;
}
static int __attribute__((thiscall,force_align_arg_pointer)) rental_tick(void *rr,void *event) {
    int result=rental.tick(rr,event);
    rental_button(rr);
    return result;
}

/* The return, built as the screen's own requests are (0x482870): a YO3Request through the front connection. */
static int rental_send(BYTE *rr,unsigned id) {
    BYTE *front=*(BYTE **)(rr+0x34);
    void *sender=front?*(void **)(front+0x74):NULL;
    if(!sender) return 0;
    __attribute__((aligned(8))) BYTE request[0x80]; /* YO3Request is 0x6C bytes (0x52FCC0 allocates that much) */
    memset(request,0,sizeof request);
    ((void *(__attribute__((stdcall)) *)(void *))game_address(0x40a190))(request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4); /* YO3Request's vtable */
    request[0x34]=4;
    ((void (__attribute__((thiscall)) *)(void *,unsigned))game_address(0x41bca0))(request,0x7a14);
    void *map=*(void **)(request+0x60);
    GameString key; game_string(&key,"rental_id");
    ((void (__attribute__((thiscall)) *)(void *,GameString,unsigned))(*(DWORD **)map)[0x14/4])(map,key,id);
    ((void (__attribute__((thiscall)) *)(void *,void *))(*(DWORD **)sender)[0x50/4])(sender,request);
    *(DWORD *)request=(DWORD)(uintptr_t)game_address(0x7955f4);
    ((void (__attribute__((thiscall)) *)(void *))game_address(0x40a210))(request);
    return 1;
}
/* BorrowReturan(row): FxDelegateArgs +0x10 the handler (RentalRecord +0x2C), +0x14 the movie, +0x18 the arguments,
 * +0x1C their count; a callback as GetBorrowList 0x482A90 (cdecl, the arguments' reference). */
static void __attribute__((cdecl,force_align_arg_pointer)) rental_return_pressed(BYTE *params) {
    BYTE *handler=*(BYTE **)(params+0x10);
    const GfxValue *args=*(const GfxValue **)(params+0x18);
    unsigned count=*(unsigned *)(params+0x1c);
    if(!handler || !args || count<1 || (args[0].type & 0x8f)!=3) return;
    BYTE *rr=handler-0x2c;
    int row;
    if(!yo3_rental_row(*(DWORD *)(rr+0x90),*(DWORD *)(rr+0x94),args[0].v.number,&row)) return;
    /* The list keeps each record's fields, not the record: the 0x2D6E handler stores item->+0x68 (its field map)
     * ->+4 (a copy) (0x48217C-0x4821A2), which GetBorrowList reads with the typed getters (0x4090F0). */
    void *map=((void **)(uintptr_t)*(DWORD *)(rr+0x90))[row];
    unsigned id=game_uint(game_field(map,"rental_id")), over=game_uint(game_field(map,"confirm_to"));
    if(!yo3_rental_should_return(&rental.returned,id,over)) return;
    if(!rental_send(rr,id)) return;
    yo3_rental_remember(&rental.returned,id);
    char line[96]; snprintf(line,sizeof line,"[YO3 MOD] Rental %u returned from the Rental Card List.\r\n",id);
    starter_log(line);
    rental.flag_movie=rental_movie(rr); rental.flag_row=row;
}
static void __attribute__((thiscall,force_align_arg_pointer)) rental_accept(void *handler,void *processor) {
    rental.accept(handler,processor);
    if(!processor) return;
    void *name=NULL; /* a GString: its node, flags in the low bits; Process keeps its own reference */
    ((void (__attribute__((thiscall)) *)(void **,const char *))game_address(0x5c8c90))(&name,"BorrowReturan");
    ((void (__attribute__((thiscall)) *)(void *,void **,void *))(*(DWORD **)processor)[1])(processor,&name,(void *)rental_return_pressed);
    BYTE *node=(BYTE *)((uintptr_t)name & ~(uintptr_t)3);
    if(node && InterlockedExchangeAdd((LONG *)(node+4),-1)==1) ((void (__attribute__((stdcall)) *)(void *))game_address(0x5bbc90))(node);
}
static void __attribute__((thiscall,force_align_arg_pointer)) rental_events(BYTE *rr,BYTE *event) {
    if(event && *(DWORD *)(event+4)==*(DWORD *)(rr+0x34) && *(DWORD *)(event+8)==0x1004) {
        BYTE *packet=*(BYTE **)(event+0xc);
        if(packet && packet[0x34]==4) {
            unsigned id=game_uint(game_field(game_get(packet,0x68),"request"));
            if(yo3_rental_skip_packet(id,*(int32_t *)(rr+0x40))) return;
        }
    }
    rental.events(rr,event);
}

static int rental_return_install(void) {
    BYTE *base=(BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt=(IMAGE_NT_HEADERS32 *)(base+((IMAGE_DOS_HEADER *)base)->e_lfanew);
    /* As deck-upload.h: the contract fixtures are small executables; the other guards have identified v1.153. */
    if(nt->OptionalHeader.SizeOfImage<0x42e000) return 1;
    DWORD *slots[3], values[3];
    for(int i=0;i<3;++i) { slots[i]=(DWORD *)game_address(yo3_rental_slots[i][0]); values[i]=*slots[i]; }
    if(!yo3_rental_slots_ok((uint32_t)yo3_game_image_base,(const uint32_t *)values)) return 0;
    rental.tick=(RentalTick)(uintptr_t)values[0];
    rental.events=(RentalEvents)(uintptr_t)values[1];
    rental.accept=(RentalAccept)(uintptr_t)values[2];
    const DWORD hooks[3]={(DWORD)(uintptr_t)rental_tick,(DWORD)(uintptr_t)rental_events,(DWORD)(uintptr_t)rental_accept};
    for(int i=0;i<3;++i) {
        DWORD previous;
        if(!VirtualProtect(slots[i],sizeof *slots[i],PAGE_READWRITE,&previous)) return 0;
        *slots[i]=hooks[i];
        VirtualProtect(slots[i],sizeof *slots[i],previous,&previous);
    }
    return 1;
}
#endif
