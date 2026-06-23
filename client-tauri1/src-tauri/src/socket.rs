//! 本地 socket worker：连接 PrintClient，监听任务推送，转发打印任务并做 token 失效重取。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const DEFAULT_LISTEN_PORT: u16 = 18101;

use crate::printclient::local_socket_url;
use crate::result::CommandResult;
use crate::services;
use crate::{
    now_iso_string, ClientEventPayload, ClientTodoTaskPayload, CLIENT_NOTIFICATION_EVENT,
    CLIENT_TODO_TASK_EVENT, CLIENT_TO_VIEW_EVENT, MAIN_WINDOW_LABEL,
};

const FORWARD_RETRY_INTERVAL: Duration = Duration::from_secs(30);
const PENDING_FORWARD_DIR: &str = "pending-forwards";
const MAX_FORWARD_ATTEMPTS: u64 = 3;
// 在途任务（.processing）运行期回收阈值：须大于上传最长耗时（30 分钟），避免误回收正在上传的任务。
const PROCESSING_RECLAIM_AFTER: Duration = Duration::from_secs(35 * 60);
const SOCKET_RETRY_INTERVAL: Duration = Duration::from_secs(3);
const SOCKET_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// 手动重连标志：调试页按钮置位，worker 检测到后立即断开并重连。
static RECONNECT_FLAG: AtomicBool = AtomicBool::new(false);

/// 待重试队列唤醒标志：任意任务转发成功后置位，提示 worker 立即再处理一次积压任务。
static WAKE_RETRY_FLAG: AtomicBool = AtomicBool::new(false);

/// 请求 worker 立即重连本地 socket 服务。
pub(crate) fn request_reconnect() {
    RECONNECT_FLAG.store(true, Ordering::SeqCst);
}

/// 任意任务转发成功后唤醒待重试队列，使此前达到重试上限的积压任务能立即再试。
pub(crate) fn wake_forward_retry_worker() {
    WAKE_RETRY_FLAG.store(true, Ordering::SeqCst);
}

/// token 更新后立即唤醒并处理待转发队列。
pub(crate) fn process_pending_forwards_after_token_update(app: AppHandle) {
    wake_forward_retry_worker();
    thread::spawn(move || {
        process_pending_forwards(&app, true);
    });
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
                            tauri::async_runtime::spawn(handle_push_connection(
                                app.clone(),
                                stream,
                            ));
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
        services::log_service::info(app, "socket", "识别为打印任务推送，已落盘待转发");
        // 收到即落盘：先写入待转发队列，确保即使首次转发未完成就崩溃，重启后也能重发。
        enqueue_pending_forward(app, payload);
        emit_socket_forward_received(app);
        if has_cached_auth_token(app) {
            emit_print_desktop_notification(app, "info", "收到打印任务，正在上传服务器！");
            let app_handle = app.clone();
            thread::spawn(move || {
                // 立即认领并转发（与周期 worker 共用认领机制，避免重复转发）。
                process_pending_forwards(&app_handle, false);
            });
        } else {
            services::log_service::info(
                app,
                "socket",
                "当前无会话 token，打印任务已暂存，等待登录状态同步后转发",
            );
            emit_print_desktop_notification(app, "info", "收到打印任务，等待登录状态同步后上传！");
        }
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

fn has_cached_auth_token(app: &AppHandle) -> bool {
    services::cached_auth_token(app)
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false)
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

/// 收到打印任务即落盘到待转发队列（attempts=0），确保转发未完成就崩溃也能在重启后重发。
fn enqueue_pending_forward(app: &AppHandle, message: &str) {
    let Some(dir) = pending_dir(app) else {
        return;
    };
    let record =
        json!({ "message": message, "attempts": 0, "status": "active", "at": now_iso_string() });
    let file = dir.join(format!("{}.json", Uuid::new_v4()));
    if fs::write(&file, record.to_string()).is_ok() {
        services::log_service::info(app, "socket", "打印任务已落盘待转发队列");
    }
}

/// 启动待转发 worker：先回收上次崩溃残留的在途任务，立即处理一次队列，
/// 之后定期处理 active 任务；任意任务转发成功后会被唤醒，立即再处理一次待重试任务。
pub(crate) fn start_forward_retry_worker(app: AppHandle) {
    thread::spawn(move || {
        // 启动即回收上次进程崩溃残留的在途任务（.processing → .json）。
        reclaim_orphaned_processing(&app, true);
        // 启动后立即处理一次队列；pending-retry 也只在启动/成功唤醒时再试一次。
        process_pending_forwards(&app, true);
        loop {
            let include_waiting = sleep_or_wake(FORWARD_RETRY_INTERVAL);
            process_pending_forwards(&app, include_waiting);
        }
    });
}

/// 等待固定间隔，期间若收到唤醒标志则提前返回（并消费标志）。
fn sleep_or_wake(total: Duration) -> bool {
    let mut elapsed = Duration::ZERO;
    let step = Duration::from_millis(250);
    while elapsed < total {
        if WAKE_RETRY_FLAG.swap(false, Ordering::SeqCst) {
            return true;
        }
        thread::sleep(step);
        elapsed += step;
    }
    false
}

/// 处理待转发队列：逐个认领（原子 rename → .processing）后转发，成功出队、失败回写计数。
/// 认领机制保证「收到即转发」线程、周期 worker、以及多次扫描之间不会重复转发同一任务。
fn process_pending_forwards(app: &AppHandle, include_waiting: bool) {
    if !has_cached_auth_token(app) {
        return;
    }

    let Some(dir) = pending_dir(app) else {
        return;
    };

    // 回收僵死的在途任务（运行期 panic 残留，超阈值才回收以免误伤正在上传的任务）。
    reclaim_orphaned_processing(app, false);

    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };

    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| pending_record_sort_key(path));

    for path in paths {
        let status = pending_record_status(&path);
        if status == "pending-retry" && !include_waiting {
            continue;
        }

        // 认领：原子 rename 到 .processing；失败说明已被并发认领，跳过。
        let claimed = path.with_extension("processing");
        if fs::rename(&path, &claimed).is_err() {
            continue;
        }

        process_claimed_forward(app, &claimed, status == "pending-retry");
    }
}

