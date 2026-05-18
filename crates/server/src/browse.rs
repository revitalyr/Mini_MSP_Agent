//! Directory and File Browser
//!
//! Provides native file dialog functionality for selecting directories and files.

use axum::response::Json;
use serde_json::{json, Value};
use mini_msp_shared::{FilePath, c_str_to_string};

#[cfg(target_os = "windows")]
use windows::{
    Win32::{
        System::Console::GetConsoleWindow,
        UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE},
    }
};

/// Browse for a directory using native file dialog
pub async fn browse_directory() -> Json<Value> {
    let path: Option<String> = tokio::task::spawn_blocking(|| {
        bring_to_front();
        rfd::FileDialog::new()
            .set_title("Select Directory")
            .pick_folder()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None);

    Json(json!({ "path": path }))
}

/// Browse for a file using native file dialog
pub async fn browse_file() -> Json<Value> {
    let path: Option<String> = tokio::task::spawn_blocking(|| {
        bring_to_front();
        rfd::FileDialog::new()
            .set_title("Select File")
            .pick_file()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None);

    Json(json!({ "path": path }))
}

fn bring_to_front() {
    #[cfg(target_os = "windows")]
    unsafe {
        // Получаем HWND консольного окна сервера
        let hwnd = GetConsoleWindow();
        
        // Проверяем, что валидный HWND
        if !hwnd.is_invalid() {
            // Восстанавливаем окно, если оно свернуто
            let _ = ShowWindow(hwnd, SW_RESTORE);
            // Устанавливаем фокус на окно
            let _ = SetForegroundWindow(hwnd);
            
            // Даем время на активацию окна перед показом диалога
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}