use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::{multipart, Client};
use serde_json::{json, Value};
use tauri::AppHandle;

use super::http_service;
use super::models::{HubPreferences, ServerData, UserData};
use super::preferences::load_preferences;

const UPLOAD_EXEC_PATH: &str = "/cpms/api/jobs/xps/exec";
const DEFAULT_CLIENT_IP: &str = "127.0.0.1";

struct UploadContext {
    server: ServerData,
    user: UserData,
    product_type: i32,
    auth_direct_device: Option<Value>,
}

/// 转发本地 PrintClient 经 websocket 推送的打印任务到线上服务。
pub fn forward_socket_task_message(app: AppHandle, message: &str) -> Result<Value, String> {
    let preferences = load_preferences(&app)?;
    let Some(context) = build_upload_context(preferences) else {
        return Err("用户未登录或服务器未配置，无法转发打印任务".into());
    };
    let task_payload = parse_socket_task_payload(message)?;
    let file_path = task_payload
        .get("filePath")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| "socket 任务缺少 filePath".to_string())?;

    if !file_path.exists() {
        return Err(format!(
            "socket 任务文件不存在: {}",
            file_path.to_string_lossy()
        ));
    }

    upload_print_payload(&file_path, &task_payload, &context)?;

    Ok(json!({
        "filePath": file_path,
        "documentName": task_payload
            .get("printProperties")
            .and_then(|value| value.get("documentName"))
            .and_then(Value::as_str),
    }))
}

fn build_upload_context(preferences: HubPreferences) -> Option<UploadContext> {
    let user = preferences.user?;
    let token = user.token.as_deref()?.trim();
    if token.is_empty() {
        return None;
    }

    Some(UploadContext {
        server: preferences.server?,
        user,
        product_type: preferences.product_type,
        auth_direct_device: preferences.auth_direct_device,
    })
}

fn parse_socket_task_payload(message: &str) -> Result<Value, String> {
    let parsed = serde_json::from_str::<Value>(message).map_err(|error| error.to_string())?;
    match parsed {
        Value::String(raw) => {
            serde_json::from_str::<Value>(&raw).map_err(|error| error.to_string())
        }
        Value::Object(_) => Ok(parsed),
        _ => Err("socket 任务消息不是 JSON 对象".into()),
    }
}

fn upload_print_payload(
    file_path: &Path,
    param: &Value,
    context: &UploadContext,
) -> Result<(), String> {
    let params = build_print_query_params(param, context);
    let sign_query = http_service::query_string(&params, false);
    let url = format!(
        "{}?{}",
        http_service::build_cpms_url(&context.server, UPLOAD_EXEC_PATH)?,
        http_service::query_string(&params, true)
    );
    let token = context.user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), UPLOAD_EXEC_PATH, &sign_query)?;

    let form = multipart::Form::new()
        .file("file", file_path)
        .map_err(|error| error.to_string())?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30 * 60))
        .danger_accept_invalid_certs(http_service::allow_insecure_tls())
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.post(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let response = request
        .multipart(form)
        .send()
        .map_err(|error| error.to_string())?;

    let status = response.status();
    let body = response.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("上传失败，HTTP status={status}，body={body}"));
    }

    if let Ok(payload) = serde_json::from_str::<Value>(&body) {
        let code = payload.get("code").and_then(Value::as_i64);
        if !matches!(code, Some(200) | None) {
            return Err(format!("上传失败，服务端响应={payload}"));
        }
    }

    Ok(())
}

fn build_print_query_params(param: &Value, context: &UploadContext) -> Vec<(String, String)> {
    let print_properties = param.get("printProperties").unwrap_or(param);
    let document_name =
        normalized_document_name(text_field(print_properties, "documentName", "print.pdf"));
    let paper = text_field(print_properties, "paper", "A4");
    let paper = if paper.starts_with("ISO") {
        paper
    } else {
        format!("ISO{paper}")
    };
    let duplexing = text_field(print_properties, "duplexing", "TwoSided");
    let duplexing = if duplexing == "None" {
        "TwoSided".into()
    } else {
        duplexing
    };

    let mut params = vec![
        ("fileSuffix".into(), "pdf".into()),
        ("driverType".into(), "pdf".into()),
        (
            "clientIp".into(),
            text_field(print_properties, "clientIp", DEFAULT_CLIENT_IP),
        ),
        ("printProperties.driverName".into(), "PdfDriver".into()),
        ("printProperties.portShared".into(), "0".into()),
        ("printProperties.terminalType".into(), "harmony".into()),
        (
            "printProperties.pageCount".into(),
            text_field(print_properties, "pageCount", "1"),
        ),
        (
            "printProperties.copyCount".into(),
            text_field(print_properties, "copyCount", "1"),
        ),
        ("printProperties.paper".into(), paper),
        ("printProperties.duplexing".into(), duplexing),
        (
            "printProperties.color".into(),
            text_field(print_properties, "color", "Color"),
        ),
        (
            "printProperties.pageOrientation".into(),
            text_field(print_properties, "pageOrientation", "Portrait"),
        ),
        (
            "printProperties.documentCollate".into(),
            text_field(print_properties, "documentCollate", "Uncollate"),
        ),
        ("printProperties.isPSDriver".into(), "true".into()),
        ("title".into(), document_name.clone()),
        ("printProperties.documentName".into(), document_name),
    ];

    if let Some(device_id) = direct_device_id(context.auth_direct_device.as_ref()) {
        params.push(("directDeviceId".into(), device_id));
    }
    params.push(("productType".into(), context.product_type.to_string()));

    params
}

fn text_field(value: &Value, key: &str, default_value: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_value.into())
}

fn normalized_document_name(value: String) -> String {
    let mut next = value.replace(['#', '?', '&', '='], "");
    if !next.to_lowercase().ends_with(".pdf") {
        next.push_str(".pdf");
    }
    next
}

fn direct_device_id(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|raw| {
            raw.get("did")
                .or_else(|| raw.get("deviceId"))
                .or_else(|| raw.get("id"))
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
}
