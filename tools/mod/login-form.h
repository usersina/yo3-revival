#ifndef YO3_LOGIN_FORM_H
#define YO3_LOGIN_FORM_H
/* The game's own login form when there is no saved sign-in (#34, login-flow.h): at ONLINE the DLL asks the helper
 * whether to show it; when the game then connects to the front, goLogin has stored the entries, which go to the
 * helper (it signs in with them, and the front answers the login with the result), and the password is blanked. */
#include "helper-http.h"
#include "login-flow.h"
static void *login_window;
static int login_form;
static int login_decision __attribute__((used));

/* ONLINE (0x46964B): the helper's answer decides between the form and the direct gate connection. */
static int __attribute__((used,noinline,force_align_arg_pointer)) login_decide(void *window) {
    login_window = window; login_form = 0;
    char request[256], reply[512];
    int n = yo3_login_state_request(request, sizeof request, cookie_hex);
    if (n && helper_request(request, n, reply, sizeof reply, 20000)) login_form = yo3_login_wants_form(reply);
    if (login_form) starter_log("[YO3 MOD] No saved sign-in: the game's own login form.\r\n");
    login_decision = login_form;
    return login_form;
}

/* The game connects to the front: hand over what the form holds, once per goLogin (the password box is blank after). */
static void login_before_front(void) {
    if (!login_form || !login_window) return;
    unsigned char *window = login_window;
    char user[128], password[128], request[1024], reply[64];
    if (yo3_msvc_string(window + YO3_LOGIN_USER, user, sizeof user) <= 0
        || yo3_msvc_string(window + YO3_LOGIN_PASSWORD, password, sizeof password) <= 0) return;
    int stay = *(int *)(window + YO3_LOGIN_SAVE_PASSWORD) == 1;
    int n = yo3_login_post(request, sizeof request, cookie_hex, user, password, stay);
    SecureZeroMemory(password, sizeof password);
    /* The users\ record written on 0x219A takes its password from here: never the real one. */
    yo3_msvc_string_clear(window + YO3_LOGIN_PASSWORD);
    if (n && helper_request(request, n, reply, sizeof reply, 5000) > 12 && !memcmp(reply, "HTTP/1.1 204", 12))
        starter_log("[YO3 MOD] The login form's entries went to the helper; the password was not kept.\r\n");
    SecureZeroMemory(request, sizeof request);
}
#endif
