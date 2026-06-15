//! 本地 PrintClient（cpms 客户端）发现：
//! 1) 优先按进程名找到运行中的 PrintClient，取其 exe 目录（最可靠，不依赖安装路径）；
//! 2) 退回 env 指定 / 常见安装目录猜测；
//! 在定位到的目录里解析 DriverClient.ini，取 WebsocketPort 得到本地 socket 地址。
//! 同时向调试页暴露安装路径与 DriverClient.ini 内容。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::result::CommandResult;
use crate::{DEFAULT_LOCAL_SOCKET_PATH, DEFAULT_LOCAL_SOCKET_URL};

const DRIVER_CLIENT_INI: &str = "DriverClient.ini";
const CONFIG_FILE_NAMES: [&str; 3] = [DRIVER_CLIENT_INI, "config.conf", "config.ini"];
const PROCESS_RESCAN_INTERVAL: Duration = Duration::from_secs(10);

/// 本地 PrintClient 信息，供调试页展示。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrintClientInfo {
    /// 是否检测到 PrintClient 配置文件。
    pub(crate) installed: bool,
    /// 运行中的 PrintClient 进程所在目录（按进程名匹配）。
    pub(crate) process_dir: Option<String>,
    /// 配置文件所在目录。
    pub(crate) dir: Option<String>,
    /// 配置文件完整路径（优先 DriverClient.ini）。
    pub(crate) config_path: Option<String>,
    /// DriverClient.ini 字段 WebsocketPort 解析出的端口。
    pub(crate) websocket_port: Option<u16>,
    /// 最终解析到的本地 socket 地址。
    pub(crate) socket_url: String,
    /// 配置文件原始内容。
    pub(crate) ini_content: Option<String>,
}

#[tauri::command]
/// 读取本地 PrintClient 进程/安装路径、DriverClient.ini 内容与 WebsocketPort，供调试页展示。
pub(crate) fn get_print_client_info() -> CommandResult<PrintClientInfo> {
    CommandResult::ok(discover_print_client_info())
}

pub(crate) fn discover_print_client_info() -> PrintClientInfo {
    let socket_url = local_socket_url();
    let process_dir = process_print_client_dir().map(|dir| dir.to_string_lossy().to_string());

    match locate_print_client_config() {
        Some(path) => {
            let content = fs::read_to_string(&path).ok();
            let websocket_port = content.as_deref().and_then(parse_websocket_port);
            PrintClientInfo {
                installed: true,
                process_dir,
                dir: path.parent().map(|dir| dir.to_string_lossy().to_string()),
                config_path: Some(path.to_string_lossy().to_string()),
                websocket_port,
                socket_url,
                ini_content: content,
            }
        }
        None => PrintClientInfo {
            installed: false,
            process_dir,
            socket_url,
            ..PrintClientInfo::default()
        },
    }
}

