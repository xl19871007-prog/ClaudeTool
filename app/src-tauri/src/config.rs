use crate::error::{AppError, Result};
use crate::fs as appfs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProxyConfig {
    /// HTTP proxy URL (e.g. "http://127.0.0.1:7897"). Injected as HTTP_PROXY
    /// env var to spawned child processes (claude / git installer / powershell)
    /// and as reqwest proxy on the in-process HTTP client.
    pub http: Option<String>,
    pub https: Option<String>,
    /// Comma-separated host list to bypass proxy (NO_PROXY env var).
    pub no_proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub last_workdir: Option<String>,
    pub theme: String,
    pub suppress_login_prompt: bool,
    pub last_seen_version: Option<String>,
    /// User-supplied proxy URLs forwarded to Claude / Git / install scripts.
    /// See ADR-018 (revising ADR-013): OS-level VPNs in sysproxy mode do
    /// NOT cover Node/CLI children; only TUN-mode VPNs do. So we offer
    /// explicit proxy injection for users on the common sysproxy setup.
    pub proxy: ProxyConfig,
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
            proxy: ProxyConfig::default(),
            debug_force_claude_missing: false,
            debug_force_git_missing: false,
            debug_dry_run: false,
        }
    }
}

impl ProxyConfig {
    /// Yields (key, value) env-var pairs ready to set on a child process.
    /// Empty / None entries are skipped. Both upper and lower case are set
    /// because different tools read different conventions (Node reads
    /// upper, curl/Linux tools often read lower).
    pub fn as_env_pairs(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(v) = self.http.as_ref().filter(|s| !s.is_empty()) {
            out.push(("HTTP_PROXY".into(), v.clone()));
            out.push(("http_proxy".into(), v.clone()));
        }
        if let Some(v) = self.https.as_ref().filter(|s| !s.is_empty()) {
            out.push(("HTTPS_PROXY".into(), v.clone()));
            out.push(("https_proxy".into(), v.clone()));
        }
        if let Some(v) = self.no_proxy.as_ref().filter(|s| !s.is_empty()) {
            out.push(("NO_PROXY".into(), v.clone()));
            out.push(("no_proxy".into(), v.clone()));
        }
        out
    }

    /// Convenience predicate for callers; intentionally kept for future use.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.http.as_ref().is_none_or(|s| s.is_empty())
            && self.https.as_ref().is_none_or(|s| s.is_empty())
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
