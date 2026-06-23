use std::process::Command;
use std::time::Instant;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPingResult {
    pub ok: bool,
    pub host: String,
    pub elapsed: u128,
    pub message: String,
}

pub fn ping_address(host: &str) -> Result<ClientPingResult, String> {
    let host = normalize_host(host)?;
    let begin = Instant::now();

    let output = if cfg!(target_os = "windows") {
        Command::new("ping")
            .args(["-n", "1", "-w", "1000", &host])
            .output()
    } else {
        Command::new("ping")
            .args(["-c", "1", "-W", "1", &host])
            .output()
    }
    .map_err(|error| format!("执行 ping 失败：{error}"))?;

    let elapsed = begin.elapsed().as_millis();

    if !output.status.success() {
        return Err(format!("Ping {host} 失败"));
    }

    Ok(ClientPingResult {
        ok: true,
        host: host.clone(),
        elapsed,
        message: format!("Ping {host} 成功"),
    })
}

fn normalize_host(input: &str) -> Result<String, String> {
    let value = input.trim();
    if value.is_empty() {
        return Err("服务地址不能为空".to_string());
    }

    let without_protocol = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or(value);
    let without_path = without_protocol
        .split('/')
        .next()
        .unwrap_or(without_protocol);
    let host = without_path
        .split(':')
        .next()
        .unwrap_or(without_path)
        .trim();

    if host.is_empty() {
        return Err("服务地址不能为空".to_string());
    }

    Ok(host.to_string())
}
