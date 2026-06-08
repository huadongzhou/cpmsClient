use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::{multipart, Client};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use super::events::{emit_job_error, emit_job_progress};
use super::http_service;
use super::models::{HubPreferences, PrintState, ServerData, UserData};
use super::preferences::load_preferences;

const PRINT_SCAN_INTERVAL: Duration = Duration::from_secs(3);
const PRINT_LOCK_SUFFIX: &str = "-doing";
const PRINT_UPLOADED_SUFFIX: &str = "-uploaded";
const UPLOAD_EXEC_PATH: &str = "/cpms/api/jobs/xps/exec";
const DEFAULT_CLIENT_IP: &str = "127.0.0.1";

struct PrintWorkerHandle {
    state: PrintState,
    stop: Arc<AtomicBool>,
    scan_join: Option<JoinHandle<()>>,
}

struct PrintJobCandidate {
    param_path: PathBuf,
    file_path: PathBuf,
    param: Value,
}

struct UploadContext {
    server: ServerData,
    user: UserData,
    product_type: i32,
    auth_direct_device: Option<Value>,
}

fn runtime() -> &'static Mutex<Option<PrintWorkerHandle>> {
    static PRINT_RUNTIME: OnceLock<Mutex<Option<PrintWorkerHandle>>> = OnceLock::new();
    PRINT_RUNTIME.get_or_init(|| Mutex::new(None))
}

/// Starts the print worker that scans cache files and submits pending jobs.
pub fn start_print_worker(app: AppHandle) -> Result<PrintState, String> {
    if !platform_print_supported() {
        return Ok(PrintState {
            print_server_ready: false,
            status: "unsupported".into(),
            ..PrintState::default()
        });
    }

    let mut guard = runtime()
        .lock()
        .map_err(|_| "打印服务状态锁已损坏".to_string())?;

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

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_app = app.clone();
    let scan_cache_dir = cache_dir.clone();
    let scan_join =
        Some(thread::spawn(move || worker_loop(worker_app, scan_cache_dir, worker_stop)));

    let state = PrintState {
        print_server_ready: true,
        status: "idle".into(),
        ..PrintState::default()
    };

    *guard = Some(PrintWorkerHandle {
        state: state.clone(),
        stop,
        scan_join,
    });

    Ok(state)
}

/// Stops the print worker and returns the unavailable print state.
pub fn stop_print_worker() -> Result<PrintState, String> {
    if !platform_print_supported() {
        return Ok(PrintState {
            print_server_ready: false,
            status: "unsupported".into(),
            ..PrintState::default()
        });
    }

    let handle = {
        let mut guard = runtime()
            .lock()
            .map_err(|_| "打印服务状态锁已损坏".to_string())?;
        guard.take()
    };

    if let Some(mut handle) = handle {
        handle.stop.store(true, Ordering::SeqCst);
        if let Some(join) = handle.scan_join.take() {
            let _ = join.join();
        }
    }

    Ok(PrintState {
        print_server_ready: false,
        status: "unavailable".into(),
        ..PrintState::default()
    })
}

const FIX_SCAN_INTERVAL: Duration = Duration::from_secs(8);

struct FixWorkerHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

fn fix_runtime() -> &'static Mutex<Option<FixWorkerHandle>> {
    static FIX_RUNTIME: OnceLock<Mutex<Option<FixWorkerHandle>>> = OnceLock::new();
    FIX_RUNTIME.get_or_init(|| Mutex::new(None))
}

/// Starts the printer fix worker that periodically restarts the print worker if it is unavailable.
pub fn start_printer_fix_worker(app: AppHandle) -> Result<PrintState, String> {
    if !platform_print_supported() {
        return Ok(PrintState {
            print_server_ready: false,
            status: "unsupported".into(),
            ..PrintState::default()
        });
    }

    let mut guard = fix_runtime()
        .lock()
        .map_err(|_| "打印机修复服务状态锁已损坏".to_string())?;

    if let Some(handle) = guard.as_ref() {
        if !handle.stop.load(Ordering::SeqCst) {
            let print_guard = runtime().lock().ok();
            if let Some(Some(ph)) = print_guard.as_ref().map(|g| g.as_ref()) {
                return Ok(ph.state.clone());
            }
            return Ok(PrintState {
                print_server_ready: false,
                status: "unavailable".into(),
                ..PrintState::default()
            });
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_app = app.clone();
    let join = thread::spawn(move || fix_worker_loop(worker_app, worker_stop));

    *guard = Some(FixWorkerHandle {
        stop,
        join: Some(join),
    });

    let print_guard = runtime().lock().ok();
    if let Some(Some(ph)) = print_guard.as_ref().map(|g| g.as_ref()) {
        Ok(ph.state.clone())
    } else {
        Ok(PrintState {
            print_server_ready: false,
            status: "unavailable".into(),
            ..PrintState::default()
        })
    }
}

/// Stops the printer fix worker and returns the unavailable print state.
pub fn stop_printer_fix_worker() -> Result<PrintState, String> {
    if !platform_print_supported() {
        return Ok(PrintState {
            print_server_ready: false,
            status: "unsupported".into(),
            ..PrintState::default()
        });
    }

    let handle = {
        let mut guard = fix_runtime()
            .lock()
            .map_err(|_| "打印机修复服务状态锁已损坏".to_string())?;
        guard.take()
    };

    if let Some(mut handle) = handle {
        handle.stop.store(true, Ordering::SeqCst);
        if let Some(join) = handle.join.take() {
            let _ = join.join();
        }
    }

    Ok(PrintState {
        print_server_ready: false,
        status: "unavailable".into(),
        ..PrintState::default()
    })
}

fn fix_worker_loop(app: AppHandle, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        if let Ok(preferences) = load_preferences(&app) {
            if preferences.user.is_some() {
                let is_unavailable = runtime()
                    .lock()
                    .ok()
                    .and_then(|guard| {
                        guard.as_ref().map(|h| {
                            h.stop.load(Ordering::SeqCst) || h.state.status == "unavailable"
                        })
                    })
                    .unwrap_or(true);

                if is_unavailable {
                    thread::sleep(Duration::from_secs(3));
                    if !stop.load(Ordering::SeqCst) {
                        let _ = start_print_worker(app.clone());
                    }
                }
            }
        }

        sleep_fix_interval(&stop);
    }
}

fn sleep_fix_interval(stop: &AtomicBool) {
    let mut elapsed = Duration::ZERO;
    while elapsed < FIX_SCAN_INTERVAL && !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(250));
        elapsed += Duration::from_millis(250);
    }
}

