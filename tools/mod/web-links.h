#ifndef YO3_WEB_LINKS_H
#define YO3_WEB_LINKS_H
/* Installs web-links-logic.h: the game's ShellExecuteA import is wrapped so its links to Konami's old website open the
 * host's own pages (the server's /web/, crates/yo3-server/src/web.rs) through the launcher's local relay. Other files
 * and programs open as before. A game without the import (the contract fixtures) has no such links: nothing to do. */
#include "web-links-logic.h"

static HINSTANCE (WINAPI *shell_execute)(HWND, LPCSTR, LPCSTR, LPCSTR, LPCSTR, INT);

static HINSTANCE WINAPI hooked_shell_execute(HWND window, LPCSTR operation, LPCSTR file, LPCSTR parameters, LPCSTR directory, INT show) {
    char url[1024];
    if (yo3_web_link(file, http_port, cookie_hex, url, sizeof url)) return shell_execute(window, operation, url, parameters, directory, show);
    return shell_execute(window, operation, file, parameters, directory, show);
}

static void web_links_install(void) {
    hook("ShellExecuteA", (void *)hooked_shell_execute, (void **)&shell_execute);
}
#endif
