//! 本地 socket worker：连接 PrintClient，监听任务推送，转发打印任务并做 token 失效重取。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

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

pub(crate) async fn start_local_socket_worker(app: AppHandle) {
    let mut last_url: Option<String> = None;
    let mut announced_failure = false;

    loop {
        let socket_url = local_socket_url();

        if last_url.as_deref() != Some(socket_url.as_str()) {
            services::log_service::info(
                &app,
                "socket",
                &format!("本地 socket 地址解析为：{socket_url}"),
            );
            last_url = Some(socket_url.clone());
        }

        match tokio_tungstenite::connect_async(&socket_url).await {
            Ok((mut stream, _)) => {
                services::log_service::info(
                    &app,
                    "socket",
                    &format!("本地 socket 已连接：{socket_url}"),
                );
                announced_failure = false;

                let mut immediate_reconnect = false;
                loop {
                    if RECONNECT_FLAG.swap(false, Ordering::SeqCst) {
                        services::log_service::info(
                            &app,
                            "socket",
                            "收到重连请求，断开当前连接",
                        );
                        immediate_reconnect = true;
                        break;
                    }

                    // 轮询读取，超时即回到循环顶部检查重连标志（不引入 tokio sync 特性）。
                    match tokio::time::timeout(SOCKET_POLL_INTERVAL, stream.next()).await {
                        Ok(Some(Ok(raw_message))) if raw_message.is_text() => {
                            if let Ok(text) = raw_message.to_text() {
                                if is_print_task_message(text) {
                                    services::log_service::info(
                                        &app,
                                        "socket",
                                        "收到打印任务推送，开始转发",
                                    );
                                    let app_handle = app.clone();
                                    let message = text.to_string();
                                    thread::spawn(move || {
                                        let result = forward_socket_task_with_token_retry(
                                            &app_handle,
                                            &message,
                                        );
                                        if result.is_err() {
                                            enqueue_failed_forward(&app_handle, &message);
                                        }
                                        emit_socket_forward_result(&app_handle, result);
                                    });
                                } else if let Some(task_payload) = parse_todo_payload(text) {
                                    let _ = app.emit_to(
                                        MAIN_WINDOW_LABEL,
                                        CLIENT_TODO_TASK_EVENT,
                                        task_payload,
                                    );
                                }
                            }
                        }
                        Ok(Some(Ok(_))) => {}
                        Ok(Some(Err(_))) | Ok(None) => break,
                        Err(_) => {}
                    }
                }

                if immediate_reconnect {
                    services::log_service::info(&app, "socket", "立即重连本地 socket 服务");
                    continue;
                }

                services::log_service::warn(&app, "socket", "本地 socket 连接断开，准备重连");
            }
            Err(error) => {
                if !announced_failure {
                    services::log_service::warn(
                        &app,
                        "socket",
                        &format!("本地 socket 连接失败，将每 3 秒重试：{error}"),
                    );
                    announced_failure = true;
                }
            }
        }

        sleep_or_reconnect(SOCKET_RETRY_INTERVAL).await;
    }
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
        ClientEventPayload {
            name: name.into(),
            payload: Some(payload),
            at: now_iso_string(),
        },
    );
}
