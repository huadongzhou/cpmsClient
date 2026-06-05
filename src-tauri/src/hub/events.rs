use serde_json::json;
use tauri::{AppHandle, Emitter};

use super::models::{NetworkState, PrintState, SocketState, StartupState, UsbState};

pub const HUB_SYSTEM_STATE_EVENT: &str = "cpms:hub-system-state";
pub const HUB_PRINT_STATE_EVENT: &str = "cpms:hub-print-state";
pub const HUB_USB_CHANGED_EVENT: &str = "cpms:hub-usb-changed";
pub const HUB_SOCKET_STATE_EVENT: &str = "cpms:hub-socket-state";
pub const HUB_NETWORK_CHANGED_EVENT: &str = "cpms:hub-network-changed";
pub const HUB_JOB_PROGRESS_EVENT: &str = "cpms:hub-job-progress";
pub const HUB_JOB_ERROR_EVENT: &str = "cpms:hub-job-error";

pub fn emit_hub_state(app: &AppHandle, state: &StartupState) {
    let _ = app.emit(HUB_SYSTEM_STATE_EVENT, state);
    emit_print_state(app, state.print_state.clone());
    emit_usb_state(app, state.usb_state.clone());
    emit_socket_state(app, state.socket_state.clone());
    emit_network_state(app, state.network_state.clone());
}

pub fn emit_background_state(app: &AppHandle, started: bool, updated_at: u128) {
    let _ = app.emit(
        HUB_SYSTEM_STATE_EVENT,
        json!({
            "backgroundStarted": started,
            "updatedAt": updated_at,
        }),
    );
}

pub fn emit_print_state(app: &AppHandle, state: PrintState) {
    let _ = app.emit(HUB_PRINT_STATE_EVENT, state);
}

pub fn emit_usb_state(app: &AppHandle, state: UsbState) {
    let _ = app.emit(HUB_USB_CHANGED_EVENT, state);
}

pub fn emit_socket_state(app: &AppHandle, state: SocketState) {
    let _ = app.emit(HUB_SOCKET_STATE_EVENT, state);
}

pub fn emit_network_state(app: &AppHandle, state: NetworkState) {
    let _ = app.emit(HUB_NETWORK_CHANGED_EVENT, state);
}

pub fn emit_job_progress(app: &AppHandle, payload: serde_json::Value) {
    let _ = app.emit(HUB_JOB_PROGRESS_EVENT, payload);
}

pub fn emit_job_error(app: &AppHandle, payload: serde_json::Value) {
    let _ = app.emit(HUB_JOB_ERROR_EVENT, payload);
}
