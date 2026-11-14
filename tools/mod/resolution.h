#ifndef YO3_RESOLUTION_H
#define YO3_RESOLUTION_H
/* Higher resolution, opt in (issue #1): YO3_RESOLUTION=WIDTHxHEIGHT or auto. Off when unset: nothing is installed.
 *
 * The game keeps laying out, drawing and hit-testing its 1024x768 (or 1024x600) frame; it is told nothing else. The
 * window and Direct3D's back buffer are larger, and every draw lands scaled in them:
 *   - the window: Irrlicht sizes its window with AdjustWindowRect from a 1024x768 client (0x53B0F1, and 0x53BE20 when
 *     it changes style), so a client rectangle of exactly the game's frame becomes the real size;
 *   - Direct3D: Irrlicht loads d3d9.dll itself and asks GetProcAddress for Direct3DCreate9 (0x5441FF). The interface
 *     it gets back has CreateDevice wrapped: the back buffer is the window's real client area, always windowed (a
 *     fullscreen request becomes a borderless window the size of the screen). The device has SetViewport,
 *     SetScissorRect, Clear and SetRenderTarget wrapped: while the back buffer is the render target, viewports,
 *     scissor rectangles and cleared rectangles in the game's frame are scaled into the content area; GetViewport and
 *     GetScissorRect answer in the game's frame. Every vertex the game, Irrlicht and Scaleform send is transformed
 *     (FVFs 0x152, 0x252, 0x140352, shaders), so the viewport carries all of it; textures stay their own size.
 *     Present clears the bars beside a content area that doesn't fill the buffer;
 *   - the mouse: Irrlicht's window procedure (0x53A720) is wrapped through the class registration (0x53B05A): mouse
 *     messages arrive in the game's frame, and WM_SIZE reports the game's frame (its handler otherwise forces the
 *     window back to 1024 wide). The cursor control (0x53A620, 0x53A480) works out the position as GetCursorPos minus
 *     the window rectangle and a border taken from the system metrics (0x53A37B); GetCursorPos and SetCursorPos give
 *     and take positions that make that sum come out in the game's frame. The IME's composition and candidate
 *     windows (0x53C300) are placed in the real client area.
 * Each site is checked byte for byte before anything is installed: a different game refuses to start rather than
 * run half scaled. Evidence: docs/client-internals.md, "Rendering, the window and the mouse". */
#include <stdarg.h>
#include <stdio.h>
#include <d3d9.h>
#include <imm.h>
#include "resolution-fit.h"

static int res_req_w, res_req_h;        /* the requested client size; 0,0: auto */
static int res_vw, res_vh;              /* the game's frame, from its window */
static int res_fullscreen;              /* the game asked for its fullscreen (borderless) window */
static HWND res_window;
static WNDPROC res_game_proc;
static Yo3Fit res_draw;                 /* the game's frame in the back buffer */
static int res_draw_ready, res_rt_back = 1, res_depth;
static IDirect3DSurface9 *res_back;     /* identity only: the swap chain's back buffer */
static D3DVIEWPORT9 res_viewport;       /* the game's frame */
static RECT res_scissor;

static void res_log(const char *format, ...) {
    char text[300]; va_list args; va_start(args, format);
    int n = vsnprintf(text, sizeof text - 2, format, args); va_end(args);
    if (n < 0 || n > (int)sizeof text - 3) n = (int)sizeof text - 3;
    text[n++] = '\r'; text[n++] = '\n';
    DWORD written; WriteFile(GetStdHandle(STD_ERROR_HANDLE), text, (DWORD)n, &written, NULL);
}
/* Each wrapped call's first use from each caller (a preferred-base address when it is game code), up to 64 pairs:
 * the launcher's session log then shows which code reaches which call. */
static struct { unsigned kind; uintptr_t caller; } res_seen[64];
static LONG res_seen_count;
static void res_first(unsigned kind, const char *what, void *caller) {
    LONG n = res_seen_count; if (n > 64) n = 64;
    for (LONG i = 0; i < n; ++i) if (res_seen[i].kind == kind && res_seen[i].caller == (uintptr_t)caller) return;
    LONG slot = InterlockedIncrement(&res_seen_count) - 1; if (slot >= 64) return;
    res_seen[slot].kind = kind; res_seen[slot].caller = (uintptr_t)caller;
    uintptr_t at = (uintptr_t)caller, base = yo3_game_image_base;
    if (at - base < 0x500000) { res_log("[YO3 MOD] resolution: %s from game 0x%08lx", what, (unsigned long)(at - base + 0x400000)); return; }
    HMODULE module = NULL; char name[MAX_PATH] = "?";
    if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, (LPCSTR)caller, &module))
        GetModuleFileNameA(module, name, sizeof name);
    const char *file = strrchr(name, '\\');
    res_log("[YO3 MOD] resolution: %s from %s+0x%lx", what, file ? file + 1 : name, (unsigned long)(at - (uintptr_t)module));
}

