use std::error::Error;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use reqwest::blocking::Client;
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
    platform: String,
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
    let Some(context) = build_upload_context(&app, preferences, token) else {
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
            "转发打印任务 → 使用 token {}，平台 {}，文件 {}",
            context.user.token.as_deref().unwrap_or_default(),
            context.platform,
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
    app: &AppHandle,
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

    // 平台标识优先使用 iframe 通过 cpms:platform 推送的会话缓存，未推送时默认 windows。
    let platform = super::cached_platform(app).unwrap_or_else(|| "windows".to_string());

    Some(UploadContext {
        // 域名已由 configure.ini 的 ServerAddr 提供（build_cpms_url 优先用它），不强依赖 ServerData。
        server: preferences.server.unwrap_or_default(),
        user,
        product_type: preferences.product_type,
        platform,
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
    let headers = http_service::build_signed_headers(
        Some(token),
        Some(&context.platform),
        UPLOAD_EXEC_PATH,
        &sign_query,
    )?;

    let filename = file_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "print.pdf".into());
    let boundary = format!(
        "----cpmsBoundary{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let gzip_body = build_gzip_multipart_body(file_path, &filename, &boundary)?;

    // 请求发起：记完整最终 URL（含 query）+ 完整请求头 + multipart 文件名。
    super::log_service::http_request(
        app,
        "打印上传",
        "POST",
        &url,
        &super::log_service::format_headers_for_log(&headers),
        &format!(
            "multipart(gzip)：file={}，filename={}，bodyBytes={}",
            file_path.to_string_lossy(),
            filename,
            gzip_body.len()
        ),
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(30 * 60))
        .connect_timeout(Duration::from_secs(10))
        .http1_only()
        .danger_accept_invalid_certs(http_service::allow_insecure_tls())
        .build()
        .map_err(|error| format_reqwest_error(&error))?;
    let mut request = client
        .post(url)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header("Content-Encoding", "gzip")
        .header("Content-Length", gzip_body.len().to_string())
        .body(gzip_body);
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let response = match request.send() {
        Ok(response) => response,
        Err(error) => {
            let detail = format_reqwest_error(&error);
            super::log_service::http_error(app, "打印上传", &detail);
            return Err(detail);
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
        (
            "printProperties.terminalType".into(),
            context.platform.clone(),
        ),
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

fn format_reqwest_error(error: &reqwest::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(err) = source {
        message.push_str(&format!(" | {err}"));
        source = err.source();
    }
    message
}

/// 构造完整的 multipart/form-data 请求体后使用 gzip 压缩，与鸿蒙端 `Content-Encoding: gzip` 对齐。
fn build_gzip_multipart_body(
    file_path: &Path,
    filename: &str,
    boundary: &str,
) -> Result<Vec<u8>, String> {
    let raw = build_multipart_body(file_path, filename, boundary)?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&raw)
        .map_err(|error| format!("gzip 压缩请求体失败: {error}"))?;
    encoder
        .finish()
        .map_err(|error| format!("gzip 压缩请求体失败: {error}"))
}

fn build_multipart_body(
    file_path: &Path,
    filename: &str,
    boundary: &str,
) -> Result<Vec<u8>, String> {
    let mut file =
        std::fs::File::open(file_path).map_err(|error| format!("打开上传文件失败: {error}"))?;
    let mut file_bytes = Vec::new();
    file.read_to_end(&mut file_bytes)
        .map_err(|error| format!("读取上传文件失败: {error}"))?;

    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
    body.extend_from_slice(&file_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Ok(body)
}
