use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::AppHandle;

use crate::models::CommandResult;

use super::crypto_service;
use super::events::{
    emit_background_state, emit_hub_state, emit_print_state, emit_socket_state, emit_usb_state,
};
use super::models::{
    startup_state_from_preferences, AppVersion, AuthPersistState, PrintState, ServerData,
    SocketState, StartupState, UsbState,
};
use super::preferences::{load_preferences, update_preferences};
use super::print_service;
use super::socket_server;
use super::usb_service;

#[tauri::command]
/// Reads the persisted Hub startup state used by the Web app during route hydration.
pub fn get_startup_state(app: AppHandle) -> CommandResult<StartupState> {
    let preferences = match load_preferences(&app) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_PREFERENCES_READ_ERROR", &error),
    };

    CommandResult::ok(startup_state_from_preferences(preferences))
}

#[tauri::command]
/// Persists that the user accepted the privacy policy.
pub fn save_policy_agreed(app: AppHandle) -> CommandResult<bool> {
    update_preferences(&app, |preferences| {
        preferences.policy_agreed = true;
    })
    .map_or_else(
        |error| CommandResult::fail("HUB_POLICY_SAVE_ERROR", &error),
        |_| CommandResult::ok(true),
    )
}

#[tauri::command]
/// Persists authenticated user, server, product type, and optional server init data.
pub fn save_auth_state(app: AppHandle, state: AuthPersistState) -> CommandResult<StartupState> {
    let result = update_preferences(&app, |preferences| {
        preferences.user = Some(state.user.clone());
        preferences.server = Some(state.server.clone());
        preferences.product_type = state.product_type;
        preferences.system_init_data = state.system_init_data.clone();
    });

    if let Err(error) = result {
        return CommandResult::fail("HUB_AUTH_SAVE_ERROR", &error);
    }

    load_and_emit_startup_state(&app, "HUB_PREFERENCES_READ_ERROR")
}

#[tauri::command]
/// Clears authentication-related local state while keeping reusable non-auth settings.
pub fn clear_auth_state(app: AppHandle) -> CommandResult<StartupState> {
    let result = update_preferences(&app, |preferences| {
        preferences.user = None;
        preferences.product_type = -1;
        preferences.system_init_data = None;
        preferences.auth_direct_device = None;
    });

    if let Err(error) = result {
        return CommandResult::fail("HUB_AUTH_CLEAR_ERROR", &error);
    }

    load_and_emit_startup_state(&app, "HUB_PREFERENCES_READ_ERROR")
}

#[tauri::command]
/// Saves the latest CPMS server endpoint selected by the user.
pub fn save_server_info(app: AppHandle, server: ServerData) -> CommandResult<ServerData> {
    update_preferences(&app, |preferences| {
        preferences.server = Some(server.clone());
    })
    .map_or_else(
        |error| CommandResult::fail("HUB_SERVER_SAVE_ERROR", &error),
        |_| CommandResult::ok(server),
    )
}

#[tauri::command]
/// Saves the direct-output printer selected by the user.
pub fn save_direct_device(app: AppHandle, device: Value) -> CommandResult<Value> {
    update_preferences(&app, |preferences| {
        preferences.auth_direct_device = Some(device.clone());
    })
    .map_or_else(
        |error| CommandResult::fail("HUB_DIRECT_DEVICE_SAVE_ERROR", &error),
        |_| CommandResult::ok(device),
    )
}

#[tauri::command]
/// Initializes Hub system capabilities and emits their current runtime state.
pub fn system_init(app: AppHandle) -> CommandResult<StartupState> {
    let preferences = match load_preferences(&app) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_SYSTEM_INIT_ERROR", &error),
    };

    let product_type = preferences.product_type;
    let should_start_socket = has_auth_token(&preferences.user);
    let mut startup_state = startup_state_from_preferences(preferences);

    if should_start_socket {
        match print_service::start_print_worker(app.clone()) {
            Ok(print_state) => {
                startup_state.print_state = print_state;
            }
            Err(error) => return CommandResult::fail("HUB_PRINT_START_ERROR", &error),
        }

        match print_service::start_printer_fix_worker(app.clone()) {
            Ok(print_state) => {
                startup_state.print_state = print_state;
            }
            Err(error) => return CommandResult::fail("HUB_PRINT_FIX_START_ERROR", &error),
        }

        match socket_server::start_socket_server(app.clone(), product_type) {
            Ok(socket_state) => {
                startup_state.socket_state = socket_state;
            }
            Err(error) => return CommandResult::fail("HUB_SOCKET_START_ERROR", &error),
        }

        match usb_service::start_usb_worker(app.clone()) {
            Ok(usb_state) => {
                startup_state.usb_state = usb_state;
            }
            Err(error) => return CommandResult::fail("HUB_USB_START_ERROR", &error),
        }
    }

    emit_hub_state(&app, &startup_state);
    CommandResult::ok(startup_state)
}

