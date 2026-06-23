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
    let mut user = state.user.clone();
    let pushed_token = user
        .token
        .take()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty());
    let _ = match pushed_token.as_deref() {
        Some(token) => super::save_cached_auth_token(&app, token),
        None => super::clear_cached_auth_token(&app),
    };

    let result = update_preferences(&app, |preferences| {
        preferences.user = Some(user);
        preferences.server = Some(state.server.clone());
        preferences.product_type = state.product_type;
        preferences.system_init_data = state.system_init_data.clone();
    });

    if let Err(error) = result {
        return CommandResult::fail("HUB_AUTH_SAVE_ERROR", &error);
    }

    super::log_service::info(
        &app,
        "business",
        "用户登录态已保存（token 仅保存在会话内存）",
    );
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

    let _ = super::clear_cached_auth_token(&app);
    super::log_service::info(&app, "business", "用户已登出，登录态已清理");
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
/// 设置会话级服务端地址（iframe 推送），不持久化，应用关闭即失效。
pub fn set_session_server_address(app: AppHandle, addr: String) -> CommandResult<bool> {
    match normalize_session_server_addr(&addr) {
        Some(normalized) => {
            super::session_server::set_session_server_addr(Some(normalized.clone()));
            super::log_service::info(
                &app,
                "business",
                &format!("会话服务端地址已更新: {normalized}"),
            );
            CommandResult::ok(true)
        }
        None => CommandResult::fail(
            "HUB_SESSION_SERVER_ADDR_INVALID",
            "服务端地址必须是 http:// 或 https:// 开头的有效 URL",
        ),
    }
}

#[tauri::command]
/// 清空会话级服务端地址，客户端回退到 configure.ini / 缓存地址。
pub fn clear_session_server_address(app: AppHandle) -> CommandResult<bool> {
    super::session_server::set_session_server_addr(None);
    super::log_service::info(&app, "business", "会话服务端地址已清空");
    CommandResult::ok(true)
}

#[tauri::command]
/// 获取当前会话级服务端地址（调试用）。
pub fn get_session_server_address(_app: AppHandle) -> CommandResult<Option<String>> {
    CommandResult::ok(super::session_server::session_server_addr())
}

#[tauri::command]
/// 设置会话级直连设备 ID（iframe 推送），不持久化，应用关闭即失效。
pub fn set_session_direct_device_id(app: AppHandle, device_id: String) -> CommandResult<bool> {
    let device_id = device_id.trim().to_string();
    if device_id.is_empty() {
        return CommandResult::fail("HUB_SESSION_DEVICE_ID_EMPTY", "deviceId 不能为空");
    }
    super::session_server::set_session_direct_device_id(Some(device_id.clone()));
    super::log_service::info(
        &app,
        "business",
        &format!("会话直连设备 ID 已更新: {device_id}"),
    );
    CommandResult::ok(true)
}

#[tauri::command]
/// 清空会话级直连设备 ID。
pub fn clear_session_direct_device_id(app: AppHandle) -> CommandResult<bool> {
    super::session_server::set_session_direct_device_id(None);
    super::log_service::info(&app, "business", "会话直连设备 ID 已清空");
    CommandResult::ok(true)
}

#[tauri::command]
/// 获取当前会话级直连设备 ID（调试用）。
pub fn get_session_direct_device_id(_app: AppHandle) -> CommandResult<Option<String>> {
    CommandResult::ok(super::session_server::session_direct_device_id())
}

fn normalize_session_server_addr(addr: &str) -> Option<String> {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_trailing_slash = trimmed.trim_end_matches('/');
    let (scheme, rest) = if let Some(rest) = without_trailing_slash.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = without_trailing_slash.strip_prefix("http://") {
        ("http", rest)
    } else {
        return None;
    };
    let host = rest.split('/').next().unwrap_or(rest).trim();
    if host.is_empty() {
        return None;
    }
    Some(format!("{}://{}", scheme, host))
}

#[tauri::command]
/// 兼容旧桥接入口：不再缓存 deviceId，仅同步到会话并清掉旧持久缓存。
pub fn save_direct_device(app: AppHandle, device: Value) -> CommandResult<Value> {
    let device_id = direct_device_id_from_value(&device);
    match device_id.as_deref() {
        Some(value) => super::session_server::set_session_direct_device_id(Some(value.to_string())),
        None => super::session_server::set_session_direct_device_id(None),
    }

    if let Err(error) = update_preferences(&app, |preferences| {
        preferences.auth_direct_device = None;
    }) {
        return CommandResult::fail("HUB_DIRECT_DEVICE_SAVE_ERROR", &error);
    }

    super::log_service::info(
        &app,
        "business",
        &format!(
            "旧直连设备缓存已清空，会话 deviceId 明文：{}",
            device_id.as_deref().unwrap_or("")
        ),
    );
    CommandResult::ok(device)
}

fn direct_device_id_from_value(value: &Value) -> Option<String> {
    value
        .get("deviceId")
        .or_else(|| value.get("did"))
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
}

