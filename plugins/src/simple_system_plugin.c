#include <windows.h>
#include <stdio.h>

// Минимальные определения для совместимости
typedef struct {
    char* name;
    char* version;
    int status;
} plugin_info_t;

typedef enum {
    PLUGIN_SUCCESS = 0,
    PLUGIN_ERROR = 1
} plugin_result_t;

// Экспортируемая функция
__declspec(dllexport) plugin_result_t get_plugin_info(plugin_info_t* info) {
    if (!info) return PLUGIN_ERROR;
    
    info->name = "system_plugin";
    info->version = "1.0.0";
    info->status = 1; // Active
    
    return PLUGIN_SUCCESS;
}

// Точка входа DllMain
BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID lpReserved) {
    switch (ul_reason_for_call) {
    case DLL_PROCESS_ATTACH:
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}
