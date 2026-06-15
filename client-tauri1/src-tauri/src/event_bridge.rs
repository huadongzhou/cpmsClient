//! 视图端 ↔ 客户端事件桥：监听 view→client 事件，分发内置窗口/作业/设备/token 指令，
//! 其余事件透传回视图端；并提供客户端→视图端的事件/通知/HTTP 代理命令。

use std::thread;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::result::CommandResult;
use crate::services;
use crate::{
    now_iso_string, ClientEventPayload, CLIENT_NOTIFICATION_EVENT, CLIENT_TO_VIEW_EVENT,
    MAIN_WINDOW_LABEL, VIEW_TO_CLIENT_EVENT,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewEventPayload {
    name: String,
    payload: Option<Value>,
    at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopNotificationPayload {
    #[serde(rename = "type")]
    kind: String,
    title: String,
    message: Option<String>,
    duration_ms: Option<u64>,
}

#[tauri::command]
pub fn emit_client_event(
    app: AppHandle,
    name: String,
    payload: Option<Value>,
) -> CommandResult<bool> {
    let event_payload = ClientEventPayload {
        name,
        payload,
        at: now_iso_string(),
    };

    match app.emit_to(MAIN_WINDOW_LABEL, CLIENT_TO_VIEW_EVENT, event_payload) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("EMIT_EVENT_ERROR", &error.to_string()),
    }
}

#[tauri::command]
pub(crate) fn push_desktop_notification_event(
    app: AppHandle,
    notification: DesktopNotificationPayload,
) -> CommandResult<bool> {
    match app.emit_to(MAIN_WINDOW_LABEL, CLIENT_NOTIFICATION_EVENT, notification) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("EMIT_NOTIFICATION_ERROR", &error.to_string()),
    }
}

pub(crate) fn setup_client_event_bridge(app: &AppHandle) {
    let app_handle = app.clone();

    app.listen_global(VIEW_TO_CLIENT_EVENT, move |event| {
        let raw = event.payload().unwrap_or_default();
        let payload = serde_json::from_str::<ViewEventPayload>(raw).unwrap_or(ViewEventPayload {
            name: "unknown".into(),
            payload: Some(Value::String(raw.to_string())),
            at: None,
        });

        if handle_view_event(&app_handle, &payload.name, payload.payload.clone()) {
            return;
        }

        let event_payload = ClientEventPayload {
            name: payload.name,
            payload: payload.payload,
            at: payload.at.unwrap_or_else(now_iso_string),
        };

        let _ = app_handle.emit_to(MAIN_WINDOW_LABEL, CLIENT_TO_VIEW_EVENT, event_payload);
    });
}

fn handle_view_event(app: &AppHandle, name: &str, payload: Option<Value>) -> bool {
    match name {
        "client.window.pin" | "window.pin" => {
            emit_view_command_result(
                app,
                "client.window.pin.result",
                crate::window::set_always_on_top(app, true),
            );
            true
        }
        "client.window.unpin" | "window.unpin" => {
            emit_view_command_result(
                app,
                "client.window.unpin.result",
                crate::window::set_always_on_top(app, false),
            );
            true
        }
        "client.window.minimize" | "window.minimize" | "client.window.collapse" => {
            emit_view_command_result(
                app,
                "client.window.minimize.result",
                crate::window::minimize(app),
            );
            true
        }
        "client.window.hide" | "window.hide" => {
            emit_view_command_result(app, "client.window.hide.result", crate::window::hide(app));
            true
        }
        "client.window.close" | "window.close" => {
            emit_view_command_result(app, "client.window.close.result", crate::window::hide(app));
            true
        }
        "client.window.fullscreen" | "window.fullscreen" => {
            let fullscreen = payload_bool(payload.as_ref(), "fullscreen").unwrap_or(true);
            emit_view_command_result(
                app,
                "client.window.fullscreen.result",
                crate::window::set_fullscreen(app, fullscreen),
            );
            true
        }
        "client.window.exit-fullscreen" | "window.exit-fullscreen" => {
            emit_view_command_result(
                app,
                "client.window.exit-fullscreen.result",
                crate::window::set_fullscreen(app, false),
            );
            true
        }
        "client.jobs.list" | "jobs.list" => {
            let page_number = payload_i64(payload.as_ref(), "pageNumber").unwrap_or(1);
            let page_size = payload_i64(payload.as_ref(), "pageSize").unwrap_or(20);
            let job_type = payload_i64(payload.as_ref(), "type").unwrap_or(1);
            let title = payload_string(payload.as_ref(), "title");
            let search_time = payload_string(payload.as_ref(), "searchTime");
            let app_handle = app.clone();
            thread::spawn(move || {
                let result = services::get_job_list(
                    app_handle.clone(),
                    page_number,
                    page_size,
                    job_type,
                    title,
                    search_time,
                );
                emit_view_command_result(&app_handle, "client.jobs.list.result", result);
            });
            true
        }
        "client.devices.list" | "devices.list" | "client.printers.list" | "printers.list" => {
            let app_handle = app.clone();
            thread::spawn(move || {
                let result = services::get_available_devices(app_handle.clone());
                emit_view_command_result(&app_handle, "client.devices.list.result", result);
            });
            true
        }
        "client.device.select" | "device.select" | "client.printer.select" | "printer.select" => {
            let device = payload.map(|value| value.get("device").cloned().unwrap_or(value));
            let Some(device) = device else {
                emit_view_command_result::<Value>(
                    app,
                    "client.device.select.result",
                    CommandResult::fail("DEVICE_PAYLOAD_EMPTY", "device 不能为空"),
                );
                return true;
            };
            let app_handle = app.clone();
            thread::spawn(move || {
                let result = services::select_direct_device(app_handle.clone(), device);
                emit_view_command_result(&app_handle, "client.device.select.result", result);
            });
            true
        }
        "client.auth.update-token" | "auth.update-token" | "client.token.update" => {
            let token = payload_string(payload.as_ref(), "token")
                .or_else(|| payload_string(payload.as_ref(), "accessToken"))
                .or_else(|| payload.and_then(|value| value.as_str().map(str::to_string)))
                .unwrap_or_default();
            emit_view_command_result(
                app,
                "client.auth.update-token.result",
                services::save_auth_token(app.clone(), token),
            );
            true
        }
        _ => false,
    }
}

fn emit_view_command_result<T: Serialize>(app: &AppHandle, name: &str, result: CommandResult<T>) {
    let payload = serde_json::to_value(result).unwrap_or_else(|error| {
        json!({
            "success": false,
            "code": "SERIALIZE_RESULT_ERROR",
            "message": error.to_string(),
            "data": null,
            "logs": [],
        })
    });

    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        CLIENT_TO_VIEW_EVENT,
        ClientEventPayload {
            name: name.into(),
            payload: Some(payload),
            at: now_iso_string(),
        },
    );
}

fn payload_i64(payload: Option<&Value>, key: &str) -> Option<i64> {
    payload.and_then(|value| value.get(key)).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str()?.parse::<i64>().ok())
    })
}

fn payload_string(payload: Option<&Value>, key: &str) -> Option<String> {
    payload
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn payload_bool(payload: Option<&Value>, key: &str) -> Option<bool> {
    payload
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
}
