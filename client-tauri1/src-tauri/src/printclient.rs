//! 本地 PrintClient（cpms 客户端）发现：解析其配置文件得到 websocket 端口/地址，
//! 并向调试页暴露安装路径与 DriverClient.ini 内容。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::result::CommandResult;
use crate::{DEFAULT_LOCAL_SOCKET_PATH, DEFAULT_LOCAL_SOCKET_URL};

const DRIVER_CLIENT_INI: &str = "DriverClient.ini";
const CONFIG_FILE_NAMES: [&str; 3] = [DRIVER_CLIENT_INI, "config.conf", "config.ini"];

/// 本地 PrintClient 信息，供调试页展示。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrintClientInfo {
    /// 是否检测到 PrintClient 配置文件。
    pub(crate) installed: bool,
    /// 安装目录（配置文件所在目录）。
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
/// 读取本地 PrintClient 安装路径、DriverClient.ini 内容与 WebsocketPort，供调试页展示。
pub(crate) fn get_print_client_info() -> CommandResult<PrintClientInfo> {
    CommandResult::ok(discover_print_client_info())
}

pub(crate) fn discover_print_client_info() -> PrintClientInfo {
    let socket_url = local_socket_url();

    match locate_print_client_config() {
        Some(path) => {
            let content = fs::read_to_string(&path).ok();
            let websocket_port = content.as_deref().and_then(parse_websocket_port);
            PrintClientInfo {
                installed: true,
                dir: path.parent().map(|dir| dir.to_string_lossy().to_string()),
                config_path: Some(path.to_string_lossy().to_string()),
                websocket_port,
                socket_url,
                ini_content: content,
            }
        }
        None => PrintClientInfo {
            installed: false,
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

/// 定位 PrintClient 配置文件：优先 env 指定，其次候选目录下的 DriverClient.ini / config.*。
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

    if let Ok(dir) = std::env::var("CPMS_PRINTCLIENT_DIR") {
        dirs.push(PathBuf::from(dir));
    }

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