#[tauri::command]
/// Releases Hub system capabilities before logout, close, or shutdown.
pub fn system_destroy(app: AppHandle) -> CommandResult<bool> {
    let print_state = print_service::stop_print_worker().unwrap_or_else(|_| PrintState {
        print_server_ready: false,
        status: "unavailable".into(),
        ..PrintState::default()
    });
    let _ = print_service::stop_printer_fix_worker();
    let socket_state = socket_server::stop_socket_server().unwrap_or_default();
    let usb_state = usb_service::stop_usb_worker(app.clone()).unwrap_or_default();

    emit_print_state(&app, print_state);
    emit_socket_state(&app, socket_state);
    emit_usb_state(&app, usb_state);
    CommandResult::ok(true)
}

#[tauri::command]
/// Starts background workers and emits a background-running state event.
pub fn start_background_tasks(app: AppHandle) -> CommandResult<bool> {
    if let Err(error) = print_service::start_print_worker(app.clone()) {
        return CommandResult::fail("HUB_PRINT_START_ERROR", &error);
    }
    if let Err(error) = print_service::start_printer_fix_worker(app.clone()) {
        return CommandResult::fail("HUB_PRINT_FIX_START_ERROR", &error);
    }
    if let Err(error) = usb_service::start_usb_worker(app.clone()) {
        return CommandResult::fail("HUB_USB_START_ERROR", &error);
    }

    emit_background_state(&app, true, now_millis());
    CommandResult::ok(true)
}

#[tauri::command]
/// Stops background workers and emits a background-stopped state event.
pub fn stop_background_tasks(app: AppHandle) -> CommandResult<bool> {
    if let Err(error) = print_service::stop_print_worker() {
        return CommandResult::fail("HUB_PRINT_STOP_ERROR", &error);
    }
    if let Err(error) = print_service::stop_printer_fix_worker() {
        return CommandResult::fail("HUB_PRINT_FIX_STOP_ERROR", &error);
    }
    if let Err(error) = usb_service::stop_usb_worker(app.clone()) {
        return CommandResult::fail("HUB_USB_STOP_ERROR", &error);
    }

    emit_background_state(&app, false, now_millis());
    CommandResult::ok(true)
}

#[tauri::command]
/// Registers or repairs the CPMS virtual printer and returns the new print state.
pub fn add_printer(app: AppHandle) -> CommandResult<PrintState> {
    let print_state = match print_service::start_print_worker(app.clone()) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_PRINT_START_ERROR", &error),
    };

    emit_print_state(&app, print_state.clone());
    CommandResult::ok(print_state)
}

#[tauri::command]
/// Disables the CPMS virtual printer and returns the new print state.
pub fn disable_printer(app: AppHandle) -> CommandResult<PrintState> {
    let print_state = match print_service::stop_print_worker() {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_PRINT_STOP_ERROR", &error),
    };

    emit_print_state(&app, print_state.clone());
    CommandResult::ok(print_state)
}

#[tauri::command]
/// Repairs the CPMS virtual printer if it is unavailable.
/// Platform-specific implementation can replace the internal stub.
pub fn fix_printer(app: AppHandle) -> CommandResult<PrintState> {
    let print_state = match print_service::start_print_worker(app.clone()) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_PRINT_FIX_ERROR", &error),
    };

    emit_print_state(&app, print_state.clone());
    CommandResult::ok(print_state)
}

#[tauri::command]
/// Initializes USB printer discovery and returns persisted USB data if any.
/// Platform-specific hardware enumeration will be plugged in later.
pub fn init_usb_printer(app: AppHandle) -> CommandResult<Option<super::models::UsbData>> {
    let discovered = match usb_service::discover_usb_printer(&app) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_USB_INIT_ERROR", &error),
    };

    if let Some(ref usb_data) = discovered {
        if let Err(error) = update_preferences(&app, |preferences| {
            preferences.usb_data = Some(usb_data.clone());
        }) {
            return CommandResult::fail("HUB_USB_SAVE_ERROR", &error);
        }
    }

    let usb_state = match usb_service::current_usb_state(&app) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_USB_STATE_ERROR", &error),
    };

    emit_usb_state(&app, usb_state);
    CommandResult::ok(discovered)
}

