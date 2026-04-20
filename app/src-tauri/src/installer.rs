use crate::config;
use crate::error::{AppError, Result};
use crate::net;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command as AsyncCommand;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "stage", rename_all = "camelCase")]
pub enum InstallEvent {
    #[serde(rename_all = "camelCase")]
    Started { target: String },
    #[serde(rename_all = "camelCase")]
    Resolving { message: String },
    #[serde(rename_all = "camelCase")]
    Downloading {
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    Installing { message: String },
    #[serde(rename_all = "camelCase")]
    Configuring { message: String },
    #[serde(rename_all = "camelCase")]
    Verifying { message: String },
    #[serde(rename_all = "camelCase")]
    Done { message: String },
    #[serde(rename_all = "camelCase")]
    Failed {
        stage_id: String,
        message: String,
        recoverable: bool,
    },
    #[serde(rename_all = "camelCase")]
    Log { line: String },
}

const EVENT_NAME: &str = "install-progress";

fn emit(app: &AppHandle, ev: InstallEvent) {
    let _ = app.emit(EVENT_NAME, ev);
}

/// Sanitize URL/host paths for log lines (avoid leaking nothing sensitive,
/// just keeps logs readable).
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubReleaseAsset>,
}

/// Find the 64-bit Windows installer in the latest Git for Windows release.
async fn resolve_git_installer() -> Result<(String, String, Option<u64>)> {
    let url = "https://api.github.com/repos/git-for-windows/git/releases/latest";
    let release: GitHubRelease = net::client()
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // Prefer Git-X.Y.Z-64-bit.exe (the standard installer, not portable / mingit / busybox)
    let asset = release
        .assets
        .into_iter()
        .find(|a| {
            a.name.starts_with("Git-")
                && a.name.ends_with("-64-bit.exe")
                && !a.name.contains("portable")
                && !a.name.contains("MinGit")
                && !a.name.contains("busybox")
        })
        .ok_or_else(|| AppError::Config("no 64-bit Git installer in latest release".into()))?;

    Ok((release.tag_name, asset.browser_download_url, asset.size))
}

/// Stream-download a URL to a temp file, emitting progress events.
async fn download_to_temp(
    app: &AppHandle,
    url: &str,
    expected_size: Option<u64>,
    suggested_filename: &str,
) -> Result<PathBuf> {
    use std::time::Instant;
    use tokio::io::AsyncWriteExt;

    let dest = std::env::temp_dir().join(suggested_filename);
    let mut file = tokio::fs::File::create(&dest).await?;

    let mut resp = net::client().get(url).send().await?.error_for_status()?;
    let total = resp.content_length().or(expected_size);

    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();

    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        // Throttle progress events to ~10 Hz so the UI doesn't drown.
        if last_emit.elapsed().as_millis() >= 100 {
            emit(
                app,
                InstallEvent::Downloading {
                    downloaded_bytes: downloaded,
                    total_bytes: total,
                    message: format!(
                        "下载中 {:.1}MB{}",
                        downloaded as f64 / 1_048_576.0,
                        total
                            .map(|t| format!(" / {:.1}MB", t as f64 / 1_048_576.0))
                            .unwrap_or_default()
                    ),
                },
            );
            last_emit = Instant::now();
        }
    }
    file.flush().await?;
    drop(file);

    emit(
        app,
        InstallEvent::Downloading {
            downloaded_bytes: downloaded,
            total_bytes: total.or(Some(downloaded)),
            message: format!("下载完成 {:.1}MB", downloaded as f64 / 1_048_576.0),
        },
    );

    Ok(dest)
}

