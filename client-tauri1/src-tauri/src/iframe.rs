//! iframe 容器：地址获取/校验/回退、运行时状态缓存、payload(token) 查询请求。

use std::time::Duration;

use reqwest::Url;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::result::CommandResult;
use crate::services;
use crate::{
    now_iso_string, AppRuntimeState, ClientEventPayload, ClientIframeEventPayload,
    CLIENT_IFRAME_EVENT, CLIENT_IFRAME_PAYLOAD_REPORT_EVENT, CLIENT_IFRAME_PAYLOAD_REQUEST_EVENT,
    CLIENT_TO_VIEW_EVENT, DEFAULT_CPMS_BASE_URL, DEFAULT_IFRAME_CONFIG_PATH,
    DEFAULT_IFRAME_FALLBACK_URL, MAIN_WINDOW_LABEL,
};

#[tauri::command]
pub fn client_get_iframe_container_state(
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
pub async fn client_refresh_iframe_container(
    app: AppHandle,
) -> CommandResult<ClientIframeEventPayload> {
    CommandResult::ok(refresh_iframe_container(&app).await)
}

#[tauri::command]
pub fn client_request_iframe_payload(
    app: AppHandle,
    reason: Option<String>,
) -> CommandResult<String> {
    let request_id = emit_iframe_payload_request(&app, reason.as_deref().unwrap_or("manual"));
    CommandResult::ok(request_id)
}

#[tauri::command]
pub fn client_submit_iframe_payload(
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

pub(crate) fn emit_iframe_payload_request(app: &AppHandle, reason: &str) -> String {
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

pub(crate) fn initial_iframe_state() -> ClientIframeEventPayload {
    ClientIframeEventPayload {
        state: "idle".into(),
        url: None,
        message: None,
        updated_at: now_iso_string(),
    }
}

fn cpms_base_url() -> String {
    std::env::var("CPMS_BASE_URL").unwrap_or_else(|_| DEFAULT_CPMS_BASE_URL.into())
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

fn fallback_iframe_state(app: &AppHandle, reason: String) -> ClientIframeEventPayload {
    services::log_service::warn(
        app,
        "iframe",
        &format!("iframe 地址获取失败，回退默认地址：{reason}"),
    );
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

pub(crate) async fn refresh_iframe_container(app: &AppHandle) -> ClientIframeEventPayload {
    services::log_service::info(app, "iframe", "开始获取 iframe 容器地址");
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
        Ok(valid_url) => {
            services::log_service::info(
                app,
                "iframe",
                &format!("iframe 容器地址获取成功：{valid_url}"),
            );
            update_iframe_state(app, "loaded", Some(valid_url), None)
        }
        Err(message) => fallback_iframe_state(app, message),
    }
}
