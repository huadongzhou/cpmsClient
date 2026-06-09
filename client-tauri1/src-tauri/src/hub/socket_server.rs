use std::fs::{self, File};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Map, Value};
use tauri::AppHandle;

use super::events::{emit_job_error, emit_job_progress};
use super::models::SocketState;

const SOCKET_HOST: &str = "127.0.0.1";
const PERSON_PORT_BASE: u16 = 51664;
const ENTERPRISE_PORT_BASE: u16 = 52664;
const PORT_RETRY_COUNT: u16 = 10;
const FRAME_BEGIN: &str = "@jsonbegin@";
const FRAME_END: &str = "@jsonend@";

struct SocketServerHandle {
    state: SocketState,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct SocketReceiveSession {
    msg_id: Option<String>,
    print_param: Option<Value>,
    file_info: Option<FileInfo>,
    chunks: Vec<FileChunk>,
    total_received: u64,
    finalized: bool,
}

#[derive(Clone)]
struct FileInfo {
    file_size: u64,
    file_type: String,
    file_md5: Option<String>,
}

struct FileChunk {
    offset: u64,
    data: Vec<u8>,
}

fn runtime() -> &'static Mutex<Option<SocketServerHandle>> {
    static SOCKET_RUNTIME: OnceLock<Mutex<Option<SocketServerHandle>>> = OnceLock::new();
    SOCKET_RUNTIME.get_or_init(|| Mutex::new(None))
}

/// Starts the local TCP file receiver. Repeated calls are idempotent.
pub fn start_socket_server(app: AppHandle, product_type: i32) -> Result<SocketState, String> {
    let mut guard = runtime()
        .lock()
        .map_err(|_| "Socket 服务状态锁已损坏".to_string())?;

    if let Some(handle) = guard.as_ref() {
        if !handle.stop.load(Ordering::SeqCst) {
            return Ok(handle.state.clone());
        }
    }

    let cache_dir = app
        .path_resolver()
        .app_cache_dir()
        .ok_or_else(|| "无法获取应用缓存目录".to_string())?;
    fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;

    let (listener, port) = bind_available_listener(base_port(product_type))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;

    let state = SocketState {
        listening: true,
        host: SOCKET_HOST.into(),
        port: Some(port),
    };
    let stop = Arc::new(AtomicBool::new(false));
    let accept_stop = Arc::clone(&stop);
    let accept_app = app.clone();
    let join = thread::spawn(move || accept_loop(listener, accept_app, cache_dir, accept_stop));

    *guard = Some(SocketServerHandle {
        state: state.clone(),
        stop,
        join: Some(join),
    });

    Ok(state)
}

/// Stops the local TCP file receiver. Repeated calls are idempotent.
pub fn stop_socket_server() -> Result<SocketState, String> {
    let handle = {
        let mut guard = runtime()
            .lock()
            .map_err(|_| "Socket 服务状态锁已损坏".to_string())?;
        guard.take()
    };

    if let Some(mut handle) = handle {
        handle.stop.store(true, Ordering::SeqCst);
        if let Some(join) = handle.join.take() {
            let _ = join.join();
        }
    }

    Ok(SocketState {
        listening: false,
        host: SOCKET_HOST.into(),
        port: None,
    })
}

fn base_port(product_type: i32) -> u16 {
    match product_type {
        1 | 2 => ENTERPRISE_PORT_BASE,
        _ => PERSON_PORT_BASE,
    }
}

fn bind_available_listener(base_port: u16) -> Result<(TcpListener, u16), String> {
    for offset in 0..PORT_RETRY_COUNT {
        let port = base_port + offset;
        match TcpListener::bind((SOCKET_HOST, port)) {
            Ok(listener) => return Ok((listener, port)),
            Err(_) => continue,
        }
    }

    Err(format!(
        "Socket 端口不可用，已尝试 {SOCKET_HOST}:{}-{}",
        base_port,
        base_port + PORT_RETRY_COUNT - 1
    ))
}

fn accept_loop(listener: TcpListener, app: AppHandle, cache_dir: PathBuf, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let next_app = app.clone();
                let next_cache_dir = cache_dir.clone();
                thread::spawn(move || {
                    if let Err(error) =
                        handle_connection(stream, next_cache_dir.clone(), next_app.clone())
                    {
                        emit_job_error(
                            &next_app,
                            json!({
                                "source": "socket",
                                "code": "HUB_SOCKET_CONNECTION_ERROR",
                                "message": error,
                            }),
                        );
                    }
                });
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(150));
            }
            Err(error) => {
                emit_job_error(
                    &app,
                    json!({
                        "source": "socket",
                        "code": "HUB_SOCKET_ACCEPT_ERROR",
                        "message": error.to_string(),
                    }),
                );
                thread::sleep(Duration::from_millis(300));
            }
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    cache_dir: PathBuf,
    app: AppHandle,
) -> Result<(), String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(15)));

    let mut session = SocketReceiveSession::default();
    let mut frame_buffer = String::new();
    let mut read_buffer = [0_u8; 64 * 1024];

    loop {
        let read_len = match stream.read(&mut read_buffer) {
            Ok(0) => break,
            Ok(value) => value,
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                break
            }
            Err(error) => return Err(error.to_string()),
        };

        frame_buffer.push_str(&String::from_utf8_lossy(&read_buffer[..read_len]));
        while let Some(frame) = take_next_frame(&mut frame_buffer) {
            handle_frame(&frame, &mut stream, &mut session, &cache_dir, &app)?;
        }
    }

    Ok(())
}