/// 解析本地 socket 地址：优先 env 覆盖，其次发现 PrintClient 配置，最后回退默认值。
pub(crate) fn local_socket_url() -> String {
    std::env::var("CPMS_PRINTCLIENT_SOCKET_URL")
        .or_else(|_| std::env::var("CPMS_LOCAL_SOCKET_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(discover_print_client_socket_url)
        .unwrap_or_else(|| DEFAULT_LOCAL_SOCKET_URL.into())
}

fn discover_print_client_socket_url() -> Option<String> {
    locate_print_client_config().and_then(|path| socket_url_from_config_file(&path))
}

/// 定位 PrintClient 配置文件：优先 env 指定，其次候选目录（含运行进程目录）下的
/// DriverClient.ini / config.*。
fn locate_print_client_config() -> Option<PathBuf> {
    if let Ok(config_path) = std::env::var("CPMS_PRINTCLIENT_CONFIG_PATH") {
        let path = PathBuf::from(config_path);
        if path.is_file() {
            return Some(path);
        }
    }

    for dir in print_client_candidate_dirs() {
        for file_name in CONFIG_FILE_NAMES {
            let path = dir.join(file_name);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

fn print_client_candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. 运行中的 PrintClient 进程所在目录及其常见相对位置（最可靠）。
    if let Some(exe_dir) = process_print_client_dir() {
        dirs.push(exe_dir.clone());
        dirs.push(exe_dir.join("config"));
        if let Some(parent) = exe_dir.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    // 2. env 指定的安装目录。
    if let Ok(dir) = std::env::var("CPMS_PRINTCLIENT_DIR") {
        dirs.push(PathBuf::from(dir));
    }

    // 3. 常见安装目录猜测。
    for env_key in [
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
        "LOCALAPPDATA",
        "APPDATA",
    ] {
        if let Ok(base) = std::env::var(env_key) {
            let base_path = PathBuf::from(base);
            dirs.push(base_path.join("PrintClient"));
            dirs.push(base_path.join("CPMS").join("PrintClient"));
            dirs.push(base_path.join("Insolu").join("PrintClient"));
        }
    }

    dirs
}

/// 按进程名定位运行中的 PrintClient 的 exe 目录（带 10s 节流缓存，避免反复扫描）。
fn process_print_client_dir() -> Option<PathBuf> {
    static CACHE: OnceLock<Mutex<Option<(Instant, Option<PathBuf>)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    let Ok(mut guard) = cache.lock() else {
        return scan_process_exe_dir();
    };

    let need_scan = match guard.as_ref() {
        // 已找到的结果稳定复用；未找到的每 10s 重扫一次（PrintClient 可能后启动）。
        Some((scanned_at, result)) => result.is_none() && scanned_at.elapsed() >= PROCESS_RESCAN_INTERVAL,
        None => true,
    };

    if need_scan {
        let dir = scan_process_exe_dir();
        *guard = Some((Instant::now(), dir.clone()));
        return dir;
    }

    guard.as_ref().and_then(|(_, result)| result.clone())
}

#[cfg(target_os = "windows")]
fn scan_process_exe_dir() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Process -Name PrintClient -ErrorAction SilentlyContinue | \
             Select-Object -First 1 -ExpandProperty Path",
        ])
        .output()
        .ok()?;

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }

    PathBuf::from(path).parent().map(|dir| dir.to_path_buf())
}

#[cfg(not(target_os = "windows"))]
fn scan_process_exe_dir() -> Option<PathBuf> {
    // 非 Windows 平台没有 PrintClient.exe 进程，按进程名发现不适用。
    None
}

fn socket_url_from_config_file(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let socket_path = std::env::var("CPMS_PRINTCLIENT_SOCKET_PATH")
        .ok()
        .filter(|value| value.starts_with('/'))
        .unwrap_or_else(|| DEFAULT_LOCAL_SOCKET_PATH.into());

    // 1. DriverClient.ini 字段 WebsocketPort（端口权威来源）。
    if let Some(port) = parse_websocket_port(&raw) {
        return Some(format!("ws://127.0.0.1:{port}{socket_path}"));
    }

    // 2. 显式 ws:// / wss:// 地址。
    for line in raw.lines() {
        if let Some(url) = extract_websocket_url(line) {
            return Some(url);
        }
    }

    // 3. 兜底：含 websocket/socket/port 字样行里的端口。
    for line in raw.lines() {
        let lower = line.to_lowercase();
        if !(lower.contains("websocket") || lower.contains("socket") || lower.contains("port")) {
            continue;
        }

        if let Some(port) = extract_port(line) {
            return Some(format!("ws://127.0.0.1:{port}{socket_path}"));
        }
    }

    None
}

/// 解析 DriverClient.ini 中的 `WebsocketPort=<port>` 字段（忽略大小写与首尾空白）。
fn parse_websocket_port(content: &str) -> Option<u16> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim().eq_ignore_ascii_case("WebsocketPort") {
                if let Ok(port) = value.trim().parse::<u16>() {
                    if port > 0 {
                        return Some(port);
                    }
                }
            }
        }
    }

    None
}

fn extract_websocket_url(line: &str) -> Option<String> {
    let start = line.find("ws://").or_else(|| line.find("wss://"))?;
    let candidate = line[start..]
        .trim()
        .trim_matches(|character: char| {
            character.is_whitespace() || matches!(character, '"' | '\'' | ';' | ',')
        })
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '"' | '\'' | ';' | ',')
        })
        .next()?;

    if candidate.starts_with("ws://") || candidate.starts_with("wss://") {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn extract_port(line: &str) -> Option<u16> {
    let value_part = line
        .split_once('=')
        .map(|(_, value)| value)
        .or_else(|| line.split_once(':').map(|(_, value)| value))
        .unwrap_or(line);
    let mut digits = String::new();

    for character in value_part.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
            continue;
        }

        if !digits.is_empty() {
            if let Ok(port) = digits.parse::<u16>() {
                if port > 0 {
                    return Some(port);
                }
            }
            digits.clear();
        }
    }

    digits.parse::<u16>().ok().filter(|port| *port > 0)
}
