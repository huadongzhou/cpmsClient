mod commands;
mod crypto_service;
mod events;
pub(crate) mod http_service;
pub(crate) mod log_service;
mod models;
mod network_service;
pub(crate) mod ping_service;
mod preferences;
mod print_service;
pub(crate) mod session_server;

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

/// 读取当前会话 token。token 由 iframe 主动推送，仅保存在内存中。
pub fn cached_auth_token(_app: &tauri::AppHandle) -> Option<String> {
    session_server::session_auth_token()
}

/// 清理当前会话 token，token 失效重取流程的第一步。
pub fn clear_cached_auth_token(_app: &tauri::AppHandle) -> Result<(), String> {
    session_server::set_session_auth_token(None);
    Ok(())
}

/// 写入当前会话 token，不落盘缓存。
pub fn save_cached_auth_token(_app: &tauri::AppHandle, token: &str) -> Result<(), String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        session_server::set_session_auth_token(None);
    } else {
        session_server::set_session_auth_token(Some(token));
    }
    Ok(())
}

/// 读取当前会话平台标识。
pub fn cached_platform(_app: &tauri::AppHandle) -> Option<String> {
    session_server::session_platform()
}
