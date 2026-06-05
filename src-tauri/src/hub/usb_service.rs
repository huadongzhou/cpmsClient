use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use super::events::{emit_job_error, emit_job_progress, emit_usb_state};
use super::http_service;
use super::models::{HubPreferences, ServerData, UsbData, UsbState, UserData};
use super::preferences::load_preferences;

const USB_SCAN_INTERVAL: Duration = Duration::from_secs(3);
const USB_DOWNLOAD_PREFIX: &str = "cpms_usb_prn";
const USB_DOWNLOADING_SUFFIX: &str = "-downloading";
const USB_DONE_SUFFIX: &str = "-downloaded.json";
const USB_PRINTING_SUFFIX: &str = "-printing";
const USB_PRINTED_SUFFIX: &str = "-printed";
const USB_JOB_LIST_PATH: &str = "/cpms/api/jobs/getUsbJobList";
const USB_DOWNLOAD_PATH: &str = "/cpms/api/jobs/downLoadUsbPdf";

struct UsbWorkerHandle {
    state: UsbState,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

#[derive(Clone)]
struct UsbContext {
    server: ServerData,
    user: UserData,
    usb_data: UsbData,
}

#[derive(Debug, Clone)]
struct UsbJob {
    id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UsbDownloadDoneFileData {
    uuid: String,
    job_id: String,
    file_name: String,
    file_size: u64,
}

fn runtime() -> &'static Mutex<Option<UsbWorkerHandle>> {
    static USB_RUNTIME: OnceLock<Mutex<Option<UsbWorkerHandle>>> = OnceLock::new();
    USB_RUNTIME.get_or_init(|| Mutex::new(None))
}

/// Starts the USB download worker. It is active only when persisted usbData exists.
pub fn start_usb_worker(app: AppHandle) -> Result<UsbState, String> {
    let preferences = load_preferences(&app)?;
    let state = state_from_preferences(&preferences, false);
    if preferences.usb_data.is_none() {
        return Ok(state);
    }

    let mut guard = runtime()
        .lock()
        .map_err(|_| "USB 服务状态锁已损坏".to_string())?;

    if let Some(handle) = guard.as_ref() {
        if !handle.stop.load(Ordering::SeqCst) {
            return Ok(handle.state.clone());
        }
    }

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;

    let running_state = UsbState {
        running: true,
        ..state_from_preferences(&preferences, true)
    };
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_app = app.clone();
    let join = thread::spawn(move || worker_loop(worker_app, cache_dir, worker_stop));

    *guard = Some(UsbWorkerHandle {
        state: running_state.clone(),
        stop,
        join: Some(join),
    });

    emit_usb_state(&app, running_state.clone());
    Ok(running_state)
}

/// Stops the USB download worker and returns the last persisted USB state.
pub fn stop_usb_worker(app: AppHandle) -> Result<UsbState, String> {
    let handle = {
        let mut guard = runtime()
            .lock()
            .map_err(|_| "USB 服务状态锁已损坏".to_string())?;
        guard.take()
    };

    if let Some(mut handle) = handle {
        handle.stop.store(true, Ordering::SeqCst);
        if let Some(join) = handle.join.take() {
            let _ = join.join();
        }
    }

    let preferences = load_preferences(&app)?;
    let state = state_from_preferences(&preferences, false);
    emit_usb_state(&app, state.clone());
    Ok(state)
}

/// Reads the current USB state, including whether the worker is running.
pub fn current_usb_state(app: &AppHandle) -> Result<UsbState, String> {
    let preferences = load_preferences(app)?;
    let running = runtime()
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .map(|handle| !handle.stop.load(Ordering::SeqCst))
        })
        .unwrap_or(false);

    Ok(state_from_preferences(&preferences, running))
}

/// Discovers USB printer hardware.
/// Current placeholder reads persisted `usb_data`; platform enumeration will be plugged in later.
pub fn discover_usb_printer(app: &AppHandle) -> Result<Option<UsbData>, String> {
    let preferences = load_preferences(app)?;
    // TODO: 预留后续平台枚举逻辑（如通过 libusb / WinUSB 枚举 USB 打印机）
    Ok(preferences.usb_data)
}

/// Writes a downloaded job file to the USB printer via bulk-out transfer.
/// Current placeholder only logs intent; actual bulk write will be implemented later.
pub fn write_usb_bulk_out(
    _app: &AppHandle,
    _job_id: &str,
    _file_path: &Path,
) -> Result<(), String> {
    // TODO: 预留后续分块写入、预热、调用 saveJobInfo / updateJobErrorStatus 逻辑
    Ok(())
}

fn worker_loop(app: AppHandle, cache_dir: PathBuf, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        if let Err(error) = run_download_scan(&app, &cache_dir) {
            emit_job_error(
                &app,
                json!({
                    "source": "usb-worker",
                    "code": "HUB_USB_WORKER_ERROR",
                    "message": error,
                }),
            );
        }

        sleep_until_next_scan(&stop);
    }
}

fn sleep_until_next_scan(stop: &AtomicBool) {
    let mut elapsed = Duration::ZERO;
    while elapsed < USB_SCAN_INTERVAL && !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(250));
        elapsed += Duration::from_millis(250);
    }
}

fn run_download_scan(app: &AppHandle, cache_dir: &Path) -> Result<usize, String> {
    let preferences = load_preferences(app)?;
    let Some(context) = build_usb_context(preferences) else {
        return Ok(0);
    };

    let jobs = fetch_usb_jobs(&context)?;
    let mut downloaded = 0_usize;

    for job in jobs {
        match download_usb_job(app, cache_dir, &context, &job) {
            Ok(true) => downloaded += 1,
            Ok(false) => {}
            Err(error) => emit_job_error(
                app,
                json!({
                    "source": "usb-worker",
                    "code": "HUB_USB_JOB_DOWNLOAD_ERROR",
                    "jobId": job.id,
                    "message": error,
                }),
            ),
        }
    }

    Ok(downloaded)
}