#[tauri::command]
/// Reads current USB printer state known by the client.
pub fn get_usb_state(app: AppHandle) -> CommandResult<UsbState> {
    match usb_service::current_usb_state(&app) {
        Ok(value) => CommandResult::ok(value),
        Err(error) => return CommandResult::fail("HUB_USB_STATE_ERROR", &error),
    }
}

#[tauri::command]
/// Starts the local print-file socket server and returns its listening address.
pub fn start_socket_server(app: AppHandle) -> CommandResult<SocketState> {
    let preferences = match load_preferences(&app) {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_PREFERENCES_READ_ERROR", &error),
    };
    if !has_auth_token(&preferences.user) {
        return CommandResult::fail("HUB_SOCKET_NEED_LOGIN", "用户未登录，不能启动 Socket 服务");
    }

    let socket_state =
        match socket_server::start_socket_server(app.clone(), preferences.product_type) {
            Ok(value) => value,
            Err(error) => return CommandResult::fail("HUB_SOCKET_START_ERROR", &error),
        };

    emit_socket_state(&app, socket_state.clone());
    CommandResult::ok(socket_state)
}

#[tauri::command]
/// Stops the local print-file socket server and returns its stopped state.
pub fn stop_socket_server(app: AppHandle) -> CommandResult<SocketState> {
    let socket_state = match socket_server::stop_socket_server() {
        Ok(value) => value,
        Err(error) => return CommandResult::fail("HUB_SOCKET_STOP_ERROR", &error),
    };

    emit_socket_state(&app, socket_state.clone());
    CommandResult::ok(socket_state)
}

#[tauri::command]
/// Closes the application after releasing system resources.
/// Web should show a confirmation dialog before invoking this command.
pub async fn close_window_with_confirm(app: AppHandle) -> CommandResult<bool> {
    let _ = system_destroy(app.clone());
    app.exit(0);
    CommandResult::ok(true)
}

#[tauri::command]
/// Returns the current application version.
pub fn get_app_version() -> CommandResult<AppVersion> {
    CommandResult::ok(AppVersion {
        version: env!("CARGO_PKG_VERSION").into(),
        build_number: env!("CARGO_PKG_VERSION").into(),
    })
}

#[tauri::command]
/// Opens a URL in the system default browser.
pub async fn open_external(url: String) -> CommandResult<bool> {
    if url.trim().is_empty() {
        return CommandResult::fail("OPEN_EXTERNAL_EMPTY", "url 不能为空");
    }

    match tauri_plugin_opener::open_url(&url, None::<&str>) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("OPEN_EXTERNAL_ERROR", &error.to_string()),
    }
}

#[tauri::command]
/// Pings a server host and returns latency summary data.
pub fn ping_server(host: String) -> CommandResult<Value> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return CommandResult::fail("HUB_PING_HOST_EMPTY", "host 不能为空");
    }

    CommandResult::ok(json!({
        "host": trimmed,
        "packetLossRate": 0,
        "minMs": 0,
        "maxMs": 0,
        "avgMs": 0,
        "message": "ping command interface is ready; platform implementation can replace this stub",
    }))
}

#[tauri::command]
/// Generates an access_sign-compatible value for a CPMS request.
pub fn sign_request(uri: String, params: String) -> CommandResult<String> {
    match crypto_service::sign_request(&uri, &params) {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("HUB_SIGN_ERROR", &error),
    }
}

#[tauri::command]
/// Encrypts text with the client-compatible password encryption interface.
pub fn sm4_encrypt(text: String) -> CommandResult<String> {
    match crypto_service::sm4_encrypt_hex(&text) {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("HUB_SM4_ENCRYPT_ERROR", &error),
    }
}

fn load_and_emit_startup_state(app: &AppHandle, error_code: &str) -> CommandResult<StartupState> {
    let startup_state = match load_preferences(app) {
        Ok(value) => startup_state_from_preferences(value),
        Err(error) => return CommandResult::fail(error_code, &error),
    };

    emit_hub_state(app, &startup_state);
    CommandResult::ok(startup_state)
}

fn has_auth_token(user: &Option<super::models::UserData>) -> bool {
    user.as_ref()
        .and_then(|user| user.token.as_deref())
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false)
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default()
}
