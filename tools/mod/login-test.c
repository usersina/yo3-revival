/* Exercise the connection step used by the real DLL, without the game or a display. */
#include <assert.h>
#include <stdio.h>
#include "login-flow.h"
static unsigned copied, connected;
static unsigned char gate[0xa0], window[0x1100];
static void copy_string(void *to, const void *from) {
    assert(to==gate+0x78 && from==window+0x10dc);
    memcpy(to,from,28); ++copied;
}
static void connect_gate(void *connection) {
    assert(connection==gate);
    uint32_t port; memcpy(&port,gate+0x94,4); assert(port==32123);
    assert(!memcmp(gate+0x78,window+0x10dc,28));
    ++connected;
}
static void attempt(int saved_credentials) {
    memset(window,saved_credentials?0xa5:0,sizeof window); memset(gate,0x55,sizeof gate);
    void *connection=gate; memcpy(window+0xb0,&connection,sizeof connection);
    uint32_t port=32123; memcpy(window+0x10f8,&port,4);
    memcpy(window+0x10dc,"private loopback gate",22);
    unsigned char original[sizeof window], expected[sizeof gate];
    memcpy(original,window,sizeof window); memcpy(expected,gate,sizeof gate);
    memcpy(expected+0x78,window+0x10dc,28); memcpy(expected+0x94,&port,4);
    copied=connected=0;
    yo3_login_connect(window,copy_string,connect_gate);
    assert(copied==1 && connected==1);
    /* Saved login fields, remember flags, player selection and native error state stay untouched. */
    assert(!memcmp(original,window,sizeof window)); assert(!memcmp(expected,gate,sizeof gate));
}
int main(void) {
    unsigned char call[]={0xe8,0x50,0x6c,0x08,0x00};
    assert(yo3_login_call_ok(call,sizeof call));
    for(unsigned i=0;i<sizeof call;++i) {
        call[i]^=1; assert(!yo3_login_call_ok(call,sizeof call)); call[i]^=1;
        assert(!yo3_login_call_ok(call,i));
    }
    assert(!yo3_login_call_ok(call,sizeof call+1));
    attempt(0); attempt(1); attempt(0); /* a new native login window can retry after cancel/error */
    /* #34: the form's entries, read from MSVC strings and the password blanked before the game saves it. */
    unsigned char s[28] = {0}; char out[64];
    memcpy(s + 4, "Yugi", 5); uint32_t size = 4, reserved = 15; memcpy(s + 0x14, &size, 4); memcpy(s + 0x18, &reserved, 4);
    assert(yo3_msvc_string(s, out, sizeof out) == 4 && !strcmp(out, "Yugi"));
    char heap[40] = "a long passphrase!"; char *p = heap; size = 18; reserved = 31;
    memcpy(s + 4, &p, sizeof p); memcpy(s + 0x14, &size, 4); memcpy(s + 0x18, &reserved, 4);
    assert(yo3_msvc_string(s, out, sizeof out) == 18 && !strcmp(out, "a long passphrase!"));
    assert(yo3_msvc_string(s, out, 10) == -1); /* never a cut password */
    yo3_msvc_string_clear(s);
    assert(yo3_msvc_string(s, out, sizeof out) == 0 && !memchr(heap, 'p', sizeof heap)); /* nothing of it left to save */
    size = 40; memcpy(s + 0x14, &size, 4); assert(yo3_msvc_string(s, out, sizeof out) == -1); /* size past capacity */
    char request[1024], cookie[] = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    int n = yo3_login_post(request, sizeof request, cookie, "Yu\\gi \"M\"", "pass word", 1);
    assert(n > 0 && strstr(request, "POST /session/0123") && strstr(request, "/revival/login HTTP/1.1\r\n"));
    assert(strstr(request, "\r\n\r\n{\"user\":\"Yu\\\\gi \\\"M\\\"\",\"password\":\"pass word\",\"stay\":true}"));
    char length[32]; snprintf(length, sizeof length, "Content-Length: %d\r\n", (int)strlen(strstr(request, "\r\n\r\n") + 4));
    assert(strstr(request, length));
    assert(!yo3_login_post(request, sizeof request, cookie, "Yugi", "", 0)); /* an empty box: nothing to send */
    assert(!yo3_login_post(request, sizeof request, cookie, "Yu\ngi", "x", 0)); /* control bytes refused */
    assert(yo3_login_state_request(request, sizeof request, cookie) > 0 && strstr(request, "/revival/login-state HTTP/1.1"));
    assert(yo3_login_wants_form("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nform"));
    assert(!yo3_login_wants_form("HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nready"));
    assert(!yo3_login_wants_form("") && !yo3_login_wants_form(NULL));
    puts("PASS: login connects once with fresh/saved data, preserving native state and guarding the call; the form's entries go to the helper and the password is blanked");
    return 0;
}
