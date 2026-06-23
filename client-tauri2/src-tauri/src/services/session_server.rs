//! 会话级服务端地址、直连设备 ID 与登录 token。
//!
//! 由 iframe（hub-platform）在登录成功后通过 postMessage 推送：
//! - `cpms:serverAddress` → 服务端地址
//! - `cpms:deviceId` → 直连设备 ID
//! - `cpms:token` → 登录 token
//! - `cpms:platform` → 平台标识（harmony/windows），用于 platform 头与 printProperties.terminalType
//!
//! 这些值仅存于应用进程内存，关闭即失效；回到首页时清空。

use std::sync::{Mutex, OnceLock};

static SESSION_SERVER_ADDR: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static SESSION_DIRECT_DEVICE_ID: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static SESSION_AUTH_TOKEN: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static SESSION_PLATFORM: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn get_or_init_mutex<T>(once: &OnceLock<Mutex<Option<T>>>) -> &Mutex<Option<T>> {
    once.get_or_init(|| Mutex::new(None))
}

fn read_session_string(once: &OnceLock<Mutex<Option<String>>>) -> Option<String> {
    get_or_init_mutex(once)
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .filter(|value| !value.trim().is_empty())
}

fn write_session_string(once: &OnceLock<Mutex<Option<String>>>, value: Option<String>) {
    if let Ok(mut guard) = get_or_init_mutex(once).lock() {
        *guard = value
            .filter(|v| !v.trim().is_empty())
            .map(|v| v.trim().to_string());
    }
}

/// 读取当前会话服务端地址。
pub(crate) fn session_server_addr() -> Option<String> {
    read_session_string(&SESSION_SERVER_ADDR)
}

/// 设置或清空会话服务端地址。
pub(crate) fn set_session_server_addr(addr: Option<String>) {
    write_session_string(&SESSION_SERVER_ADDR, addr);
}

/// 读取当前会话直连设备 ID。
pub(crate) fn session_direct_device_id() -> Option<String> {
    read_session_string(&SESSION_DIRECT_DEVICE_ID)
}

/// 设置或清空会话直连设备 ID。
pub(crate) fn set_session_direct_device_id(device_id: Option<String>) {
    write_session_string(&SESSION_DIRECT_DEVICE_ID, device_id);
}

/// 读取当前会话登录 token。
pub(crate) fn session_auth_token() -> Option<String> {
    read_session_string(&SESSION_AUTH_TOKEN)
}

/// 设置或清空会话登录 token。
pub(crate) fn set_session_auth_token(token: Option<String>) {
    write_session_string(&SESSION_AUTH_TOKEN, token);
}

/// 读取当前会话平台标识（由 iframe 通过 cpms:platform 推送）。
pub(crate) fn session_platform() -> Option<String> {
    read_session_string(&SESSION_PLATFORM)
}

/// 设置或清空会话平台标识。
pub(crate) fn set_session_platform(platform: Option<String>) {
    write_session_string(&SESSION_PLATFORM, platform);
}
