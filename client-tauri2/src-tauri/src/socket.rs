//! 本地 socket worker：连接 PrintClient，监听任务推送，转发打印任务并做 token 失效重取。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const DEFAULT_LISTEN_PORT: u16 = 18101;

use crate::printclient::local_socket_url;
use crate::result::CommandResult;
use crate::services;
use crate::{
    now_iso_string, ClientEventPayload, ClientTodoTaskPayload, CLIENT_TODO_TASK_EVENT,
    CLIENT_TO_VIEW_EVENT, MAIN_WINDOW_LABEL,
};

const FORWARD_RETRY_INTERVAL: Duration = Duration::from_secs(30);
const PENDING_FORWARD_DIR: &str = "pending-forwards";
const MAX_FORWARD_ATTEMPTS: u64 = 10;
const SOCKET_RETRY_INTERVAL: Duration = Duration::from_secs(3);
const SOCKET_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// 手动重连标志：调试页按钮置位，worker 检测到后立即断开并重连。
static RECONNECT_FLAG: AtomicBool = AtomicBool::new(false);

/// 请求 worker 立即重连本地 socket 服务。
pub(crate) fn request_reconnect() {
    RECONNECT_FLAG.store(true, Ordering::SeqCst);
}

#[tauri::command]
/// 手动重连本地 PrintClient socket 服务（调试页按钮触发）。
pub fn reconnect_socket(app: AppHandle) -> CommandResult<bool> {
    request_reconnect();
    services::log_service::info(&app, "socket", "收到手动重连本地 socket 服务请求");
    CommandResult::ok(true)
}

const SOCKET_STATE_EVENT: &str = "cpms:client-socket";

/// 本地 socket 连接状态：解析出的完整地址、端口、连接状态与最近一次说明。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SocketLinkState {
    pub(crate) url: String,
    pub(crate) port: Option<u16>,
    pub(crate) status: String,
    pub(crate) message: Option<String>,
    pub(crate) updated_at: String,
}

fn socket_state() -> &'static Mutex<SocketLinkState> {
    static STATE: OnceLock<Mutex<SocketLinkState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(SocketLinkState::default()))
}

/// 更新并广播 socket 连接状态（同时落入可查询的全局状态）。
fn update_socket_state(app: &AppHandle, url: &str, status: &str, message: Option<String>) {
    let port = Url::parse(url).ok().and_then(|parsed| parsed.port());
    let state = SocketLinkState {
        url: url.to_string(),
        port,
        status: status.to_string(),
        message,
        updated_at: now_iso_string(),
    };

    if let Ok(mut locked) = socket_state().lock() {
        *locked = state.clone();
    }
    let _ = app.emit_to(MAIN_WINDOW_LABEL, SOCKET_STATE_EVENT, state);
}

#[tauri::command]
/// 读取本地 socket 连接状态（完整地址/端口/连接状态），供调试页展示。
pub(crate) fn get_socket_state() -> CommandResult<SocketLinkState> {
    let state = socket_state()
        .lock()
        .map(|locked| locked.clone())
        .unwrap_or_default();
    CommandResult::ok(state)
}

