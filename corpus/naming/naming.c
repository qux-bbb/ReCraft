/*
 * SPDX-License-Identifier: AGPL-3.0-only
 * Copyright (c) 2026 ReCraft contributors
 *
 * Naming-accuracy benchmark source (see docs/AI.md, `recraft-cli ai eval-names`).
 *
 * Every function has a semantic name and real Win32 evidence (imported APIs, string
 * literals, recognizable control flow), which is what a reverse engineer names functions
 * from. The binary is built with the CRT and exported symbols: the export table is the
 * ground truth the benchmark scores against, and the eval blinds those names so the model
 * cannot read its own answer.
 *
 * The functions are never executed; they only need to exist as code.
 */

#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <shellapi.h>
#include <tlhelp32.h>
#include <wininet.h>

#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "wininet.lib")
#pragma comment(lib, "ws2_32.lib")

static HANDLE open_path(const wchar_t *path, DWORD access) {
    return CreateFileW(path, access, FILE_SHARE_READ, 0, OPEN_EXISTING,
                       FILE_ATTRIBUTE_NORMAL, 0);
}

__declspec(dllexport) int open_config_file(void) {
    HANDLE file = open_path(L"config.ini", GENERIC_READ);
    if (file == INVALID_HANDLE_VALUE) {
        return -1;
    }
    char buffer[512];
    DWORD read = 0;
    int ok = ReadFile(file, buffer, sizeof(buffer), &read, 0) ? (int)read : -1;
    CloseHandle(file);
    return ok;
}

__declspec(dllexport) int save_config_file(void) {
    HANDLE file = CreateFileW(L"config.ini", GENERIC_WRITE, 0, 0, CREATE_ALWAYS,
                              FILE_ATTRIBUTE_NORMAL, 0);
    if (file == INVALID_HANDLE_VALUE) {
        return -1;
    }
    const char text[] = "enabled=1\n";
    DWORD written = 0;
    int ok = WriteFile(file, text, sizeof(text) - 1, &written, 0) ? (int)written : -1;
    FlushFileBuffers(file);
    CloseHandle(file);
    return ok;
}

__declspec(dllexport) int read_registry_string(void) {
    HKEY key = 0;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, L"Software\\ReCraft", 0, KEY_READ, &key) !=
        ERROR_SUCCESS) {
        return -1;
    }
    wchar_t value[256];
    DWORD size = sizeof(value);
    DWORD type = 0;
    LONG status = RegQueryValueExW(key, L"InstallPath", 0, &type, (BYTE *)value, &size);
    RegCloseKey(key);
    return status == ERROR_SUCCESS ? (int)size : -1;
}

__declspec(dllexport) int write_registry_dword(void) {
    HKEY key = 0;
    DWORD disposition = 0;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, L"Software\\ReCraft", 0, 0, 0, KEY_WRITE, 0,
                        &key, &disposition) != ERROR_SUCCESS) {
        return -1;
    }
    DWORD enabled = 1;
    LONG status = RegSetValueExW(key, L"Enabled", 0, REG_DWORD, (const BYTE *)&enabled,
                                 sizeof(enabled));
    RegCloseKey(key);
    return status == ERROR_SUCCESS ? 0 : -1;
}

__declspec(dllexport) int delete_registry_tree(void) {
    LONG status = RegDeleteKeyW(HKEY_CURRENT_USER, L"Software\\ReCraft\\Old");
    return status == ERROR_SUCCESS ? 0 : -1;
}

__declspec(dllexport) int download_update_file(void) {
    HINTERNET session = InternetOpenW(L"ReCraft", INTERNET_OPEN_TYPE_PRECONFIG, 0, 0, 0);
    if (session == 0) {
        return -1;
    }
    HINTERNET request =
        InternetOpenUrlW(session, L"http://example.invalid/update.bin", 0, 0, 0, 0);
    if (request == 0) {
        InternetCloseHandle(session);
        return -1;
    }
    char chunk[1024];
    DWORD read = 0;
    int total = 0;
    while (InternetReadFile(request, chunk, sizeof(chunk), &read) && read > 0) {
        total += (int)read;
    }
    InternetCloseHandle(request);
    InternetCloseHandle(session);
    return total;
}

