use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use tauri::{AppHandle, Manager};

const LOG_FILE_NAME: &str = "cpms-client.log";
const LOG_ROTATE_BYTES: u64 = 5 * 1024 * 1024;
const CLIENT_LOG_EVENT: &str = "cpms:client-log";
const MAIN_WINDOW_LABEL: &str = "main";

fn sink() -> &'static Mutex<Option<PathBuf>> {
    static SINK: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    SINK.get_or_init(|| Mutex::new(None))
}

/// 初始化日志系统：解析日志目录、写入启动分隔行，返回日志文件路径。
pub fn init(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path_resolver()
        .app_log_dir()
        .ok_or_else(|| "无法获取应用日志目录".to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join(LOG_FILE_NAME);

    if let Ok(mut locked) = sink().lock() {
        *locked = Some(path.clone());
    }

    write_line(&format_line(
        "INFO",
        "startup",
        &format!("==== cpmsClient v{} 进程启动 ====", env!("CARGO_PKG_VERSION")),
        None,
    ));
    Ok(path)
}

/// 当前日志文件路径与大小，供调试面板展示。
pub fn current_state() -> Option<(PathBuf, u64)> {
    let path = sink().lock().ok()?.clone()?;
    let size = fs::metadata(&path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    Some((path, size))
}

pub fn info(app: &AppHandle, source: &str, message: &str) {
    log(app, "INFO", source, message, None);
}

pub fn warn(app: &AppHandle, source: &str, message: &str) {
    log(app, "WARN", source, message, None);
}

pub fn error(app: &AppHandle, source: &str, message: &str) {
    log(app, "ERROR", source, message, None);
}

/// 记录一条客户端日志：写入日志文件，并推送给视图端日志面板。
pub fn log(app: &AppHandle, level: &str, source: &str, message: &str, detail: Option<&str>) {
    write_line(&format_line(level, source, message, detail));

    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_LOG_EVENT,
        json!({
            "at": timestamp(),
            "level": level.to_lowercase(),
            "source": source,
            "title": message,
            "detail": detail,
        }),
    );
}

/// 记录前端推送的日志：仅写入文件，不回发事件（前端已自行展示）。
pub fn log_from_frontend(level: &str, source: &str, message: &str, detail: Option<&str>) {
    write_line(&format_line(level, source, message, detail));
}

fn format_line(level: &str, source: &str, message: &str, detail: Option<&str>) -> String {
    let head = format!("[{}] [{level}] [{source}] {message}", timestamp());
    match detail {
        Some(detail) if !detail.trim().is_empty() => {
            format!("{head} | {}", detail.replace('\n', "\\n"))
        }
        _ => head,
    }
}

fn write_line(line: &str) {
    let Ok(guard) = sink().lock() else {
        return;
    };
    let Some(path) = guard.as_ref() else {
        return;
    };

    rotate_if_needed(path);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn rotate_if_needed(path: &PathBuf) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };

    if metadata.len() < LOG_ROTATE_BYTES {
        return;
    }

    let mut backup = path.as_os_str().to_owned();
    backup.push(".1");
    let backup = PathBuf::from(backup);
    let _ = fs::remove_file(&backup);
    let _ = fs::rename(path, &backup);
}

/// UTC 时间戳（yyyy-MM-dd HH:mm:ss.SSSZ）；不引入 chrono，避免改动 legacy 依赖锁。
fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    let tod = secs % 86_400;

    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}.{millis:03}Z",
        tod / 3_600,
        (tod % 3_600) / 60,
        tod % 60
    )
}

/// Howard Hinnant 的 civil_from_days 算法：epoch 天数转公历年月日。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;

    (if month <= 2 { year + 1 } else { year }, month, day)
}
