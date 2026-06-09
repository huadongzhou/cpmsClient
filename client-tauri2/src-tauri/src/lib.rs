mod hub;
mod models;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use futures_util::StreamExt;
use models::CommandResult;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Listener, Manager, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

const MAIN_WINDOW_LABEL: &str = "main";
const VIEW_TO_CLIENT_EVENT: &str = "cpms:view-to-client";
const CLIENT_TO_VIEW_EVENT: &str = "cpms:client-to-view";
const CLIENT_NOTIFICATION_EVENT: &str = "cpms:desktop-notification";
const CLIENT_IFRAME_EVENT: &str = "cpms:client-iframe";
const CLIENT_TODO_TASK_EVENT: &str = "cpms:client-todo-task";
const CLIENT_IFRAME_PAYLOAD_REQUEST_EVENT: &str = "client.iframe_payload.request";
const CLIENT_IFRAME_PAYLOAD_REPORT_EVENT: &str = "client.iframe_payload.reported";
const TRAY_SHOW: &str = "tray.show";
const TRAY_HIDE: &str = "tray.hide";
const TRAY_AUTOSTART_ENABLE: &str = "tray.autostart.enable";
const TRAY_AUTOSTART_DISABLE: &str = "tray.autostart.disable";
const TRAY_QUIT: &str = "tray.quit";
const AUTOSTART_INIT_MARKER: &str = ".autostart-initialized";
const DEFAULT_CPMS_BASE_URL: &str = "http://localhost:8080";
const DEFAULT_IFRAME_CONFIG_PATH: &str = "/api/client/iframe-config";
const DEFAULT_LOCAL_SOCKET_URL: &str = "ws://127.0.0.1:18080/ws/task";
const DEFAULT_IFRAME_FALLBACK_URL: &str = "http://localhost:9528/#/";
const DEFAULT_LOCAL_SOCKET_PATH: &str = "/ws/task";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewEventPayload {
    name: String,
    payload: Option<Value>,
    at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ClientEventPayload {
    name: String,
    payload: Option<Value>,
    at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DesktopNotificationPayload {
    #[serde(rename = "type")]
    kind: String,
    title: String,
    message: Option<String>,
    duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientIframeEventPayload {
    state: String,
    url: Option<String>,
    message: Option<String>,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientTodoTaskPayload {
    id: String,
    title: String,
    detail: Option<String>,
    state: String,
    source: String,
    created_at: String,
    updated_at: String,
}

struct AppRuntimeState {
    iframe: Mutex<ClientIframeEventPayload>,
    iframe_payload: Mutex<Option<Value>>,
}

#[tauri::command]
fn greet(name: &str) -> CommandResult<String> {
    let value = name.trim();

    if value.is_empty() {
        return CommandResult::fail("VALIDATION_ERROR", "name 不能为空");
    }

    CommandResult::ok(format!("Hello, {}! You've been greeted from Rust!", value))
}

#[tauri::command]
fn emit_client_event(app: AppHandle, name: String, payload: Option<Value>) -> CommandResult<bool> {
    let event_payload = ClientEventPayload {
        name,
        payload,
        at: now_iso_string(),
    };

    match app.emit_to(MAIN_WINDOW_LABEL, CLIENT_TO_VIEW_EVENT, event_payload) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("EMIT_EVENT_ERROR", &error.to_string()),
    }
}

#[tauri::command]
fn push_desktop_notification_event(
    app: AppHandle,
    notification: DesktopNotificationPayload,
) -> CommandResult<bool> {
    match app.emit_to(MAIN_WINDOW_LABEL, CLIENT_NOTIFICATION_EVENT, notification) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("EMIT_NOTIFICATION_ERROR", &error.to_string()),
    }
}

#[tauri::command]
fn client_get_iframe_container_state(
    state: tauri::State<'_, AppRuntimeState>,
) -> CommandResult<ClientIframeEventPayload> {
    let current = state
        .iframe
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| initial_iframe_state());

    CommandResult::ok(current)
}

#[tauri::command]
async fn client_refresh_iframe_container(
    app: AppHandle,
) -> CommandResult<ClientIframeEventPayload> {
    CommandResult::ok(refresh_iframe_container(&app).await)
}

#[tauri::command]
fn client_request_iframe_payload(app: AppHandle, reason: Option<String>) -> CommandResult<String> {
    let request_id = emit_iframe_payload_request(&app, reason.as_deref().unwrap_or("manual"));
    CommandResult::ok(request_id)
}

#[tauri::command]
fn client_submit_iframe_payload(
    app: AppHandle,
    state: tauri::State<'_, AppRuntimeState>,
    request_id: String,
    payload: Option<Value>,
) -> CommandResult<bool> {
    let report_payload = json!({
        "requestId": request_id,
        "payload": payload,
        "at": now_iso_string(),
    });

    if let Ok(mut locked) = state.iframe_payload.lock() {
        *locked = Some(report_payload.clone());
    }

    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_TO_VIEW_EVENT,
        ClientEventPayload {
            name: CLIENT_IFRAME_PAYLOAD_REPORT_EVENT.into(),
            payload: Some(report_payload),
            at: now_iso_string(),
        },
    );

    CommandResult::ok(true)
}

