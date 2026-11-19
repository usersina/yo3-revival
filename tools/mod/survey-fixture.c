/* A stand-in for another program's DLL in the game folder (#68's tests): it exports a function the test points the game's
 * send import, or the start of a Windows function, at. Never shipped. */
#include <winsock2.h>
__declspec(dllexport) int WSAAPI Yo3FixtureHook(SOCKET socket, const char *data, int length, int flags) { (void)socket; (void)data; (void)flags; return length; }
BOOL WINAPI DllMain(HINSTANCE module, DWORD reason, LPVOID reserved) { (void)module; (void)reason; (void)reserved; return TRUE; }
