use crate::config::{self, AppConfig};
use crate::env_checker::{
    self, AuthStatus, ClaudeStatus, GitBashEnvStatus, GitStatus, NetworkStatus, UpdateInfo,
};
use crate::error::Result;
use crate::history_parser::{self, SessionMeta};
use crate::skills_scanner::{self, SkillsReport};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub claude: ClaudeStatus,
    pub auth: AuthStatus,
    pub network: NetworkStatus,
    pub update: UpdateInfo,
    pub git: GitStatus,
    pub git_bash_env: GitBashEnvStatus,
}

#[tauri::command]
pub async fn check_environment() -> EnvironmentReport {
    let cfg = config::load();

    let claude_handle = tokio::task::spawn_blocking(env_checker::check_claude_installed);
    let auth_handle = tokio::task::spawn_blocking(env_checker::check_claude_auth_status);
    let git_handle = tokio::task::spawn_blocking(env_checker::check_git_installed);
    let git_bash_handle = tokio::task::spawn_blocking(env_checker::check_git_bash_env);
    let network_fut = env_checker::check_network();
    let update_fut = env_checker::check_for_updates();

    let (mut claude, auth, mut git, git_bash_env, network, update) = tokio::join!(
        async { claude_handle.await.unwrap_or(ClaudeStatus::NotInstalled) },
        async { auth_handle.await.unwrap_or(AuthStatus::Unknown) },
        async { git_handle.await.unwrap_or(GitStatus::NotInstalled) },
        async { git_bash_handle.await.unwrap_or(GitBashEnvStatus::NotConfigured) },
        network_fut,
        update_fut,
    );

    // Debug overrides: pretend things are missing without touching real state.
    if cfg.debug_force_claude_missing {
        claude = ClaudeStatus::NotInstalled;
    }
    if cfg.debug_force_git_missing {
        git = GitStatus::NotInstalled;
    }

    EnvironmentReport {
        claude,
        auth,
        network,
        update,
        git,
        git_bash_env,
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

#[tauri::command]
pub fn set_debug_flag(name: String, value: bool) -> Result<AppConfig> {
    let mut cfg = config::load();
    match name.as_str() {
        "forceClaudeMissing" => cfg.debug_force_claude_missing = value,
        "forceGitMissing" => cfg.debug_force_git_missing = value,
        "dryRun" => cfg.debug_dry_run = value,
        other => {
            return Err(crate::error::AppError::Config(format!(
                "unknown debug flag: {other}"
            )))
        }
    }
    config::save(&cfg)?;
    Ok(cfg)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRefinePayload {
    pub workdir: String,
    pub session_id: String,
    pub turn_count: u32,
}

#[tauri::command]
pub async fn list_sessions(workdir: String, app: AppHandle) -> Result<Vec<SessionMeta>> {
    let path = PathBuf::from(&workdir);
    let workdir_for_task = workdir.clone();
    let sessions =
        tokio::task::spawn_blocking(move || history_parser::list_sessions_quick(&path))
            .await
            .unwrap_or_else(|_| Ok(Vec::new()))?;

    // Spawn background refinement: emit one event per session as turnCount becomes known.
    let session_ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
    tauri::async_runtime::spawn(async move {
        for session_id in session_ids {
            let workdir = workdir_for_task.clone();
            let id_for_task = session_id.clone();
            let count = tokio::task::spawn_blocking(move || {
                history_parser::refine_turn_count(&PathBuf::from(&workdir), &id_for_task)
            })
            .await
            .ok()
            .flatten();
            if let Some(turn_count) = count {
                let _ = app.emit(
                    "session-refined",
                    SessionRefinePayload {
                        workdir: workdir_for_task.clone(),
                        session_id,
                        turn_count,
                    },
                );
            }
        }
    });

    Ok(sessions)
}

#[tauri::command]
pub async fn list_skills(workdir: Option<String>) -> Result<SkillsReport> {
    tokio::task::spawn_blocking(move || {
        let workdir_path = workdir.as_ref().map(PathBuf::from);
        skills_scanner::list_all(workdir_path.as_deref())
    })
    .await
    .unwrap_or_else(|_| Ok(SkillsReport {
        installed: Vec::new(),
        recommended: Vec::new(),
    }))
}

#[tauri::command]
pub async fn read_skill_md(path: String) -> Result<String> {
    tokio::task::spawn_blocking(move || skills_scanner::read_skill_md(&PathBuf::from(&path)))
        .await
        .unwrap_or_else(|_| Ok(String::new()))
}

#[tauri::command]
pub async fn install_git(app: AppHandle) -> Result<()> {
    crate::installer::install_git_for_windows(app).await
}

#[tauri::command]
pub async fn install_claude_code(app: AppHandle) -> Result<()> {
    crate::installer::install_claude_code(app).await
}