__declspec(dllexport) int resolve_host_address(void) {
    ADDRINFOW hints;
    ZeroMemory(&hints, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    PADDRINFOW result = 0;
    int status = GetAddrInfoW(L"update.example.invalid", L"443", &hints, &result);
    if (status == 0) {
        FreeAddrInfoW(result);
        return 0;
    }
    return status;
}

__declspec(dllexport) int check_single_instance(void) {
    HANDLE mutex = CreateMutexW(0, FALSE, L"Global\\ReCraft.SingleInstance");
    if (mutex == 0) {
        return -1;
    }
    if (GetLastError() == ERROR_ALREADY_EXISTS) {
        CloseHandle(mutex);
        return 1;
    }
    return 0;
}

__declspec(dllexport) int load_plugin_library(void) {
    HMODULE module = LoadLibraryW(L"plugins\\sample.dll");
    if (module == 0) {
        return -1;
    }
    FARPROC entry = GetProcAddress(module, "PluginEntry");
    if (entry == 0) {
        FreeLibrary(module);
        return -2;
    }
    return 0;
}

__declspec(dllexport) int get_temp_directory(void) {
    wchar_t path[MAX_PATH];
    DWORD length = GetTempPathW(MAX_PATH, path);
    if (length == 0 || length > MAX_PATH) {
        return -1;
    }
    return (int)length;
}

__declspec(dllexport) int copy_file_bytes(void) {
    if (!CopyFileW(L"config.ini", L"config.ini.bak", FALSE)) {
        return -1;
    }
    return 0;
}

__declspec(dllexport) int delete_file_silently(void) {
    if (!DeleteFileW(L"config.ini.tmp")) {
        return -1;
    }
    return 0;
}

__declspec(dllexport) int fill_random_bytes(void) {
    HCRYPTPROV provider = 0;
    if (!CryptAcquireContextW(&provider, 0, 0, PROV_RSA_FULL, CRYPT_VERIFYCONTEXT)) {
        return -1;
    }
    BYTE buffer[32];
    int ok = CryptGenRandom(provider, sizeof(buffer), buffer) ? 0 : -1;
    CryptReleaseContext(provider, 0);
    return ok;
}

__declspec(dllexport) int hash_password_text(void) {
    HCRYPTPROV provider = 0;
    if (!CryptAcquireContextW(&provider, 0, 0, PROV_RSA_FULL, CRYPT_VERIFYCONTEXT)) {
        return -1;
    }
    HCRYPTHASH hash = 0;
    if (!CryptCreateHash(provider, CALG_SHA1, 0, 0, &hash)) {
        CryptReleaseContext(provider, 0);
        return -1;
    }
    const char password[] = "secret";
    CryptHashData(hash, (const BYTE *)password, sizeof(password) - 1, 0);
    BYTE digest[20];
    DWORD size = sizeof(digest);
    CryptGetHashParam(hash, HP_HASHVAL, digest, &size, 0);
    CryptDestroyHash(hash);
    CryptReleaseContext(provider, 0);
    return (int)size;
}

__declspec(dllexport) int enumerate_running_processes(void) {
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snapshot == INVALID_HANDLE_VALUE) {
        return -1;
    }
    PROCESSENTRY32W entry;
    entry.dwSize = sizeof(entry);
    int count = 0;
    if (Process32FirstW(snapshot, &entry)) {
        do {
            count++;
        } while (Process32NextW(snapshot, &entry));
    }
    CloseHandle(snapshot);
    return count;
}

__declspec(dllexport) int read_clipboard_text(void) {
    if (!OpenClipboard(0)) {
        return -1;
    }
    HANDLE data = GetClipboardData(CF_UNICODETEXT);
    int length = data != 0 ? (int)GlobalSize(data) : -1;
    CloseClipboard();
    return length;
}

__declspec(dllexport) int show_error_message(void) {
    int result = MessageBoxW(0, L"Configuration is missing.", L"ReCraft", MB_ICONERROR | MB_OK);
    return result;
}

__declspec(dllexport) int format_hex_string(void) {
    wchar_t text[64];
    int length = wsprintfW(text, L"%08x-%04x", 0x140001000u, 0x20u);
    return length;
}

__declspec(dllexport) int count_text_lines(void) {
    const char *text = "alpha\nbeta\ngamma\n";
    int lines = 0;
    for (const char *cursor = text; *cursor != 0; cursor++) {
        if (*cursor == '\n') {
            lines++;
        }
    }
    return lines;
}

__declspec(dllexport) int split_command_line(void) {
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    if (argv == 0) {
        return -1;
    }
    LocalFree(argv);
    return argc;
}

__declspec(dllexport) int query_disk_free_space(void) {
    ULARGE_INTEGER available;
    if (!GetDiskFreeSpaceExW(L"C:\\", &available, 0, 0)) {
        return -1;
    }
    return (int)(available.QuadPart >> 20);
}

__declspec(dllexport) int start_service_process(void) {
    SC_HANDLE manager = OpenSCManagerW(0, 0, SC_MANAGER_CREATE_SERVICE);
    if (manager == 0) {
        return -1;
    }
    SC_HANDLE service = CreateServiceW(
        manager, L"ReCraftAgent", L"ReCraft Agent", SERVICE_ALL_ACCESS,
        SERVICE_WIN32_OWN_PROCESS, SERVICE_AUTO_START, SERVICE_ERROR_NORMAL,
        L"C:\\Windows\\System32\\svchost.exe", 0, 0, 0, 0, 0);
    if (service == 0) {
        CloseServiceHandle(manager);
        return -2;
    }
    SERVICE_STATUS status;
    StartServiceW(service, 0, 0);
    QueryServiceStatus(service, &status);
    CloseServiceHandle(service);
    CloseServiceHandle(manager);
    return (int)status.dwCurrentState;
}

__declspec(dllexport) int compute_crc32(void) {
    unsigned int crc = 0xFFFFFFFFu;
    const unsigned char data[16] = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16};
    for (int index = 0; index < 16; index++) {
        crc ^= data[index];
        for (int bit = 0; bit < 8; bit++) {
            crc = (crc >> 1) ^ (0xEDB88320u & (0u - (crc & 1u)));
        }
    }
    return (int)~crc;
}

__declspec(dllexport) int base64_encoded_length(void) {
    int input = 100;
    int groups = (input + 2) / 3;
    int padding = (3 - input % 3) % 3;
    return groups * 4 - padding;
}

__declspec(dllexport) int compare_version_strings(void) {
    const char *left = "1.2.3";
    const char *right = "1.3.0";
    return strcmp(left, right);
}

__declspec(dllexport) int write_log_line(void) {
    HANDLE file = CreateFileW(L"recraft.log", FILE_APPEND_DATA, FILE_SHARE_READ, 0,
                              OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, 0);
    if (file == INVALID_HANDLE_VALUE) {
        return -1;
    }
    const char line[] = "[info] started\n";
    DWORD written = 0;
    int ok = WriteFile(file, line, sizeof(line) - 1, &written, 0) ? 0 : -1;
    CloseHandle(file);
    return ok;
}

int main(void) {
    return 0;
}