/* ---- the window ---- */
static BOOL (WINAPI *res_adjust)(LPRECT, DWORD, BOOL);
static ATOM (WINAPI *res_register)(const WNDCLASSEXA *);
static BOOL (WINAPI *res_get_cursor)(LPPOINT);
static BOOL (WINAPI *res_set_cursor)(int, int);

/* The client size for the game's frame: the screen for its fullscreen window, else the request, else the largest
 * frame of the game's shape in the work area. */
static void res_client_size(DWORD style, int *w, int *h) {
    if (res_fullscreen) { *w = GetSystemMetrics(SM_CXSCREEN); *h = GetSystemMetrics(SM_CYSCREEN); return; }
    if (res_req_w) { *w = res_req_w; *h = res_req_h; return; }
    RECT work, frame = {0, 0, 0, 0};
    if (!SystemParametersInfoA(SPI_GETWORKAREA, 0, &work, 0)) SetRect(&work, 0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
    res_adjust(&frame, style, FALSE);
    yo3_fit_largest(res_vw, res_vh, (work.right - work.left) - (frame.right - frame.left),
        (work.bottom - work.top) - (frame.bottom - frame.top), w, h);
    if (*w < YO3_RESOLUTION_MIN || *h < YO3_RESOLUTION_MIN) { *w = res_vw; *h = res_vh; }
}

static BOOL WINAPI res_adjust_hook(LPRECT rect, DWORD style, BOOL menu) {
    if (rect && !rect->left && !rect->top && rect->right == 1024 && (rect->bottom == 768 || rect->bottom == 600)) {
        res_vw = rect->right; res_vh = rect->bottom;
        res_fullscreen = (style & WS_POPUP) != 0; /* 0x53B0D1: WS_POPUP for fullscreen, else a captioned window */
        int w, h; res_client_size(style, &w, &h);
        rect->right = w; rect->bottom = h;
        res_log("[YO3 MOD] resolution: game frame %dx%d in a %s client area of %dx%d", res_vw, res_vh,
            res_fullscreen ? "borderless fullscreen" : "windowed", w, h);
    }
    return res_adjust(rect, style, menu);
}

/* The game's frame inside the window's client area as it is now (Windows may have kept a window smaller). */
static int res_window_fit(HWND window, Yo3Fit *f) {
    RECT client;
    if (!window || !res_vw || !GetClientRect(window, &client) || client.right < 1 || client.bottom < 1) return 0;
    yo3_fit(f, res_vw, res_vh, client.right, client.bottom); return 1;
}
static int res_mouse_fit(Yo3Fit *f) { return res_window_fit(res_window, f); }
static short res_clamp16(int v) { return (short)(v < -32768 ? -32768 : v > 32767 ? 32767 : v); }

static LRESULT CALLBACK res_proc(HWND window, UINT message, WPARAM w, LPARAM l) {
    /* Each mode the game tries (0x40B370: fullscreen, then windowed, 768 then 600 high) makes a new window of this
     * class; the newest is the game's. All of them are translated, and the cursor follows the newest. */
    if (message == WM_NCCREATE || !res_window) res_window = window;
    Yo3Fit f;
    if (res_vw) {
        if (message == WM_SIZE) l = MAKELPARAM(res_vw, res_vh);
        else if (message >= WM_MOUSEFIRST && message <= 0x20e && res_window_fit(window, &f)) {
            if (message == WM_MOUSEWHEEL || message == 0x20e) { /* screen coordinates; 0x53A720 subtracts the client origin */
                POINT origin = {0, 0}; ClientToScreen(window, &origin);
                int x = yo3_unfit_x(&f, (short)LOWORD(l) - origin.x), y = yo3_unfit_y(&f, (short)HIWORD(l) - origin.y);
                l = MAKELPARAM((WORD)res_clamp16(origin.x + x), (WORD)res_clamp16(origin.y + y));
            } else {
                l = MAKELPARAM((WORD)res_clamp16(yo3_unfit_x(&f, (short)LOWORD(l))), (WORD)res_clamp16(yo3_unfit_y(&f, (short)HIWORD(l))));
            }
        }
    }
    LRESULT result = CallWindowProcA(res_game_proc, window, message, w, l);
    if (message == WM_NCDESTROY && window == res_window) res_window = NULL;
    return result;
}

static ATOM WINAPI res_register_hook(const WNDCLASSEXA *c) {
    if (c && HIWORD((uintptr_t)c->lpszClassName) && !strcmp(c->lpszClassName, "CIrrDeviceWin32")
        && (void *)c->lpfnWndProc == game_address(0x53a720)) {
        WNDCLASSEXA own = *c; res_game_proc = c->lpfnWndProc; own.lpfnWndProc = res_proc;
        return res_register(&own);
    }
    return res_register(c);
}

/* 0x53A620 computes position = cursor - window rectangle - border (0x53A37B: SM_CXDLGFRAME, SM_CYDLGFRAME +
 * SM_CYCAPTION for a captioned window, none for fullscreen); this cursor makes that the position in the game's frame. */
static void res_border(int *x, int *y) {
    *x = res_fullscreen ? 0 : GetSystemMetrics(SM_CXDLGFRAME);
    *y = res_fullscreen ? 0 : GetSystemMetrics(SM_CYDLGFRAME) + GetSystemMetrics(SM_CYCAPTION);
}
static BOOL WINAPI res_get_cursor_hook(LPPOINT p) {
    BOOL ok = res_get_cursor(p);
    Yo3Fit f; RECT frame; POINT origin = {0, 0}; int bx, by;
    if (!ok || !p || !res_mouse_fit(&f) || !GetWindowRect(res_window, &frame) || !ClientToScreen(res_window, &origin)) return ok;
    res_first(0, "GetCursorPos", __builtin_return_address(0));
    res_border(&bx, &by);
    p->x = frame.left + bx + yo3_unfit_x(&f, p->x - origin.x);
    p->y = frame.top + by + yo3_unfit_y(&f, p->y - origin.y);
    return ok;
}
static BOOL WINAPI res_set_cursor_hook(int x, int y) {
    Yo3Fit f; RECT frame; POINT origin = {0, 0}; int bx, by;
    if (res_mouse_fit(&f) && GetWindowRect(res_window, &frame) && ClientToScreen(res_window, &origin)) {
        res_first(1, "SetCursorPos", __builtin_return_address(0));
        res_border(&bx, &by);
        x = origin.x + yo3_fit_x(&f, x - frame.left - bx); y = origin.y + yo3_fit_y(&f, y - frame.top - by);
    }
    return res_set_cursor(x, y);
}

/* The IME's composition window is placed at the text cursor in the game's frame (0x53C300: CFS_POINT at x, y -
 * height); the candidate list likewise when Scaleform's IME support places it. Both in the real client area. */
static BOOL (WINAPI *res_ime_composition)(HIMC, LPCOMPOSITIONFORM);
static BOOL (WINAPI *res_ime_candidate)(HIMC, LPCANDIDATEFORM);
static void res_ime_point(const Yo3Fit *f, POINT *p) { p->x = yo3_fit_x(f, p->x); p->y = yo3_fit_y(f, p->y); }
static void res_ime_rect(const Yo3Fit *f, RECT *r) { long l = r->left, t = r->top, rt = r->right, b = r->bottom; yo3_fit_rect(f, &l, &t, &rt, &b); SetRect(r, l, t, rt, b); }
static BOOL WINAPI res_ime_composition_hook(HIMC context, LPCOMPOSITIONFORM form) {
    Yo3Fit f; COMPOSITIONFORM own;
    if (!form || !res_mouse_fit(&f)) return res_ime_composition(context, form);
    res_first(9, "ImmSetCompositionWindow", __builtin_return_address(0));
    own = *form; res_ime_point(&f, &own.ptCurrentPos);
    if (own.dwStyle & CFS_RECT) res_ime_rect(&f, &own.rcArea);
    return res_ime_composition(context, &own);
}
static BOOL WINAPI res_ime_candidate_hook(HIMC context, LPCANDIDATEFORM form) {
    Yo3Fit f; CANDIDATEFORM own;
    if (!form || !res_mouse_fit(&f)) return res_ime_candidate(context, form);
    res_first(10, "ImmSetCandidateWindow", __builtin_return_address(0));
    own = *form; res_ime_point(&f, &own.ptCurrentPos);
    if (own.dwStyle & CFS_EXCLUDE) res_ime_rect(&f, &own.rcArea);
    return res_ime_candidate(context, &own);
}

/* ---- Direct3D ---- */
static FARPROC (WINAPI *res_get_proc)(HMODULE, LPCSTR);
static IDirect3D9 *(WINAPI *res_create9)(UINT);
static HRESULT (WINAPI *res_create_device)(IDirect3D9 *, UINT, D3DDEVTYPE, HWND, DWORD, D3DPRESENT_PARAMETERS *, IDirect3DDevice9 **);
static HRESULT (WINAPI *res_reset)(IDirect3DDevice9 *, D3DPRESENT_PARAMETERS *);
static HRESULT (WINAPI *res_present)(IDirect3DDevice9 *, const RECT *, const RECT *, HWND, const RGNDATA *);
static HRESULT (WINAPI *res_set_target)(IDirect3DDevice9 *, DWORD, IDirect3DSurface9 *);
static HRESULT (WINAPI *res_clear)(IDirect3DDevice9 *, DWORD, const D3DRECT *, DWORD, D3DCOLOR, float, DWORD);
static HRESULT (WINAPI *res_set_viewport)(IDirect3DDevice9 *, const D3DVIEWPORT9 *);
static HRESULT (WINAPI *res_get_viewport)(IDirect3DDevice9 *, D3DVIEWPORT9 *);
static HRESULT (WINAPI *res_set_scissor)(IDirect3DDevice9 *, const RECT *);
static HRESULT (WINAPI *res_get_scissor)(IDirect3DDevice9 *, RECT *);
static HRESULT (WINAPI *res_stretch)(IDirect3DDevice9 *, IDirect3DSurface9 *, const RECT *, IDirect3DSurface9 *, const RECT *, D3DTEXTUREFILTERTYPE);

static int res_active(void) { return res_draw_ready && res_rt_back && !res_depth; }

/* The game's present parameters, for the real window: its client area, windowed. Returns whether they changed. */
static int res_parameters(D3DPRESENT_PARAMETERS *pp, HWND window) {
    if (!res_vw || (int)pp->BackBufferWidth != res_vw || (int)pp->BackBufferHeight != res_vh) return 0;
    RECT client; int w = 0, h = 0;
    if (window && GetClientRect(window, &client)) { w = client.right; h = client.bottom; }
    if (w < 1 || h < 1) res_client_size(GetWindowLongA(window, GWL_STYLE), &w, &h);
    pp->BackBufferWidth = (UINT)w; pp->BackBufferHeight = (UINT)h;
    pp->Windowed = TRUE; pp->FullScreen_RefreshRateInHz = 0;
    return 1;
}

/* After a device is made or reset: the back buffer's identity, and the full viewport in the game's frame. */
static void res_device_ready(IDirect3DDevice9 *device, const D3DPRESENT_PARAMETERS *pp) {
    IDirect3DSurface9 *back = NULL;
    if (SUCCEEDED(IDirect3DDevice9_GetBackBuffer(device, 0, 0, D3DBACKBUFFER_TYPE_MONO, &back)) && back) {
        res_back = back; IDirect3DSurface9_Release(back);
    }
    yo3_fit(&res_draw, res_vw, res_vh, (int)pp->BackBufferWidth, (int)pp->BackBufferHeight);
    res_draw_ready = 1; res_rt_back = 1;
    D3DVIEWPORT9 full = {0, 0, (DWORD)res_vw, (DWORD)res_vh, 0.0f, 1.0f}; res_viewport = full;
    SetRect(&res_scissor, 0, 0, res_vw, res_vh);
    D3DVIEWPORT9 real = {(DWORD)res_draw.ox, (DWORD)res_draw.oy, (DWORD)res_draw.cw, (DWORD)res_draw.ch, 0.0f, 1.0f};
    res_set_viewport(device, &real);
    RECT scissor = {res_draw.ox, res_draw.oy, res_draw.ox + res_draw.cw, res_draw.oy + res_draw.ch};
    res_set_scissor(device, &scissor);
}

static HRESULT WINAPI res_set_viewport_hook(IDirect3DDevice9 *device, const D3DVIEWPORT9 *v) {
    if (!v || !res_active()) return res_set_viewport(device, v);
    res_first(2, "SetViewport", __builtin_return_address(0));
    long l = (long)v->X, t = (long)v->Y, r = (long)(v->X + v->Width), b = (long)(v->Y + v->Height);
    yo3_fit_rect(&res_draw, &l, &t, &r, &b);
    if (r <= l) r = l + 1;
    if (b <= t) b = t + 1;
    D3DVIEWPORT9 real = {(DWORD)l, (DWORD)t, (DWORD)(r - l), (DWORD)(b - t), v->MinZ, v->MaxZ};
    HRESULT hr = res_set_viewport(device, &real);
    if (SUCCEEDED(hr)) res_viewport = *v;
    return hr;
}
static HRESULT WINAPI res_get_viewport_hook(IDirect3DDevice9 *device, D3DVIEWPORT9 *v) {
    if (!v || !res_active()) return res_get_viewport(device, v);
    *v = res_viewport; return D3D_OK;
}
static HRESULT WINAPI res_set_scissor_hook(IDirect3DDevice9 *device, const RECT *s) {
    if (!s || !res_active()) return res_set_scissor(device, s);
    res_first(3, "SetScissorRect", __builtin_return_address(0));
    long l = s->left, t = s->top, r = s->right, b = s->bottom; yo3_fit_rect(&res_draw, &l, &t, &r, &b);
    RECT real = {l, t, r, b};
    HRESULT hr = res_set_scissor(device, &real);
    if (SUCCEEDED(hr)) res_scissor = *s;
    return hr;
}
static HRESULT WINAPI res_get_scissor_hook(IDirect3DDevice9 *device, RECT *s) {
    if (!s || !res_active()) return res_get_scissor(device, s);
    *s = res_scissor; return D3D_OK;
}
/* Setting a render target resets the viewport and scissor rectangle to the whole target: for the back buffer that
 * is the game's whole frame, so the content area. */
static HRESULT WINAPI res_set_target_hook(IDirect3DDevice9 *device, DWORD index, IDirect3DSurface9 *surface) {
    if (!res_draw_ready || res_depth || index) return res_set_target(device, index, surface);
    ++res_depth; HRESULT hr = res_set_target(device, index, surface); --res_depth;
    if (FAILED(hr)) return hr;
    res_rt_back = surface == res_back;
    if (!res_rt_back) { res_first(4, "SetRenderTarget (a texture)", __builtin_return_address(0)); return hr; }
    D3DVIEWPORT9 full = {0, 0, (DWORD)res_vw, (DWORD)res_vh, 0.0f, 1.0f};
    res_set_viewport_hook(device, &full);
    RECT all = {0, 0, res_vw, res_vh}; res_set_scissor_hook(device, &all);
    return hr;
}
static HRESULT WINAPI res_clear_hook(IDirect3DDevice9 *device, DWORD count, const D3DRECT *rects, DWORD flags, D3DCOLOR color, float z, DWORD stencil) {
    if (!count || !rects || !res_active()) return res_clear(device, count, rects, flags, color, z, stencil);
    res_first(5, "Clear with rectangles", __builtin_return_address(0));
    D3DRECT real[16]; DWORD done = 0; HRESULT hr = D3D_OK;
    while (done < count && SUCCEEDED(hr)) {
        DWORD n = count - done > 16 ? 16 : count - done;
        for (DWORD i = 0; i < n; ++i) {
            long l = rects[done + i].x1, t = rects[done + i].y1, r = rects[done + i].x2, b = rects[done + i].y2;
            yo3_fit_rect(&res_draw, &l, &t, &r, &b); real[i].x1 = l; real[i].y1 = t; real[i].x2 = r; real[i].y2 = b;
        }
        hr = res_clear(device, n, real, flags, color, z, stencil); done += n;
    }
    return hr;
}
static HRESULT WINAPI res_stretch_hook(IDirect3DDevice9 *device, IDirect3DSurface9 *from, const RECT *from_rect,
    IDirect3DSurface9 *to, const RECT *to_rect, D3DTEXTUREFILTERTYPE filter) {
    if (!res_draw_ready || res_depth || (from != res_back && to != res_back)) return res_stretch(device, from, from_rect, to, to_rect, filter);
    res_first(6, "StretchRect with the back buffer", __builtin_return_address(0));
    /* The back buffer's part is the game's frame: given in it, or all of it when no rectangle is given. */
    RECT a, b; long l, t, r, bt;
    if (from == res_back) {
        l = from_rect ? from_rect->left : 0; t = from_rect ? from_rect->top : 0;
        r = from_rect ? from_rect->right : res_vw; bt = from_rect ? from_rect->bottom : res_vh;
        yo3_fit_rect(&res_draw, &l, &t, &r, &bt); SetRect(&a, l, t, r, bt); from_rect = &a;
    }
    if (to == res_back) {
        l = to_rect ? to_rect->left : 0; t = to_rect ? to_rect->top : 0;
        r = to_rect ? to_rect->right : res_vw; bt = to_rect ? to_rect->bottom : res_vh;
        yo3_fit_rect(&res_draw, &l, &t, &r, &bt); SetRect(&b, l, t, r, bt); to_rect = &b;
    }
    return res_stretch(device, from, from_rect, to, to_rect, filter);
}
/* The bars outside the content area are never drawn by the game: clear them before each frame is shown. */
static HRESULT WINAPI res_present_hook(IDirect3DDevice9 *device, const RECT *from, const RECT *to, HWND window, const RGNDATA *dirty) {
    if (res_draw_ready && !res_depth) {
        res_first(7, "Present", __builtin_return_address(0));
        if (from || to) res_first(8, "Present with rectangles", __builtin_return_address(0));
        if (res_rt_back && (res_draw.cw < res_draw.rw || res_draw.ch < res_draw.rh)) {
            D3DVIEWPORT9 saved, full = {0, 0, (DWORD)res_draw.rw, (DWORD)res_draw.rh, 0.0f, 1.0f};
            DWORD scissor = 0;
            res_get_viewport(device, &saved); res_set_viewport(device, &full);
            IDirect3DDevice9_GetRenderState(device, D3DRS_SCISSORTESTENABLE, &scissor);
            if (scissor) IDirect3DDevice9_SetRenderState(device, D3DRS_SCISSORTESTENABLE, FALSE);
            D3DRECT bars[2];
            if (res_draw.cw < res_draw.rw) {
                bars[0] = (D3DRECT){0, 0, res_draw.ox, res_draw.rh};
                bars[1] = (D3DRECT){res_draw.ox + res_draw.cw, 0, res_draw.rw, res_draw.rh};
            } else {
                bars[0] = (D3DRECT){0, 0, res_draw.rw, res_draw.oy};
                bars[1] = (D3DRECT){0, res_draw.oy + res_draw.ch, res_draw.rw, res_draw.rh};
            }
            res_clear(device, 2, bars, D3DCLEAR_TARGET, D3DCOLOR_XRGB(0, 0, 0), 1.0f, 0);
            if (scissor) IDirect3DDevice9_SetRenderState(device, D3DRS_SCISSORTESTENABLE, TRUE);
            res_set_viewport(device, &saved);
        }
        RECT a; long l, t, r, b;
        if (from) { l = from->left; t = from->top; r = from->right; b = from->bottom; yo3_fit_rect(&res_draw, &l, &t, &r, &b); SetRect(&a, l, t, r, b); from = &a; }
        to = NULL; /* the window's whole client area: the back buffer is its size */
    }
    return res_present(device, from, to, window, dirty);
}
static HRESULT WINAPI res_reset_hook(IDirect3DDevice9 *device, D3DPRESENT_PARAMETERS *pp) {
    if (!pp || !res_vw) return res_reset(device, pp);
    D3DPRESENT_PARAMETERS own = *pp; int scaled = res_parameters(&own, own.hDeviceWindow ? own.hDeviceWindow : res_window);
    ++res_depth; HRESULT hr = res_reset(device, &own); --res_depth;
    if (scaled) {
        UINT w = pp->BackBufferWidth, h = pp->BackBufferHeight, refresh = pp->FullScreen_RefreshRateInHz; BOOL windowed = pp->Windowed;
        *pp = own; pp->BackBufferWidth = w; pp->BackBufferHeight = h; pp->Windowed = windowed; pp->FullScreen_RefreshRateInHz = refresh;
        if (SUCCEEDED(hr)) res_device_ready(device, &own);
    }
    return hr;
}

static int res_patch(void **slot, void *hook, void **original) {
    DWORD previous;
    if (!VirtualProtect(slot, sizeof *slot, PAGE_READWRITE, &previous)) return 0;
    if (*slot != hook) { *original = *slot; *slot = hook; }
    VirtualProtect(slot, sizeof *slot, previous, &previous);
    return 1;
}
static HRESULT WINAPI res_create_device_hook(IDirect3D9 *d3d, UINT adapter, D3DDEVTYPE type, HWND focus, DWORD flags,
    D3DPRESENT_PARAMETERS *pp, IDirect3DDevice9 **out) {
    if (!pp || !out || !res_vw) return res_create_device(d3d, adapter, type, focus, flags, pp, out);
    HWND window = pp->hDeviceWindow ? pp->hDeviceWindow : focus;
    D3DPRESENT_PARAMETERS own = *pp;
    if (!res_parameters(&own, window)) return res_create_device(d3d, adapter, type, focus, flags, pp, out);
    HRESULT hr = res_create_device(d3d, adapter, type, focus, flags, &own, out);
    res_log("[YO3 MOD] resolution: device %dx%d (game %ux%u%s): 0x%08lx", (int)own.BackBufferWidth, (int)own.BackBufferHeight,
        pp->BackBufferWidth, pp->BackBufferHeight, pp->Windowed ? "" : ", fullscreen", (unsigned long)hr);
    if (FAILED(hr) || !*out) return hr;
    IDirect3DDevice9Vtbl *v = (*out)->lpVtbl;
    if (!(res_patch((void **)&v->Reset, (void *)res_reset_hook, (void **)&res_reset)
        && res_patch((void **)&v->Present, (void *)res_present_hook, (void **)&res_present)
        && res_patch((void **)&v->SetRenderTarget, (void *)res_set_target_hook, (void **)&res_set_target)
        && res_patch((void **)&v->Clear, (void *)res_clear_hook, (void **)&res_clear)
        && res_patch((void **)&v->SetViewport, (void *)res_set_viewport_hook, (void **)&res_set_viewport)
        && res_patch((void **)&v->GetViewport, (void *)res_get_viewport_hook, (void **)&res_get_viewport)
        && res_patch((void **)&v->SetScissorRect, (void *)res_set_scissor_hook, (void **)&res_set_scissor)
        && res_patch((void **)&v->GetScissorRect, (void *)res_get_scissor_hook, (void **)&res_get_scissor)
        && res_patch((void **)&v->StretchRect, (void *)res_stretch_hook, (void **)&res_stretch))) {
        /* A device the game would draw into unscaled: not a game anyone should be left with. */
        IDirect3DDevice9_Release(*out); *out = NULL;
        res_log("[YO3 MOD] resolution: stage=mod-resolution: the Direct3D device could not be wrapped");
        return D3DERR_NOTAVAILABLE;
    }
    UINT w = pp->BackBufferWidth, h = pp->BackBufferHeight, refresh = pp->FullScreen_RefreshRateInHz; BOOL windowed = pp->Windowed;
    *pp = own; pp->BackBufferWidth = w; pp->BackBufferHeight = h; pp->Windowed = windowed; pp->FullScreen_RefreshRateInHz = refresh;
    res_device_ready(*out, &own);
    return hr;
}
static IDirect3D9 *WINAPI res_create9_hook(UINT sdk) {
    IDirect3D9 *d3d = res_create9(sdk);
    if (d3d && !res_patch((void **)&d3d->lpVtbl->CreateDevice, (void *)res_create_device_hook, (void **)&res_create_device)) {
        IDirect3D9_Release(d3d); d3d = NULL;
        res_log("[YO3 MOD] resolution: stage=mod-resolution: Direct3D could not be wrapped");
    }
    return d3d;
}
static FARPROC WINAPI res_get_proc_hook(HMODULE module, LPCSTR name) {
    FARPROC found = res_get_proc(module, name);
    if (found && HIWORD((uintptr_t)name) && !strcmp(name, "Direct3DCreate9")) {
        res_create9 = (void *)found; return (FARPROC)(void *)res_create9_hook;
    }
    return found;
}

/* ---- the guards ---- */
/* `n` bytes at a game address: `expected`, with each 4-byte slot listed in `slots` holding that preferred address
 * relocated to where the game is loaded. */
static int res_code(uintptr_t at, const BYTE *expected, size_t n, const unsigned *slots, unsigned count) {
    BYTE want[32]; if (n > sizeof want) return 0;
    memcpy(want, expected, n);
    for (unsigned i = 0; i < count; ++i) {
        uint32_t preferred; memcpy(&preferred, want + slots[i], 4);
        uint32_t real = (uint32_t)(uintptr_t)game_address(preferred); memcpy(want + slots[i], &real, 4);
    }
    return !memcmp(game_address(at), want, n);
}

static const char *resolution_install(void) {
    char text[32]; DWORD n = GetEnvironmentVariableA("YO3_RESOLUTION", text, sizeof text);
    int on = n >= sizeof text ? -1 : yo3_resolution_parse(n ? text : NULL, &res_req_w, &res_req_h);
    if (!on) return NULL;
    if (on < 0) return "stage=mod-resolution: YO3_RESOLUTION must be WIDTHxHEIGHT (320 to 8192 each), auto or off.\r\n";
    static const char unsupported[] = "stage=mod-resolution: This game's drawing code is unsupported, so the higher "
        "resolution can't be used. Turn it off, or validate the original v1.153 executable.\r\n";
    BYTE *base = (BYTE *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    if (nt->OptionalHeader.SizeOfImage < 0x42e000) return unsupported; /* the contract fixtures: no game to scale */
    /* 0x5441FF: push "Direct3DCreate9"; push eax; call [GetProcAddress]. */
    static const BYTE d3d[] = {0x68,0x40,0x61,0x7b,0x00, 0x50, 0xff,0x15,0xb4,0x81,0x75,0x00};
    static const unsigned d3d_slots[] = {1, 8};
    /* 0x53B05A: the class's window procedure, mov [esp+0x50],0x53A720; 0x53B151: push "CIrrDeviceWin32". */
    static const BYTE proc[] = {0xc7,0x44,0x24,0x50,0x20,0xa7,0x53,0x00}, name[] = {0x68,0xcc,0x4d,0x7b,0x00};
    static const unsigned proc_slots[] = {4}, name_slots[] = {1};
    /* 0x53A720: the window procedure's prologue (push ebp; mov ebp,esp; and esp,-8; sub esp,0x164; push ebx, esi,
     * edi; call [GetCapture]). */
    static const BYTE prologue[] = {0x55,0x8b,0xec,0x83,0xe4,0xf8,0x81,0xec,0x64,0x01,0x00,0x00,0x53,0x56,0x57,0xff,0x15,0xb4,0x86,0x75,0x00};
    static const unsigned prologue_slots[] = {17};
    /* 0x53B0D1: style WS_POPUP when fullscreen, else 0x06C80000; 0x53B0F1: call [AdjustWindowRect]. */
    static const BYTE style[] = {0xc7,0x44,0x24,0x10,0x00,0x00,0x00,0x80, 0x38,0x5f,0x72, 0x75,0x08, 0xc7,0x44,0x24,0x10,0x00,0x00,0xc8,0x06};
    static const BYTE adjust[] = {0xff,0x15,0x14,0x87,0x75,0x00};
    static const unsigned adjust_slots[] = {2};
    /* 0x53A620: sub esp,0x18; lea eax,[esp]; push eax; call [GetCursorPos]; cmp byte [esi+0x30],0 (no reference
     * rectangle); 0x53A65D: x - [esi+0x28] - rect.left, y - [esi+0x2C] - rect.top. */
    static const BYTE cursor[] = {0x83,0xec,0x18,0x8d,0x04,0x24,0x50,0xff,0x15,0xe8,0x86,0x75,0x00,0x80,0x7e,0x30,0x00};
    static const unsigned cursor_slots[] = {9};
    static const BYTE position[] = {0x8b,0x14,0x24,0x2b,0x56,0x28,0x8b,0x44,0x24,0x04,0x2b,0x46,0x2c,0x2b,0x54,0x24,0x08,0x2b,0x44,0x24,0x0c};
    /* 0x53A37B: [esi+0x28] = GetSystemMetrics(7); then GetSystemMetrics(8) + GetSystemMetrics(4) for +0x2C. */
    static const BYTE border[] = {0x8b,0x3d,0x40,0x87,0x75,0x00,0x53,0x6a,0x07,0xff,0xd7,0x6a,0x08,0x89,0x46,0x28,0xff,0xd7,0x6a,0x04,0x8b,0xd8,0xff,0xd7};
    static const unsigned border_slots[] = {2};
    if (!res_code(0x5441ff, d3d, sizeof d3d, d3d_slots, 2) || memcmp(game_address(0x7b6140), "Direct3DCreate9", 16)
        || !res_code(0x53b05a, proc, sizeof proc, proc_slots, 1) || !res_code(0x53b151, name, sizeof name, name_slots, 1)
        || memcmp(game_address(0x7b4dcc), "CIrrDeviceWin32", 16)
        || !res_code(0x53a720, prologue, sizeof prologue, prologue_slots, 1)
        || !res_code(0x53b0d1, style, sizeof style, NULL, 0) || !res_code(0x53b0f1, adjust, sizeof adjust, adjust_slots, 1)
        || !res_code(0x53a620, cursor, sizeof cursor, cursor_slots, 1) || !res_code(0x53a65d, position, sizeof position, NULL, 0)
        || !res_code(0x53a37b, border, sizeof border, border_slots, 1))
        return unsupported;
    if (!(hook("GetProcAddress", (void *)res_get_proc_hook, (void **)&res_get_proc)
        && hook("RegisterClassExA", (void *)res_register_hook, (void **)&res_register)
        && hook("AdjustWindowRect", (void *)res_adjust_hook, (void **)&res_adjust)
        && hook("GetCursorPos", (void *)res_get_cursor_hook, (void **)&res_get_cursor)
        && hook("SetCursorPos", (void *)res_set_cursor_hook, (void **)&res_set_cursor)
        && hook("ImmSetCompositionWindow", (void *)res_ime_composition_hook, (void **)&res_ime_composition)
        && hook("ImmSetCandidateWindow", (void *)res_ime_candidate_hook, (void **)&res_ime_candidate)))
        return unsupported;
    res_log("[YO3 MOD] resolution: %s requested; window, Direct3D and mouse hooks installed", text);
    return NULL;
}
#endif
