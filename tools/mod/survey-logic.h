#ifndef YO3_SURVEY_LOGIC_H
#define YO3_SURVEY_LOGIC_H
/* What else runs inside the game (#68), without the game or Windows: the survey's decisions and its two outputs (log
 * lines for the game log and the report, a JSON summary for the launcher). survey.h gathers the facts; this file only
 * judges and writes them, so survey-test.c can check every rule.
 *   - SHA-256 of a module's file, written in groups of 16 so the log's credential filter (which hides any 20 or more
 *     letters and digits in a row) keeps it, as the executable's fingerprint is;
 *   - whether a path is inside a folder (Windows', the game's), case-blind;
 *   - where a function's first bytes jump to (a detour), and whether a Winsock provider is Microsoft's;
 *   - the report: modules from outside Windows, hooked network functions, Winsock providers.
 * Paths here are UTF-8. Nothing is changed on the player's PC; this only reads and reports. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <stdarg.h>

/* ---- SHA-256 (FIPS 180-4) ---- */
typedef struct { uint32_t h[8]; uint64_t bytes; unsigned char block[64]; unsigned used; } Yo3Sha256;
static const uint32_t yo3_sha_k[64] = {
    0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,
    0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
    0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,
    0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
    0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,
    0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2 };
#define YO3_ROR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
static void yo3_sha256_block(Yo3Sha256 *s, const unsigned char *p) {
    uint32_t w[64], a, b, c, d, e, f, g, h;
    for (int i = 0; i < 16; ++i) w[i] = (uint32_t)p[4*i] << 24 | (uint32_t)p[4*i+1] << 16 | (uint32_t)p[4*i+2] << 8 | p[4*i+3];
    for (int i = 16; i < 64; ++i) {
        uint32_t s0 = YO3_ROR(w[i-15], 7) ^ YO3_ROR(w[i-15], 18) ^ (w[i-15] >> 3), s1 = YO3_ROR(w[i-2], 17) ^ YO3_ROR(w[i-2], 19) ^ (w[i-2] >> 10);
        w[i] = w[i-16] + s0 + w[i-7] + s1;
    }
    a = s->h[0]; b = s->h[1]; c = s->h[2]; d = s->h[3]; e = s->h[4]; f = s->h[5]; g = s->h[6]; h = s->h[7];
    for (int i = 0; i < 64; ++i) {
        uint32_t t1 = h + (YO3_ROR(e, 6) ^ YO3_ROR(e, 11) ^ YO3_ROR(e, 25)) + ((e & f) ^ (~e & g)) + yo3_sha_k[i] + w[i];
        uint32_t t2 = (YO3_ROR(a, 2) ^ YO3_ROR(a, 13) ^ YO3_ROR(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
        h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    s->h[0] += a; s->h[1] += b; s->h[2] += c; s->h[3] += d; s->h[4] += e; s->h[5] += f; s->h[6] += g; s->h[7] += h;
}
static void yo3_sha256_init(Yo3Sha256 *s) {
    static const uint32_t start[8] = { 0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19 };
    memcpy(s->h, start, sizeof start); s->bytes = 0; s->used = 0;
}
static void yo3_sha256_update(Yo3Sha256 *s, const void *data, size_t n) {
    const unsigned char *p = data;
    s->bytes += n;
    while (n) {
        unsigned take = 64 - s->used; if (take > n) take = (unsigned)n;
        memcpy(s->block + s->used, p, take); s->used += take; p += take; n -= take;
        if (s->used == 64) { yo3_sha256_block(s, s->block); s->used = 0; }
    }
}
/* The digest as 64 lowercase hex digits (and a NUL). */
static void yo3_sha256_hex(Yo3Sha256 *s, char out[65]) {
    uint64_t bits = s->bytes * 8;
    unsigned char pad = 0x80, zero = 0, length[8];
    yo3_sha256_update(s, &pad, 1);
    while (s->used != 56) yo3_sha256_update(s, &zero, 1);
    for (int i = 0; i < 8; ++i) length[i] = (unsigned char)(bits >> (56 - 8 * i));
    yo3_sha256_update(s, length, 8);
    for (int i = 0; i < 8; ++i) snprintf(out + 8 * i, 9, "%08x", (unsigned)s->h[i]);
}
/* "aaaa…:bbbb…:cccc…:dddd…" (16 digits a group) from the 64 digits: the log's filter keeps a fingerprint in this form. */
static void yo3_sha256_grouped(const char hex[65], char out[68]) {
    size_t n = 0;
    for (int g = 0; g < 4; ++g) { if (g) out[n++] = ':'; memcpy(out + n, hex + 16 * g, 16); n += 16; }
    out[n] = 0;
}

/* ---- paths ---- */
static int yo3_lower(int c) { return c >= 'A' && c <= 'Z' ? c + 32 : c; }
static int yo3_same_char(int a, int b) { a = yo3_lower(a); b = yo3_lower(b); if (a == '/') a = '\\'; if (b == '/') b = '\\'; return a == b; }
/* Whether `path` is a file or folder inside `dir` (a folder path, with or without its last separator). Case-blind,
 * / and \ alike. A look-alike folder (C:\Windows2 against C:\Windows) is not inside. */
static int yo3_path_under(const char *path, const char *dir) {
    size_t n = dir ? strlen(dir) : 0;
    if (!path || !n) return 0;
    while (n && (dir[n-1] == '\\' || dir[n-1] == '/')) --n;
    if (!n) return 0;
    for (size_t i = 0; i < n; ++i) if (!path[i] || !yo3_same_char(path[i], dir[i])) return 0;
    return path[n] == '\\' || path[n] == '/';
}
static const char *yo3_path_base(const char *path) {
    const char *base = path;
    for (const char *p = path; *p; ++p) if (*p == '\\' || *p == '/') base = p + 1;
    return base;
}
static int yo3_name_is(const char *name, const char *expected) {
    for (; *name && *expected; ++name, ++expected) if (yo3_lower(*name) != yo3_lower(*expected)) return 0;
    return !*name && !*expected;
}
static int yo3_has_prefix(const char *text, const char *prefix) {
    for (; *prefix; ++text, ++prefix) if (!*text || yo3_lower(*text) != yo3_lower(*prefix)) return 0;
    return 1;
}

/* ---- hooks: where a function's first bytes lead ---- */
typedef int (*Yo3ReadMemory)(uintptr_t address, void *out, size_t n);
/* The place `at`'s code jumps to at once, when its first bytes are a jump: E9 rel32, EB rel8, FF 25 [abs32] (32-bit), the
 * two byte hot patch's 8B FF before them, push imm32 + ret, and mov eax, imm32 + jmp eax. 0 when they are not (a
 * function's ordinary start), leaving `*destination` untouched. `code` holds up to 12 bytes read at `at`. */
static int yo3_jump_target(const unsigned char *code, uintptr_t at, Yo3ReadMemory read, uintptr_t *destination) {
    if (code[0] == 0x8b && code[1] == 0xff) { code += 2; at += 2; } /* mov edi, edi: the hot patch's room */
    if (code[0] == 0xe9) { int32_t rel; memcpy(&rel, code + 1, 4); *destination = at + 5 + (intptr_t)rel; return 1; }
    if (code[0] == 0xeb) { *destination = at + 2 + (intptr_t)(signed char)code[1]; return 1; }
    if (code[0] == 0xff && code[1] == 0x25) {
        uint32_t slot, value; memcpy(&slot, code + 2, 4);
        if (!read || !read(slot, &value, 4)) return 0;
        *destination = value; return 1;
    }
    if (code[0] == 0x68 && code[5] == 0xc3) { uint32_t value; memcpy(&value, code + 1, 4); *destination = value; return 1; }
    if (code[0] == 0xb8 && code[5] == 0xff && code[6] == 0xe0) { uint32_t value; memcpy(&value, code + 1, 4); *destination = value; return 1; }
    return 0;
}

/* The WSOCK32 functions by ordinal: the game imports them that way. The ones the connection DLL hooks are 3, 4 and 19. */
static const char *yo3_wsock32_name(unsigned ordinal) {
    static const char *const names[] = { NULL, "accept", "bind", "closesocket", "connect", "getpeername", "getsockname", "getsockopt",
        "htonl", "htons", "inet_addr", "inet_ntoa", "ioctlsocket", "listen", "ntohl", "ntohs", "recv", "recvfrom", "select", "send",
        "sendto", "setsockopt", "shutdown", "socket" };
    return ordinal < sizeof names / sizeof *names ? names[ordinal] : NULL;
}
/* Whether the survey looks at an import: the network's functions, and the three the DLL redirects besides connect, send and
 * closesocket (the game's INI reads and its patch check). `dll` is the import's DLL, `function` its name. */
static int yo3_import_matters(const char *dll, const char *function) {
    if (!dll || !function) return 0;
    if (yo3_name_is(dll, "WSOCK32.dll") || yo3_name_is(dll, "WS2_32.dll")) return 1;
    if (yo3_name_is(dll, "WININET.dll")) return yo3_name_is(function, "InternetOpenUrlA");
    if (yo3_name_is(dll, "KERNEL32.dll")) return yo3_name_is(function, "GetPrivateProfileStringA") || yo3_name_is(function, "GetPrivateProfileIntA");
    return 0;
}

/* ---- Winsock providers ---- */
/* Whether a Winsock provider is Microsoft's: its DLL (the catalog's path) is one of Windows' own, else, with no path
 * known, its name is one of Windows' own. The game's 32-bit Winsock catalog is the one that matters (#65). */
static int yo3_provider_microsoft(const char *name, const char *path) {
    if (path && *path) {
        static const char *const own[] = { "mswsock.dll", "rsvpsp.dll", "wshbth.dll", "wshtcpip.dll", "wship6.dll", "napinsp.dll", "pnrpnsp.dll", "winrnr.dll", "hvsocket.dll" };
        const char *base = yo3_path_base(path);
        for (unsigned i = 0; i < sizeof own / sizeof *own; ++i) if (yo3_name_is(base, own[i])) return 1;
        return 0;
    }
    return name && (yo3_has_prefix(name, "MSAFD") || yo3_has_prefix(name, "RSVP") || yo3_has_prefix(name, "Hyper-V") || yo3_has_prefix(name, "Tcpip")
        || yo3_has_prefix(name, "NLA") || yo3_has_prefix(name, "Bluetooth"));
}

/* A product's name for a DLL some security programs inject into every program (their hooks are the cause of #65-like
 * disconnects); NULL for any other. Not a verdict, only a name for the player. */
static const char *yo3_security_product(const char *module) {
    static const struct { const char *prefix, *product; } known[] = {
        { "aswhook", "Avast" }, { "snxhk", "Avast" }, { "avghook", "AVG" }, { "atcuf", "Bitdefender" }, { "bdhkm", "Bitdefender" },
        { "klhk", "Kaspersky" }, { "mbae", "Malwarebytes" }, { "eplghooks", "ESET" }, { "mfehidk", "McAfee" }, { "wrusr", "Webroot" },
        { "sbiedll", "Sandboxie" } };
    if (!module) return NULL;
    for (unsigned i = 0; i < sizeof known / sizeof *known; ++i) if (yo3_has_prefix(module, known[i].prefix)) return known[i].product;
    return NULL;
}

/* ---- the report ---- */
#define YO3_SURVEY_PATH 600
typedef struct { char function[24], how[8], module[64], path[YO3_SURVEY_PATH], product[24]; } Yo3Hook;
typedef struct { char name[64], path[YO3_SURVEY_PATH], sha256[65], product[24]; unsigned long size; int game_folder, loaded; } Yo3Module;
typedef struct { char name[96], path[YO3_SURVEY_PATH]; int microsoft; } Yo3Provider;
typedef struct {
    Yo3Hook hooks[24]; int hooks_n;
    Yo3Module modules[48]; int modules_n;
    Yo3Provider providers[32]; int providers_n;
    int providers_known;
} Yo3Survey;

/* Copies `text` into the fixed field `field`, cut to fit. */
#define YO3_SET(field, text) do { snprintf((field), sizeof (field), "%s", (text) ? (text) : ""); } while (0)

/* An output buffer that never overruns: what doesn't fit is dropped and `full` says so. */
typedef struct { char *text; size_t size, used; int full; } Yo3Text;
static void yo3_text_add(Yo3Text *out, const char *format, ...) {
    va_list args; va_start(args, format);
    if (out->used + 1 >= out->size) out->full = 1;
    else {
        int n = vsnprintf(out->text + out->used, out->size - out->used, format, args);
        if (n < 0 || (size_t)n >= out->size - out->used) { out->full = 1; out->text[out->used] = 0; } else out->used += (size_t)n;
    }
    va_end(args);
}
static void yo3_text_json(Yo3Text *out, const char *text) {
    yo3_text_add(out, "\"");
    for (; *text; ++text) {
        unsigned char c = (unsigned char)*text;
        if (c == '"' || c == '\\') yo3_text_add(out, "\\%c", c);
        else if (c < 32) yo3_text_add(out, "\\u%04x", c);
        else yo3_text_add(out, "%c", c);
    }
    yo3_text_add(out, "\"");
}
/* The summary the launcher reads: {"version":1,"hooked":[…],"modules":[…],"providers":[…],"providers_known":bool}.
 * Returns the length, or 0 when it didn't fit. */
static size_t yo3_survey_json(const Yo3Survey *s, char *buffer, size_t size) {
    Yo3Text out = { buffer, size, 0, 0 };
    if (size) buffer[0] = 0;
    yo3_text_add(&out, "{\"version\":1,\"hooked\":[");
    for (int i = 0; i < s->hooks_n; ++i) {
        const Yo3Hook *h = &s->hooks[i];
        yo3_text_add(&out, "%s{\"function\":", i ? "," : ""); yo3_text_json(&out, h->function);
        yo3_text_add(&out, ",\"how\":"); yo3_text_json(&out, h->how);
        yo3_text_add(&out, ",\"module\":"); yo3_text_json(&out, h->module);
        yo3_text_add(&out, ",\"path\":"); yo3_text_json(&out, h->path);
        yo3_text_add(&out, ",\"product\":"); yo3_text_json(&out, h->product); yo3_text_add(&out, "}");
    }
    yo3_text_add(&out, "],\"modules\":[");
    for (int i = 0; i < s->modules_n; ++i) {
        const Yo3Module *m = &s->modules[i];
        yo3_text_add(&out, "%s{\"name\":", i ? "," : ""); yo3_text_json(&out, m->name);
        yo3_text_add(&out, ",\"path\":"); yo3_text_json(&out, m->path);
        yo3_text_add(&out, ",\"size\":%lu,\"sha256\":", m->size); yo3_text_json(&out, m->sha256);
        yo3_text_add(&out, ",\"game_folder\":%s,\"loaded\":%s,\"product\":", m->game_folder ? "true" : "false", m->loaded ? "true" : "false");
        yo3_text_json(&out, m->product); yo3_text_add(&out, "}");
    }
    yo3_text_add(&out, "],\"providers\":[");
    for (int i = 0; i < s->providers_n; ++i) {
        const Yo3Provider *p = &s->providers[i];
        yo3_text_add(&out, "%s{\"name\":", i ? "," : ""); yo3_text_json(&out, p->name);
        yo3_text_add(&out, ",\"path\":"); yo3_text_json(&out, p->path);
        yo3_text_add(&out, ",\"microsoft\":%s}", p->microsoft ? "true" : "false");
    }
    yo3_text_add(&out, "],\"providers_known\":%s}", s->providers_known ? "true" : "false");
    return out.full ? 0 : out.used;
}

/* The report as log lines, one call of `emit` for each (no line break). The module lines carry the SHA-256 grouped. */
typedef void (*Yo3Emit)(const char *line, void *context);
static void yo3_survey_lines(const Yo3Survey *s, Yo3Emit emit, void *context) {
    char line[2 * YO3_SURVEY_PATH + 400], grouped[68];
    for (int i = 0; i < s->hooks_n; ++i) {
        const Yo3Hook *h = &s->hooks[i];
        snprintf(line, sizeof line, "[YO3 MOD] Survey: hooked function=%s how=%s module=%s path=%s%s%s", h->function, h->how,
            h->module[0] ? h->module : "unknown", h->path[0] ? h->path : "none", h->product[0] ? " product=" : "", h->product);
        emit(line, context);
    }
    for (int i = 0; i < s->modules_n; ++i) {
        const Yo3Module *m = &s->modules[i];
        if (strlen(m->sha256) == 64) yo3_sha256_grouped(m->sha256, grouped); else snprintf(grouped, sizeof grouped, "unknown");
        snprintf(line, sizeof line, "[YO3 MOD] Survey: module outside Windows name=%s path=%s size=%lu sha256=%s game_folder=%d loaded=%d%s%s",
            m->name, m->path, m->size, grouped, m->game_folder, m->loaded, m->product[0] ? " product=" : "", m->product);
        emit(line, context);
    }
    for (int i = 0; i < s->providers_n; ++i) {
        const Yo3Provider *p = &s->providers[i];
        snprintf(line, sizeof line, "[YO3 MOD] Survey: Winsock provider name=\"%s\" path=%s microsoft=%d", p->name, p->path[0] ? p->path : "unknown", p->microsoft);
        emit(line, context);
    }
    snprintf(line, sizeof line, "[YO3 MOD] Survey: done modules=%d hooked=%d providers=%s%d", s->modules_n, s->hooks_n,
        s->providers_known ? "" : "unread:", s->providers_n);
    emit(line, context);
}
#endif