/// 本地 socket worker（监听端）：在本地端口监听，等待 PrintClient 等连接进来推送任务。
pub(crate) async fn start_local_socket_worker(app: AppHandle) {
    let mut last_addr: Option<String> = None;
    let mut announced_failure = false;

    loop {
        let socket_url = local_socket_url();
        let listen_port = Url::parse(&socket_url)
            .ok()
            .and_then(|parsed| parsed.port())
            .unwrap_or(DEFAULT_LISTEN_PORT);
        let listen_addr = format!("127.0.0.1:{listen_port}");

        if last_addr.as_deref() != Some(listen_addr.as_str()) {
            services::log_service::info(
                &app,
                "socket",
                &format!("本地 socket 监听地址解析为：{listen_addr}（{socket_url}）"),
            );
            last_addr = Some(listen_addr.clone());
        }

        update_socket_state(&app, &socket_url, "binding", None);
        match TcpListener::bind(&listen_addr).await {
            Ok(listener) => {
                services::log_service::info(
                    &app,
                    "socket",
                    &format!("本地 socket 已监听 {listen_addr}，等待推送连接"),
                );
                update_socket_state(&app, &socket_url, "listening", None);
                announced_failure = false;

                loop {
                    if RECONNECT_FLAG.swap(false, Ordering::SeqCst) {
                        services::log_service::info(&app, "socket", "收到重连请求，重启监听");
                        break;
                    }

                    // 轮询 accept，超时即回到循环顶部检查重启标志（不引入 tokio sync 特性）。
                    match tokio::time::timeout(SOCKET_POLL_INTERVAL, listener.accept()).await {
                        Ok(Ok((stream, peer))) => {
                            services::log_service::info(
                                &app,
                                "socket",
                                &format!("接受推送连接：{peer}"),
                            );
                            tauri::async_runtime::spawn(handle_push_connection(app.clone(), stream));
                        }
                        Ok(Err(error)) => {
                            services::log_service::warn(
                                &app,
                                "socket",
                                &format!("接受连接失败：{error}"),
                            );
                        }
                        Err(_) => {}
                    }
                }

                // 内层仅通过重启标志退出 → 立即重新监听（跳过重试等待）。
                continue;
            }
            Err(error) => {
                if !announced_failure {
                    services::log_service::warn(
                        &app,
                        "socket",
                        &format!("本地 socket 监听失败，将每 3 秒重试：{error}"),
                    );
                    announced_failure = true;
                }
                update_socket_state(&app, &socket_url, "failed", Some(error.to_string()));
            }
        }

        sleep_or_reconnect(SOCKET_RETRY_INTERVAL).await;
    }
}

/// 处理一条推送连接：完成 websocket 握手后持续接收推送的任务消息。
async fn handle_push_connection(app: AppHandle, stream: TcpStream) {
    let peer = stream
        .peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "unknown".into());

    match accept_async(stream).await {
        Ok(mut websocket) => {
            services::log_service::info(&app, "socket", &format!("推送连接已建立：{peer}"));

            while let Some(next_message) = websocket.next().await {
                match next_message {
                    Ok(raw_message) if raw_message.is_text() => {
                        if let Ok(text) = raw_message.to_text() {
                            process_push_message(&app, text);
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        // 回 Pong 维持心跳，避免对端按超时主动断开。
                        let _ = websocket.send(Message::Pong(payload)).await;
                    }
                    Ok(raw_message) if raw_message.is_close() => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
            }

            services::log_service::info(&app, "socket", &format!("推送连接断开：{peer}"));
        }
        Err(error) => {
            services::log_service::warn(
                &app,
                "socket",
                &format!("websocket 握手失败（{peer}）：{error}"),
            );
        }
    }
}

/// 处理一条推送消息：记录日志，打印任务转发、待办任务推送视图端。
fn process_push_message(app: &AppHandle, text: &str) {
    // 兼容带 "data:" 前缀（SSE 风格）的推送，取 JSON 主体。
    let payload = strip_push_envelope(text);
    let preview: String = payload.chars().take(4000).collect();
    services::log_service::log(app, "INFO", "socket", "socket 收到推送", Some(&preview));

    if is_print_task_message(payload) {
        services::log_service::info(app, "socket", "识别为打印任务推送，开始转发");
        let app_handle = app.clone();
        let message = payload.to_string();
        thread::spawn(move || {
            let result = forward_socket_task_with_token_retry(&app_handle, &message);
            if result.is_err() {
                enqueue_failed_forward(&app_handle, &message);
            }
            emit_socket_forward_result(&app_handle, result);
        });
    } else if let Some(task_payload) = parse_todo_payload(payload) {
        let _ = app.emit_to(MAIN_WINDOW_LABEL, CLIENT_TODO_TASK_EVENT, task_payload);
    } else {
        services::log_service::warn(app, "socket", "推送消息未识别为打印/待办任务，已忽略");
    }
}

/// 兼容带 "data:" 前缀（SSE 风格）的推送：剥离前缀，取 JSON 主体。
fn strip_push_envelope(text: &str) -> &str {
    let trimmed = text.trim();
    for prefix in ["data:", "DATA:", "data："] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim();
        }
    }
    trimmed
}

