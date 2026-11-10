#ifndef YO3_LOGIN_FLOW_H
#define YO3_LOGIN_FLOW_H
#include <stdint.h>
#include <string.h>
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
#endif
