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
const CONFIGURE_INI: &str = "configure.ini";
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
    /// configure.ini 字段 ServerAddr：所有服务端请求使用的域名。
    pub(crate) server_addr: Option<String>,
    /// configure.ini 字段 CenterServerAddr。
    pub(crate) center_server_addr: Option<String>,
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
    // 定位到目录即一起读 configure.ini（ServerAddr / CenterServerAddr）并缓存。
    let configure = configure_data();

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
                server_addr: configure.server_addr,
                center_server_addr: configure.center_server_addr,
                socket_url,
                ini_content: content,
            }
        }
        None => PrintClientInfo {
            installed: false,
            process_dir,
            server_addr: configure.server_addr,
            center_server_addr: configure.center_server_addr,
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
                remember_print_client_dir(&dir);
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

    // 1b. 上次成功定位并持久化缓存的目录：CPMS 未运行（进程发现失败）时仍能据此读到配置。
    if let Some(cached) = cached_print_client_dir() {
        dirs.push(cached);
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

    // 4. Linux 常见安装目录猜测（系统级 + 用户级）。
    #[cfg(target_os = "linux")]
    {
        for base in ["/opt", "/usr/local", "/usr/local/share", "/usr/share"] {
            let base_path = PathBuf::from(base);
            dirs.push(base_path.join("PrintClient"));
            dirs.push(base_path.join("printclient"));
            dirs.push(base_path.join("DriverClient"));
            dirs.push(base_path.join("cpms").join("PrintClient"));
            dirs.push(base_path.join("CPMS").join("PrintClient"));
            dirs.push(base_path.join("Insolu").join("PrintClient"));
        }
        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            dirs.push(home_path.join("PrintClient"));
            dirs.push(home_path.join(".config").join("PrintClient"));
            dirs.push(home_path.join(".local").join("share").join("PrintClient"));
            dirs.push(home_path.join("Insolu").join("PrintClient"));
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

#[cfg(target_os = "linux")]
fn scan_process_exe_dir() -> Option<PathBuf> {
    // 遍历 /proc，按进程名（comm）匹配运行中的 PrintClient，取其可执行文件所在目录。
    const PROCESS_NAME_CANDIDATES: [&str; 4] =
        ["PrintClient", "printclient", "DriverClient", "driverclient"];

    for entry in fs::read_dir("/proc").ok()?.flatten() {
        let pid_dir = entry.path();
        // 仅看纯数字的 pid 目录。
        let is_pid = pid_dir
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()))
            .unwrap_or(false);
        if !is_pid {
            continue;
        }

        // 进程名：/proc/<pid>/comm（内核截断到 15 字符，候选名均在范围内）。
        let matched = fs::read_to_string(pid_dir.join("comm"))
            .ok()
            .map(|comm| {
                let name = comm.trim();
                PROCESS_NAME_CANDIDATES
                    .iter()
                    .any(|candidate| name.eq_ignore_ascii_case(candidate))
            })
            .unwrap_or(false);
        if !matched {
            continue;
        }

        // /proc/<pid>/exe 符号链接 → 可执行文件全路径 → 取所在目录。
        if let Ok(exe_path) = fs::read_link(pid_dir.join("exe")) {
            if let Some(dir) = exe_path.parent() {
                return Some(dir.to_path_buf());
            }
        }
    }

    None
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn scan_process_exe_dir() -> Option<PathBuf> {
    // 其余平台暂不支持按进程名发现 PrintClient。
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

/// 解析 DriverClient.ini 中的 `WebsocketPort=<port>` 字段。
fn parse_websocket_port(content: &str) -> Option<u16> {
    parse_ini_field(content, "WebsocketPort")
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port > 0)
}

/// 通用 INI 解析：取 `key=value`（忽略大小写键名、注释行、首尾空白与外层引号）。
fn parse_ini_field(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if let Some((found_key, value)) = trimmed.split_once('=') {
            if found_key.trim().eq_ignore_ascii_case(key) {
                let value = value.trim().trim_matches('"').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

/// configure.ini 缓存的服务端地址（与 PrintClient 发现共用同一组候选目录）。
#[derive(Clone, Default)]
struct ConfigureData {
    server_addr: Option<String>,
    center_server_addr: Option<String>,
}

/// 所有服务端请求使用的域名：
/// env(CPMS_SERVER_ADDR) → 会话 serverAddress（iframe 推送） → configure.ini 的 ServerAddr → None。
/// 形如 `https://127.0.0.1:8085`（已去尾部 `/`）。
pub(crate) fn cpms_server_base() -> Option<String> {
    env_base("CPMS_SERVER_ADDR")
        .or_else(crate::services::session_server::session_server_addr)
        .or_else(|| configure_data().server_addr)
}

fn env_base(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

/// 读取并缓存 configure.ini：已解析到 ServerAddr 则稳定复用，否则每 10s 重读
/// （PrintClient 可能后安装/后写配置）。
fn configure_data() -> ConfigureData {
    static CACHE: OnceLock<Mutex<Option<(Instant, ConfigureData)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    let Ok(mut guard) = cache.lock() else {
        return read_configure_data();
    };

    let reuse = matches!(
        guard.as_ref(),
        Some((scanned_at, data))
            if data.server_addr.is_some() || scanned_at.elapsed() < PROCESS_RESCAN_INTERVAL
    );

    if reuse {
        return guard
            .as_ref()
            .map(|(_, data)| data.clone())
            .unwrap_or_default();
    }

    let data = read_configure_data();
    *guard = Some((Instant::now(), data.clone()));
    data
}

fn read_configure_data() -> ConfigureData {
    let Some(path) = locate_named_config(CONFIGURE_INI) else {
        return ConfigureData::default();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return ConfigureData::default();
    };

    ConfigureData {
        server_addr: parse_ini_field(&content, "ServerAddr").map(normalize_base),
        center_server_addr: parse_ini_field(&content, "CenterServerAddr").map(normalize_base),
    }
}

fn normalize_base(addr: String) -> String {
    addr.trim().trim_end_matches('/').to_string()
}

/// 在候选目录（含运行进程目录）里定位指定配置文件。
fn locate_named_config(file_name: &str) -> Option<PathBuf> {
    for dir in print_client_candidate_dirs() {
        let path = dir.join(file_name);
        if path.is_file() {
            remember_print_client_dir(&dir);
            return Some(path);
        }
    }

    None
}

/// 持久化缓存文件：记录上次成功定位到 PrintClient 配置的目录。
/// 不依赖 AppHandle，用每用户稳定的缓存目录（Linux: $XDG_CACHE_HOME 或 ~/.cache；
/// Windows: %LOCALAPPDATA%），让 CPMS 未运行时也能据此读到 configure.ini。
fn printclient_cache_file() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
    base.map(|dir| dir.join("cpmsClient").join("printclient-dir"))
}

/// 读取缓存目录；为空或目录已不存在则视为失效返回 None。
fn cached_print_client_dir() -> Option<PathBuf> {
    let raw = fs::read_to_string(printclient_cache_file()?).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let dir = PathBuf::from(trimmed);
    dir.is_dir().then_some(dir)
}

/// 记住成功定位到的目录（与现值相同则跳过写盘）。best-effort，失败静默忽略。
fn remember_print_client_dir(dir: &Path) {
    if cached_print_client_dir().as_deref() == Some(dir) {
        return;
    }
    let Some(path) = printclient_cache_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, dir.to_string_lossy().as_bytes());
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
