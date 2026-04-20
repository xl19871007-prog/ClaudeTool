use crate::net;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClaudeStatus {
    #[serde(rename_all = "camelCase")]
    Installed { version: String, path: String },
    NotInstalled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AuthStatus {
    #[serde(rename_all = "camelCase")]
    LoggedIn { account: Option<String> },
    NotLoggedIn,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NetworkStatus {
    #[serde(rename_all = "camelCase")]
    Ok { latency_ms: u64 },
    #[serde(rename_all = "camelCase")]
    Slow { latency_ms: u64 },
    #[serde(rename_all = "camelCase")]
    Unreachable { error: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub has_update: bool,
}

pub fn check_claude_installed() -> ClaudeStatus {
    let Some(version) = run_claude_version() else {
        return ClaudeStatus::NotInstalled;
    };
    let path = find_claude_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "claude".into());
    ClaudeStatus::Installed { version, path }
}

fn run_claude_version() -> Option<String> {
    let output = Command::new("claude").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(stdout.trim().to_string())
}

fn find_claude_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let cmd = "which";

    let output = Command::new(cmd).arg("claude").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?;
    Some(PathBuf::from(first_line.trim()))
}

pub fn check_claude_auth_status() -> AuthStatus {
    let output = match Command::new("claude").args(["auth", "status"]).output() {
        Ok(o) => o,
        Err(_) => return AuthStatus::Unknown,
    };
    if !output.status.success() {
        return AuthStatus::NotLoggedIn;
    }
    // Best-effort parse account from JSON output
    let account = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .ok()
        .and_then(|v| {
            v.get("account")
                .and_then(|a| a.as_str())
                .or_else(|| v.get("email").and_then(|e| e.as_str()))
                .map(String::from)
        });
    AuthStatus::LoggedIn { account }
}

pub async fn check_network() -> NetworkStatus {
    let result = net::probe("https://api.anthropic.com").await;
    if !result.reachable {
        return NetworkStatus::Unreachable {
            error: result.error.unwrap_or_default(),
        };
    }
    let latency = result.latency_ms.unwrap_or(0);
    if latency > 1000 {
        NetworkStatus::Slow { latency_ms: latency }
    } else {
        NetworkStatus::Ok { latency_ms: latency }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

pub async fn check_for_updates() -> UpdateInfo {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let url = "https://api.github.com/repos/xl19871007-prog/ClaudeTool/releases/latest";

    let latest = match net::client()
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp
            .json::<GitHubRelease>()
            .await
            .ok()
            .map(|r| r.tag_name.trim_start_matches('v').to_string()),
        _ => None,
    };

    let has_update = match &latest {
        Some(l) => !l.is_empty() && l != &current,
        None => false,
    };

    UpdateInfo {
        current,
        latest,
        has_update,
    }
}
