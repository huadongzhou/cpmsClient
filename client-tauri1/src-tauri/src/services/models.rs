use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub user_account: Option<String>,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
    pub expire_time: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerData {
    pub server: String,
    pub https: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkState {
    pub online: bool,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self { online: true }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppVersion {
    pub version: String,
    pub build_number: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartupState {
    pub policy_agreed: bool,
    pub user: Option<UserData>,
    pub server: Option<ServerData>,
    pub product_type: i32,
    pub system_init_data: Option<Value>,
    pub network_state: NetworkState,
    pub auth_direct_device: Option<Value>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthPersistState {
    pub user: UserData,
    pub server: ServerData,
    pub product_type: i32,
    pub system_init_data: Option<Value>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HubPreferences {
    pub policy_agreed: bool,
    pub user: Option<UserData>,
    pub server: Option<ServerData>,
    pub product_type: i32,
    pub system_init_data: Option<Value>,
    pub auth_direct_device: Option<Value>,
}

pub fn startup_state_from_preferences(preferences: HubPreferences) -> StartupState {
    StartupState {
        policy_agreed: preferences.policy_agreed,
        user: preferences.user,
        server: preferences.server,
        product_type: preferences.product_type,
        system_init_data: preferences.system_init_data,
        network_state: NetworkState::default(),
        auth_direct_device: preferences.auth_direct_device,
    }
}
