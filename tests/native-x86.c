/* Build from the repository root in an x86 Native Tools Command Prompt:
   cl /nologo /W4 /WX /O2 /MT tests\native-x86.c /Fotarget\native-x86.obj /Fetarget\native-x86.exe user32.lib
   This is a disposable 32-bit target fixture, not a WinRoll release binary. */
#define UNICODE
#define _UNICODE
#include <windows.h>

#ifndef _M_IX86
#error Build this fixture with the x86 compiler.
#endif

static LRESULT CALLBACK window_proc(HWND window, UINT message, WPARAM w, LPARAM l)
{
    if (message == WM_DESTROY) {
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(window, message, w, l);
}

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, PWSTR command, int show)
{
    WNDCLASSW type = {0};
    HWND window;
    MSG message;
    BOOL received;
    (void)previous;
    (void)command;
    if (!SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2))
        return 1;
    type.lpfnWndProc = window_proc;
    type.hInstance = instance;
    type.hCursor = LoadCursorW(NULL, IDC_ARROW);
    type.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    type.lpszClassName = L"WinRollX86Fixture";
    if (!RegisterClassW(&type))
        return 1;
    window = CreateWindowExW(0, type.lpszClassName, L"WinRoll RS x86 native fixture",
        WS_OVERLAPPEDWINDOW, 100, 100, 800, 600, NULL, NULL, instance, NULL);
    if (!window)
        return 1;
    ShowWindow(window, show);
    while ((received = GetMessageW(&message, NULL, 0, 0)) > 0) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    return received == -1 ? 1 : (int)message.wParam;
}
