use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};
use tauri::AppHandle;

use super::crypto_service;
use super::models::ServerData;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientHttpRequest {
    pub method: Option<String>,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub query: Option<HashMap<String, Value>>,
    pub body: Option<Value>,
    pub timeout_ms: Option<u64>,
}

/// 是否放开 TLS 证书校验：默认 false（校验），仅当 env `CPMS_ALLOW_INSECURE_TLS`
/// 为 1/true/yes 时放开，供内网自签名服务按需启用。
pub(crate) fn allow_insecure_tls() -> bool {
    std::env::var("CPMS_ALLOW_INSECURE_TLS")
        .map(|value| matches!(value.trim().to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

/// Executes the generic Web-to-client HTTP proxy request.
pub async fn execute_client_http_request(
    app: &AppHandle,
    request: ClientHttpRequest,
) -> Result<Value, String> {
    let method_str = request.method.as_deref().unwrap_or("GET").to_string();
    let method = method_str
        .parse::<reqwest::Method>()
        .map_err(|_| "method 非法".to_string())?;
    let url = request.url.clone();
    let timeout = Duration::from_millis(request.timeout_ms.unwrap_or(15_000));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(allow_insecure_tls())
        .build()
        .map_err(|error| error.to_string())?;
    let mut builder = client.request(method, &url);

    let mut header_log = String::new();
    if let Some(headers) = request.headers {
        let pairs: Vec<(String, String)> = headers.into_iter().collect();
        header_log = super::log_service::format_headers_for_log(&pairs);
        for (key, value) in pairs {
            builder = builder.header(key, value);
        }
    }

    // 记录请求参数（query/body）。
    let mut params = String::new();
    if let Some(query) = request.query {
        let normalized: Vec<(String, String)> = query
            .into_iter()
            .filter_map(|(key, value)| stringify_query_value(value).map(|next| (key, next)))
            .collect();

        if !normalized.is_empty() {
            params.push_str(&format!("query={normalized:?}"));
            builder = builder.query(&normalized);
        }
    }

    if let Some(body) = request.body {
        params.push_str(&format!("；body={body}"));
        builder = builder.json(&body);
    }

    super::log_service::http_request(app, "代理请求", &method_str, &url, &header_log, &params);

    let response = match builder.send().await {
        Ok(value) => value,
        Err(error) => {
            super::log_service::http_error(app, "代理请求", &error.to_string());
            return Err(error.to_string());
        }
    };
    let status_code = response.status().as_u16();
    let response_text = match response.text().await {
        Ok(value) => value,
        Err(error) => {
            super::log_service::http_error(app, "代理请求", &error.to_string());
            return Err(error.to_string());
        }
    };
    super::log_service::http_response(app, "代理请求", status_code, &response_text);
    let response_payload = parse_response_text(response_text);

    if status_code >= 400 {
        return Err(format!("客户端代理请求失败，status={status_code}"));
    }

    Ok(json!({
        "status": status_code,
        "data": response_payload,
    }))
}

/// Builds a CPMS absolute URL from persisted server data and an API path.
pub fn build_cpms_url(server: &ServerData, path: &str) -> Result<String, String> {
    // 优先使用 PrintClient 安装目录 configure.ini 的 ServerAddr 作为服务端域名；
    // 未发现时回退到 preferences 里登录配置的 ServerData。
    if let Some(base) = crate::printclient::cpms_server_base() {
        return Ok(format!("{base}{path}"));
    }

    let scheme = if server.https { "https" } else { "http" };
    let server = server
        .server
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');

    if server.is_empty() {
        return Err("服务端地址为空".into());
    }

    Ok(format!("{scheme}://{server}{path}"))
}

/// Builds standard CPMS headers with token, signature, client, and platform fields.
pub fn build_signed_headers(
    token: Option<&str>,
    uri: &str,
    params: &str,
) -> Result<Vec<(String, String)>, String> {
    let mut headers = vec![
        (
            "access_sign".into(),
            crypto_service::sign_request(uri, params)?,
        ),
        ("client".into(), "client".into()),
        ("platform".into(), "windows".into()),
    ];

    if let Some(token) = token.map(str::trim).filter(|value| !value.is_empty()) {
        headers.push(("Authorization".into(), token.into()));
    }

    Ok(headers)
}

/// Serializes string pairs to query format. Values are encoded when used in URLs.
pub fn query_string(params: &[(String, String)], encode: bool) -> String {
    params
        .iter()
        .map(|(key, value)| {
            if encode {
                format!(
                    "{}={}",
                    urlencoding::encode(key),
                    urlencoding::encode(value)
                )
            } else {
                format!("{key}={value}")
            }
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn parse_response_text(response_text: String) -> Value {
    if response_text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str::<Value>(&response_text).unwrap_or(Value::String(response_text))
    }
}

fn stringify_query_value(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(raw) => Some(raw.to_string()),
        Value::Number(raw) => Some(raw.to_string()),
        Value::String(raw) => Some(raw),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
    }
}
