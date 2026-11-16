#ifndef YO3_HELPER_HTTP_H
#define YO3_HELPER_HTTP_H
/* One request to the helper on this launch's local HTTP port, with the DLL's own Winsock imports (only the game's are
 * hooked, so no launch-cookie preface: the cookie is in the path). Returns the reply's length, 0 on failure. */
static int helper_request(const char *request, int length, char *reply, int capacity, DWORD wait) {
    if (!http_port || length <= 0 || capacity < 2) return 0;
    SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == INVALID_SOCKET) return 0;
    setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, (const char *)&wait, sizeof wait);
    struct sockaddr_in helper = {0}; helper.sin_family = AF_INET; helper.sin_port = htons(http_port);
    helper.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    int got = 0;
    if (connect(s, (struct sockaddr *)&helper, sizeof helper) == 0 && send(s, request, length, 0) == length) {
        for (;;) { int n = recv(s, reply + got, capacity - 1 - got, 0); if (n <= 0) break; got += n; if (got >= capacity - 1) break; }
    }
    closesocket(s);
    reply[got] = 0;
    return got;
}
#endif