#[tauri::command]
async fn client_http_request(request: hub::ClientHttpRequest) -> CommandResult<Value> {
    match hub::http_service::execute_client_http_request(request).await {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("HTTP_REQUEST_ERROR", &error),
    }
}

#[tauri::command]
fn autostart_is_enabled(app: AppHandle) -> CommandResult<bool> {
    match app.autolaunch().is_enabled() {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("AUTOSTART_QUERY_ERROR", &error.to_string()),
    }
}

#[tauri::command]
fn autostart_set_enabled(app: AppHandle, enabled: bool) -> CommandResult<bool> {
    match set_autostart_enabled(&app, enabled) {
        Ok(_) => CommandResult::ok(enabled),
        Err(error) => CommandResult::fail("AUTOSTART_UPDATE_ERROR", &error.to_string()),
    }
}

fn set_autostart_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|error| error.to_string())
    } else {
        manager.disable().map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn window_minimize(app: AppHandle) -> CommandResult<bool> {
    control_main_window(&app, |window| window.minimize())
}

#[tauri::command]
fn window_maximize(app: AppHandle) -> CommandResult<bool> {
    control_main_window(&app, |window| window.maximize())
}

#[tauri::command]
fn window_unmaximize(app: AppHandle) -> CommandResult<bool> {
    control_main_window(&app, |window| window.unmaximize())
}

#[tauri::command]
fn window_set_fullscreen(app: AppHandle, fullscreen: bool) -> CommandResult<bool> {
    control_main_window(&app, |window| window.set_fullscreen(fullscreen))
}

#[tauri::command]
fn window_set_always_on_top(app: AppHandle, always_on_top: bool) -> CommandResult<bool> {
    control_main_window(&app, |window| window.set_always_on_top(always_on_top))
}

#[tauri::command]
fn window_hide(app: AppHandle) -> CommandResult<bool> {
    control_main_window(&app, |window| window.hide())
}

#[tauri::command]
fn window_show(app: AppHandle) -> CommandResult<bool> {
    control_main_window(&app, |window| {
        window.show()?;
        window.set_focus()
    })
}

#[tauri::command]
fn window_close(app: AppHandle) -> CommandResult<bool> {
    control_main_window(&app, |window| window.hide())
}

fn control_main_window<F>(app: &AppHandle, op: F) -> CommandResult<bool>
where
    F: FnOnce(&tauri::WebviewWindow) -> tauri::Result<()>,
{
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return CommandResult::fail("WINDOW_NOT_FOUND", "主窗口不存在");
    };

    match op(&window) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("WINDOW_CONTROL_ERROR", &error.to_string()),
    }
}

fn now_iso_string() -> String {
    format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or_default()
    )
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

