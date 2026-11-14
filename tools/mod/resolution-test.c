/* The higher-resolution geometry (resolution-fit.h), without the game or a display. */
#include <assert.h>
#include <stdio.h>
#include "resolution-fit.h"

int main(void) {
    int w, h;
    assert(yo3_resolution_parse(NULL, &w, &h) == 0 && yo3_resolution_parse("", &w, &h) == 0);
    assert(yo3_resolution_parse("0", &w, &h) == 0 && yo3_resolution_parse("Off", &w, &h) == 0);
    assert(yo3_resolution_parse("original", &w, &h) == 0);
    assert(yo3_resolution_parse("auto", &w, &h) == 1 && w == 0 && h == 0);
    assert(yo3_resolution_parse("1440x1080", &w, &h) == 1 && w == 1440 && h == 1080);
    assert(yo3_resolution_parse("1920X1080", &w, &h) == 1 && w == 1920 && h == 1080);
    const char *bad[] = {"1440", "1440x", "x1080", "1440x1080x", "-1440x1080", "1440x+1080", " 1440x1080", "100x100",
        "9000x1080", "1440*1080", "banana", "1440x1080 "};
    for (unsigned i = 0; i < sizeof bad / sizeof *bad; ++i) assert(yo3_resolution_parse(bad[i], &w, &h) == -1);

    /* 1440x1080: an exact 4:3 multiple (1.40625), no bars. */
    Yo3Fit f; yo3_fit(&f, 1024, 768, 1440, 1080);
    assert(f.cw == 1440 && f.ch == 1080 && f.ox == 0 && f.oy == 0);
    assert(yo3_fit_x(&f, 0) == 0 && yo3_fit_x(&f, 1024) == 1440 && yo3_fit_y(&f, 768) == 1080);
    assert(yo3_fit_x(&f, 512) == 720 && yo3_fit_y(&f, 384) == 540);
    /* Every real pixel maps back inside the frame, and every virtual pixel's real centre maps back to itself. */
    for (int x = 0; x < 1440; ++x) { int v = yo3_unfit_x(&f, x); assert(v >= 0 && v < 1024); }
    for (int v = 0; v < 1024; ++v) {
        int left = yo3_fit_x(&f, v), right = yo3_fit_x(&f, v + 1);
        assert(right > left && yo3_unfit_x(&f, (left + right - 1) / 2) == v);
    }

    /* 1920x1080: 1440x1080 content centred, 240-pixel bars left and right. */
    yo3_fit(&f, 1024, 768, 1920, 1080);
    assert(f.cw == 1440 && f.ch == 1080 && f.ox == 240 && f.oy == 0);
    assert(yo3_fit_x(&f, 0) == 240 && yo3_fit_x(&f, 1024) == 1680);
    assert(yo3_unfit_x(&f, 0) < 0 && yo3_unfit_x(&f, 239) < 0 && yo3_unfit_x(&f, 240) == 0);
    assert(yo3_unfit_x(&f, 1679) == 1023 && yo3_unfit_x(&f, 1680) >= 1024 && yo3_unfit_x(&f, 1919) >= 1024);

    /* Taller than 4:3 (1280x1024): bars above and below. */
    yo3_fit(&f, 1024, 768, 1280, 1024);
    assert(f.cw == 1280 && f.ch == 960 && f.ox == 0 && f.oy == 32);
    assert(yo3_fit_y(&f, 0) == 32 && yo3_fit_y(&f, 768) == 992 && yo3_unfit_y(&f, 31) < 0);

    /* The game's own 1024x600 mode. */
    yo3_fit(&f, 1024, 600, 1920, 1080);
    assert(f.ch == 1080 && f.cw == 1843 && f.ox == 38);
    /* Identity: the original size changes nothing. */
    yo3_fit(&f, 1024, 768, 1024, 768);
    for (int v = -5; v < 1030; ++v) assert(yo3_fit_x(&f, v) == v && yo3_unfit_x(&f, v) == v && yo3_fit_y(&f, v) == v);

    /* A rectangle's edges map one by one: a viewport and the one beside it still meet. */
    yo3_fit(&f, 1024, 768, 1600, 1200);
    long l = 0, t = 0, r = 341, b = 768, l2 = 341, t2 = 0, r2 = 1024, b2 = 768;
    yo3_fit_rect(&f, &l, &t, &r, &b); yo3_fit_rect(&f, &l2, &t2, &r2, &b2);
    assert(l == 0 && r == l2 && r2 == 1600 && b == 1200);

    /* Edges read back from Direct3D map to the edges that were set, at every size at least the frame's. */
    const int sizes[][2] = {{1024, 768}, {1440, 1080}, {1440, 1060}, {1920, 1080}, {1600, 900}, {1397, 1048}, {2560, 1440}, {7680, 4320}};
    for (unsigned i = 0; i < sizeof sizes / sizeof *sizes; ++i) {
        yo3_fit(&f, 1024, 768, sizes[i][0], sizes[i][1]);
        for (int v = 0; v <= 1024; ++v) assert(yo3_unfit_edge_x(&f, yo3_fit_x(&f, v)) == v);
        for (int v = 0; v <= 768; ++v) assert(yo3_unfit_edge_y(&f, yo3_fit_y(&f, v)) == v);
    }

    /* "auto": the largest 4:3 frame in the space available. */
    yo3_fit_largest(1024, 768, 1920, 1040, &w, &h); assert(w == 1386 && h == 1040);
    yo3_fit_largest(1024, 768, 1280, 1024, &w, &h); assert(w == 1280 && h == 960);
    puts("PASS: higher-resolution parsing, fitting and mapping");
    return 0;
}
