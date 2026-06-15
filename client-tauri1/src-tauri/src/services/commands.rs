use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::result::CommandResult;

use super::crypto_service;
use super::events::{emit_background_state, emit_hub_state};
use super::http_service;
use super::models::{
    startup_state_from_preferences, AppVersion, AuthPersistState, ServerData, StartupState,
    UserData,
};
use super::preferences::{load_preferences, update_preferences};

const JOB_LIST_PATH: &str = "/cpms/api/jobs/list";
const DEVICE_LIST_PATH: &str = "/cpms/api/userManager/listAvailDevices";
const UPDATE_DIRECT_DEVICE_PATH: &str = "/cpms/api/userManager/updateDirectDeviceId";

#[tauri::command]
/// Reads the persisted Hub startup state used by the Web app during route hydration.
pub fn get_startup_state(app: AppHandle) -> CommandResult<StartupState> {
    let preferences = match load_preferences(&app) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_PREFERENCES_READ_ERROR", &error),
    };

    CommandResult::ok(startup_state_from_preferences(preferences))
}

#[tauri::command]
/// Persists that the user accepted the privacy policy.
pub fn save_policy_agreed(app: AppHandle) -> CommandResult<bool> {
    update_preferences(&app, |preferences| {
        preferences.policy_agreed = true;
    })
    .map_or_else(
        |error| CommandResult::fail("HUB_POLICY_SAVE_ERROR", &error),
        |_| CommandResult::ok(true),
    )
}

#[tauri::command]
/// Persists authenticated user, server, product type, and optional server init data.
pub fn save_auth_state(app: AppHandle, state: AuthPersistState) -> CommandResult<StartupState> {
    let result = update_preferences(&app, |preferences| {
        preferences.user = Some(state.user.clone());
        preferences.server = Some(state.server.clone());
        preferences.product_type = state.product_type;
        preferences.system_init_data = state.system_init_data.clone();
    });

    if let Err(error) = result {
        return CommandResult::fail("HUB_AUTH_SAVE_ERROR", &error);
    }

    super::log_service::info(&app, "lifecycle", "登录态已保存");
    load_and_emit_startup_state(&app, "HUB_PREFERENCES_READ_ERROR")
}

#[tauri::command]
/// Clears authentication-related local state while keeping reusable non-auth settings.
pub fn clear_auth_state(app: AppHandle) -> CommandResult<StartupState> {
    let result = update_preferences(&app, |preferences| {
        preferences.user = None;
        preferences.product_type = -1;
        preferences.system_init_data = None;
        preferences.auth_direct_device = None;
    });

    if let Err(error) = result {
        return CommandResult::fail("HUB_AUTH_CLEAR_ERROR", &error);
    }

    super::log_service::info(&app, "lifecycle", "登录态已清理");
    load_and_emit_startup_state(&app, "HUB_PREFERENCES_READ_ERROR")
}

#[tauri::command]
/// Saves the latest CPMS server endpoint selected by the user.
pub fn save_server_info(app: AppHandle, server: ServerData) -> CommandResult<ServerData> {
    update_preferences(&app, |preferences| {
        preferences.server = Some(server.clone());
    })
    .map_or_else(
        |error| CommandResult::fail("HUB_SERVER_SAVE_ERROR", &error),
        |_| CommandResult::ok(server),
    )
}

#[tauri::command]
/// Saves the direct-output printer selected by the user.
pub fn save_direct_device(app: AppHandle, device: Value) -> CommandResult<Value> {
    update_preferences(&app, |preferences| {
        preferences.auth_direct_device = Some(device.clone());
    })
    .map_or_else(
        |error| CommandResult::fail("HUB_DIRECT_DEVICE_SAVE_ERROR", &error),
        |_| CommandResult::ok(device),
    )
}

#[tauri::command]
/// Updates only the cached auth token pushed by the iframe/Web side after login.
pub fn save_auth_token(app: AppHandle, token: String) -> CommandResult<StartupState> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return CommandResult::fail("HUB_AUTH_TOKEN_EMPTY", "token 不能为空");
    }

    let result = update_preferences(&app, |preferences| {
        if let Some(user) = preferences.user.as_mut() {
            user.token = Some(token.clone());
        } else {
            preferences.user = Some(UserData {
                token: Some(token.clone()),
                ..UserData::default()
            });
        }
    });

    if let Err(error) = result {
        return CommandResult::fail("HUB_AUTH_TOKEN_SAVE_ERROR", &error);
    }

    super::log_service::info(&app, "lifecycle", "缓存 token 已更新");
    load_and_emit_startup_state(&app, "HUB_PREFERENCES_READ_ERROR")
}

