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
pub enum GitStatus {
    #[serde(rename_all = "camelCase")]
    Installed {
        version: String,
        path: String,
        bash_path: Option<String>,
    },
    NotInstalled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GitBashEnvStatus {
    #[serde(rename_all = "camelCase")]
    Configured { path: String },
    NotConfigured,
    #[serde(rename_all = "camelCase")]
    InvalidPath { path: String },
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

pub fn check_git_installed() -> GitStatus {
    let Some(version) = run_git_version() else {
        return GitStatus::NotInstalled;
    };
    let path = find_git_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "git".into());
    let bash_path = find_git_bash_path();
    GitStatus::Installed {
        version,
        path,
        bash_path: bash_path.map(|p| p.to_string_lossy().to_string()),
    }
}

fn run_git_version() -> Option<String> {
    let output = Command::new("git").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn find_git_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let cmd = "which";

    let output = Command::new(cmd).arg("git").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(|l| PathBuf::from(l.trim()))
}

/// On Windows, find git-bash.exe by inspecting the git install directory.
/// `git.exe` is typically at `<install>/cmd/git.exe`; bash is at `<install>/bin/bash.exe`.
#[cfg(target_os = "windows")]
fn find_git_bash_path() -> Option<PathBuf> {
    let git_path = find_git_path()?;
    // git.exe is in <install>/cmd/ or <install>/mingw64/bin/
    let mut dir = git_path.parent()?.to_path_buf();
    // Walk up to find install root (the dir containing both `cmd/` and `bin/`).
    for _ in 0..3 {
        let bash = dir.join("bin").join("bash.exe");
        if bash.exists() {
            return Some(bash);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn find_git_bash_path() -> Option<PathBuf> {
    None
}

pub fn check_git_bash_env() -> GitBashEnvStatus {
    match std::env::var("CLAUDE_CODE_GIT_BASH_PATH") {
        Ok(path) if !path.is_empty() => {
            if std::path::Path::new(&path).exists() {
                GitBashEnvStatus::Configured { path }
            } else {
                GitBashEnvStatus::InvalidPath { path }
            }
        }
        _ => GitBashEnvStatus::NotConfigured,
    }
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
    // Two-shot probe: a single failed probe must not declare unreachable,
    // since users on flaky VPN often see one transient timeout. Retry once
    // before giving up.
    let probe_url = "https://api.anthropic.com";
    let mut last_error: Option<String> = None;
    for attempt in 0..2 {
        let result = net::probe(probe_url).await;
        if result.reachable {
            let latency = result.latency_ms.unwrap_or(0);
            return if latency > 1500 {
                NetworkStatus::Slow { latency_ms: latency }
            } else {
                NetworkStatus::Ok { latency_ms: latency }
            };
        }
        last_error = result.error;
        if attempt == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    NetworkStatus::Unreachable {
        error: last_error.unwrap_or_default(),
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
