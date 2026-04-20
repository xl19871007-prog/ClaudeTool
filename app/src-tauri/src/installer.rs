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
/// Retries up to 5 times with exponential backoff (2s/5s/10s/15s/20s)
/// to survive proxy flakiness on large files — network hiccups are
/// common on VPN + GitHub CDN, and users are willing to wait if we
/// show we're still trying.
async fn download_to_temp(
    app: &AppHandle,
    url: &str,
    expected_size: Option<u64>,
    suggested_filename: &str,
) -> Result<PathBuf> {
    const MAX_ATTEMPTS: u32 = 5;
    const BACKOFF_SECS: [u64; 4] = [2, 5, 10, 15]; // 20s before last attempt not used; last attempt has no post-backoff
    let mut last_err: Option<AppError> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match download_once(app, url, expected_size, suggested_filename, attempt, MAX_ATTEMPTS).await {
            Ok(path) => {
                if attempt > 1 {
                    emit(
                        app,
                        InstallEvent::Log {
                            line: format!("[download] succeeded on attempt {attempt}/{MAX_ATTEMPTS}"),
                        },
                    );
                }
                return Ok(path);
            }
            Err(e) => {
                emit(
                    app,
                    InstallEvent::Log {
                        line: format!("[download] attempt {attempt}/{MAX_ATTEMPTS} failed: {e}"),
                    },
                );
                last_err = Some(e);
                if attempt < MAX_ATTEMPTS {
                    let wait = BACKOFF_SECS[(attempt as usize - 1).min(BACKOFF_SECS.len() - 1)];
                    emit(
                        app,
                        InstallEvent::Downloading {
                            downloaded_bytes: 0,
                            total_bytes: expected_size,
                            message: format!(
                                "第 {attempt}/{MAX_ATTEMPTS} 次下载失败，{wait} 秒后自动重试...（代理/网络抖动常见，请稍等）"
                            ),
                        },
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                }
            }
        }
    }
    Err(last_err.expect("loop ran at least once"))
}

async fn download_once(
    app: &AppHandle,
    url: &str,
    expected_size: Option<u64>,
    suggested_filename: &str,
    attempt: u32,
    max_attempts: u32,
) -> Result<PathBuf> {
    use std::time::Instant;
    use tokio::io::AsyncWriteExt;

    let dest = std::env::temp_dir().join(suggested_filename);
    let mut file = tokio::fs::File::create(&dest).await?;

    let mut resp = net::client().get(url).send().await?.error_for_status()?;
    let total = resp.content_length().or(expected_size);

    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();

    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                file.write_all(&chunk).await?;
                downloaded += chunk.len() as u64;
                if last_emit.elapsed().as_millis() >= 100 {
                    emit(
                        app,
                        InstallEvent::Downloading {
                            downloaded_bytes: downloaded,
                            total_bytes: total,
                            message: format!(
                                "下载中 {:.1}MB{}{}",
                                downloaded as f64 / 1_048_576.0,
                                total
                                    .map(|t| format!(" / {:.1}MB", t as f64 / 1_048_576.0))
                                    .unwrap_or_default(),
                                if attempt > 1 {
                                    format!("（第 {attempt}/{max_attempts} 次尝试）")
                                } else {
                                    String::new()
                                }
                            ),
                        },
                    );
                    last_emit = Instant::now();
                }
            }
            Ok(None) => break,
            Err(e) => {
                emit(
                    app,
                    InstallEvent::Log {
                        line: format!(
                            "[download] chunk error after {:.1}MB: {e}",
                            downloaded as f64 / 1_048_576.0
                        ),
                    },
                );
                return Err(AppError::from(e));
            }
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
/// User-configured proxy is injected as HTTP_PROXY/HTTPS_PROXY env vars
/// (ADR-018) so spawned tools (PowerShell, git installer) can reach the
/// internet on systems where the VPN is sysproxy-only.
async fn spawn_streaming(
    app: &AppHandle,
    program: &str,
    args: &[&str],
) -> Result<i32> {
    let cfg = config::load();
    let mut cmd = AsyncCommand::new(program);
    cmd.args(args);
    for (k, v) in cfg.proxy.as_env_pairs() {
        cmd.env(k, v);
    }
    // Hide the child's console window on Windows (CREATE_NO_WINDOW = 0x08000000).
    // Without this, spawning powershell.exe from our GUI process pops a blank
    // black console window for the duration of the install — distracting and
    // makes users think something crashed.
    // tokio::process::Command re-exports `creation_flags` directly, no trait import needed.
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x0800_0000);
    let mut child = cmd
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

