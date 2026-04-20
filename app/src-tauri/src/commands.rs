use crate::config::{self, AppConfig};
use crate::env_checker::{
    self, AuthStatus, ClaudeStatus, NetworkStatus, UpdateInfo,
};
use crate::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub claude: ClaudeStatus,
    pub auth: AuthStatus,
    pub network: NetworkStatus,
    pub update: UpdateInfo,
}

#[tauri::command]
pub async fn check_environment() -> EnvironmentReport {
    let claude_handle = tokio::task::spawn_blocking(env_checker::check_claude_installed);
    let auth_handle = tokio::task::spawn_blocking(env_checker::check_claude_auth_status);
    let network_fut = env_checker::check_network();
    let update_fut = env_checker::check_for_updates();

    let (claude, auth, network, update) = tokio::join!(
        async { claude_handle.await.unwrap_or(ClaudeStatus::NotInstalled) },
        async { auth_handle.await.unwrap_or(AuthStatus::Unknown) },
        network_fut,
        update_fut,
    );

    EnvironmentReport {
        claude,
        auth,
        network,
        update,
    }
}

#[tauri::command]
pub fn get_config() -> AppConfig {
    config::load()
}

#[tauri::command]
pub fn set_suppress_login_prompt(value: bool) -> Result<()> {
    let mut cfg = config::load();
    cfg.suppress_login_prompt = value;
    config::save(&cfg)
}

#[tauri::command]
pub fn set_last_seen_version(value: String) -> Result<()> {
    let mut cfg = config::load();
    cfg.last_seen_version = Some(value);
    config::save(&cfg)
}
