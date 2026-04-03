#include <windows.h>
#include <stdio.h>

// Базовые типы для совместимости
typedef enum {
    PLUGIN_SUCCESS = 0,
    PLUGIN_ERROR = 1
} plugin_result_t;

// Указатели на функции (Option в Rust)
typedef void* option_fn_t;

// Структура интерфейса плагина как в Rust
typedef struct {
    option_fn_t get_plugin_info;           // Option<unsafe extern "C" fn() -> *mut PluginInfo>
    option_fn_t init;                      // Option<unsafe extern "C" fn() -> bool>
    option_fn_t cleanup;                   // Option<unsafe extern "C" fn()>
    option_fn_t get_system_metrics;        // Option<unsafe extern "C" fn(*mut SystemMetrics) -> bool>
    option_fn_t get_processes;             // Option<unsafe extern "C" fn(*mut *mut ProcessInfo, *mut usize) -> bool>
    option_fn_t execute_command;           // Option<unsafe extern "C" fn(*const c_char, *mut CommandResult) -> bool>
    option_fn_t read_file;                 // Option<unsafe extern "C" fn(*const c_char, *mut FileContent) -> bool>
    option_fn_t get_system_info;           // Option<unsafe extern "C" fn(*mut SystemInfo) -> bool>
    option_fn_t get_directory_info_data;   // Option<unsafe extern "C" fn(*const c_char, bool, bool, u32) -> *mut CDirectoryInfoData>
    option_fn_t get_event_data;            // Option<unsafe extern "C" fn(*const c_char) -> *mut CEventData>
    option_fn_t get_watchers_data;         // Option<unsafe extern "C" fn() -> *mut CWatchersData>
    option_fn_t get_file_reader_data;      // Option<unsafe extern "C" fn(*const c_char) -> *mut CFileReaderData>
    option_fn_t get_sensor_data;            // Option<unsafe extern "C" fn() -> *mut CSensorData>
    option_fn_t get_camera_data;            // Option<unsafe extern "C" fn() -> *mut CCameraData>
    option_fn_t get_processing_results;     // Option<unsafe extern "C" fn() -> *mut CProcessingResults>
    option_fn_t get_video_frame;            // Option<unsafe extern "C" fn() -> *mut CVideoFrame>
    option_fn_t free_memory;                // Option<unsafe extern "C" fn(*mut c_void)>
} plugin_interface_t;

// Глобальный интерфейс - все поля NULL кроме обязательных
static plugin_interface_t g_plugin_interface = {
    .get_plugin_info = NULL,
    .init = NULL,
    .cleanup = NULL,
    .get_system_metrics = NULL,
    .get_processes = NULL,
    .execute_command = NULL,
    .read_file = NULL,
    .get_system_info = NULL,
    .get_directory_info_data = NULL,
    .get_event_data = NULL,
    .get_watchers_data = NULL,
    .get_file_reader_data = NULL,
    .get_sensor_data = NULL,
    .get_camera_data = NULL,
    .get_processing_results = NULL,
    .get_video_frame = NULL,
    .free_memory = NULL
};

// Структура PluginInfo как в Rust
typedef struct {
    const char* name;
    const char* version;
    const char* description;
} plugin_info_t;

// Функция get_plugin_info
__declspec(dllexport) plugin_info_t* get_plugin_info(void) {
    static plugin_info_t info = {
        .name = "system_plugin",
        .version = "1.0.0",
        .description = "Simple system plugin for Mini MSP Agent"
    };
    return &info;
}

// Экспортируемая функция get_plugin_interface
__declspec(dllexport) plugin_interface_t* get_plugin_interface(void) {
    // Устанавливаем только get_plugin_info, остальные NULL
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