/// 等待重连间隔，期间收到重连请求则立即返回（并消费标志）。
async fn sleep_or_reconnect(total: Duration) {
    let mut elapsed = Duration::ZERO;
    let step = Duration::from_millis(250);
    while elapsed < total {
        if RECONNECT_FLAG.swap(false, Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(step).await;
        elapsed += step;
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

/// 转发 socket 推送的打印任务，复用通用 token 失效重取（需求3）。
fn forward_socket_task_with_token_retry(app: &AppHandle, message: &str) -> Result<Value, String> {
    crate::token_refresh::with_token_retry(app, || {
        services::forward_socket_task_message(app.clone(), message)
    })
}

/// 待重试队列目录（app_data_dir/pending-forwards），不存在则创建。
fn pending_dir(app: &AppHandle) -> Option<PathBuf> {
    let dir = services::app_data_dir(app)?.join(PENDING_FORWARD_DIR);
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 转发失败（含 token 重取后仍失败）时把原始任务消息落盘，等待重试 worker 重发。
fn enqueue_failed_forward(app: &AppHandle, message: &str) {
    let Some(dir) = pending_dir(app) else {
        return;
    };
    let record = json!({ "message": message, "attempts": 0, "at": now_iso_string() });
    let file = dir.join(format!("{}.json", Uuid::new_v4()));
    if fs::write(&file, record.to_string()).is_ok() {
        services::log_service::info(app, "socket", "转发失败，已加入待重试队列");
    }
}

/// 启动待重试 worker：定期重发 pending-forwards 中的任务，成功出队、超次丢弃。
pub(crate) fn start_forward_retry_worker(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(FORWARD_RETRY_INTERVAL);
        retry_pending_forwards(&app);
    });
}

fn retry_pending_forwards(app: &AppHandle) {
    let Some(dir) = pending_dir(app) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(record) = serde_json::from_str::<Value>(&raw) else {
            let _ = fs::remove_file(&path);
            continue;
        };
        let Some(message) = record.get("message").and_then(Value::as_str) else {
            let _ = fs::remove_file(&path);
            continue;
        };
        let attempts = record.get("attempts").and_then(Value::as_u64).unwrap_or(0);

        match forward_socket_task_with_token_retry(app, message) {
            Ok(_) => {
                let _ = fs::remove_file(&path);
                services::log_service::info(app, "socket", "待重试任务转发成功，已出队");
            }
            Err(error) => {
                let next = attempts + 1;
                if next >= MAX_FORWARD_ATTEMPTS {
                    let _ = fs::remove_file(&path);
                    services::log_service::error(
                        app,
                        "socket",
                        &format!("待重试任务超过最大次数已丢弃：{error}"),
                    );
                } else {
                    let updated = json!({
                        "message": message,
                        "attempts": next,
                        "at": record.get("at").cloned().unwrap_or(Value::Null),
                    });
                    let _ = fs::write(&path, updated.to_string());
                }
            }
        }
    }
}

fn emit_socket_forward_result(app: &AppHandle, result: Result<Value, String>) {
    match &result {
        Ok(value) => {
            services::log_service::info(app, "socket", &format!("打印任务转发成功：{value}"));
        }
        Err(error) => {
            services::log_service::error(app, "socket", &format!("打印任务转发失败：{error}"));
        }
    }

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
        ClientEventPayload::new(name, Some(payload)),
    );
}