fn take_next_frame(buffer: &mut String) -> Option<String> {
    let begin_index = buffer.find(FRAME_BEGIN)?;
    if begin_index > 0 {
        buffer.drain(..begin_index);
    }

    let content_start = FRAME_BEGIN.len();
    let end_index = buffer[content_start..].find(FRAME_END)? + content_start;
    let frame = buffer[content_start..end_index].to_string();
    let drain_end = end_index + FRAME_END.len();
    buffer.drain(..drain_end);
    Some(frame)
}

fn handle_frame(
    frame: &str,
    stream: &mut TcpStream,
    session: &mut SocketReceiveSession,
    cache_dir: &Path,
    app: &AppHandle,
) -> Result<(), String> {
    let message: Value = serde_json::from_str(frame).map_err(|error| error.to_string())?;
    let msg_type = message
        .get("msgType")
        .and_then(Value::as_str)
        .ok_or_else(|| "Socket 消息缺少 msgType".to_string())?;
    let msg_id = message
        .get("msgId")
        .and_then(Value::as_str)
        .unwrap_or("socket")
        .to_string();
    session.msg_id = Some(msg_id.clone());

    match msg_type {
        "version" => {
            send_framed(stream, feedback(&message, message.get("msgData").cloned()))?;
        }
        "print_param" => {
            session.print_param = message.get("msgData").cloned();
            send_framed(stream, feedback(&message, None))?;
        }
        "file_info" => {
            let file_info = parse_file_info(message.get("msgData"))?;
            emit_job_progress(
                app,
                json!({
                    "source": "socket",
                    "msgId": msg_id,
                    "step": "file_info",
                    "fileSize": file_info.file_size,
                    "fileType": file_info.file_type,
                }),
            );
            session.file_info = Some(file_info);
            send_framed(stream, feedback(&message, None))?;
        }
        "file_contents" => {
            let chunk = parse_file_chunk(message.get("msgData"))?;
            session.total_received = session
                .total_received
                .saturating_add(u64::try_from(chunk.data.len()).unwrap_or_default());
            session.chunks.push(chunk);
            send_framed(stream, feedback(&message, None))?;

            let Some(file_info) = session.file_info.clone() else {
                return Err("Socket 文件内容早于 file_info".into());
            };

            emit_job_progress(
                app,
                json!({
                    "source": "socket",
                    "msgId": msg_id,
                    "step": "file_contents",
                    "receivedSize": session.total_received,
                    "fileSize": file_info.file_size,
                }),
            );

            if !session.finalized && session.total_received >= file_info.file_size {
                session.finalized = true;
                match write_received_file(cache_dir, session, &file_info) {
                    Ok(paths) => {
                        send_framed(stream, file_check_feedback(&message, true, ""))?;
                        emit_job_progress(
                            app,
                            json!({
                                "source": "socket",
                                "msgId": msg_id,
                                "step": "file_check",
                                "filePath": paths.file_path,
                                "paramPath": paths.param_path,
                            }),
                        );
                    }
                    Err(error) => {
                        send_framed(stream, file_check_feedback(&message, false, &error))?;
                        emit_job_error(
                            app,
                            json!({
                                "source": "socket",
                                "msgId": msg_id,
                                "code": "HUB_SOCKET_FILE_WRITE_ERROR",
                                "message": error,
                            }),
                        );
                    }
                }
            }
        }
        "file_check" => {}
        _ => {
            return Err(format!("不支持的 Socket msgType: {msg_type}"));
        }
    }

    Ok(())
}

fn parse_file_info(value: Option<&Value>) -> Result<FileInfo, String> {
    let Some(value) = value else {
        return Err("file_info 缺少 msgData".into());
    };

    let file_size = value
        .get("fileSize")
        .or_else(|| value.get("fileTotalSize"))
        .and_then(Value::as_u64)
        .ok_or_else(|| "file_info 缺少 fileSize".to_string())?;
    let file_type = value
        .get("fileType")
        .and_then(Value::as_str)
        .map(sanitize_file_extension)
        .unwrap_or_else(|| "pdf".into());
    let file_md5 = value
        .get("fileMd5")
        .and_then(Value::as_str)
        .map(|raw| raw.trim().to_lowercase())
        .filter(|raw| !raw.is_empty());

    Ok(FileInfo {
        file_size,
        file_type,
        file_md5,
    })
}

