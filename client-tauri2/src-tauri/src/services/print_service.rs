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
}

/// 转发本地 PrintClient 经 websocket 推送的打印任务到线上服务。
pub fn forward_socket_task_message(app: AppHandle, message: &str) -> Result<Value, String> {
    let task_payload = parse_socket_task_payload(message)?;
    let file_path = task_payload
        .get("filePath")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| "socket 任务缺少 filePath".to_string())?;

    let preferences = load_preferences(&app)?;
    let token = super::cached_auth_token(&app);
    let Some(context) = build_upload_context(preferences, token) else {
        // 未登录（无会话 token）：此时签名头无法构建，只记错误（带任务参数）；
        // 该错误会被 with_token_retry 识别为鉴权失败 → 自动向 iframe 取 token 后重试。
        let reason = "未登录：无会话 token，无法转发打印任务";
        super::log_service::http_error(
            &app,
            "打印上传",
            &format!("{reason} | 任务参数: {task_payload}"),
        );
        return Err(reason.into());
    };

    if !file_path.exists() {
        let reason = format!("socket 任务文件不存在: {}", file_path.to_string_lossy());
        super::log_service::http_error(
            &app,
            "打印上传",
            &format!("{reason} | 任务参数: {task_payload}"),
        );
        return Err(reason);
    }

    super::log_service::info(
        &app,
        "socket",
        &format!(
            "转发打印任务 → 使用 token {}，文件 {}",
            context.user.token.as_deref().unwrap_or_default(),
            file_path.to_string_lossy()
        ),
    );

    upload_print_payload(&app, &file_path, &task_payload, &context)?;

    Ok(json!({
        "filePath": file_path,
        "documentName": task_payload
            .get("printProperties")
            .and_then(|value| value.get("documentName"))
            .and_then(Value::as_str),
    }))
}

fn build_upload_context(
    preferences: HubPreferences,
    token: Option<String>,
) -> Option<UploadContext> {
    let mut user = preferences.user.unwrap_or_default();
    let token = token?;
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    user.token = Some(token.to_string());

    Some(UploadContext {
        // 域名已由 configure.ini 的 ServerAddr 提供（build_cpms_url 优先用它），不强依赖 ServerData。
        server: preferences.server.unwrap_or_default(),
        user,
        product_type: preferences.product_type,
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
    app: &AppHandle,
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

    // 请求发起：记完整最终 URL（含 query）+ 完整请求头 + multipart 文件名。
    super::log_service::http_request(
        app,
        "打印上传",
        "POST",
        &url,
        &super::log_service::format_headers_for_log(&headers),
        &format!("multipart：file={}", file_path.to_string_lossy()),
    );

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
    let response = match request.multipart(form).send() {
        Ok(response) => response,
        Err(error) => {
            super::log_service::http_error(app, "打印上传", &error.to_string());
            return Err(error.to_string());
        }
    };

    let status = response.status();
    let body = response.text().unwrap_or_default();
    super::log_service::http_response(app, "打印上传", status.as_u16(), &body);
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
        ("printProperties.terminalType".into(), "windows".into()),
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

    if let Some(device_id) = super::session_server::session_direct_device_id() {
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
