mod event_bridge;
mod iframe;
mod printclient;
mod result;
mod services;
mod single_instance;
mod socket;
mod token_refresh;
mod window;

use std::fs;
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

use result::CommandResult;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const VIEW_TO_CLIENT_EVENT: &str = "cpms:view-to-client";
pub(crate) const CLIENT_TO_VIEW_EVENT: &str = "cpms:client-to-view";
pub(crate) const CLIENT_NOTIFICATION_EVENT: &str = "cpms:desktop-notification";
pub(crate) const CLIENT_IFRAME_EVENT: &str = "cpms:client-iframe";
pub(crate) const CLIENT_TODO_TASK_EVENT: &str = "cpms:client-todo-task";
pub(crate) const CLIENT_IFRAME_PAYLOAD_REQUEST_EVENT: &str = "client.iframe_payload.request";
pub(crate) const CLIENT_IFRAME_PAYLOAD_REPORT_EVENT: &str = "client.iframe_payload.reported";
pub(crate) const DEFAULT_CPMS_BASE_URL: &str = "http://localhost:8080";
pub(crate) const DEFAULT_IFRAME_CONFIG_PATH: &str = "/api/client/iframe-config";
pub(crate) const DEFAULT_LOCAL_SOCKET_URL: &str = "ws://127.0.0.1:18080/ws/task";
pub(crate) const DEFAULT_IFRAME_FALLBACK_URL: &str = "http://192.168.98.158:8086/cpms/#/";
pub(crate) const DEFAULT_LOCAL_SOCKET_PATH: &str = "/ws/task";