#[tauri::command]
/// Updates only the session auth token pushed by the iframe/Web side after login.
pub fn save_auth_token(app: AppHandle, token: String) -> CommandResult<StartupState> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return CommandResult::fail("HUB_AUTH_TOKEN_EMPTY", "token 不能为空");
    }

    if let Err(error) = super::save_cached_auth_token(&app, &token) {
        return CommandResult::fail("HUB_AUTH_TOKEN_SAVE_ERROR", &error);
    }

    super::log_service::info(&app, "business", "登录 token 已更新（会话内存）");
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

    super::log_service::info(
        &app,
        "business",
        &format!("查询作业列表（页码 {page_number}，每页 {page_size}，类型 {job_type}）"),
    );
    match cpms_form_post(&app, JOB_LIST_PATH, &params) {
        Ok(value) => {
            super::log_service::info(&app, "business", "作业列表查询成功");
            CommandResult::ok(value)
        }
        Err(error) => {
            super::log_service::error(&app, "business", &format!("作业列表查询失败：{error}"));
            CommandResult::fail("HUB_JOB_LIST_ERROR", &error)
        }
    }
}

#[tauri::command]
/// Fetches CPMS direct-output printer devices available to the current user.
pub fn get_available_devices(app: AppHandle) -> CommandResult<Value> {
    super::log_service::info(&app, "business", "查询可用直连设备列表");
    match cpms_get(&app, DEVICE_LIST_PATH) {
        Ok(value) => {
            super::log_service::info(&app, "business", "设备列表查询成功");
            CommandResult::ok(value)
        }
        Err(error) => {
            super::log_service::error(&app, "business", &format!("设备列表查询失败：{error}"));
            CommandResult::fail("HUB_DEVICE_LIST_ERROR", &error)
        }
    }
}

#[tauri::command]
/// Updates the selected direct-output device on CPMS and keeps it in session memory.
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

    super::log_service::info(
        &app,
        "business",
        &format!("选择直连设备 deviceId={device_id}"),
    );
    let params = vec![("deviceId".into(), device_id)];
    if let Err(error) = cpms_form_post(&app, UPDATE_DIRECT_DEVICE_PATH, &params) {
        super::log_service::error(&app, "business", &format!("设备选择失败：{error}"));
        return CommandResult::fail("HUB_DIRECT_DEVICE_UPDATE_ERROR", &error);
    }

    super::session_server::set_session_direct_device_id(
        direct_device_id_from_value(&device).or_else(|| {
            params
                .first()
                .map(|(_, value)| value.clone())
                .filter(|value| !value.trim().is_empty())
        }),
    );

    if let Err(error) = update_preferences(&app, |preferences| {
        preferences.auth_direct_device = None;
    }) {
        return CommandResult::fail("HUB_DIRECT_DEVICE_SAVE_ERROR", &error);
    }

    super::log_service::info(&app, "business", "设备选择成功，旧直连设备缓存已清空");
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

    let should_start = has_auth_token(&app);
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

    match tauri_plugin_opener::open_url(&url, None::<&str>) {
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

fn has_auth_token(app: &AppHandle) -> bool {
    super::cached_auth_token(app)
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

    super::log_service::http_request(
        app,
        "CPMS GET",
        "GET",
        &url,
        &super::log_service::format_headers_for_log(&headers),
        "",
    );

    let client = cpms_client()?;
    let mut request = client.get(url);

    for (key, value) in headers {
        request = request.header(key, value);
    }

    let response = match request.send() {
        Ok(response) => response,
        Err(error) => {
            super::log_service::http_error(app, "CPMS GET", &error.to_string());
            return Err(error.to_string());
        }
    };
    read_cpms_response(app, "CPMS GET", response)
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

    super::log_service::http_request(
        app,
        "CPMS POST",
        "POST",
        &url,
        &super::log_service::format_headers_for_log(&headers),
        &body,
    );

    let client = cpms_client()?;
    let mut request = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body);

    for (key, value) in headers {
        request = request.header(key, value);
    }

    let response = match request.send() {
        Ok(response) => response,
        Err(error) => {
            super::log_service::http_error(app, "CPMS POST", &error.to_string());
            return Err(error.to_string());
        }
    };
    read_cpms_response(app, "CPMS POST", response)
}

fn load_server_user(app: &AppHandle) -> Result<(ServerData, UserData), String> {
    let preferences = load_preferences(app)?;
    // 域名已由 configure.ini 的 ServerAddr 提供（build_cpms_url 优先用它），不强依赖 ServerData。
    let server = preferences.server.unwrap_or_default();
    let mut user = preferences.user.ok_or_else(|| "用户未登录".to_string())?;
    let token = super::cached_auth_token(app).unwrap_or_default();
    let token = token.trim();

    if token.is_empty() {
        return Err("用户 token 为空".into());
    }
    user.token = Some(token.to_string());

    Ok((server, user))
}

fn cpms_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(http_service::allow_insecure_tls())
        .build()
        .map_err(|error| error.to_string())
}

fn read_cpms_response(
    app: &AppHandle,
    label: &str,
    response: reqwest::blocking::Response,
) -> Result<Value, String> {
    let status = response.status();
    let body = response.text().unwrap_or_default();

    super::log_service::http_response(app, label, status.as_u16(), &body);

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
