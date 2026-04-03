// server/src/browse.rs
use axum::response::Json;
use serde_json::{json, Value};

#[cfg(target_os = "windows")]
use windows::{
    Win32::{
        UI::WindowsAndMessaging::{GetForegroundWindow},
    }
};

pub async fn browse_directory() -> Json<Value> {
    let path = tokio::task::spawn_blocking(|| {
        // bring_to_front(); // Временно отключено из-за проблем с Windows API
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
        // bring_to_front(); // Временно отключено из-за проблем с Windows API
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
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, SetForegroundWindow, ShowWindow, SW_RESTORE,
        };

        // Получаем и устанавливаем текущее активное окно
        if let Some(hwnd) = GetForegroundWindow() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}