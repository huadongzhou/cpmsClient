//! 本地 PrintClient（cpms 客户端）发现：解析其配置文件得到 websocket 端口/地址。

use std::fs;
use std::path::{Path, PathBuf};

use crate::{DEFAULT_LOCAL_SOCKET_PATH, DEFAULT_LOCAL_SOCKET_URL};

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
    if let Ok(config_path) = std::env::var("CPMS_PRINTCLIENT_CONFIG_PATH") {
        let path = PathBuf::from(config_path);
        if let Some(url) = socket_url_from_config_file(&path) {
            return Some(url);
        }
    }

    for dir in print_client_candidate_dirs() {
        for file_name in ["DriverClient.ini", "config.conf", "config.ini"] {
            let config_path = dir.join(file_name);
            if let Some(url) = socket_url_from_config_file(&config_path) {
                return Some(url);
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

    for line in raw.lines() {
        if let Some(url) = extract_websocket_url(line) {
            return Some(url);
        }
    }

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