#[tauri::command]
/// Fetches the current user's CPMS job list.
pub fn get_job_list(
    app: AppHandle,
    page_number: i64,
    page_size: i64,
    job_type: i64,
    title: Option<String>,
    search_time: Option<String>,
) -> CommandResult<Value> {
    let params = vec![
        ("pageNumber".into(), page_number.max(1).to_string()),
        ("pageSize".into(), page_size.max(1).to_string()),
        ("type".into(), job_type.to_string()),
        ("title".into(), title.unwrap_or_default()),
        ("searchTime".into(), search_time.unwrap_or_default()),
    ];

    match cpms_form_post(&app, JOB_LIST_PATH, &params) {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("HUB_JOB_LIST_ERROR", &error),
    }
}

#[tauri::command]
/// Fetches CPMS direct-output printer devices available to the current user.
pub fn get_available_devices(app: AppHandle) -> CommandResult<Value> {
    match cpms_get(&app, DEVICE_LIST_PATH) {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("HUB_DEVICE_LIST_ERROR", &error),
    }
}

#[tauri::command]
/// Updates the selected direct-output device on CPMS and persists it locally.
pub fn select_direct_device(app: AppHandle, device: Value) -> CommandResult<Value> {
    let Some(device_id) = device
        .get("deviceId")
        .or_else(|| device.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
    else {
        return CommandResult::fail("HUB_DIRECT_DEVICE_ID_EMPTY", "deviceId 不能为空");
    };

    let params = vec![("deviceId".into(), device_id)];
    if let Err(error) = cpms_form_post(&app, UPDATE_DIRECT_DEVICE_PATH, &params) {
        return CommandResult::fail("HUB_DIRECT_DEVICE_UPDATE_ERROR", &error);
    }

    if let Err(error) = update_preferences(&app, |preferences| {
        preferences.auth_direct_device = Some(device.clone());
    }) {
        return CommandResult::fail("HUB_DIRECT_DEVICE_SAVE_ERROR", &error);
    }

    CommandResult::ok(json!({
        "success": true,
        "code": "OK",
        "message": "success",
        "data": device,
        "logs": [],
    }))
}

#[tauri::command]
/// Initializes Hub system capabilities (network monitor) and emits runtime state.
pub fn system_init(app: AppHandle) -> CommandResult<StartupState> {
    let preferences = match load_preferences(&app) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_SYSTEM_INIT_ERROR", &error),
    };

    let should_start = has_auth_token(&preferences.user);
    let startup_state = startup_state_from_preferences(preferences);

    if should_start {
        if let Err(error) = super::network_service::start_network_monitor(app.clone()) {
            return CommandResult::fail("HUB_NETWORK_MONITOR_START_ERROR", &error);
        }
    }

    super::log_service::info(
        &app,
        "lifecycle",
        &format!("系统能力初始化完成（已登录：{should_start}）"),
    );
    emit_hub_state(&app, &startup_state);
    CommandResult::ok(startup_state)
}

#[tauri::command]
/// Releases Hub system capabilities before logout, close, or shutdown.
pub fn system_destroy(app: AppHandle) -> CommandResult<bool> {
    let _ = super::network_service::stop_network_monitor();
    super::log_service::info(&app, "lifecycle", "系统能力已销毁");
    CommandResult::ok(true)
}

#[tauri::command]
/// Starts background workers (network monitor) and emits a background-running state event.
pub fn start_background_tasks(app: AppHandle) -> CommandResult<bool> {
    if let Err(error) = super::network_service::start_network_monitor(app.clone()) {
        return CommandResult::fail("HUB_NETWORK_MONITOR_START_ERROR", &error);
    }

    super::log_service::info(&app, "lifecycle", "后台任务已启动");
    emit_background_state(&app, true, now_millis());
    CommandResult::ok(true)
}

#[tauri::command]
/// Stops background workers (network monitor) and emits a background-stopped state event.
pub fn stop_background_tasks(app: AppHandle) -> CommandResult<bool> {
    if let Err(error) = super::network_service::stop_network_monitor() {
        return CommandResult::fail("HUB_NETWORK_MONITOR_STOP_ERROR", &error);
    }

    super::log_service::info(&app, "lifecycle", "后台任务已停止");
    emit_background_state(&app, false, now_millis());
    CommandResult::ok(true)
}

#[tauri::command]
/// Closes the application after releasing system resources.
/// Web should show a confirmation dialog before invoking this command.
pub async fn close_window_with_confirm(app: AppHandle) -> CommandResult<bool> {
    let _ = system_destroy(app.clone());
    app.exit(0);
    CommandResult::ok(true)
}

#[tauri::command]
/// Returns the current application version.
pub fn get_app_version() -> CommandResult<AppVersion> {
    CommandResult::ok(AppVersion {
        version: env!("CARGO_PKG_VERSION").into(),
        build_number: env!("CARGO_PKG_VERSION").into(),
    })
}

#[tauri::command]
/// Opens a URL in the system default browser.
pub async fn open_external(url: String) -> CommandResult<bool> {
    if url.trim().is_empty() {
        return CommandResult::fail("OPEN_EXTERNAL_EMPTY", "url 不能为空");
    }

    match open::that(&url) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("OPEN_EXTERNAL_ERROR", &error.to_string()),
    }
}

