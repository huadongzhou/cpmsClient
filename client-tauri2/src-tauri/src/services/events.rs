use serde_json::json;
use tauri::{AppHandle, Emitter};

use super::models::{NetworkState, StartupState};

pub const HUB_SYSTEM_STATE_EVENT: &str = "cpms:hub-system-state";
pub const HUB_NETWORK_CHANGED_EVENT: &str = "cpms:hub-network-changed";

pub fn emit_hub_state(app: &AppHandle, state: &StartupState) {
    let _ = app.emit(HUB_SYSTEM_STATE_EVENT, state);
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

pub fn emit_network_state(app: &AppHandle, state: NetworkState) {
    let _ = app.emit(HUB_NETWORK_CHANGED_EVENT, state);
}