fn setup_client_event_bridge(app: &AppHandle) {
    let app_handle = app.clone();

    app.listen_any(VIEW_TO_CLIENT_EVENT, move |event| {
        let raw = event.payload();
        let payload = serde_json::from_str::<ViewEventPayload>(raw).unwrap_or(ViewEventPayload {
            name: "unknown".into(),
            payload: Some(Value::String(raw.to_string())),
            at: None,
        });

        if handle_view_event(&app_handle, &payload.name, payload.payload.clone()) {
            return;
        }

        let event_payload = ClientEventPayload {
            name: payload.name,
            payload: payload.payload,
            at: payload.at.unwrap_or_else(now_iso_string),
        };

        let _ = app_handle.emit_to(MAIN_WINDOW_LABEL, CLIENT_TO_VIEW_EVENT, event_payload);
    });
}

fn handle_view_event(app: &AppHandle, name: &str, payload: Option<Value>) -> bool {
    match name {
        "client.window.pin" | "window.pin" => {
            emit_view_command_result(
                app,
                "client.window.pin.result",
                window_set_always_on_top(app.clone(), true),
            );
            true
        }
        "client.window.unpin" | "window.unpin" => {
            emit_view_command_result(
                app,
                "client.window.unpin.result",
                window_set_always_on_top(app.clone(), false),
            );
            true
        }
        "client.window.minimize" | "window.minimize" | "client.window.collapse" => {
            emit_view_command_result(
                app,
                "client.window.minimize.result",
                window_minimize(app.clone()),
            );
            true
        }
        "client.window.hide" | "window.hide" => {
            emit_view_command_result(app, "client.window.hide.result", window_hide(app.clone()));
            true
        }
        "client.window.close" | "window.close" => {
            emit_view_command_result(app, "client.window.close.result", window_close(app.clone()));
            true
        }
        "client.window.fullscreen" | "window.fullscreen" => {
            let fullscreen = payload_bool(payload.as_ref(), "fullscreen").unwrap_or(true);
            emit_view_command_result(
                app,
                "client.window.fullscreen.result",
                window_set_fullscreen(app.clone(), fullscreen),
            );
            true
        }
        "client.window.exit-fullscreen" | "window.exit-fullscreen" => {
            emit_view_command_result(
                app,
                "client.window.exit-fullscreen.result",
                window_set_fullscreen(app.clone(), false),
            );
            true
        }
        "client.jobs.list" | "jobs.list" => {
            let page_number = payload_i64(payload.as_ref(), "pageNumber").unwrap_or(1);
            let page_size = payload_i64(payload.as_ref(), "pageSize").unwrap_or(20);
            let job_type = payload_i64(payload.as_ref(), "type").unwrap_or(1);
            let title = payload_string(payload.as_ref(), "title");
            let search_time = payload_string(payload.as_ref(), "searchTime");
            let app_handle = app.clone();
            thread::spawn(move || {
                let result = hub::get_job_list(
                    app_handle.clone(),
                    page_number,
                    page_size,
                    job_type,
                    title,
                    search_time,
                );
                emit_view_command_result(&app_handle, "client.jobs.list.result", result);
            });
            true
        }
        "client.devices.list" | "devices.list" | "client.printers.list" | "printers.list" => {
            let app_handle = app.clone();
            thread::spawn(move || {
                let result = hub::get_available_devices(app_handle.clone());
                emit_view_command_result(&app_handle, "client.devices.list.result", result);
            });
            true
        }
        "client.device.select" | "device.select" | "client.printer.select" | "printer.select" => {
            let device = payload.map(|value| value.get("device").cloned().unwrap_or(value));
            let Some(device) = device else {
                emit_view_command_result::<Value>(
                    app,
                    "client.device.select.result",
                    CommandResult::fail("DEVICE_PAYLOAD_EMPTY", "device 不能为空"),
                );
                return true;
            };
            let app_handle = app.clone();
            thread::spawn(move || {
                let result = hub::select_direct_device(app_handle.clone(), device);
                emit_view_command_result(&app_handle, "client.device.select.result", result);
            });
            true
        }
        "client.auth.update-token" | "auth.update-token" | "client.token.update" => {
            let token = payload_string(payload.as_ref(), "token")
                .or_else(|| payload_string(payload.as_ref(), "accessToken"))
                .or_else(|| payload.and_then(|value| value.as_str().map(str::to_string)))
                .unwrap_or_default();
            emit_view_command_result(
                app,
                "client.auth.update-token.result",
                hub::save_auth_token(app.clone(), token),
            );
            true
        }
        _ => false,
    }
}

