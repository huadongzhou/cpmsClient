use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::models::HubPreferences;

const HUB_PREFERENCES_FILE: &str = "hub-preferences.json";

pub fn load_preferences(app: &AppHandle) -> Result<HubPreferences, String> {
    let path = preferences_path(app)?;
    if !path.exists() {
        return Ok(HubPreferences {
            product_type: -1,
            ..HubPreferences::default()
        });
    }

    let raw = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let mut preferences =
        serde_json::from_str::<HubPreferences>(&raw).map_err(|error| error.to_string())?;

    // 落盘 token 是加密的（旧明文兼容），读取时透明解密。
    if let (Some(dir), Some(user)) = (path.parent(), preferences.user.as_mut()) {
        if let Some(token) = user.token.take() {
            user.token = super::token_store::decrypt(dir, &token);
        }
    }

    Ok(preferences)
}

pub fn save_preferences(app: &AppHandle, preferences: &HubPreferences) -> Result<(), String> {
    let path = preferences_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    // 写盘前加密 token，避免凭据明文落盘。
    let mut to_persist = preferences.clone();
    if let (Some(dir), Some(user)) = (path.parent(), to_persist.user.as_mut()) {
        if let Some(token) = user.token.take().filter(|value| !value.is_empty()) {
            user.token = Some(super::token_store::encrypt(dir, &token));
        }
    }

    let raw = serde_json::to_string_pretty(&to_persist).map_err(|error| error.to_string())?;
    fs::write(path, raw).map_err(|error| error.to_string())
}

pub fn update_preferences<F>(app: &AppHandle, update: F) -> Result<(), String>
where
    F: FnOnce(&mut HubPreferences),
{
    let mut preferences = load_preferences(app)?;
    update(&mut preferences);
    save_preferences(app, &preferences)
}

fn preferences_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    dir.push(HUB_PREFERENCES_FILE);
    Ok(dir)
}

/// 应用数据目录（封装 v1/v2 路径 API 差异），供待重试队列等模块复用。
pub(crate) fn data_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}
