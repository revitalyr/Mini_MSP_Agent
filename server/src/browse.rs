// server/src/browse.rs
use axum::response::Json;
use serde_json::{json, Value};

#[cfg(target_os = "windows")]
use windows::{
    Win32::{
        UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow, ShowWindow, SW_RESTORE},
    }
};

pub async fn browse_directory() -> Json<Value> {
    let path = tokio::task::spawn_blocking(|| {
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

pub async fn browse_file() -> Json<Value> {
    let path = tokio::task::spawn_blocking(|| {
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
        // Получаем HWND текущего активного окна
        let hwnd = GetForegroundWindow();
        
        // Проверяем, что валидный HWND
        if !hwnd.is_invalid() {
            // Восстанавливаем окно, если оно свернуто
            let _ = ShowWindow(hwnd, SW_RESTORE);
            // Устанавливаем фокус на окно
            let _ = SetForegroundWindow(hwnd);
        }
    }
}