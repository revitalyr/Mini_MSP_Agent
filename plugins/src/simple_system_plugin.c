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

// Структура интерфейса плагина
typedef struct {
    plugin_result_t (*get_plugin_info)(plugin_info_t* info);
    // Другие функции могут быть добавлены позже
} plugin_interface_t;

// Глобальный интерфейс
static plugin_interface_t g_plugin_interface = {
    .get_plugin_info = NULL
};

// Экспортиемые функции
__declspec(dllexport) plugin_result_t get_plugin_info(plugin_info_t* info) {
    if (!info) return PLUGIN_ERROR;
    
    info->name = "system_plugin";
    info->version = "1.0.0";
    info->status = 1; // Active
    
    return PLUGIN_SUCCESS;
}

__declspec(dllexport) plugin_interface_t* get_plugin_interface(void) {
    // Устанавливаем указатель на функцию
    g_plugin_interface.get_plugin_info = get_plugin_info;
    return &g_plugin_interface;
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
