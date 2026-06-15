//! 主窗口控制：对外暴露 tauri command，对内提供普通辅助函数供事件桥/托盘复用。

use tauri::{AppHandle, Manager};

use crate::result::CommandResult;
use crate::MAIN_WINDOW_LABEL;

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
    F: FnOnce(&tauri::Window) -> tauri::Result<()>,
{
    let Some(window) = app.get_window(MAIN_WINDOW_LABEL) else {
        return CommandResult::fail("WINDOW_NOT_FOUND", "主窗口不存在");
    };

    match op(&window) {
        Ok(_) => CommandResult::ok(true),
        Err(error) => CommandResult::fail("WINDOW_CONTROL_ERROR", &error.to_string()),
    }
}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub(crate) fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}
