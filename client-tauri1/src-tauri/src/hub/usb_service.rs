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
use tauri::AppHandle;

use super::events::{emit_job_error, emit_job_progress, emit_usb_state};
use super::http_service;
use super::models::{HubPreferences, ServerData, UsbData, UsbState, UserData};
use super::preferences::{load_preferences, update_preferences};

const USB_SCAN_INTERVAL: Duration = Duration::from_secs(3);
const USB_WRITE_SCAN_INTERVAL: Duration = Duration::from_secs(1);
const USB_DOWNLOAD_PREFIX: &str = "cpms_usb_prn";
const USB_DOWNLOADING_SUFFIX: &str = "-downloading";
const USB_DONE_SUFFIX: &str = "-downloaded.json";
const USB_PRINTING_SUFFIX: &str = "-printing";
const USB_PRINTED_SUFFIX: &str = "-printed";
const USB_JOB_LIST_PATH: &str = "/cpms/api/jobs/getUsbJobList";
const USB_DOWNLOAD_PATH: &str = "/cpms/api/jobs/downLoadUsbPdf";
const USB_SAVE_JOB_PATH: &str = "/cpms/api/jobs/saveJobInfo";
const USB_UPDATE_JOB_ERROR_PATH: &str = "/cpms/api/jobs/updateJobErrorStatus";
const USB_BULK_CHUNK_SIZE: usize = 180 * 1024;

struct UsbWorkerHandle {
    state: UsbState,
    stop: Arc<AtomicBool>,
    download_join: Option<JoinHandle<()>>,
    write_join: Option<JoinHandle<()>>,
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

/// Starts the USB download and write workers.
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
        .path_resolver()
        .app_cache_dir()
        .ok_or_else(|| "无法获取应用缓存目录".to_string())?;
    fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;

    let running_state = UsbState {
        running: true,
        ..state_from_preferences(&preferences, true)
    };
    let stop = Arc::new(AtomicBool::new(false));
    let download_stop = Arc::clone(&stop);
    let write_stop = Arc::clone(&stop);
    let download_app = app.clone();
    let write_app = app.clone();
    let write_cache_dir = cache_dir.clone();

    let download_join = thread::spawn(move || {
        download_worker_loop(download_app, cache_dir, download_stop)
    });
    let write_join = thread::spawn(move || {
        write_worker_loop(write_app, write_cache_dir, write_stop)
    });

    *guard = Some(UsbWorkerHandle {
        state: running_state.clone(),
        stop,
        download_join: Some(download_join),
        write_join: Some(write_join),
    });

    emit_usb_state(&app, running_state.clone());
    Ok(running_state)
}