const TRAY_SHOW: &str = "tray.show";
const TRAY_HIDE: &str = "tray.hide";
const TRAY_AUTOSTART_ENABLE: &str = "tray.autostart.enable";
const TRAY_AUTOSTART_DISABLE: &str = "tray.autostart.disable";
const TRAY_QUIT: &str = "tray.quit";
const AUTOSTART_INIT_MARKER: &str = ".autostart-initialized";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientEventPayload {
    pub(crate) name: String,
    pub(crate) payload: Option<Value>,
    pub(crate) at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientIframeEventPayload {
    pub(crate) state: String,
    pub(crate) url: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientTodoTaskPayload {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) detail: Option<String>,
    pub(crate) state: String,
    pub(crate) source: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

pub(crate) struct AppRuntimeState {
    pub(crate) iframe: Mutex<ClientIframeEventPayload>,
    pub(crate) iframe_payload: Mutex<Option<Value>>,
}

pub(crate) fn now_iso_string() -> String {
    format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or_default()
    )
}

#[tauri::command]
async fn client_http_request(request: services::ClientHttpRequest) -> CommandResult<Value> {
    match services::http_service::execute_client_http_request(request).await {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("HTTP_REQUEST_ERROR", &error),
    }
}

#[tauri::command]
fn autostart_is_enabled(app: AppHandle) -> CommandResult<bool> {
    match app.autolaunch().is_enabled() {
        Ok(value) => CommandResult::ok(value),
        Err(error) => CommandResult::fail("AUTOSTART_QUERY_ERROR", &error.to_string()),
    }
}

#[tauri::command]
fn autostart_set_enabled(app: AppHandle, enabled: bool) -> CommandResult<bool> {
    match set_autostart_enabled(&app, enabled) {
        Ok(_) => CommandResult::ok(enabled),
        Err(error) => CommandResult::fail("AUTOSTART_UPDATE_ERROR", &error.to_string()),
    }
}

fn set_autostart_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|error| error.to_string())
    } else {
        manager.disable().map_err(|error| error.to_string())
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, TRAY_SHOW, "显示主窗口", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, TRAY_HIDE, "隐藏到托盘", true, None::<&str>)?;
    let autostart_enable_item = MenuItem::with_id(
        app,
        TRAY_AUTOSTART_ENABLE,
        "开启开机自启动",
        true,
        None::<&str>,
    )?;
    let autostart_disable_item = MenuItem::with_id(
        app,
        TRAY_AUTOSTART_DISABLE,
        "关闭开机自启动",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, TRAY_QUIT, "退出", true, None::<&str>)?;
    let tray_menu = Menu::with_items(
        app,
        &[
            &show_item,
            &hide_item,
            &autostart_enable_item,
            &autostart_disable_item,
            &quit_item,
        ],
    )?;

    let mut tray_builder = TrayIconBuilder::with_id("cpms-tray")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .tooltip("CPMS Client");

    if let Some(default_icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(default_icon);
    }

    tray_builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW => window::show_main_window(app),
            TRAY_HIDE => window::hide_main_window(app),
            TRAY_AUTOSTART_ENABLE => {
                let _ = set_autostart_enabled(app, true);
            }
            TRAY_AUTOSTART_DISABLE => {
                let _ = set_autostart_enabled(app, false);
            }
            TRAY_QUIT => {
                let _ = services::system_destroy(app.clone());
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show_main_window(&tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn init_autostart_on_first_launch(app: &AppHandle) {
    let marker_path = app.path().app_data_dir().ok().map(|mut dir| {
        dir.push(AUTOSTART_INIT_MARKER);
        dir
    });

    let Some(marker_path) = marker_path else {
        return;
    };

    if marker_path.exists() {
        return;
    }

    if let Some(parent) = marker_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let manager = app.autolaunch();
    if matches!(manager.is_enabled(), Ok(false)) {
        let _ = manager.enable();
        services::log_service::info(app, "startup", "首次启动：已默认开启开机自启动");
    }

    let _ = fs::write(marker_path, b"ok");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        let message = format!("{info}");
        services::log_service::log_from_frontend("ERROR", "panic", &message, None);
        eprintln!("[panic] {message}");
    }));

    let singleton = match single_instance::try_acquire() {
        single_instance::Acquire::Secondary => return,
        single_instance::Acquire::Primary(listener) => Some(listener),
        single_instance::Acquire::Foreign => None,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .manage(AppRuntimeState {
            iframe: Mutex::new(iframe::initial_iframe_state()),
            iframe_payload: Mutex::new(None),
        })
        .setup(move |app| {
            if let Some(listener) = singleton {
                single_instance::serve(listener, app.handle().clone());
            }

            match services::log_service::init(app.handle()) {
                Ok(path) => services::log_service::info(
                    app.handle(),
                    "startup",
                    &format!("日志系统就绪：{}", path.display()),
                ),
                Err(error) => eprintln!("初始化日志系统失败: {error}"),
            }

            event_bridge::setup_client_event_bridge(app.handle());
            services::log_service::info(app.handle(), "startup", "视图端事件桥已注册");

            init_autostart_on_first_launch(app.handle());
            setup_tray(app.handle())?;
            window::restore_geometry(app.handle());
            services::log_service::info(app.handle(), "startup", "托盘图标已创建");

            let app_handle_for_iframe = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = iframe::refresh_iframe_container(&app_handle_for_iframe).await;
            });

            let app_handle_for_socket = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                socket::start_local_socket_worker(app_handle_for_socket).await;
            });
            socket::start_forward_retry_worker(app.handle().clone());
            services::log_service::info(app.handle(), "startup", "本地 socket 监听 worker 已启动");

            let app_handle_for_payload = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let _ = iframe::emit_iframe_payload_request(&app_handle_for_payload, "startup");
            });
            services::log_service::info(app.handle(), "startup", "客户端初始化完成");

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window::save_geometry(&window.app_handle());
                let _ = window.hide();
                services::log_service::info(
                    &window.app_handle(),
                    "window",
                    "主窗口收到关闭请求，已隐藏到托盘",
                );
            }
        })
        .invoke_handler(tauri::generate_handler![
            event_bridge::emit_client_event,
            event_bridge::push_desktop_notification_event,
            iframe::client_get_iframe_container_state,
            iframe::client_refresh_iframe_container,
            iframe::client_request_iframe_payload,
            iframe::client_submit_iframe_payload,
            client_http_request,
            autostart_is_enabled,
            autostart_set_enabled,
            window::window_minimize,
            window::window_set_fullscreen,
            window::window_set_always_on_top,
            window::window_hide,
            window::window_show,
            window::window_close,
            socket::reconnect_socket,
            socket::get_socket_state,
            services::get_startup_state,
            services::save_policy_agreed,
            services::save_auth_state,
            services::clear_auth_state,
            services::save_auth_token,
            services::save_server_info,
            services::save_direct_device,
            services::get_job_list,
            services::get_available_devices,
            services::select_direct_device,
            services::system_init,
            services::system_destroy,
            services::start_background_tasks,
            services::stop_background_tasks,
            services::close_window_with_confirm,
            services::get_app_version,
            services::open_external,
            services::sign_request,
            services::push_client_log,
            services::get_client_log_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