fn build_usb_context(preferences: HubPreferences) -> Option<UsbContext> {
    let user = preferences.user?;
    let token = user.token.as_deref()?.trim();
    if token.is_empty() {
        return None;
    }

    Some(UsbContext {
        server: preferences.server?,
        user,
        usb_data: preferences.usb_data?,
    })
}

fn fetch_usb_jobs(context: &UsbContext) -> Result<Vec<UsbJob>, String> {
    let uri = format!("{USB_JOB_LIST_PATH}/{}", context.usb_data.uuid);
    let url = http_service::build_cpms_url(&context.server, &uri)?;
    let token = context.user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), &uri, "")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let response = request.send().map_err(|error| error.to_string())?;

    let status = response.status();
    let body = response.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "USB 作业列表请求失败，HTTP status={status}，body={body}"
        ));
    }

    let payload = serde_json::from_str::<Value>(&body).map_err(|error| error.to_string())?;
    Ok(extract_usb_jobs(&payload))
}

fn extract_usb_jobs(payload: &Value) -> Vec<UsbJob> {
    let data = payload.get("data").unwrap_or(payload);
    let Some(items) = data.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            item.get("id")
                .or_else(|| item.get("jobId"))
                .and_then(|value| {
                    value
                        .as_str()
                        .map(str::to_string)
                        .or_else(|| value.as_i64().map(|id| id.to_string()))
                })
                .filter(|id| !id.trim().is_empty())
                .map(|id| UsbJob { id })
        })
        .collect()
}

fn download_usb_job(
    app: &AppHandle,
    cache_dir: &Path,
    context: &UsbContext,
    job: &UsbJob,
) -> Result<bool, String> {
    let file_name = usb_download_file_name(&context.usb_data.uuid, &job.id);
    let file_path = cache_dir.join(&file_name);
    let downloading_path = PathBuf::from(format!(
        "{}{USB_DOWNLOADING_SUFFIX}",
        file_path.to_string_lossy()
    ));
    let done_path = PathBuf::from(format!("{}{USB_DONE_SUFFIX}", file_path.to_string_lossy()));
    let printed_path = PathBuf::from(format!(
        "{}{USB_PRINTED_SUFFIX}",
        file_path.to_string_lossy()
    ));
    let printing_path = PathBuf::from(format!(
        "{}{USB_PRINTING_SUFFIX}",
        done_path.to_string_lossy()
    ));

    if done_path.exists() || printed_path.exists() || printing_path.exists() {
        return Ok(false);
    }

    let _lock = acquire_download_lock(&downloading_path)?;
    emit_job_progress(
        app,
        json!({
            "source": "usb-worker",
            "step": "downloading",
            "jobId": job.id,
            "uuid": context.usb_data.uuid,
            "filePath": file_path,
        }),
    );

    let result = download_file(context, &job.id, &file_path)
        .and_then(|file_size| write_done_file(&done_path, context, job, &file_name, file_size));
    let _ = fs::remove_file(&downloading_path);

    match result {
        Ok(file_size) => {
            emit_job_progress(
                app,
                json!({
                    "source": "usb-worker",
                    "step": "downloaded",
                    "jobId": job.id,
                    "uuid": context.usb_data.uuid,
                    "fileSize": file_size,
                    "filePath": file_path,
                    "donePath": done_path,
                }),
            );
            Ok(true)
        }
        Err(error) => Err(error),
    }
}

fn acquire_download_lock(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "USB 作业正在下载中".to_string()
            } else {
                error.to_string()
            }
        })
}

fn download_file(context: &UsbContext, job_id: &str, file_path: &Path) -> Result<u64, String> {
    let uri = format!("{USB_DOWNLOAD_PATH}/{job_id}");
    let url = http_service::build_cpms_url(&context.server, &uri)?;
    let token = context.user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), &uri, "")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30 * 60))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let mut response = request.send().map_err(|error| error.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "USB 文件下载失败，HTTP status={status}，body={body}"
        ));
    }

    let mut file = File::create(file_path).map_err(|error| error.to_string())?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut total_size = 0_u64;
    loop {
        let read_len = response
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read_len == 0 {
            break;
        }
        file.write_all(&buffer[..read_len])
            .map_err(|error| error.to_string())?;
        total_size = total_size.saturating_add(u64::try_from(read_len).unwrap_or_default());
    }
    file.flush().map_err(|error| error.to_string())?;

    Ok(total_size)
}

fn write_done_file(
    done_path: &Path,
    context: &UsbContext,
    job: &UsbJob,
    file_name: &str,
    file_size: u64,
) -> Result<u64, String> {
    let payload = UsbDownloadDoneFileData {
        uuid: context.usb_data.uuid.clone(),
        job_id: job.id.clone(),
        file_name: file_name.into(),
        file_size,
    };
    fs::write(
        done_path,
        serde_json::to_vec_pretty(&payload).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    Ok(file_size)
}

fn usb_download_file_name(uuid: &str, job_id: &str) -> String {
    format!(
        "{USB_DOWNLOAD_PREFIX}_{}_{}",
        sanitize_file_part(uuid),
        sanitize_file_part(job_id)
    )
}

fn sanitize_file_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .filter(|item| item.is_ascii_alphanumeric() || matches!(item, '-' | '_'))
        .take(80)
        .collect();

    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

fn state_from_preferences(preferences: &HubPreferences, running: bool) -> UsbState {
    UsbState {
        usb_printer_exists: preferences.usb_data.is_some(),
        usb_data: preferences.usb_data.clone(),
        running,
    }
}