fn emit_view_command_result<T: Serialize>(app: &AppHandle, name: &str, result: CommandResult<T>) {
    let payload = serde_json::to_value(result).unwrap_or_else(|error| {
        json!({
            "success": false,
            "code": "SERIALIZE_RESULT_ERROR",
            "message": error.to_string(),
            "data": null,
            "logs": [],
        })
    });

    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_TO_VIEW_EVENT,
        ClientEventPayload {
            name: name.into(),
            payload: Some(payload),
            at: now_iso_string(),
        },
    );
}

fn payload_i64(payload: Option<&Value>, key: &str) -> Option<i64> {
    payload.and_then(|value| value.get(key)).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str()?.parse::<i64>().ok())
    })
}

fn payload_string(payload: Option<&Value>, key: &str) -> Option<String> {
    payload
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn payload_bool(payload: Option<&Value>, key: &str) -> Option<bool> {
    payload
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
}

fn emit_iframe_payload_request(app: &AppHandle, reason: &str) -> String {
    let request_id = format!("iframe-payload-{}", now_iso_string());
    let payload = json!({
        "requestId": request_id,
        "reason": reason,
    });

    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_TO_VIEW_EVENT,
        ClientEventPayload {
            name: CLIENT_IFRAME_PAYLOAD_REQUEST_EVENT.into(),
            payload: Some(payload),
            at: now_iso_string(),
        },
    );

    request_id
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, TRAY_SHOW, "显示主窗口", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, TRAY_HIDE, "隐藏到托盘", true, None::<&str>)?;
    let autostart_enable_item = MenuItem::with_id(
        app,
        TRAY_AUTOSTART_ENABLE,
        "开启开机自启动",
        true,
        None::<&str>,
    )?;
    let autostart_disable_item = MenuItem::with_id(
        app,
        TRAY_AUTOSTART_DISABLE,
        "关闭开机自启动",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, TRAY_QUIT, "退出", true, None::<&str>)?;
    let tray_menu = Menu::with_items(
        app,
        &[
            &show_item,
            &hide_item,
            &autostart_enable_item,
            &autostart_disable_item,
            &quit_item,
        ],
    )?;

    let mut tray_builder = TrayIconBuilder::with_id("cpms-tray")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .tooltip("CPMS Client");

    if let Some(default_icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(default_icon);
    }

    tray_builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW => show_main_window(app),
            TRAY_HIDE => hide_main_window(app),
            TRAY_AUTOSTART_ENABLE => {
                let _ = set_autostart_enabled(app, true);
            }
            TRAY_AUTOSTART_DISABLE => {
                let _ = set_autostart_enabled(app, false);
            }
            TRAY_QUIT => {
                let _ = hub::system_destroy(app.clone());
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(&tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn cpms_base_url() -> String {
    std::env::var("CPMS_BASE_URL").unwrap_or_else(|_| DEFAULT_CPMS_BASE_URL.into())
}

fn init_autostart_on_first_launch(app: &AppHandle) {
    let marker_path = app.path().app_data_dir().ok().map(|mut dir| {
        dir.push(AUTOSTART_INIT_MARKER);
        dir
    });

    let Some(marker_path) = marker_path else {
        return;
    };

    if marker_path.exists() {
        return;
    }

    if let Some(parent) = marker_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let manager = app.autolaunch();
    if matches!(manager.is_enabled(), Ok(false)) {
        let _ = manager.enable();
    }

    let _ = fs::write(marker_path, b"ok");
}

fn iframe_config_path() -> String {
    std::env::var("CPMS_IFRAME_CONFIG_PATH").unwrap_or_else(|_| DEFAULT_IFRAME_CONFIG_PATH.into())
}

fn iframe_allowed_hosts() -> Vec<String> {
    let raw = std::env::var("CPMS_IFRAME_ALLOW_HOSTS").unwrap_or_default();
    raw.split(',')
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn local_socket_url() -> String {
    std::env::var("CPMS_PRINTCLIENT_SOCKET_URL")
        .or_else(|_| std::env::var("CPMS_LOCAL_SOCKET_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(discover_print_client_socket_url)
        .unwrap_or_else(|| DEFAULT_LOCAL_SOCKET_URL.into())
}

fn discover_print_client_socket_url() -> Option<String> {
    if let Ok(config_path) = std::env::var("CPMS_PRINTCLIENT_CONFIG_PATH") {
        let path = PathBuf::from(config_path);
        if let Some(url) = socket_url_from_config_file(&path) {
            return Some(url);
        }
    }

    for dir in print_client_candidate_dirs() {
        for file_name in ["DriverClient.ini", "config.conf", "config.ini"] {
            let config_path = dir.join(file_name);
            if let Some(url) = socket_url_from_config_file(&config_path) {
                return Some(url);
            }
        }
    }

    None
}

fn print_client_candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(dir) = std::env::var("CPMS_PRINTCLIENT_DIR") {
        dirs.push(PathBuf::from(dir));
    }

    for env_key in [
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
        "LOCALAPPDATA",
        "APPDATA",
    ] {
        if let Ok(base) = std::env::var(env_key) {
            let base_path = PathBuf::from(base);
            dirs.push(base_path.join("PrintClient"));
            dirs.push(base_path.join("CPMS").join("PrintClient"));
            dirs.push(base_path.join("Insolu").join("PrintClient"));
        }
    }

    dirs
}

fn socket_url_from_config_file(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let socket_path = std::env::var("CPMS_PRINTCLIENT_SOCKET_PATH")
        .ok()
        .filter(|value| value.starts_with('/'))
        .unwrap_or_else(|| DEFAULT_LOCAL_SOCKET_PATH.into());

    for line in raw.lines() {
        if let Some(url) = extract_websocket_url(line) {
            return Some(url);
        }
    }

    for line in raw.lines() {
        let lower = line.to_lowercase();
        if !(lower.contains("websocket") || lower.contains("socket") || lower.contains("port")) {
            continue;
        }

        if let Some(port) = extract_port(line) {
            return Some(format!("ws://127.0.0.1:{port}{socket_path}"));
        }
    }

    None
}

fn extract_websocket_url(line: &str) -> Option<String> {
    let start = line.find("ws://").or_else(|| line.find("wss://"))?;
    let candidate = line[start..]
        .trim()
        .trim_matches(|character: char| {
            character.is_whitespace() || matches!(character, '"' | '\'' | ';' | ',')
        })
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '"' | '\'' | ';' | ',')
        })
        .next()?;

    if candidate.starts_with("ws://") || candidate.starts_with("wss://") {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn extract_port(line: &str) -> Option<u16> {
    let value_part = line
        .split_once('=')
        .map(|(_, value)| value)
        .or_else(|| line.split_once(':').map(|(_, value)| value))
        .unwrap_or(line);
    let mut digits = String::new();

    for character in value_part.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
            continue;
        }

        if !digits.is_empty() {
            if let Ok(port) = digits.parse::<u16>() {
                if port > 0 {
                    return Some(port);
                }
            }
            digits.clear();
        }
    }

    digits.parse::<u16>().ok().filter(|port| *port > 0)
}

fn initial_iframe_state() -> ClientIframeEventPayload {
    ClientIframeEventPayload {
        state: "idle".into(),
        url: None,
        message: None,
        updated_at: now_iso_string(),
    }
}

fn fallback_iframe_state(app: &AppHandle, reason: String) -> ClientIframeEventPayload {
    update_iframe_state(
        app,
        "loaded",
        Some(DEFAULT_IFRAME_FALLBACK_URL.into()),
        Some(format!("地址查询失败，已回退默认地址：{reason}")),
    )
}

fn update_iframe_state(
    app: &AppHandle,
    state: &str,
    url: Option<String>,
    message: Option<String>,
) -> ClientIframeEventPayload {
    let payload = ClientIframeEventPayload {
        state: state.into(),
        url,
        message,
        updated_at: now_iso_string(),
    };

    {
        let runtime_state: tauri::State<'_, AppRuntimeState> = app.state();
        let lock_result = runtime_state.iframe.lock();
        if let Ok(mut locked) = lock_result {
            *locked = payload.clone();
        }
    }

    let _ = app.emit_to(MAIN_WINDOW_LABEL, CLIENT_IFRAME_EVENT, payload.clone());
    payload
}

fn build_iframe_config_url() -> Result<String, String> {
    let base = cpms_base_url();
    let parsed = Url::parse(&base).map_err(|_| format!("无效 CPMS_BASE_URL: {base}"))?;
    let url = parsed
        .join(iframe_config_path().trim_start_matches('/'))
        .map_err(|error| format!("拼接 iframe 配置地址失败: {error}"))?;

    Ok(url.to_string())
}

fn extract_iframe_url(value: &Value) -> Option<String> {
    value
        .get("iframeUrl")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| value.get("url").and_then(Value::as_str).map(str::to_string))
        .or_else(|| {
            value
                .get("data")
                .and_then(Value::as_object)
                .and_then(|item| {
                    item.get("iframeUrl")
                        .or_else(|| item.get("url"))
                        .and_then(Value::as_str)
                })
                .map(str::to_string)
        })
}

