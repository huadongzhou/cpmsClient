use std::fs;
use std::path::PathBuf;

use tauri::AppHandle;

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

    let mut cleared_legacy_cache = false;
    if let Some(user) = preferences.user.as_mut() {
        if user.token.take().is_some() {
            cleared_legacy_cache = true;
        }
    }
    if preferences.auth_direct_device.take().is_some() {
        cleared_legacy_cache = true;
    }
    if cleared_legacy_cache {
        let _ = save_preferences(app, &preferences);
    }

    Ok(preferences)
}

pub fn save_preferences(app: &AppHandle, preferences: &HubPreferences) -> Result<(), String> {
    let path = preferences_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    // token/deviceId 只允许作为 iframe 主动推送的会话态存在，写盘前统一剥离旧值。
    let mut to_persist = preferences.clone();
    if let Some(user) = to_persist.user.as_mut() {
        user.token = None;
    }
    to_persist.auth_direct_device = None;

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
        .path_resolver()
        .app_data_dir()
        .ok_or_else(|| "无法获取应用数据目录".to_string())?;
    dir.push(HUB_PREFERENCES_FILE);
    Ok(dir)
}

/// 应用数据目录（封装 v1/v2 路径 API 差异），供待重试队列等模块复用。
pub(crate) fn data_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path_resolver().app_data_dir()
}
