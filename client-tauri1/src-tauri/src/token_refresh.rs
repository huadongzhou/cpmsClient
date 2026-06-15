//! token 失效重取（DESIGN 需求3 通用规则）：与服务端通信遇鉴权失败时，清缓存 token、
//! 主动向 iframe 取一次新 token，若与原值不一致则重试一次。供 socket 转发与 CPMS 请求共用。

use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::iframe::emit_iframe_payload_request;
use crate::services;
use crate::AppRuntimeState;

const TOKEN_REFRESH_TIMEOUT: Duration = Duration::from_secs(10);

/// 对一次「客户端→服务端」请求套用 token 失效重取：
/// 首次失败若为鉴权失败 → 清 token → 向 iframe 取新 token → 不一致则重试一次。
/// `attempt` 每次调用都应重新读取缓存 token 后再发请求。
pub(crate) fn with_token_retry<F>(app: &AppHandle, mut attempt: F) -> Result<Value, String>
where
    F: FnMut() -> Result<Value, String>,
{
    let result = attempt();
    let Err(error) = result else {
        return result;
    };

    if !is_auth_failure_error(&error) {
        return Err(error);
    }

    services::log_service::warn(
        app,
        "token",
        "请求鉴权失败，清理缓存 token 并向 iframe 重新获取",
    );
    let old_token = services::cached_auth_token(app);
    let _ = services::clear_cached_auth_token(app);

    let Some(fresh_token) = refresh_token_via_iframe(app, TOKEN_REFRESH_TIMEOUT) else {
        services::log_service::warn(app, "token", "未能从 iframe 重新获取 token，请求保持失败");
        return Err(format!("{error}（token 已清理，且未能从 iframe 重新获取 token）"));
    };

    let _ = services::save_cached_auth_token(app, &fresh_token);

    if old_token.as_deref() == Some(fresh_token.as_str()) {
        services::log_service::info(app, "token", "重新获取的 token 与原值一致，不再重试");
        return Err(error);
    }

    services::log_service::info(app, "token", "已获取新 token，重新发起请求");
    attempt()
}

/// 判断失败是否由 token 失效（鉴权失败）引起。
pub(crate) fn is_auth_failure_error(error: &str) -> bool {
    error.contains("status=401")
        || error.contains("status=403")
        || error.contains("\"code\":401")
        || error.contains("\"code\": 401")
        || error.contains("\"code\":403")
        || error.contains("\"code\": 403")
}

/// 向视图端发起一次 iframe payload 查询，轮询等待回传并提取 token。
fn refresh_token_via_iframe(app: &AppHandle, timeout: Duration) -> Option<String> {
    let request_id = emit_iframe_payload_request(app, "token-refresh");
    let deadline = std::time::Instant::now() + timeout;

    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(250));

        let report = {
            let state: tauri::State<'_, AppRuntimeState> = app.state();
            let locked = state.iframe_payload.lock();
            locked.ok().and_then(|value| value.clone())
        };

        let Some(report) = report else {
            continue;
        };

        if report.get("requestId").and_then(Value::as_str) != Some(request_id.as_str()) {
            continue;
        }

        return report
            .get("payload")
            .and_then(|result| result.get("payload"))
            .and_then(extract_token_from_iframe_payload);
    }

    None
}

/// 从 iframe 回传的 payload 中提取 token，兼容顶层与嵌套 payload 两种位置。
fn extract_token_from_iframe_payload(payload: &Value) -> Option<String> {
    extract_token_field(payload).or_else(|| payload.get("payload").and_then(extract_token_field))
}

fn extract_token_field(value: &Value) -> Option<String> {
    ["token", "accessToken", "access_token"]
        .iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}