fn parse_file_chunk(value: Option<&Value>) -> Result<FileChunk, String> {
    let Some(value) = value else {
        return Err("file_contents 缺少 msgData".into());
    };

    let offset = value
        .get("fileOffset")
        .and_then(Value::as_u64)
        .ok_or_else(|| "file_contents 缺少 fileOffset".to_string())?;
    let expected_size = value
        .get("fileContentsSize")
        .and_then(Value::as_u64)
        .ok_or_else(|| "file_contents 缺少 fileContentsSize".to_string())?;
    let encoded = value
        .get("fileBase64Contents")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_contents 缺少 fileBase64Contents".to_string())?;
    let data = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| error.to_string())?;

    if u64::try_from(data.len()).unwrap_or_default() != expected_size {
        return Err(format!(
            "文件分片大小不一致，声明={expected_size}，实际={}",
            data.len()
        ));
    }

    Ok(FileChunk { offset, data })
}

struct WrittenPaths {
    file_path: String,
    param_path: String,
}

fn write_received_file(
    cache_dir: &Path,
    session: &SocketReceiveSession,
    file_info: &FileInfo,
) -> Result<WrittenPaths, String> {
    let msg_id = session.msg_id.as_deref().unwrap_or("socket");
    let safe_msg_id = sanitize_file_stem(msg_id);
    let file_path = cache_dir.join(format!("cpms_hm_{safe_msg_id}.{}", file_info.file_type));
    let param_path = cache_dir.join(format!("cpms_hm_{safe_msg_id}.json"));

    let mut print_file = File::create(&file_path).map_err(|error| error.to_string())?;
    print_file
        .set_len(file_info.file_size)
        .map_err(|error| error.to_string())?;

    let mut chunks: Vec<&FileChunk> = session.chunks.iter().collect();
    chunks.sort_by_key(|chunk| chunk.offset);

    for chunk in chunks {
        print_file
            .seek(SeekFrom::Start(chunk.offset))
            .map_err(|error| error.to_string())?;
        print_file
            .write_all(&chunk.data)
            .map_err(|error| error.to_string())?;
    }
    print_file.flush().map_err(|error| error.to_string())?;
    drop(print_file);

    if let Some(expected_md5) = file_info.file_md5.as_deref() {
        let actual_md5 = format!(
            "{:x}",
            md5::compute(fs::read(&file_path).map_err(|error| { error.to_string() })?)
        );
        if actual_md5 != expected_md5 {
            let invalid_path = file_path.with_file_name(format!(
                "{}-md5invalid",
                file_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("cpms_hm_socket")
            ));
            let _ = fs::rename(&file_path, invalid_path);
            return Err(format!(
                "文件 MD5 校验失败，期望={expected_md5}，实际={actual_md5}"
            ));
        }
    }

    let param_payload = build_param_payload(session.print_param.clone(), &file_path);
    fs::write(
        &param_path,
        serde_json::to_vec_pretty(&param_payload).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    Ok(WrittenPaths {
        file_path: file_path.to_string_lossy().to_string(),
        param_path: param_path.to_string_lossy().to_string(),
    })
}

fn build_param_payload(print_param: Option<Value>, file_path: &Path) -> Value {
    let file_path_value = Value::String(file_path.to_string_lossy().to_string());

    match print_param {
        Some(Value::Object(mut object)) => {
            object.insert("filePath".into(), file_path_value);
            Value::Object(object)
        }
        Some(value) => {
            let mut object = Map::new();
            object.insert("raw".into(), value);
            object.insert("filePath".into(), file_path_value);
            Value::Object(object)
        }
        None => {
            let mut object = Map::new();
            object.insert("filePath".into(), file_path_value);
            Value::Object(object)
        }
    }
}

fn feedback(message: &Value, data: Option<Value>) -> Value {
    json!({
        "code": 1,
        "msgId": message.get("msgId").and_then(Value::as_str).unwrap_or(""),
        "msgType": message.get("msgType").and_then(Value::as_str).unwrap_or(""),
        "msgData": data.unwrap_or(Value::Null),
        "msgTime": now_millis(),
        "msgError": "",
    })
}

fn file_check_feedback(message: &Value, ok: bool, error: &str) -> Value {
    json!({
        "msgId": message.get("msgId").and_then(Value::as_str).unwrap_or(""),
        "msgType": "file_check",
        "msgData": {
            "result": if ok { "ok" } else { "error" },
            "errMsg": error,
        },
        "msgTime": now_millis(),
    })
}

fn send_framed(stream: &mut TcpStream, value: Value) -> Result<(), String> {
    let raw = format!("{FRAME_BEGIN}{value}{FRAME_END}");
    stream
        .write_all(raw.as_bytes())
        .map_err(|error| error.to_string())
}

fn sanitize_file_extension(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .filter(|item| item.is_ascii_alphanumeric())
        .take(16)
        .collect();

    if sanitized.is_empty() {
        "pdf".into()
    } else {
        sanitized.to_lowercase()
    }
}

fn sanitize_file_stem(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .filter(|item| item.is_ascii_alphanumeric() || matches!(item, '-' | '_'))
        .take(80)
        .collect();

    if sanitized.is_empty() {
        "socket".into()
    } else {
        sanitized
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default()
}