fn worker_loop(app: AppHandle, cache_dir: PathBuf, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        if let Err(error) = run_scan(&app, &cache_dir) {
            emit_job_error(
                &app,
                json!({
                    "source": "print-worker",
                    "code": "HUB_PRINT_WORKER_ERROR",
                    "message": error,
                }),
            );
        }

        sleep_until_next_scan(&stop);
    }
}

fn sleep_until_next_scan(stop: &AtomicBool) {
    let mut elapsed = Duration::ZERO;
    while elapsed < PRINT_SCAN_INTERVAL && !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(250));
        elapsed += Duration::from_millis(250);
    }
}

#[cfg(target_os = "linux")]
fn platform_print_supported() -> bool {
    true
}

#[cfg(not(target_os = "linux"))]
fn platform_print_supported() -> bool {
    false
}

fn run_scan(app: &AppHandle, cache_dir: &Path) -> Result<usize, String> {
    let preferences = load_preferences(app)?;
    let Some(context) = build_upload_context(preferences) else {
        return Ok(0);
    };

    let candidates = find_print_jobs(cache_dir)?;
    let mut handled = 0_usize;

    for candidate in candidates {
        match handle_print_job(app, candidate, &context) {
            Ok(uploaded) => {
                if uploaded {
                    handled += 1;
                }
            }
            Err(error) => {
                emit_job_error(
                    app,
                    json!({
                        "source": "print-worker",
                        "code": "HUB_PRINT_JOB_UPLOAD_ERROR",
                        "message": error,
                    }),
                );
            }
        }
    }

    Ok(handled)
}

fn build_upload_context(preferences: HubPreferences) -> Option<UploadContext> {
    let user = preferences.user?;
    let token = user.token.as_deref()?.trim();
    if token.is_empty() {
        return None;
    }

    Some(UploadContext {
        server: preferences.server?,
        user,
        product_type: preferences.product_type,
        auth_direct_device: preferences.auth_direct_device,
    })
}

fn find_print_jobs(cache_dir: &Path) -> Result<Vec<PrintJobCandidate>, String> {
    if !cache_dir.exists() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(cache_dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();

        if !path.is_file() || !is_pending_param_file(&path) {
            continue;
        }

        let raw = match fs::read_to_string(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let param = match serde_json::from_str::<Value>(&raw) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(file_path) = param
            .get("filePath")
            .and_then(Value::as_str)
            .map(PathBuf::from)
        else {
            continue;
        };

        if !file_path.exists() {
            continue;
        }

        candidates.push(PrintJobCandidate {
            param_path: path,
            file_path,
            param,
        });
    }

    candidates.sort_by_key(|candidate| {
        candidate
            .param_path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH)
    });

    Ok(candidates)
}

fn is_pending_param_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    name.starts_with("cpms_hm_")
        && name.ends_with(".json")
        && !name.ends_with(".json-doing")
        && !name.ends_with(".json-uploaded")
}

fn handle_print_job(
    app: &AppHandle,
    candidate: PrintJobCandidate,
    context: &UploadContext,
) -> Result<bool, String> {
    let lock_path = PathBuf::from(format!(
        "{}{PRINT_LOCK_SUFFIX}",
        candidate.param_path.to_string_lossy()
    ));
    let _lock = acquire_lock(&lock_path)?;

    emit_job_progress(
        app,
        json!({
            "source": "print-worker",
            "step": "uploading",
            "paramPath": candidate.param_path,
            "filePath": candidate.file_path,
        }),
    );

    match upload_print_file(&candidate, context) {
        Ok(_) => {
            rename_uploaded(&candidate.param_path)?;
            rename_uploaded(&candidate.file_path)?;
            let _ = fs::remove_file(&lock_path);
            emit_job_progress(
                app,
                json!({
                    "source": "print-worker",
                    "step": "uploaded",
                    "paramPath": candidate.param_path,
                    "filePath": candidate.file_path,
                }),
            );
            Ok(true)
        }
        Err(error) => {
            let _ = fs::remove_file(&lock_path);
            Err(error)
        }
    }
}