fn pending_record_sort_key(path: &Path) -> (String, String) {
    let at = fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|record| record.get("at").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    (at, name)
}

/// 转发一个已认领（.processing）的任务：成功删文件出队；失败回写 .json（计数+1），达到单轮上限后等待下次唤醒。
fn process_claimed_forward(app: &AppHandle, claimed: &Path, was_waiting: bool) {
    let Some((message, attempts, at, status)) = read_pending_record(claimed) else {
        let _ = fs::remove_file(claimed);
        return;
    };
    let was_waiting = was_waiting || status == "pending-retry";

    let result = forward_socket_task_with_token_retry(app, &message);
    emit_socket_forward_result(app, result.clone());

    match result {
        Ok(_) => {
            let _ = fs::remove_file(claimed);
            services::log_service::info(app, "socket", "打印任务转发成功，已出队");
            // 任意任务成功后，立即唤醒待重试队列，让此前积压的待重试任务有机会再发。
            wake_forward_retry_worker();
        }
        Err(error) => {
            let next = attempts + 1;
            let exhausted = was_waiting || next >= MAX_FORWARD_ATTEMPTS;
            let attempts_to_persist = if exhausted { 0 } else { next };
            let status = if exhausted { "pending-retry" } else { "active" };
            let record = json!({ "message": message, "attempts": attempts_to_persist, "status": status, "at": at });
            let _ = fs::write(claimed.with_extension("json"), record.to_string());
            let _ = fs::remove_file(claimed);
            if exhausted {
                services::log_service::warn(
                    app,
                    "socket",
                    &format!(
                        "打印任务转发已达本轮最大次数（{MAX_FORWARD_ATTEMPTS} 次），已保留等待下次唤醒重试：{error}"
                    ),
                );
            } else {
                services::log_service::warn(
                    app,
                    "socket",
                    &format!("打印任务转发失败，保留待重试（第 {next} 次）：{error}"),
                );
            }
        }
    }
}

fn pending_record_status(path: &Path) -> String {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|record| {
            record
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "active".into())
}

fn read_pending_record(path: &Path) -> Option<(String, u64, Value, String)> {
    let raw = fs::read_to_string(path).ok()?;
    let record = serde_json::from_str::<Value>(&raw).ok()?;
    let message = record.get("message").and_then(Value::as_str)?.to_string();
    let attempts = record.get("attempts").and_then(Value::as_u64).unwrap_or(0);
    let at = record.get("at").cloned().unwrap_or(Value::Null);
    let status = record
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("active")
        .to_string();
    Some((message, attempts, at, status))
}

/// 回收僵死的 .processing 文件（改回 .json 重新入队）。
/// `force_all=true`（启动时）回收全部；否则仅回收超过 `PROCESSING_RECLAIM_AFTER` 的（运行期防 panic 残留）。
fn reclaim_orphaned_processing(app: &AppHandle, force_all: bool) {
    let Some(dir) = pending_dir(app) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("processing") {
            continue;
        }

        let reclaim = force_all
            || fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .map(|elapsed| elapsed >= PROCESSING_RECLAIM_AFTER)
                .unwrap_or(true);

        if reclaim && fs::rename(&path, path.with_extension("json")).is_ok() {
            services::log_service::warn(app, "socket", "回收在途转发任务，重新入队重试");
        }
    }
}

fn emit_socket_forward_received(app: &AppHandle) {
    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_TO_VIEW_EVENT,
        ClientEventPayload::new(
            "client.socket_task.received",
            Some(json!({
                "ok": true,
                "message": "收到打印任务，正在上传服务器！",
            })),
        ),
    );
}

fn emit_socket_forward_result(app: &AppHandle, result: Result<Value, String>) {
    match &result {
        Ok(value) => {
            services::log_service::info(app, "socket", &format!("打印任务转发成功：{value}"));
            emit_print_desktop_notification(app, "success", "打印任务上传成功！");
        }
        Err(error) => {
            services::log_service::error(app, "socket", &format!("打印任务转发失败：{error}"));
            emit_print_desktop_notification(app, "error", "打印任务上传失败，请联系管理员！");
        }
    }

    let (name, payload) = match result {
        Ok(value) => (
            "client.socket_task.forwarded",
            json!({
                "ok": true,
                "message": "打印任务上传成功！",
                "task": value,
            }),
        ),
        Err(error) => (
            "client.socket_task.forward_failed",
            json!({
                "ok": false,
                "message": "打印任务上传失败，请联系管理员！",
                "detail": error,
            }),
        ),
    };

    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_TO_VIEW_EVENT,
        ClientEventPayload::new(name, Some(payload)),
    );
}

fn emit_print_desktop_notification(app: &AppHandle, kind: &str, message: &str) {
    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_NOTIFICATION_EVENT,
        json!({
            "type": kind,
            "title": "打印任务通知",
            "message": message,
            "durationMs": if kind == "error" { 6000 } else { 3500 },
        }),
    );
}