fn validate_iframe_url(url: &str) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|_| "iframe 地址格式非法".to_string())?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("iframe 仅支持 http/https 协议".into());
    }

    let mut allow_hosts = iframe_allowed_hosts();

    if allow_hosts.is_empty() {
        if let Ok(base) = Url::parse(&cpms_base_url()) {
            if let Some(host) = base.host_str() {
                allow_hosts.push(host.to_lowercase());
            }
        }
    }

    if !allow_hosts.is_empty() {
        let host = parsed.host_str().unwrap_or_default().to_lowercase();

        if !allow_hosts.iter().any(|item| item == &host) {
            return Err(format!("iframe 域名不在白名单: {host}"));
        }
    }

    Ok(parsed.to_string())
}

async fn refresh_iframe_container(app: &AppHandle) -> ClientIframeEventPayload {
    update_iframe_state(app, "loading", None, None);

    let endpoint = match build_iframe_config_url() {
        Ok(value) => value,
        Err(error) => return fallback_iframe_state(app, error),
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(8_000))
        .build()
    {
        Ok(value) => value,
        Err(error) => return fallback_iframe_state(app, error.to_string()),
    };

    let response = match client.get(endpoint).send().await {
        Ok(value) => value,
        Err(error) => return fallback_iframe_state(app, error.to_string()),
    };

    let payload = match response.json::<Value>().await {
        Ok(value) => value,
        Err(error) => return fallback_iframe_state(app, error.to_string()),
    };

    let Some(candidate_url) = extract_iframe_url(&payload) else {
        return fallback_iframe_state(app, "线上服务未返回 iframe URL".into());
    };

    match validate_iframe_url(&candidate_url) {
        Ok(valid_url) => update_iframe_state(app, "loaded", Some(valid_url), None),
        Err(message) => fallback_iframe_state(app, message),
    }
}

