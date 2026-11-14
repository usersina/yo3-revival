#ifndef YO3_RESOLUTION_FIT_H
#define YO3_RESOLUTION_FIT_H
/* The geometry of the opt-in higher resolution (resolution.h), kept apart so it is tested without the game
 * (resolution-test.c). The game lays out, draws and hit-tests a virtual frame of 1024x768 (or 1024x600) pixels. The
 * real frame (the back buffer, or the window's client area) holds that frame scaled to fit and centred: content
 * cw x ch at ox,oy, bars beside or above it when the aspect ratios differ. Integer arithmetic only: the game runs
 * Direct3D without FPU_PRESERVE, so the x87 unit is at single precision whenever these run. 32-bit: sizes are at
 * most 8192 and positions within ±32768, so no product passes 2^31 (and the DLL needs no 64-bit division helper). */
#include <stdlib.h>
#include <string.h>
typedef struct { int vw, vh, rw, rh, cw, ch, ox, oy; } Yo3Fit;

#define YO3_RESOLUTION_MIN 320
#define YO3_RESOLUTION_MAX 8192

/* YO3_RESOLUTION: "" / "0" / "off" / "original": off (0). "auto": the largest frame the screen holds, *w = *h = 0
 * (1). "WIDTHxHEIGHT", each 320..8192 (1). Anything else: invalid (-1), which refuses to start the game. */
static int yo3_resolution_parse(const char *text, int *w, int *h) {
    *w = *h = 0;
    if (!text || !*text || !strcmp(text, "0") || !_stricmp(text, "off") || !_stricmp(text, "original")) return 0;
    if (!_stricmp(text, "auto")) return 1;
    char *end; long width = strtol(text, &end, 10);
    if (end == text || (*end != 'x' && *end != 'X')) return -1;
    const char *rest = end + 1; long height = strtol(rest, &end, 10);
    if (end == rest || *end || *text < '0' || *text > '9' || *rest < '0' || *rest > '9') return -1;
    if (width < YO3_RESOLUTION_MIN || width > YO3_RESOLUTION_MAX || height < YO3_RESOLUTION_MIN || height > YO3_RESOLUTION_MAX)
        return -1;
    *w = (int)width; *h = (int)height; return 1;
}

static int yo3_floor_div(int a, int b) { int q = a / b; return (a % b && (a < 0) != (b < 0)) ? q - 1 : q; }

/* The virtual frame vw x vh fitted into rw x rh: as large as fits, centred. */
static void yo3_fit(Yo3Fit *f, int vw, int vh, int rw, int rh) {
    f->vw = vw; f->vh = vh; f->rw = rw; f->rh = rh;
    if (rw * vh <= rh * vw) { f->cw = rw; f->ch = (rw * vh * 2 + vw) / (2 * vw); }
    else { f->ch = rh; f->cw = (rh * vw * 2 + vh) / (2 * vh); }
    if (f->cw < 1) f->cw = 1;
    if (f->ch < 1) f->ch = 1;
    f->ox = (rw - f->cw) / 2; f->oy = (rh - f->ch) / 2;
}

/* The largest frame of the virtual aspect ratio inside aw x ah (the "auto" size), rounded down. */
static void yo3_fit_largest(int vw, int vh, int aw, int ah, int *w, int *h) {
    if (aw * vh <= ah * vw) { *w = aw; *h = aw * vh / vw; }
    else { *h = ah; *w = ah * vw / vh; }
}

/* Virtual → real: a coordinate or an edge, rounded to the nearest pixel. Edges map one by one, so neighbouring
 * rectangles still meet and a full-frame rectangle covers exactly the content. */
static int yo3_fit_x(const Yo3Fit *f, int x) { return f->ox + yo3_floor_div(x * f->cw * 2 + f->vw, 2 * f->vw); }
static int yo3_fit_y(const Yo3Fit *f, int y) { return f->oy + yo3_floor_div(y * f->ch * 2 + f->vh, 2 * f->vh); }
/* Real → virtual: the virtual pixel under a real pixel's centre (a mouse position), the inverse of the rounded edges
 * above. Bars give positions outside 0..vw-1. */
static int yo3_unfit_x(const Yo3Fit *f, int x) { return yo3_floor_div(((x - f->ox) * 2 + 1) * f->vw, 2 * f->cw); }
static int yo3_unfit_y(const Yo3Fit *f, int y) { return yo3_floor_div(((y - f->oy) * 2 + 1) * f->vh, 2 * f->ch); }

/* Real → virtual for an edge (of a viewport read back from Direct3D): the nearest virtual edge, which undoes
 * yo3_fit_x/y exactly whenever the frame is drawn at least as large as it is. */
static int yo3_unfit_edge_x(const Yo3Fit *f, int x) { return yo3_floor_div((x - f->ox) * f->vw * 2 + f->cw, 2 * f->cw); }
static int yo3_unfit_edge_y(const Yo3Fit *f, int y) { return yo3_floor_div((y - f->oy) * f->vh * 2 + f->ch, 2 * f->ch); }

/* A virtual rectangle (left, top, right, bottom exclusive) → real. */
static void yo3_fit_rect(const Yo3Fit *f, long *l, long *t, long *r, long *b) {
    *l = yo3_fit_x(f, (int)*l); *t = yo3_fit_y(f, (int)*t); *r = yo3_fit_x(f, (int)*r); *b = yo3_fit_y(f, (int)*b);
}
/* Where a window of w x h at (left, top) goes to lie inside the work area (al, at, ar, ab): moved as little as possible,
 * to the area's top left when it is larger. Irrlicht centres its window on the whole screen, so a window sized for the
 * work area ("auto") can reach under the taskbar. */
static void yo3_keep_inside(int left, int top, int w, int h, int al, int at, int ar, int ab, int *x, int *y) {
    *x = left + w > ar ? ar - w : left; if (*x < al) *x = al;
    *y = top + h > ab ? ab - h : top; if (*y < at) *y = at;
}

#endif
