#ifndef YO3_LOGIN_FLOW_H
#define YO3_LOGIN_FLOW_H
#include <stdint.h>
#include <string.h>
#include <stdio.h>
static int yo3_login_call_ok(const unsigned char *bytes, size_t length) {
    static const unsigned char expected[]={0xe8,0x50,0x6c,0x08,0x00};
    return length==sizeof expected && !memcmp(bytes,expected,sizeof expected);
}
static void yo3_login_connect(void *window, void (*copy_string)(void *,const void *), void (*connect_gate)(void *)) {
    /* LoginWindow::goLogin (0x46B5FF..0x46B62C). Leave saved fields and native error state alone. */
    unsigned char *screen=window; void *gate;
    memcpy(&gate,screen+0xb0,sizeof gate);
    copy_string((unsigned char *)gate+0x78,screen+0x10dc);
    memcpy((unsigned char *)gate+0x94,screen+0x10f8,sizeof(uint32_t));
    connect_gate(gate);
}

/* The game's own login form (#34). goLogin (0x46B550) puts the User ID and Password boxes in the LoginWindow's
 * std::strings at +0x4C and +0x68 (and UserData +0x338/+0x354), starts the gate connection, then reads Save User ID
 * and Save Password into +0x88 and +0x8C (1 = ticked). On 0x219A the handler 0x469C00 writes lastLogin (only with
 * Save User ID) and a users\ record with isSavePassword, uname and password from +0x68 whatever Save Password says
 * (0x46A2C7..0x46A33A): the DLL hands the entries to the helper and blanks +0x68 first, so no password is written. */
#define YO3_LOGIN_USER 0x4c
#define YO3_LOGIN_PASSWORD 0x68
#define YO3_LOGIN_SAVE_PASSWORD 0x8c
#define YO3_START_LOGIN 0x4f02a0u

/* MSVC 2008 std::string (28 bytes): the text inline at +4 while capacity (+0x18) < 16, else a pointer there; size +0x14. */
static __attribute__((unused)) int yo3_msvc_string(const unsigned char *s, char *out, size_t capacity) {
    uint32_t size, reserved; memcpy(&size, s + 0x14, 4); memcpy(&reserved, s + 0x18, 4);
    if (size >= capacity || size > reserved) return -1;
    const char *text; if (reserved < 16) text = (const char *)s + 4; else memcpy(&text, s + 4, sizeof text);
    if (!text) return -1;
    memcpy(out, text, size); out[size] = 0;
    return (int)size;
}
static __attribute__((unused)) void yo3_msvc_string_clear(unsigned char *s) {
    uint32_t reserved, zero = 0; memcpy(&reserved, s + 0x18, 4);
    char *text; if (reserved < 16) text = (char *)s + 4; else memcpy(&text, s + 4, sizeof text);
    if (text) { uint32_t size; memcpy(&size, s + 0x14, 4); memset(text, 0, size < reserved ? size : reserved); text[0] = 0; }
    memcpy(s + 0x14, &zero, 4);
}

/* JSON text of a typed entry: printable ASCII and UTF-8 kept, quotes and backslashes escaped, control bytes refused. */
static __attribute__((unused)) int yo3_json_text(char *out, size_t capacity, const char *text) {
    size_t n = 0;
    for (; *text; ++text) {
        unsigned char c = (unsigned char)*text;
        if (c < 32 || c == 127) return -1;
        if (n + 3 >= capacity) return -1;
        if (c == '"' || c == '\\') out[n++] = '\\';
        out[n++] = (char)c;
    }
    out[n] = 0; return (int)n;
}

/* The helper's local requests (yo3-launcher LOGIN_STATE_PATH, LOGIN_PATH) under this launch's cookie. */
static __attribute__((unused)) int yo3_login_state_request(char *out, size_t capacity, const char *cookie) {
    int n = snprintf(out, capacity, "GET /session/%s/revival/login-state HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", cookie);
    return n > 0 && (size_t)n < capacity ? n : 0;
}
static __attribute__((unused)) int yo3_login_post(char *out, size_t capacity, const char *cookie, const char *user, const char *password, int stay) {
    char u[256], pw[256], body[600];
    if (!*user || !*password || yo3_json_text(u, sizeof u, user) < 0 || yo3_json_text(pw, sizeof pw, password) < 0) return 0;
    int b = snprintf(body, sizeof body, "{\"user\":\"%s\",\"password\":\"%s\",\"stay\":%s}", u, pw, stay ? "true" : "false");
    if (b <= 0 || (size_t)b >= sizeof body) return 0;
    int n = snprintf(out, capacity, "POST /session/%s/revival/login HTTP/1.1\r\nHost: localhost\r\nContent-Length: %d\r\nConnection: close\r\n\r\n%s", cookie, b, body);
    return n > 0 && (size_t)n < capacity ? n : 0;
}
/* Whether the helper's answer to login-state asks for the form. */
static __attribute__((unused)) int yo3_login_wants_form(const char *reply) {
    const char *body = reply ? strstr(reply, "\r\n\r\n") : NULL;
    return reply && !strncmp(reply, "HTTP/1.1 200", 12) && body && !strcmp(body + 4, "form");
}
#endif