fn to_todo_state(value: &str) -> String {
    match value.to_lowercase().as_str() {
        "running" | "processing" => "running".into(),
        "done" | "success" | "finished" => "done".into(),
        "failed" | "error" => "failed".into(),
        _ => "todo".into(),
    }
}

fn parse_todo_payload(message: &str) -> Option<ClientTodoTaskPayload> {
    let parsed = serde_json::from_str::<Value>(message).ok()?;
    let payload = parsed.get("payload").unwrap_or(&parsed);

    let id = payload
        .get("taskId")
        .or_else(|| payload.get("id"))
        .and_then(Value::as_str)?
        .to_string();

    let title = payload
        .get("title")
        .or_else(|| payload.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| id.clone());

    let detail = payload
        .get("detail")
        .or_else(|| payload.get("description"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let state = payload
        .get("status")
        .or_else(|| payload.get("state"))
        .and_then(Value::as_str)
        .map(to_todo_state)
        .unwrap_or_else(|| "todo".into());

    let now = now_iso_string();

    Some(ClientTodoTaskPayload {
        id,
        title,
        detail,
        state,
        source: "socket".into(),
        created_at: now.clone(),
        updated_at: now,
    })
}

async fn start_local_socket_worker(app: AppHandle) {
    loop {
        let socket_url = local_socket_url();

        match tokio_tungstenite::connect_async(&socket_url).await {
            Ok((mut stream, _)) => {
                while let Some(next_message) = stream.next().await {
                    match next_message {
                        Ok(raw_message) if raw_message.is_text() => {
                            if let Ok(text) = raw_message.to_text() {
                                if is_print_task_message(text) {
                                    let app_handle = app.clone();
                                    let message = text.to_string();
                                    thread::spawn(move || {
                                        let result = hub::forward_socket_task_message(
                                            app_handle.clone(),
                                            &message,
                                        );
                                        emit_socket_forward_result(&app_handle, result);
                                    });
                                } else if let Some(task_payload) = parse_todo_payload(text) {
                                    let _ = app.emit_to(
                                        MAIN_WINDOW_LABEL,
                                        CLIENT_TODO_TASK_EVENT,
                                        task_payload,
                                    );
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

fn is_print_task_message(message: &str) -> bool {
    normalize_socket_message_value(message)
        .and_then(|value| {
            value
                .get("filePath")
                .and_then(Value::as_str)
                .map(|file_path| !file_path.trim().is_empty())
        })
        .unwrap_or(false)
}

fn normalize_socket_message_value(message: &str) -> Option<Value> {
    let parsed = serde_json::from_str::<Value>(message).ok()?;
    match parsed {
        Value::String(raw) => serde_json::from_str::<Value>(&raw).ok(),
        Value::Object(_) => Some(parsed),
        _ => None,
    }
}

fn emit_socket_forward_result(app: &AppHandle, result: Result<Value, String>) {
    let (name, payload) = match result {
        Ok(value) => (
            "client.socket_task.forwarded",
            json!({ "ok": true, "task": value }),
        ),
        Err(error) => (
            "client.socket_task.forward_failed",
            json!({ "ok": false, "message": error }),
        ),
    };

    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_TO_VIEW_EVENT,
        ClientEventPayload {
            name: name.into(),
            payload: Some(payload),
            at: now_iso_string(),
        },
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .manage(AppRuntimeState {
            iframe: Mutex::new(initial_iframe_state()),
            iframe_payload: Mutex::new(None),
        })
        .setup(|app| {
            setup_client_event_bridge(app.handle());
            init_autostart_on_first_launch(app.handle());
            setup_tray(app.handle())?;

            let app_handle_for_iframe = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = refresh_iframe_container(&app_handle_for_iframe).await;
            });

            let app_handle_for_socket = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                start_local_socket_worker(app_handle_for_socket).await;
            });

            let app_handle_for_payload = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let _ = emit_iframe_payload_request(&app_handle_for_payload, "startup");
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            emit_client_event,
            push_desktop_notification_event,
            client_get_iframe_container_state,
            client_refresh_iframe_container,
            client_request_iframe_payload,
            client_submit_iframe_payload,
            client_http_request,
            autostart_is_enabled,
            autostart_set_enabled,
            window_minimize,
            window_maximize,
            window_unmaximize,
            window_set_fullscreen,
            window_set_always_on_top,
            window_hide,
            window_show,
            window_close,
            hub::get_startup_state,
            hub::save_policy_agreed,
            hub::save_auth_state,
            hub::clear_auth_state,
            hub::save_auth_token,
            hub::save_server_info,
            hub::save_direct_device,
            hub::get_job_list,
            hub::get_available_devices,
            hub::select_direct_device,
            hub::system_init,
            hub::system_destroy,
            hub::start_background_tasks,
            hub::stop_background_tasks,
            hub::add_printer,
            hub::disable_printer,
            hub::fix_printer,
            hub::init_usb_printer,
            hub::get_usb_state,
            hub::close_window_with_confirm,
            hub::get_app_version,
            hub::open_external,
            hub::start_socket_server,
            hub::stop_socket_server,
            hub::ping_server,
            hub::sign_request,
            hub::sm4_encrypt
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