/// Spawn a process and pipe its stdout/stderr lines as Log events.
/// Returns the exit code (or -1 if the child couldn't be killed cleanly).
async fn spawn_streaming(
    app: &AppHandle,
    program: &str,
    args: &[&str],
) -> Result<i32> {
    let mut child = AsyncCommand::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_clone = app.clone();
    let stdout_task = tokio::spawn(async move {
        if let Some(out) = stdout {
            let mut reader = tokio::io::BufReader::new(out).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                emit(&app_clone, InstallEvent::Log { line: truncate(&line, 500) });
            }
        }
    });

    let app_clone = app.clone();
    let stderr_task = tokio::spawn(async move {
        if let Some(err) = stderr {
            let mut reader = tokio::io::BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                emit(&app_clone, InstallEvent::Log { line: truncate(&line, 500) });
            }
        }
    });

    let status = child.wait().await?;
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    Ok(status.code().unwrap_or(-1))
}

/// Set CLAUDE_CODE_GIT_BASH_PATH at the user level via `setx`.
/// Note: setx writes to HKCU\Environment but does NOT update the current
/// process's env vars; new shells (and our next launch) will see it.
async fn set_git_bash_env_var(app: &AppHandle, bash_path: &str) -> Result<()> {
    emit(
        app,
        InstallEvent::Configuring {
            message: format!(
                "配置环境变量 CLAUDE_CODE_GIT_BASH_PATH = {}",
                truncate(bash_path, 100)
            ),
        },
    );
    let code = spawn_streaming(
        app,
        "setx",
        &["CLAUDE_CODE_GIT_BASH_PATH", bash_path],
    )
    .await?;
    if code != 0 {
        return Err(AppError::Config(format!(
            "setx 失败，退出码 {code}"
        )));
    }
    Ok(())
}

/// Install Git for Windows end-to-end.
/// Steps: resolve URL → download → spawn silent installer → set env var → verify.
pub async fn install_git_for_windows(app: AppHandle) -> Result<()> {
    let cfg = config::load();

    emit(
        &app,
        InstallEvent::Started {
            target: "git-for-windows".into(),
        },
    );

    if cfg.debug_dry_run {
        emit(
            &app,
            InstallEvent::Log {
                line: "[DRY-RUN] skipping real install actions".into(),
            },
        );
        emit(
            &app,
            InstallEvent::Resolving {
                message: "解析最新 Git for Windows 版本（dry-run）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        emit(
            &app,
            InstallEvent::Downloading {
                downloaded_bytes: 0,
                total_bytes: Some(60 * 1024 * 1024),
                message: "（dry-run 跳过下载）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        emit(
            &app,
            InstallEvent::Done {
                message: "Dry-run 完成。实际运行时会下载、安装、配置环境变量。".into(),
            },
        );
        return Ok(());
    }

    emit(
        &app,
        InstallEvent::Resolving {
            message: "解析最新 Git for Windows 版本...".into(),
        },
    );
    let (tag, url, size) = resolve_git_installer().await.map_err(|e| {
        emit(
            &app,
            InstallEvent::Failed {
                stage_id: "resolve".into(),
                message: format!("解析下载链接失败：{e}"),
                recoverable: true,
            },
        );
        e
    })?;
    emit(
        &app,
        InstallEvent::Log {
            line: format!("Git {tag}, asset URL: {}", truncate(&url, 200)),
        },
    );

    let installer_path = download_to_temp(&app, &url, size, "ClaudeTool-Git-Installer.exe")
        .await
        .map_err(|e| {
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "download".into(),
                    message: format!("下载失败：{e}"),
                    recoverable: true,
                },
            );
            e
        })?;

    emit(
        &app,
        InstallEvent::Installing {
            message: "正在安装 Git for Windows（可能弹出 UAC 提示，请允许）...".into(),
        },
    );
    let installer_str = installer_path.to_string_lossy().to_string();
    let code = spawn_streaming(
        &app,
        &installer_str,
        &[
            "/VERYSILENT",
            "/NORESTART",
            "/NOCANCEL",
            "/SP-",
            "/CLOSEAPPLICATIONS",
            "/RESTARTAPPLICATIONS",
            "/COMPONENTS=icons,ext\\reg\\shellhere,assoc,assoc_sh",
        ],
    )
    .await?;
    if code != 0 {
        let msg = format!("Git 安装器退出码 {code}（用户拒绝 UAC 也会得到非零）");
        emit(
            &app,
            InstallEvent::Failed {
                stage_id: "spawn".into(),
                message: msg.clone(),
                recoverable: true,
            },
        );
        return Err(AppError::Config(msg));
    }

    // Discover bash path & set env var.
    let bash_path = crate::env_checker::check_git_installed();
    let bash_path_str = match &bash_path {
        crate::env_checker::GitStatus::Installed { bash_path: Some(p), .. } => p.clone(),
        _ => {
            // Default fallback location after standard install
            "C:\\Program Files\\Git\\bin\\bash.exe".to_string()
        }
    };

    if let Err(e) = set_git_bash_env_var(&app, &bash_path_str).await {
        emit(
            &app,
            InstallEvent::Failed {
                stage_id: "configure".into(),
                message: format!("环境变量配置失败：{e}（你可以手动设置）"),
                recoverable: true,
            },
        );
        return Err(e);
    }

    emit(
        &app,
        InstallEvent::Verifying {
            message: "验证 Git 安装...".into(),
        },
    );
    match crate::env_checker::check_git_installed() {
        crate::env_checker::GitStatus::Installed { version, .. } => {
            emit(
                &app,
                InstallEvent::Done {
                    message: format!("✓ Git 已安装：{version}。环境变量已配置（重启终端后生效）。"),
                },
            );
            Ok(())
        }
        crate::env_checker::GitStatus::NotInstalled => {
            let msg = "安装器退出成功但 git --version 仍失败。可能是 PATH 未更新——请重启系统后重试。".to_string();
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "verify".into(),
                    message: msg.clone(),
                    recoverable: true,
                },
            );
            Err(AppError::Config(msg))
        }
    }
}

