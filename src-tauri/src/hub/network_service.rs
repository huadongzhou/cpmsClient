use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tauri::AppHandle;

use super::events::emit_network_state;
use super::models::NetworkState;
use super::print_service;
use super::socket_server;

const CHECK_INTERVAL: Duration = Duration::from_secs(3);
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);
const CHECK_ADDR: &str = "8.8.8.8:53";

struct NetworkMonitorHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

fn runtime() -> &'static Mutex<Option<NetworkMonitorHandle>> {
    static RUNTIME: OnceLock<Mutex<Option<NetworkMonitorHandle>>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(None))
}

/// Starts a background thread that monitors network connectivity.
/// When the state changes, it emits `cpms:hub-network-changed` and
/// replicates hm-client behaviour: offline stops print/socket services;
/// online restarts them.
pub fn start_network_monitor(app: AppHandle, product_type: i32) -> Result<(), String> {
    let mut guard = runtime()
        .lock()
        .map_err(|_| "网络监听状态锁已损坏".to_string())?;

    if let Some(handle) = guard.as_ref() {
        if !handle.stop.load(Ordering::SeqCst) {
            return Ok(());
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_app = app.clone();

    let join = thread::spawn(move || monitor_loop(worker_app, worker_stop, product_type));

    *guard = Some(NetworkMonitorHandle { stop, join: Some(join) });
    Ok(())
}

/// Stops the network monitor thread.
pub fn stop_network_monitor() -> Result<(), String> {
    let handle = {
        let mut guard = runtime()
            .lock()
            .map_err(|_| "网络监听状态锁已损坏".to_string())?;
        guard.take()
    };

    if let Some(mut handle) = handle {
        handle.stop.store(true, Ordering::SeqCst);
        if let Some(join) = handle.join.take() {
            let _ = join.join();
        }
    }

    Ok(())
}

fn monitor_loop(app: AppHandle, stop: Arc<AtomicBool>, product_type: i32) {
    let mut last_online: Option<bool> = None;

    while !stop.load(Ordering::SeqCst) {
        let online = check_network_online();

        if last_online != Some(online) {
            last_online = Some(online);
            emit_network_state(&app, NetworkState { online });

            if !online {
                let _ = print_service::stop_print_worker();
                let _ = print_service::stop_printer_fix_worker();
                let _ = socket_server::stop_socket_server();
            } else {
                let _ = print_service::start_print_worker(app.clone());
                let _ = print_service::start_printer_fix_worker(app.clone());
                let _ = socket_server::start_socket_server(app.clone(), product_type);
            }
        }

        sleep_until(&stop, CHECK_INTERVAL);
    }
}

fn check_network_online() -> bool {
    let addr: SocketAddr = match CHECK_ADDR.parse() {
        Ok(value) => value,
        Err(_) => return false,
    };

    TcpStream::connect_timeout(&addr, CHECK_TIMEOUT).is_ok()
}

fn sleep_until(stop: &AtomicBool, duration: Duration) {
    let mut elapsed = Duration::ZERO;
    while elapsed < duration && !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(250));
        elapsed += Duration::from_millis(250);
    }
}
