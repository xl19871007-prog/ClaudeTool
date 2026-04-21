use crate::net;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Build a Command that won't flash a console window on Windows.
///
/// Why: Tauri GUI processes have no attached console. When we spawn a child
/// via `silent_command(...)`, Windows allocates a fresh console for the child
/// by default. This causes two symptoms observed on user white machines:
///   1. A brief black console window flash.
///   2. For some native children (notably claude.exe as shipped by Anthropic),
///      this parent-less console allocation triggers a DLL init failure during
///      startup, surfacing as a Windows error dialog "应用程序无法正常启动
///      (0xc0000142)". Setting CREATE_NO_WINDOW (0x08000000) makes Windows
///      skip the console allocation entirely, which both hides the flash and
///      avoids the init race that produces 0xc0000142.
fn silent_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

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
        /// Whether `git` resolves on the user's current PATH. False means we
        /// found Git via registry (e.g. installed to a non-default drive without
        /// PATH being updated) — UI should offer the one-click env var repair
        /// instead of a re-install.
        in_path: bool,
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

/// Two-stage detection:
///   1. Try the user's PATH (`git --version`). Fast path, covers default installs.
///   2. Fall back to Windows registry (HKLM/HKCU GitForWindows + Uninstall key)
///      so we still find Git when the user installed it to a non-default drive
///      without updating PATH. Returns `in_path: false` in that case so the UI
///      can offer a one-click env-var repair.
pub fn check_git_installed() -> GitStatus {
    if let Some(version) = run_git_version_with("git") {
        let path = find_git_path_via_where()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "git".into());
        let bash = derive_bash_from_git_exe(&PathBuf::from(&path));
        return GitStatus::Installed {
            version,
            path,
            bash_path: bash.map(|p| p.to_string_lossy().to_string()),
            in_path: true,
        };
    }

    #[cfg(target_os = "windows")]
    if let Some(install_root) = find_git_install_via_registry() {
        let git_exe = install_root.join("cmd").join("git.exe");
        if git_exe.exists() {
            if let Some(version) = run_git_version_with(&git_exe.to_string_lossy()) {
                let bash = install_root.join("bin").join("bash.exe");
                let bash_opt = if bash.exists() { Some(bash) } else { None };
                return GitStatus::Installed {
                    version,
                    path: git_exe.to_string_lossy().to_string(),
                    bash_path: bash_opt.map(|p| p.to_string_lossy().to_string()),
                    in_path: false,
                };
            }
        }
    }

    GitStatus::NotInstalled
}

fn run_git_version_with(program: &str) -> Option<String> {
    let output = silent_command(program).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn find_git_path_via_where() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let cmd = "which";

    let output = silent_command(cmd).arg("git").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(|l| PathBuf::from(l.trim()))
}

/// Given a discovered `git.exe` path, walk up to the install root and return
/// `<install>/bin/bash.exe` if it exists. Both standard install layouts work:
/// `<install>/cmd/git.exe` (most common) and `<install>/mingw64/bin/git.exe`.
#[cfg(target_os = "windows")]
fn derive_bash_from_git_exe(git_path: &std::path::Path) -> Option<PathBuf> {
    let mut dir = git_path.parent()?.to_path_buf();
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
fn derive_bash_from_git_exe(_git_path: &std::path::Path) -> Option<PathBuf> {
    None
}

/// Read Git for Windows install root from the registry. Tries the standard
/// keys the official Inno Setup installer writes, in order:
///   HKLM\SOFTWARE\GitForWindows                              (system-wide 64-bit)
///   HKLM\SOFTWARE\WOW6432Node\GitForWindows                  (system-wide 32-bit on 64-bit OS)
///   HKCU\SOFTWARE\GitForWindows                              (user-scope install)
///   HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Git_is1  (Inno fallback)
///
/// We shell out to `reg query` rather than depending on the `winreg` crate to
/// keep the dependency footprint minimal — `reg.exe` ships with every Windows.
#[cfg(target_os = "windows")]
fn find_git_install_via_registry() -> Option<PathBuf> {
    const PROBES: &[(&str, &str)] = &[
        (r"HKLM\SOFTWARE\GitForWindows", "InstallPath"),
        (r"HKLM\SOFTWARE\WOW6432Node\GitForWindows", "InstallPath"),
        (r"HKCU\SOFTWARE\GitForWindows", "InstallPath"),
        (
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Git_is1",
            "InstallLocation",
        ),
        (
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Git_is1",
            "InstallLocation",
        ),
    ];
    for (key, value) in PROBES {
        if let Some(path) = reg_query_string(key, value) {
            let candidate = PathBuf::from(path.trim_end_matches('\\'));
            if candidate.join("cmd").join("git.exe").exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Run `reg query <key> /v <value>` and return the REG_SZ payload.
/// Returns None if the key or value is missing, or if reg.exe fails.
/// We deliberately avoid -reg:32/-reg:64 specifiers — for HKLM\SOFTWARE\WOW6432Node
/// the key is already explicit, and for the others Inno Setup writes the
/// 64-bit view on a 64-bit OS.
#[cfg(target_os = "windows")]
fn reg_query_string(key: &str, value: &str) -> Option<String> {
    let output = silent_command("reg")
        .args(["query", key, "/v", value])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(idx) = line.find("REG_SZ") {
            let payload = line[idx + "REG_SZ".len()..].trim();
            if !payload.is_empty() {
                return Some(payload.to_string());
            }
        }
        if let Some(idx) = line.find("REG_EXPAND_SZ") {
            let payload = line[idx + "REG_EXPAND_SZ".len()..].trim();
            if !payload.is_empty() {
                // Best-effort %USERPROFILE% expansion (only var Inno typically uses).
                if let Ok(user) = std::env::var("USERPROFILE") {
                    return Some(payload.replace("%USERPROFILE%", &user));
                }
                return Some(payload.to_string());
            }
        }
    }
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
    let output = silent_command("claude").arg("--version").output().ok()?;
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

    let output = silent_command(cmd).arg("claude").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?;
    Some(PathBuf::from(first_line.trim()))
}

pub fn check_claude_auth_status() -> AuthStatus {
    let output = match silent_command("claude").args(["auth", "status"]).output() {
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
