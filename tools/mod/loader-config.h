#ifndef YO3_LOADER_CONFIG_H
#define YO3_LOADER_CONFIG_H
#include <wincrypt.h>
#define LOADER_PATH 4096
typedef struct {
    wchar_t launcher[LOADER_PATH], injector[LOADER_PATH], dll[LOADER_PATH], profile[LOADER_PATH];
    wchar_t state[LOADER_PATH], logs[LOADER_PATH], manager[LOADER_PATH], sha256[66];
} LoaderConfig;
static int config_field(const wchar_t *ini, const wchar_t *key, wchar_t *out, DWORD size, int optional) {
    DWORD n = GetPrivateProfileStringW(L"revival", key, L"", out, size, ini);
    return n < size - 1 && (optional || n > 0);
}
static DWORD loader_config(LoaderConfig *config) {
    wchar_t ini[LOADER_PATH]; DWORD n = GetModuleFileNameW(loader_module, ini, LOADER_PATH);
    if (!n || n >= LOADER_PATH - 32) return ERROR_FILENAME_EXCED_RANGE;
    wchar_t *slash = wcsrchr(ini, L'\\'); if (!slash) return ERROR_PATH_NOT_FOUND;
    lstrcpyW(slash + 1, L"yo3-revival-loader.ini");
    wchar_t exe[LOADER_PATH]; n = GetModuleFileNameW(NULL, exe, LOADER_PATH);
    wchar_t *name = wcsrchr(exe, L'\\');
    if (n && n < LOADER_PATH && name && !lstrcmpiW(name + 1, L"yo3.exe")) {
        DWORD bytes = sizeof ini;
        LSTATUS status = RegGetValueW(HKEY_CURRENT_USER, L"Software\\YO3 Revival", L"LoaderConfig", RRF_RT_REG_SZ, NULL, ini, &bytes);
        if (status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND && status != ERROR_PATH_NOT_FOUND) return status;
    }
    if (GetFileAttributesW(ini) == INVALID_FILE_ATTRIBUTES) return ERROR_FILE_NOT_FOUND;
    if (!config_field(ini, L"launcher", config->launcher, LOADER_PATH, 0)
        || !config_field(ini, L"injector", config->injector, LOADER_PATH, 0)
        || !config_field(ini, L"dll", config->dll, LOADER_PATH, 0)
        || !config_field(ini, L"profile", config->profile, LOADER_PATH, 0)
        || !config_field(ini, L"state", config->state, LOADER_PATH, 0)
        || !config_field(ini, L"logs", config->logs, LOADER_PATH, 1)
        || !config_field(ini, L"manager", config->manager, LOADER_PATH, 1)
        || !config_field(ini, L"sha256", config->sha256, 66, 0)) return ERROR_INVALID_DATA;
    return ERROR_SUCCESS;
}
static int mod_hash_matches(const wchar_t *path, const wchar_t *expected) {
    if (lstrlenW(expected) != 64) return 0;
    HANDLE file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) return 0;
    LARGE_INTEGER size; HCRYPTPROV provider = 0; HCRYPTHASH hash = 0; int ok = 0;
    if (GetFileSizeEx(file, &size) && size.QuadPart <= 32 * 1024 * 1024
        && CryptAcquireContextW(&provider, NULL, NULL, PROV_RSA_AES, CRYPT_VERIFYCONTEXT)
        && CryptCreateHash(provider, CALG_SHA_256, 0, 0, &hash)) {
        BYTE buffer[4096]; DWORD count = 0; ok = 1;
        for (;;) { if (!ReadFile(file, buffer, sizeof buffer, &count, NULL)) { ok = 0; break; } if (!count) break;
            if (!CryptHashData(hash, buffer, count, 0)) { ok = 0; break; } }
        BYTE digest[32]; DWORD length = sizeof digest;
        if (ok && CryptGetHashParam(hash, HP_HASHVAL, digest, &length, 0) && length == 32) {
            wchar_t actual[65]; static const wchar_t digits[] = L"0123456789abcdef";
            for (unsigned i = 0; i < 32; ++i) { actual[2*i] = digits[digest[i] >> 4]; actual[2*i+1] = digits[digest[i] & 15]; }
            actual[64] = 0; ok = lstrcmpiW(actual, expected) == 0;
        } else ok = 0;
    }
    if (hash) CryptDestroyHash(hash);
    if (provider) CryptReleaseContext(provider, 0);
    CloseHandle(file); return ok;
}
static DWORD validate_config(LoaderConfig *config) {
    DWORD status = loader_config(config); if (status) return status;
    if (GetFileAttributesW(config->dll) == INVALID_FILE_ATTRIBUTES) return ERROR_MOD_NOT_FOUND;
    if (!mod_hash_matches(config->dll, config->sha256)) return ERROR_CRC;
    if (GetFileAttributesW(config->launcher) == INVALID_FILE_ATTRIBUTES || GetFileAttributesW(config->injector) == INVALID_FILE_ATTRIBUTES
        || GetFileAttributesW(config->profile) == INVALID_FILE_ATTRIBUTES) return ERROR_FILE_NOT_FOUND;
    DWORD attributes = GetFileAttributesW(config->state);
    if (attributes == INVALID_FILE_ATTRIBUTES || !(attributes & FILE_ATTRIBUTE_DIRECTORY)) return ERROR_PATH_NOT_FOUND;
    return ERROR_SUCCESS;
}
#endif
