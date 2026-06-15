//! 主窗口控制：对外暴露 tauri command，对内提供普通辅助函数供事件桥/托盘复用，
//! 以及主窗口几何（大小/位置）的本地持久化。

use std::fs;

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::result::CommandResult;
use crate::MAIN_WINDOW_LABEL;

const GEOMETRY_FILE: &str = "window-geometry.json";

#[tauri::command]
pub fn window_minimize(app: AppHandle) -> CommandResult<bool> {
    minimize(&app)
}

#[tauri::command]
pub fn window_set_fullscreen(app: AppHandle, fullscreen: bool) -> CommandResult<bool> {
    set_fullscreen(&app, fullscreen)
}

#[tauri::command]
pub fn window_set_always_on_top(app: AppHandle, always_on_top: bool) -> CommandResult<bool> {
    set_always_on_top(&app, always_on_top)
}

#[tauri::command]
pub fn window_hide(app: AppHandle) -> CommandResult<bool> {
    hide(&app)
}

#[tauri::command]
pub fn window_show(app: AppHandle) -> CommandResult<bool> {
    control_main_window(&app, |window| {
        window.show()?;
        window.set_focus()
    })
}

#[tauri::command]
pub fn window_close(app: AppHandle) -> CommandResult<bool> {
    hide(&app)
}

pub(crate) fn minimize(app: &AppHandle) -> CommandResult<bool> {
    control_main_window(app, |window| window.minimize())
}

pub(crate) fn set_fullscreen(app: &AppHandle, fullscreen: bool) -> CommandResult<bool> {
    control_main_window(app, |window| window.set_fullscreen(fullscreen))
}

pub(crate) fn set_always_on_top(app: &AppHandle, always_on_top: bool) -> CommandResult<bool> {
    control_main_window(app, |window| window.set_always_on_top(always_on_top))
}

pub(crate) fn hide(app: &AppHandle) -> CommandResult<bool> {
    control_main_window(app, |window| window.hide())
}

fn control_main_window<F>(app: &AppHandle, op: F) -> CommandResult<bool>
where
    F: FnOnce(&tauri::WebviewWindow) -> tauri::Result<()>,
{
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return CommandResult::fail("WINDOW_NOT_FOUND", "主窗口不存在");
    };

    match op(&window) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("WINDOW_CONTROL_ERROR", &error.to_string()),
    }
}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub(crate) fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

/// 启动时恢复上次保存的主窗口大小/位置（best-effort，失败不影响启动）。
pub(crate) fn restore_geometry(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let Some(dir) = crate::services::app_data_dir(app) else {
        return;
    };
    let Ok(raw) = fs::read_to_string(dir.join(GEOMETRY_FILE)) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };

    if let (Some(width), Some(height)) = (
        value.get("width").and_then(serde_json::Value::as_u64),
        value.get("height").and_then(serde_json::Value::as_u64),
    ) {
        if width >= 200 && height >= 200 {
            let _ = window.set_size(tauri::PhysicalSize::new(width as u32, height as u32));
        }
    }

    if let (Some(x), Some(y)) = (
        value.get("x").and_then(serde_json::Value::as_i64),
        value.get("y").and_then(serde_json::Value::as_i64),
    ) {
        let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
    }
}

/// 保存当前主窗口大小/位置（关闭到托盘时调用，best-effort）。
pub(crate) fn save_geometry(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let Some(dir) = crate::services::app_data_dir(app) else {
        return;
    };
    let (Ok(size), Ok(position)) = (window.outer_size(), window.outer_position()) else {
        return;
    };

    let value = json!({
        "width": size.width,
        "height": size.height,
        "x": position.x,
        "y": position.y,
    });

    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dir.join(GEOMETRY_FILE), value.to_string());
}