fn acquire_lock(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "打印任务正在处理中".to_string()
            } else {
                error.to_string()
            }
        })
}

fn upload_print_file(candidate: &PrintJobCandidate, context: &UploadContext) -> Result<(), String> {
    let params = build_print_query_params(&candidate.param, context);
    let sign_query = http_service::query_string(&params, false);
    let url = format!(
        "{}?{}",
        http_service::build_cpms_url(&context.server, UPLOAD_EXEC_PATH)?,
        http_service::query_string(&params, true)
    );
    let token = context.user.token.as_deref().unwrap_or_default();
    let headers = http_service::build_signed_headers(Some(token), UPLOAD_EXEC_PATH, &sign_query)?;

    let form = multipart::Form::new()
        .file("file", &candidate.file_path)
        .map_err(|error| error.to_string())?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30 * 60))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.post(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let response = request
        .multipart(form)
        .send()
        .map_err(|error| error.to_string())?;

    let status = response.status();
    let body = response.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("上传失败，HTTP status={status}，body={body}"));
    }

    if let Ok(payload) = serde_json::from_str::<Value>(&body) {
        let code = payload.get("code").and_then(Value::as_i64);
        if !matches!(code, Some(200) | None) {
            return Err(format!("上传失败，服务端响应={payload}"));
        }
    }

    Ok(())
}

fn build_print_query_params(param: &Value, context: &UploadContext) -> Vec<(String, String)> {
    let print_properties = param.get("printProperties").unwrap_or(param);
    let document_name =
        normalized_document_name(text_field(print_properties, "documentName", "print.pdf"));
    let paper = text_field(print_properties, "paper", "A4");
    let paper = if paper.starts_with("ISO") {
        paper
    } else {
        format!("ISO{paper}")
    };
    let duplexing = text_field(print_properties, "duplexing", "TwoSided");
    let duplexing = if duplexing == "None" {
        "TwoSided".into()
    } else {
        duplexing
    };

    let mut params = vec![
        ("fileSuffix".into(), "pdf".into()),
        ("driverType".into(), "pdf".into()),
        (
            "clientIp".into(),
            text_field(print_properties, "clientIp", DEFAULT_CLIENT_IP),
        ),
        ("printProperties.driverName".into(), "PdfDriver".into()),
        ("printProperties.portShared".into(), "0".into()),
        ("printProperties.terminalType".into(), "harmony".into()),
        (
            "printProperties.pageCount".into(),
            text_field(print_properties, "pageCount", "1"),
        ),
        (
            "printProperties.copyCount".into(),
            text_field(print_properties, "copyCount", "1"),
        ),
        ("printProperties.paper".into(), paper),
        ("printProperties.duplexing".into(), duplexing),
        (
            "printProperties.color".into(),
            text_field(print_properties, "color", "Color"),
        ),
        (
            "printProperties.pageOrientation".into(),
            text_field(print_properties, "pageOrientation", "Portrait"),
        ),
        (
            "printProperties.documentCollate".into(),
            text_field(print_properties, "documentCollate", "Uncollate"),
        ),
        ("printProperties.isPSDriver".into(), "true".into()),
        ("title".into(), document_name.clone()),
        ("printProperties.documentName".into(), document_name),
    ];

    if let Some(device_id) = direct_device_id(context.auth_direct_device.as_ref()) {
        params.push(("directDeviceId".into(), device_id));
    }
    params.push(("productType".into(), context.product_type.to_string()));

    params
}

fn text_field(value: &Value, key: &str, default_value: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_value.into())
}

fn normalized_document_name(value: String) -> String {
    let mut next = value.replace(['#', '?', '&', '='], "");
    if !next.to_lowercase().ends_with(".pdf") {
        next.push_str(".pdf");
    }
    next
}

fn direct_device_id(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|raw| {
            raw.get("did")
                .or_else(|| raw.get("deviceId"))
                .or_else(|| raw.get("id"))
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
}

fn rename_uploaded(path: &Path) -> Result<(), String> {
    let uploaded_path = PathBuf::from(format!("{}{PRINT_UPLOADED_SUFFIX}", path.to_string_lossy()));
    if uploaded_path.exists() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or_default();
        let archived_path = PathBuf::from(format!(
            "{}.{stamp}{PRINT_UPLOADED_SUFFIX}",
            path.to_string_lossy()
        ));
        fs::rename(path, archived_path).map_err(|error| error.to_string())
    } else {
        fs::rename(path, uploaded_path).map_err(|error| error.to_string())
    }
}
