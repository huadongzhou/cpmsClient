mod commands;
mod crypto_service;
mod events;
pub(crate) mod http_service;
mod models;
mod network_service;
mod preferences;
mod print_service;
mod socket_server;
mod usb_service;

pub use commands::*;
pub use http_service::ClientHttpRequest;

pub fn forward_socket_task_message(
    app: tauri::AppHandle,
    message: &str,
) -> Result<serde_json::Value, String> {
    print_service::forward_socket_task_message(app, message)
}