/// Install Claude Code via the official PowerShell installer.
/// `irm https://claude.ai/install.ps1 | iex`
pub async fn install_claude_code(app: AppHandle) -> Result<()> {
    let cfg = config::load();

    emit(
        &app,
        InstallEvent::Started {
            target: "claude-code".into(),
        },
    );

    if cfg.debug_dry_run {
        emit(
            &app,
            InstallEvent::Log {
                line: "[DRY-RUN] skipping real PowerShell install".into(),
            },
        );
        emit(
            &app,
            InstallEvent::Installing {
                message: "（dry-run 跳过 PowerShell 调用）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        emit(
            &app,
            InstallEvent::Done {
                message: "Dry-run 完成。实际运行会调 irm https://claude.ai/install.ps1 | iex。".into(),
            },
        );
        return Ok(());
    }

    emit(
        &app,
        InstallEvent::Installing {
            message: "执行官方 PowerShell 安装脚本（约 80MB 下载，可能需 1–3 分钟）...".into(),
        },
    );

    let code = spawn_streaming(
        &app,
        "powershell.exe",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "irm https://claude.ai/install.ps1 | iex",
        ],
    )
    .await?;

    if code != 0 {
        let msg = format!("PowerShell 脚本退出码 {code}");
        emit(
            &app,
            InstallEvent::Failed {
                stage_id: "powershell".into(),
                message: msg.clone(),
                recoverable: true,
            },
        );
        return Err(AppError::Config(msg));
    }

    emit(
        &app,
        InstallEvent::Verifying {
            message: "验证 Claude Code 安装...".into(),
        },
    );
    match crate::env_checker::check_claude_installed() {
        crate::env_checker::ClaudeStatus::Installed { version, .. } => {
            emit(
                &app,
                InstallEvent::Done {
                    message: format!("✓ Claude Code 已安装：{version}（PATH 在新终端中生效）。"),
                },
            );
            Ok(())
        }
        crate::env_checker::ClaudeStatus::NotInstalled => {
            let msg = "脚本退出成功但 claude --version 仍失败。常见原因：~/.local/bin 未加入 PATH，请重启系统或重启本工具。".to_string();
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "verify".into(),
                    message: msg.clone(),
                    recoverable: true,
                },
            );
            Err(AppError::Config(msg))
        }
    }
}
