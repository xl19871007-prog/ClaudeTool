use crate::error::{AppError, Result};
use crate::fs as appfs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub last_workdir: Option<String>,
    pub theme: String,
    pub suppress_login_prompt: bool,
    pub last_seen_version: Option<String>,
    /// Debug-only: pretend Claude Code is not installed (forces ReadinessWizard).
    pub debug_force_claude_missing: bool,
    /// Debug-only: pretend Git for Windows is not installed.
    pub debug_force_git_missing: bool,
    /// Debug-only: install commands stop before spawning real installers.
    pub debug_dry_run: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            version: 1,
            last_workdir: None,
            theme: "system".into(),
            suppress_login_prompt: false,
            last_seen_version: None,
            debug_force_claude_missing: false,
            debug_force_git_missing: false,
            debug_dry_run: false,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    appfs::app_data_dir().map(|d| d.join("config.json"))
}

pub fn load() -> AppConfig {
    let Some(path) = config_path() else {
        return AppConfig::default();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save(cfg: &AppConfig) -> Result<()> {
    let path = config_path().ok_or_else(|| AppError::Config("could not resolve app data dir".into()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(cfg)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}
