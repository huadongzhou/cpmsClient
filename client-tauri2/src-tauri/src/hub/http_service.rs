use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

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

/// Executes the generic Web-to-client HTTP proxy request.
pub async fn execute_client_http_request(request: ClientHttpRequest) -> Result<Value, String> {
    let method = request
        .method
        .as_deref()
        .unwrap_or("GET")
        .parse::<reqwest::Method>()
        .map_err(|_| "method 非法".to_string())?;
    let timeout = Duration::from_millis(request.timeout_ms.unwrap_or(15_000));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|error| error.to_string())?;
    let mut builder = client.request(method, &request.url);

    if let Some(headers) = request.headers {
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
    }

    if let Some(query) = request.query {
        let normalized: Vec<(String, String)> = query
            .into_iter()
            .filter_map(|(key, value)| stringify_query_value(value).map(|next| (key, next)))
            .collect();

        if !normalized.is_empty() {
            builder = builder.query(&normalized);
        }
    }

    if let Some(body) = request.body {
        builder = builder.json(&body);
    }

    let response = builder.send().await.map_err(|error| error.to_string())?;
    let status_code = response.status().as_u16();
    let response_text = response.text().await.map_err(|error| error.to_string())?;
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
        ("platform".into(), "harmony".into()),
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
