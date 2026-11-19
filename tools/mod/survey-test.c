/* The survey's rules (survey-logic.h), without the game or Windows: SHA-256 against the standard vectors, which paths
 * count as inside Windows' or the game's folder, where a detour leads, which Winsock providers are Microsoft's, and the
 * two reports (log lines and JSON). */
#include <assert.h>
#include <stdio.h>
#include "survey-logic.h"

static const unsigned char *memory; static uintptr_t memory_base; static size_t memory_size;
static int read_memory(uintptr_t address, void *out, size_t n) {
    if (address < memory_base || address + n > memory_base + memory_size) return 0;
    memcpy(out, memory + (address - memory_base), n); return 1;
}
static char lines[16][1700]; static int line_count;
static void collect(const char *line, void *context) { (void)context; snprintf(lines[line_count++], sizeof lines[0], "%s", line); }

int main(void) {
    char hex[65], grouped[68];
    Yo3Sha256 sha; yo3_sha256_init(&sha); yo3_sha256_update(&sha, "abc", 3); yo3_sha256_hex(&sha, hex);
    assert(!strcmp(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
    yo3_sha256_grouped(hex, grouped);
    assert(!strcmp(grouped, "ba7816bf8f01cfea:414140de5dae2223:b00361a396177a9c:b410ff61f20015ad"));
    yo3_sha256_init(&sha); yo3_sha256_hex(&sha, hex);
    assert(!strcmp(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    /* Two blocks, fed in awkward pieces: the standard 56-byte test message, and a million letters. */
    yo3_sha256_init(&sha);
    yo3_sha256_update(&sha, "abcdbcdecdefdefgefghfghighijhijk", 32); yo3_sha256_update(&sha, "ijkljklmklmnlmnomnopnopq", 24); yo3_sha256_hex(&sha, hex);
    assert(!strcmp(hex, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"));
    yo3_sha256_init(&sha); { char a[1000]; memset(a, 'a', sizeof a); for (int i = 0; i < 1000; ++i) yo3_sha256_update(&sha, a, sizeof a); } yo3_sha256_hex(&sha, hex);
    assert(!strcmp(hex, "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"));

    /* Inside a folder: case-blind, either slash, and never a look-alike. */
    assert(yo3_path_under("C:\\Windows\\System32\\ws2_32.dll", "C:\\Windows"));
    assert(yo3_path_under("c:\\windows\\system32\\ws2_32.dll", "C:\\WINDOWS\\"));
    assert(yo3_path_under("C:/Windows/System32/x.dll", "C:\\Windows"));
    assert(!yo3_path_under("C:\\Windows2\\x.dll", "C:\\Windows"));
    assert(!yo3_path_under("C:\\Windows", "C:\\Windows") && !yo3_path_under("C:\\Games\\yo3\\winmm.dll", "C:\\Windows") && !yo3_path_under("x", ""));
    assert(yo3_path_under("D:\\Konami\\Yu-Gi-Oh! ONLINE 3\\winmm.dll", "D:\\Konami\\Yu-Gi-Oh! ONLINE 3"));
    assert(!strcmp(yo3_path_base("C:\\a\\b\\Foo.dll"), "Foo.dll") && !strcmp(yo3_path_base("Foo.dll"), "Foo.dll"));

    /* Where a function's first bytes lead. */
    uintptr_t destination = 0;
    unsigned char near_jump[12] = { 0xe9, 0x10, 0, 0, 0 };
    assert(yo3_jump_target(near_jump, 0x1000, read_memory, &destination) && destination == 0x1015);
    unsigned char back_jump[12] = { 0xe9, 0xf0, 0xff, 0xff, 0xff };
    assert(yo3_jump_target(back_jump, 0x1000, read_memory, &destination) && destination == 0x0ff5);
    unsigned char short_jump[12] = { 0xeb, 0x04 };
    assert(yo3_jump_target(short_jump, 0x2000, read_memory, &destination) && destination == 0x2006);
    unsigned char hot_patch[12] = { 0x8b, 0xff, 0xe9, 0x10, 0, 0, 0 };
    assert(yo3_jump_target(hot_patch, 0x1000, read_memory, &destination) && destination == 0x1017);
    unsigned char slot_bytes[4] = { 0x78, 0x56, 0x34, 0x12 }; memory = slot_bytes; memory_base = 0x5000; memory_size = 4;
    unsigned char through_slot[12] = { 0xff, 0x25, 0x00, 0x50, 0, 0 };
    assert(yo3_jump_target(through_slot, 0x1000, read_memory, &destination) && destination == 0x12345678);
    unsigned char unreadable[12] = { 0xff, 0x25, 0x00, 0x90, 0, 0 }; destination = 7;
    assert(!yo3_jump_target(unreadable, 0x1000, read_memory, &destination) && destination == 7);
    unsigned char push_ret[12] = { 0x68, 0x00, 0x00, 0x40, 0x00, 0xc3 };
    assert(yo3_jump_target(push_ret, 0, read_memory, &destination) && destination == 0x400000);
    unsigned char mov_jmp[12] = { 0xb8, 0x00, 0x00, 0x50, 0x00, 0xff, 0xe0 };
    assert(yo3_jump_target(mov_jmp, 0, read_memory, &destination) && destination == 0x500000);
    /* An ordinary start (push ebp; mov ebp, esp) is no jump, and neither is a call. */
    unsigned char ordinary[12] = { 0x55, 0x8b, 0xec, 0x83, 0xec, 0x10 }; destination = 7;
    assert(!yo3_jump_target(ordinary, 0x1000, read_memory, &destination) && destination == 7);
    unsigned char call[12] = { 0xe8, 1, 0, 0, 0 };
    assert(!yo3_jump_target(call, 0x1000, read_memory, &destination));

    /* The imports the survey looks at, and WSOCK32's ordinals (the game's connect, send and closesocket are 4, 19, 3). */
    assert(yo3_import_matters("WSOCK32.dll", "recv") && yo3_import_matters("ws2_32.dll", "WSAStartup") && yo3_import_matters("KERNEL32.dll", "GetPrivateProfileStringA"));
    assert(yo3_import_matters("WININET.dll", "InternetOpenUrlA") && !yo3_import_matters("WININET.dll", "InternetCloseHandle"));
    assert(!yo3_import_matters("KERNEL32.dll", "CreateFileA") && !yo3_import_matters("USER32.dll", "MessageBoxA") && !yo3_import_matters(NULL, "x"));
    assert(!strcmp(yo3_wsock32_name(4), "connect") && !strcmp(yo3_wsock32_name(19), "send") && !strcmp(yo3_wsock32_name(3), "closesocket")
        && !strcmp(yo3_wsock32_name(16), "recv") && !yo3_wsock32_name(0) && !yo3_wsock32_name(500));

    /* Winsock providers: Microsoft's by their DLL, or by name when the catalog gives no path. */
    assert(yo3_provider_microsoft("MSAFD Tcpip [TCP/IP]", "C:\\Windows\\system32\\mswsock.dll"));
    assert(yo3_provider_microsoft("RSVP TCPv6 Service Provider", "%SystemRoot%\\system32\\RSVPSP.DLL"));
    assert(!yo3_provider_microsoft("MSAFD Tcpip [TCP/IP]", "C:\\Program Files\\Old VPN\\lsp.dll"));
    assert(!yo3_provider_microsoft("Adware Layer", "C:\\Windows\\system32\\mswsock2.dll"));
    assert(yo3_provider_microsoft("MSAFD Tcpip [UDP/IP]", NULL) && yo3_provider_microsoft("Hyper-V RAW", "") && !yo3_provider_microsoft("Adware Layer", NULL));
    assert(!strcmp(yo3_security_product("aswHook.dll"), "Avast") && !strcmp(yo3_security_product("SbieDll.dll"), "Sandboxie") && !yo3_security_product("steam_api.dll"));

    /* The reports. */
    static Yo3Survey survey;
    YO3_SET(survey.hooks[0].function, "send"); YO3_SET(survey.hooks[0].how, "import"); YO3_SET(survey.hooks[0].module, "asi.dll");
    YO3_SET(survey.hooks[0].path, "C:\\Games\\yo3\\asi.dll"); survey.hooks_n = 1;
    YO3_SET(survey.hooks[1].function, "connect"); YO3_SET(survey.hooks[1].how, "detour"); YO3_SET(survey.hooks[1].path, "C:\\Windows\\a\"b.dll");
    YO3_SET(survey.hooks[1].product, "Avast"); survey.hooks_n = 2;
    YO3_SET(survey.modules[0].name, "asi.dll"); YO3_SET(survey.modules[0].path, "C:\\Games\\yo3\\asi.dll");
    YO3_SET(survey.modules[0].sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"); survey.modules[0].size = 1234;
    survey.modules[0].game_folder = 1; survey.modules[0].loaded = 1; survey.modules_n = 1;
    YO3_SET(survey.modules[1].name, "plugin.asi"); YO3_SET(survey.modules[1].path, "C:\\Games\\yo3\\plugin.asi"); survey.modules[1].game_folder = 1; survey.modules_n = 2;
    YO3_SET(survey.providers[0].name, "Adware Layer"); YO3_SET(survey.providers[0].path, "C:\\Old\\lsp.dll"); survey.providers_n = 1; survey.providers_known = 1;
    yo3_survey_lines(&survey, collect, NULL);
    assert(line_count == 6);
    assert(!strcmp(lines[0], "[YO3 MOD] Survey: hooked function=send how=import module=asi.dll path=C:\\Games\\yo3\\asi.dll"));
    assert(strstr(lines[1], "how=detour module=unknown") && strstr(lines[1], "product=Avast"));
    assert(strstr(lines[2], "sha256=ba7816bf8f01cfea:414140de5dae2223:b00361a396177a9c:b410ff61f20015ad game_folder=1 loaded=1"));
    assert(strstr(lines[3], "sha256=unknown") && strstr(lines[3], "loaded=0"));
    assert(strstr(lines[4], "Winsock provider name=\"Adware Layer\"") && strstr(lines[4], "microsoft=0"));
    assert(!strcmp(lines[5], "[YO3 MOD] Survey: done modules=2 hooked=2 providers=1"));
    /* The credential filter hides tokens of 20 letters or digits; no line may have one made of the hash. */
    for (int i = 0; i < line_count; ++i) { int run = 0; for (const char *p = lines[i]; *p; ++p) { run = (*p >= '0' && *p <= '9') || (*p >= 'a' && *p <= 'z') || (*p >= 'A' && *p <= 'Z') ? run + 1 : 0; assert(run < 20 || strstr(lines[i], "path=")); } }
    static char json[16384];
    size_t length = yo3_survey_json(&survey, json, sizeof json);
    assert(length && length == strlen(json));
    assert(strstr(json, "\"version\":1,\"hooked\":[{\"function\":\"send\",\"how\":\"import\",\"module\":\"asi.dll\",\"path\":\"C:\\\\Games\\\\yo3\\\\asi.dll\",\"product\":\"\"}"));
    assert(strstr(json, "C:\\\\Windows\\\\a\\\"b.dll") && strstr(json, "\"sha256\":\"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\",\"game_folder\":true,\"loaded\":true"));
    assert(strstr(json, "\"providers\":[{\"name\":\"Adware Layer\",\"path\":\"C:\\\\Old\\\\lsp.dll\",\"microsoft\":false}],\"providers_known\":true}"));
    char tiny[64]; assert(yo3_survey_json(&survey, tiny, sizeof tiny) == 0 && "an overflow is reported, never written past");
    puts("PASS: the survey recognises foreign modules, hooked functions and providers, and writes them as log lines and JSON");
    return 0;
}
