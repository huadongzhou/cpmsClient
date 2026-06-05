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

    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str::<HubPreferences>(&raw).map_err(|error| error.to_string())
}

pub fn save_preferences(app: &AppHandle, preferences: &HubPreferences) -> Result<(), String> {
    let path = preferences_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let raw = serde_json::to_string_pretty(preferences).map_err(|error| error.to_string())?;
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