/// Stops the USB workers and returns the last persisted USB state.
pub fn stop_usb_worker(app: AppHandle) -> Result<UsbState, String> {
    let handle = {
        let mut guard = runtime()
            .lock()
            .map_err(|_| "USB 服务状态锁已损坏".to_string())?;
        guard.take()
    };

    if let Some(mut handle) = handle {
        handle.stop.store(true, Ordering::SeqCst);
        if let Some(join) = handle.download_join.take() {
            let _ = join.join();
        }
        if let Some(join) = handle.write_join.take() {
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

/// Discovers USB printer hardware via nusb enumeration.
/// Falls back to persisted `usb_data` if no printer is found.
pub fn discover_usb_printer(app: &AppHandle) -> Result<Option<UsbData>, String> {
    match discover_usb_printer_hardware() {
        Ok(Some(usb_data)) => {
            let _ = update_preferences(app, |preferences| {
                preferences.usb_data = Some(usb_data.clone());
            });
            return Ok(Some(usb_data));
        }
        Ok(None) => {}
        Err(error) => {
            emit_job_error(
                app,
                json!({
                    "source": "usb-discovery",
                    "code": "HUB_USB_DISCOVER_WARNING",
                    "message": error,
                }),
            );
        }
    }

    let preferences = load_preferences(app)?;
    Ok(preferences.usb_data)
}

fn discover_usb_printer_hardware() -> Result<Option<UsbData>, String> {
    let devices = nusb::list_devices().map_err(|error| error.to_string())?;

    for device_info in devices {
        for interface in device_info.interfaces() {
            if interface.class() == 7 {
                let manufacturer = device_info
                    .manufacturer_string()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Vendor_{:04X}", device_info.vendor_id()));
                let product = device_info
                    .product_string()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Product_{:04X}", device_info.product_id()));
                let uuid = generate_stable_uuid(&format!("{}_{}", product, manufacturer));

                return Ok(Some(UsbData {
                    manufacturer_name: manufacturer,
                    product_name: product,
                    uuid,
                }));
            }
        }
    }

    Ok(None)
}

fn generate_stable_uuid(key: &str) -> String {
    let result = md5::compute(key.as_bytes());
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_le_bytes([result[0], result[1], result[2], result[3]]),
        u16::from_le_bytes([result[4], result[5]]),
        u16::from_le_bytes([result[6], result[7]]),
        u16::from_le_bytes([result[8], result[9]]),
        u64::from_le_bytes([0, 0, result[10], result[11], result[12], result[13], result[14], result[15]])
    )
}

/// Writes a downloaded job file to the USB printer via bulk-out transfer.
pub fn write_usb_bulk_out(
    app: &AppHandle,
    job_id: &str,
    file_path: &Path,
) -> Result<(), String> {
    let preferences = load_preferences(app)?;
    let Some(usb_data) = preferences.usb_data else {
        return Err("未配置 USB 打印机".to_string());
    };

    let file_data = std::fs::read(file_path).map_err(|error| error.to_string())?;
    if file_data.is_empty() {
        return Err("USB 打印文件为空".to_string());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    rt.block_on(async {
        do_usb_bulk_write(&usb_data, &file_data).await
    })
    .map_err(|error| {
        let _ = report_usb_job_error(app, &usb_data, job_id, &error);
        error
    })?;

    if let Err(error) = report_usb_job_saved(app, &usb_data, job_id) {
        emit_job_error(
            app,
            json!({
                "source": "usb-write",
                "code": "HUB_USB_REPORT_SAVE_WARNING",
                "jobId": job_id,
                "message": error,
            }),
        );
    }

    Ok(())
}

async fn do_usb_bulk_write(usb_data: &UsbData, file_data: &[u8]) -> Result<(), String> {
    let mut devices = nusb::list_devices().map_err(|error| error.to_string())?;

    let device_info = devices
        .find(|device| {
            device.manufacturer_string().as_deref() == Some(&usb_data.manufacturer_name)
                && device.product_string().as_deref() == Some(&usb_data.product_name)
        })
        .ok_or("未找到匹配的 USB 打印机")?;

    let device = device_info.open().map_err(|error| error.to_string())?;
    let config = device
        .active_configuration()
        .map_err(|error| error.to_string())?;

    let (interface_number, endpoint_address) = find_printer_bulk_out_endpoint(&config)
        .ok_or("未找到打印机 Bulk OUT 接口")?;

    let interface = device
        .claim_interface(interface_number)
        .map_err(|error| error.to_string())?;

    let total = file_data.len();
    let mut offset = 0_usize;

    while offset < total {
        let end = (offset + USB_BULK_CHUNK_SIZE).min(total);
        let chunk = file_data[offset..end].to_vec();

        let completion = interface.bulk_out(endpoint_address, chunk).await;
        let transferred = completion
            .into_result()
            .map_err(|error| format!("USB Bulk 传输失败: {:?}", error))?;

        offset += transferred.actual_length();
    }

    Ok(())
}

fn find_printer_bulk_out_endpoint(
    config: &nusb::descriptors::Configuration,
) -> Option<(u8, u8)> {
    for interface in config.interfaces() {
        for alt in interface.alt_settings() {
            if alt.class() == 7 {
                for endpoint in alt.endpoints() {
                    if endpoint.address() < 0x80
                        && endpoint.transfer_type() == nusb::transfer::EndpointType::Bulk
                    {
                        return Some((alt.interface_number(), endpoint.address()));
                    }
                }
            }
        }
    }
    None
}

fn download_worker_loop(app: AppHandle, cache_dir: PathBuf, stop: Arc<AtomicBool>) {
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

        sleep_until(&stop, USB_SCAN_INTERVAL);
    }
}

fn write_worker_loop(app: AppHandle, cache_dir: PathBuf, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        if let Err(error) = run_write_scan(&app, &cache_dir) {
            emit_job_error(
                &app,
                json!({
                    "source": "usb-write-worker",
                    "code": "HUB_USB_WRITE_WORKER_ERROR",
                    "message": error,
                }),
            );
        }

        sleep_until(&stop, USB_WRITE_SCAN_INTERVAL);
    }
}

fn sleep_until(stop: &AtomicBool, duration: Duration) {
    let mut elapsed = Duration::ZERO;
    while elapsed < duration && !stop.load(Ordering::SeqCst) {
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

fn run_write_scan(app: &AppHandle, cache_dir: &Path) -> Result<usize, String> {
    let preferences = load_preferences(app)?;
    if preferences.usb_data.is_none() {
        return Ok(0);
    }

    let entries = fs::read_dir(cache_dir).map_err(|error| error.to_string())?;
    let mut processed = 0_usize;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if !name.starts_with(USB_DOWNLOAD_PREFIX) || !name.ends_with(USB_DONE_SUFFIX) {
            continue;
        }

        let pdf_path = path.with_extension("").with_extension("");
        let printing_path = PathBuf::from(format!(
            "{}{}",
            path.to_string_lossy(),
            USB_PRINTING_SUFFIX
        ));
        let printed_path = PathBuf::from(format!(
            "{}{}",
            path.to_string_lossy(),
            USB_PRINTED_SUFFIX
        ));

        if printed_path.exists() || printing_path.exists() {
            continue;
        }

        let done_data: UsbDownloadDoneFileData = match fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
        {
            Some(data) => data,
            None => continue,
        };

        if !pdf_path.exists() {
            continue;
        }

        let pdf_meta = fs::metadata(&pdf_path).map_err(|error| error.to_string())?;
        if pdf_meta.len() != done_data.file_size {
            continue;
        }

        let _lock = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&printing_path)
        {
            Ok(file) => file,
            Err(_) => continue,
        };

        emit_job_progress(
            app,
            json!({
                "source": "usb-write-worker",
                "step": "printing",
                "jobId": done_data.job_id,
                "uuid": done_data.uuid,
                "filePath": pdf_path,
            }),
        );

        match write_usb_bulk_out(app, &done_data.job_id, &pdf_path) {
            Ok(()) => {
                let _ = fs::rename(&path, &printed_path);
                let _ = fs::rename(&pdf_path, format!("{}{}", pdf_path.to_string_lossy(), USB_PRINTED_SUFFIX));
                let _ = fs::remove_file(&printing_path);
                processed += 1;
            }
            Err(error) => {
                let _ = fs::remove_file(&printing_path);
                emit_job_error(
                    app,
                    json!({
                        "source": "usb-write-worker",
                        "code": "HUB_USB_WRITE_ERROR",
                        "jobId": done_data.job_id,
                        "message": error,
                    }),
                );
            }
        }
    }

    Ok(processed)
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

fn report_usb_job_saved(
    app: &AppHandle,
    _usb_data: &UsbData,
    job_id: &str,
) -> Result<(), String> {
    let preferences = load_preferences(app)?;
    let Some(server) = preferences.server else {
        return Err("服务器未配置".to_string());
    };
    let Some(user) = preferences.user else {
        return Err("用户未登录".to_string());
    };

    let uri = format!("{USB_SAVE_JOB_PATH}/{job_id}");
    let url = http_service::build_cpms_url(&server, &uri)?;
    let token = user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), &uri, "")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }

    let response = request.send().map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("saveJobInfo 失败: HTTP {}", response.status()));
    }

    Ok(())
}

fn report_usb_job_error(
    app: &AppHandle,
    _usb_data: &UsbData,
    job_id: &str,
    error_msg: &str,
) -> Result<(), String> {
    let preferences = load_preferences(app)?;
    let Some(server) = preferences.server else {
        return Err("服务器未配置".to_string());
    };
    let Some(user) = preferences.user else {
        return Err("用户未登录".to_string());
    };

    let uri = format!("{USB_UPDATE_JOB_ERROR_PATH}/{job_id}");
    let url = http_service::build_cpms_url(&server, &uri)?;
    let token = user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), &uri, "")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }

    let response = request.send().map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "updateJobErrorStatus 失败: HTTP {}",
            response.status()
        ));
    }

    let _ = error_msg;
    Ok(())
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