#[tauri::command]
/// Generates an access_sign-compatible value for a CPMS request.
pub fn sign_request(uri: String, params: String) -> CommandResult<String> {
    match crypto_service::sign_request(&uri, &params) {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("HUB_SIGN_ERROR", &error),
    }
}

#[tauri::command]
/// 接收前端（视图端 / iframe 业务端）推送的日志，写入客户端日志文件。
pub fn push_client_log(
    level: Option<String>,
    source: Option<String>,
    message: String,
    detail: Option<String>,
) -> CommandResult<bool> {
    let message = message.trim().to_string();
    if message.is_empty() {
        return CommandResult::fail("CLIENT_LOG_EMPTY", "message 不能为空");
    }

    let level = normalize_log_level(level.as_deref());
    let source = source
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "frontend".into());

    super::log_service::log_from_frontend(&level, &source, &message, detail.as_deref());
    CommandResult::ok(true)
}

#[tauri::command]
/// 返回客户端日志文件路径与当前大小，供调试面板展示。
pub fn get_client_log_state() -> CommandResult<Value> {
    match super::log_service::current_state() {
        Some((path, size)) => CommandResult::ok(json!({
            "path": path.to_string_lossy(),
            "sizeBytes": size,
        })),
        None => CommandResult::fail("CLIENT_LOG_UNINITIALIZED", "日志系统尚未初始化"),
    }
}

fn normalize_log_level(level: Option<&str>) -> String {
    match level.unwrap_or("info").trim().to_lowercase().as_str() {
        "warn" | "warning" => "WARN".into(),
        "error" => "ERROR".into(),
        "debug" => "DEBUG".into(),
        _ => "INFO".into(),
    }
}

fn load_and_emit_startup_state(app: &AppHandle, error_code: &str) -> CommandResult<StartupState> {
    let startup_state = match load_preferences(app) {
        Ok(value) => startup_state_from_preferences(value),
        Err(error) => return CommandResult::fail(error_code, &error),
    };

    emit_hub_state(app, &startup_state);
    CommandResult::ok(startup_state)
}

fn has_auth_token(user: &Option<super::models::UserData>) -> bool {
    user.as_ref()
        .and_then(|user| user.token.as_deref())
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false)
}

// 作业/设备/选机等 CPMS 请求统一套用 token 失效重取（需求3 通用规则）。
fn cpms_get(app: &AppHandle, path: &str) -> Result<Value, String> {
    crate::token_refresh::with_token_retry(app, || cpms_get_once(app, path))
}

fn cpms_get_once(app: &AppHandle, path: &str) -> Result<Value, String> {
    let (server, user) = load_server_user(app)?;
    let url = http_service::build_cpms_url(&server, path)?;
    let token = user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), path, "")?;
    let client = cpms_client()?;
    let mut request = client.get(url);

    for (key, value) in headers {
        request = request.header(key, value);
    }

    read_cpms_response(request.send().map_err(|error| error.to_string())?)
}

fn cpms_form_post(
    app: &AppHandle,
    path: &str,
    params: &[(String, String)],
) -> Result<Value, String> {
    crate::token_refresh::with_token_retry(app, || cpms_form_post_once(app, path, params))
}

fn cpms_form_post_once(
    app: &AppHandle,
    path: &str,
    params: &[(String, String)],
) -> Result<Value, String> {
    let (server, user) = load_server_user(app)?;
    let url = http_service::build_cpms_url(&server, path)?;
    let sign_params = http_service::query_string(params, false);
    let token = user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), path, &sign_params)?;
    let body = http_service::query_string(params, true);
    let client = cpms_client()?;
    let mut request = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body);

    for (key, value) in headers {
        request = request.header(key, value);
    }

    read_cpms_response(request.send().map_err(|error| error.to_string())?)
}

fn load_server_user(app: &AppHandle) -> Result<(ServerData, UserData), String> {
    let preferences = load_preferences(app)?;
    let server = preferences
        .server
        .ok_or_else(|| "服务器未配置".to_string())?;
    let user = preferences.user.ok_or_else(|| "用户未登录".to_string())?;
    let token = user.token.as_deref().unwrap_or_default().trim();

    if token.is_empty() {
        return Err("用户 token 为空".into());
    }

    Ok((server, user))
}

fn cpms_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(http_service::allow_insecure_tls())
        .build()
        .map_err(|error| error.to_string())
}

fn read_cpms_response(response: reqwest::blocking::Response) -> Result<Value, String> {
    let status = response.status();
    let body = response.text().unwrap_or_default();

    if !status.is_success() {
        return Err(format!("CPMS 请求失败，HTTP status={status}，body={body}"));
    }

    serde_json::from_str::<Value>(&body).map_err(|error| error.to_string())
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default()
}
