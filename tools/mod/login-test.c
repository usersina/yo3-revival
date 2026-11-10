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
    puts("PASS: login connects once with fresh/saved data, preserving native state and guarding the call");
    return 0;
}
