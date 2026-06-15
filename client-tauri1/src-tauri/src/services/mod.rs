mod commands;
mod crypto_service;
mod events;
pub(crate) mod http_service;
pub(crate) mod log_service;
mod models;
mod network_service;
mod preferences;
mod print_service;
mod token_store;

pub use commands::*;
pub use http_service::ClientHttpRequest;

/// 应用数据目录，供 lib 层（待重试队列等）使用。
pub fn app_data_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    preferences::data_dir(app)
}

pub fn forward_socket_task_message(
    app: tauri::AppHandle,
    message: &str,
) -> Result<serde_json::Value, String> {
    print_service::forward_socket_task_message(app, message)
}

/// 读取本地缓存的登录 token。
pub fn cached_auth_token(app: &tauri::AppHandle) -> Option<String> {
    preferences::load_preferences(app)
        .ok()
        .and_then(|preferences| preferences.user)
        .and_then(|user| user.token)
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

/// 清理本地缓存的登录 token，token 失效重取流程的第一步。
pub fn clear_cached_auth_token(app: &tauri::AppHandle) -> Result<(), String> {
    preferences::update_preferences(app, |preferences| {
        if let Some(user) = preferences.user.as_mut() {
            user.token = None;
        }
    })
}

/// 写入重新获取到的登录 token。
pub fn save_cached_auth_token(app: &tauri::AppHandle, token: &str) -> Result<(), String> {
    let token = token.trim().to_string();
    preferences::update_preferences(app, move |preferences| {
        if let Some(user) = preferences.user.as_mut() {
            user.token = Some(token);
        } else {
            preferences.user = Some(models::UserData {
                token: Some(token),
                ..models::UserData::default()
            });
        }
    })
}