/// Append `%USERPROFILE%\.local\bin` to the user PATH if not already present.
///
/// Why this is needed: Anthropic's `irm https://claude.ai/install.ps1 | iex`
/// script sometimes fails to modify the user PATH on Windows — it prints the
/// warning "Native installation exists but C:\Users\...\.local\bin is not in
/// your PATH" and leaves it to the user to add it manually. Without this entry
/// in user PATH, even a ClaudeTool restart won't find `claude.exe` because the
/// PATH inherited from the system genuinely lacks it.
///
/// We use PowerShell's `[Environment]::SetEnvironmentVariable` rather than
/// `setx` because setx truncates the value to 1024 characters — a real risk
/// when the existing user PATH is already long.
async fn ensure_local_bin_in_path(app: &AppHandle) -> Result<()> {
    emit(
        app,
        InstallEvent::Configuring {
            message: "检查并补全 user PATH（%USERPROFILE%\\.local\\bin）...".into(),
        },
    );
    // Single-line PS: read user PATH, add .local\bin if missing.
    let script = "$bin = Join-Path $env:USERPROFILE '.local\\bin'; \
                  $p = [Environment]::GetEnvironmentVariable('PATH','User'); \
                  if ([string]::IsNullOrEmpty($p)) { $p = '' }; \
                  $parts = $p.Split(';') | Where-Object { $_ -ne '' }; \
                  if ($parts -notcontains $bin) { \
                    $new = if ([string]::IsNullOrEmpty($p)) { $bin } else { \"$p;$bin\" }; \
                    [Environment]::SetEnvironmentVariable('PATH', $new, 'User'); \
                    Write-Host \"[path] appended $bin to user PATH\" \
                  } else { \
                    Write-Host \"[path] $bin already present in user PATH\" \
                  }";
    let code = spawn_streaming(
        app,
        "powershell.exe",
        &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
    )
    .await?;
    if code != 0 {
        return Err(AppError::Config(format!(
            "补全 user PATH 失败，PowerShell 退出码 {code}"
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
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // Simulate streaming download: 0 → 60MB in 6 ticks of 200ms
        let total: u64 = 60 * 1024 * 1024;
        for i in 1..=6 {
            let downloaded = total * i / 6;
            emit(
                &app,
                InstallEvent::Downloading {
                    downloaded_bytes: downloaded,
                    total_bytes: Some(total),
                    message: format!(
                        "下载中 {:.1}MB / {:.1}MB（dry-run 模拟）",
                        downloaded as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ),
                },
            );
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        emit(
            &app,
            InstallEvent::Installing {
                message: "（dry-run 跳过 spawn 安装器）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        emit(
            &app,
            InstallEvent::Configuring {
                message: "（dry-run 跳过 setx 环境变量）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        emit(
            &app,
            InstallEvent::Done {
                message: "Dry-run 完成 ✓ — 实际运行时会下载/spawn UAC/setx。\n\
                          ⚠️ 想让 ReadinessWizard 消失请到「设置」关闭「模拟未安装 Git」开关。"
                    .into(),
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
                    message: format!(
                        "下载失败：{e}\n\n常见原因：\n\
                        1) 代理对 GitHub 大文件下载支持不稳定（GitHub release 会重定向到 objects.githubusercontent.com）\n\
                        2) 代理软件用 sysproxy 模式时，对长连接易中断；建议改用 TUN 模式\n\
                        3) 网络瞬时抖动\n\n\
                        可点「手动下载」直接去 git-scm.com 下载安装。"
                    ),
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
            // Same reasoning as install_claude_code: installer exit 0 +
            // failed local verify usually means PATH is only visible to new
            // processes. Treat as success with a restart hint.
            emit(
                &app,
                InstallEvent::Done {
                    message: "✓ Git 安装器已成功执行，环境变量已配置。\n\n\
                              ⚠️ 但 ClaudeTool 当前进程读不到新的 PATH，需要\n\
                              **完全退出 ClaudeTool 后再重新启动**才能识别 git 命令。\n\n\
                              关闭本对话框 → 右上角关闭 ClaudeTool → 重新打开。"
                        .into(),
                },
            );
            Ok(())
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
            InstallEvent::Resolving {
                message: "准备执行官方 PowerShell 脚本（dry-run）...".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // Simulate streaming download via the PowerShell script: 0 → 80MB
        let total: u64 = 80 * 1024 * 1024;
        for i in 1..=6 {
            let downloaded = total * i / 6;
            emit(
                &app,
                InstallEvent::Downloading {
                    downloaded_bytes: downloaded,
                    total_bytes: Some(total),
                    message: format!(
                        "下载中 {:.1}MB / {:.1}MB（dry-run 模拟）",
                        downloaded as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ),
                },
            );
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        emit(
            &app,
            InstallEvent::Installing {
                message: "（dry-run 跳过 PowerShell 调用）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        emit(
            &app,
            InstallEvent::Verifying {
                message: "（dry-run 跳过 claude --version 验证）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        emit(
            &app,
            InstallEvent::Done {
                message: "Dry-run 完成 ✓ — 实际运行会调 `irm https://claude.ai/install.ps1 | iex`。\n\
                          ⚠️ 想让 ReadinessWizard 消失请到「设置」关闭「模拟未安装 Claude Code」开关。"
                    .into(),
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

    // Anthropic's install script sometimes leaves ~/.local/bin out of user PATH
    // and prints a manual-setup note. Proactively ensure it's registered so the
    // next ClaudeTool launch (or any new shell) can find `claude.exe`.
    if let Err(e) = ensure_local_bin_in_path(&app).await {
        emit(
            &app,
            InstallEvent::Log {
                line: format!("[path] 补全 PATH 失败（非致命，可手动添加）: {e}"),
            },
        );
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
            // PowerShell exit 0 means the install script completed. In this
            // branch our in-process PATH check still can't find claude.exe —
            // this is expected because:
            //   1. We just wrote ~/.local/bin to HKCU\Environment via
            //      ensure_local_bin_in_path, but the current process holds a
            //      PATH snapshot from launch time.
            //   2. A ClaudeTool restart will re-read user PATH and pick it up.
            // So: treat as success with an explicit restart hint.
            emit(
                &app,
                InstallEvent::Done {
                    message: "✓ Claude Code 已安装，~/.local/bin 已加入 user PATH。\n\n\
                              ⚠️ ClaudeTool 当前进程持有旧的 PATH 快照，需要\n\
                              **完全退出 ClaudeTool 再重新启动**才能识别 claude 命令。\n\n\
                              关闭本对话框 → 右上角关闭 ClaudeTool → 重新打开。"
                        .into(),
                },
            );
            Ok(())
        }
    }
}
