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

/// Append an arbitrary directory to the user PATH if not already present.
///
/// Why PowerShell `[Environment]::SetEnvironmentVariable` instead of `setx`:
/// setx truncates the value at 1024 characters — a real risk when the existing
/// user PATH is already long. SetEnvironmentVariable has no such limit.
///
/// Idempotent: if `target_dir` is already in user PATH, we just log and return.
async fn add_dir_to_user_path(app: &AppHandle, target_dir: &str) -> Result<()> {
    emit(
        app,
        InstallEvent::Configuring {
            message: format!("追加到 user PATH：{}", truncate(target_dir, 100)),
        },
    );
    // Escape single quotes (`'` → `''`) so an install path with a quote can't
    // close our PowerShell string. Standard install paths shouldn't have them
    // but defending costs nothing.
    let escaped = target_dir.replace('\'', "''");
    let script = format!(
        "$bin = '{escaped}'; \
         $p = [Environment]::GetEnvironmentVariable('PATH','User'); \
         if ([string]::IsNullOrEmpty($p)) {{ $p = '' }}; \
         $parts = $p.Split(';') | Where-Object {{ $_ -ne '' }}; \
         if ($parts -notcontains $bin) {{ \
           $new = if ([string]::IsNullOrEmpty($p)) {{ $bin }} else {{ \"$p;$bin\" }}; \
           [Environment]::SetEnvironmentVariable('PATH', $new, 'User'); \
           Write-Host \"[path] appended $bin to user PATH\" \
         }} else {{ \
           Write-Host \"[path] $bin already present in user PATH\" \
         }}"
    );
    let code = spawn_streaming(
        app,
        "powershell.exe",
        &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script],
    )
    .await?;
    if code != 0 {
        return Err(AppError::Config(format!(
            "追加 user PATH 失败，PowerShell 退出码 {code}"
        )));
    }
    Ok(())
}

/// Append `%USERPROFILE%\.local\bin` to user PATH (where Claude Code installs
/// `claude.exe`). Wraps `add_dir_to_user_path` with the resolved absolute path,
/// since Anthropic's `install.ps1` sometimes leaves PATH untouched and prints
/// "Native installation exists but ... is not in your PATH" — without this
/// entry, even a ClaudeTool restart can't find `claude.exe`.
async fn ensure_local_bin_in_path(app: &AppHandle) -> Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Config("could not resolve user home dir".into()))?;
    let bin = home.join(".local").join("bin");
    add_dir_to_user_path(app, &bin.to_string_lossy()).await
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

/// One-click repair for "Git is installed but env vars aren't configured".
///
/// Triggered when the user installed Git for Windows themselves to a non-default
/// location (e.g. `D:\Program Files\Git`) and either skipped the "add to PATH"
/// option in the installer, or it didn't take effect. We discover the install
/// via registry (handled by `env_checker::check_git_installed`), then:
///   1. Append `<install>\cmd` to user PATH (so `git` resolves in shells)
///   2. Set `CLAUDE_CODE_GIT_BASH_PATH = <install>\bin\bash.exe` (so Claude
///      Code's plugin commands can find bash)
///
/// Idempotent: safe to re-run. No re-download, no UAC.
pub async fn repair_git_env(app: AppHandle) -> Result<()> {
    emit(
        &app,
        InstallEvent::Started {
            target: "git-env-repair".into(),
        },
    );

    let status = crate::env_checker::check_git_installed();
    let (git_path_str, bash_path_opt) = match status {
        crate::env_checker::GitStatus::Installed {
            path, bash_path, ..
        } => (path, bash_path),
        crate::env_checker::GitStatus::NotInstalled => {
            let msg = "未检测到 Git。请先点「一键安装 Git」。\n\n\
                       （如果你确认装了 Git 但仍提示未检测到，可能是绿色版/Scoop 版没写注册表——\
                       这种情况请手动设置环境变量，或卸载后用官方安装器重装。）"
                .to_string();
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "detect".into(),
                    message: msg.clone(),
                    recoverable: false,
                },
            );
            return Err(AppError::Config(msg));
        }
    };

    // Resolve <install>\cmd from the discovered git.exe path.
    let git_exe = PathBuf::from(&git_path_str);
    let cmd_dir = match git_exe.parent() {
        Some(d) => d.to_path_buf(),
        None => {
            let msg = format!("无法解析 git.exe 父目录：{git_path_str}");
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "resolve".into(),
                    message: msg.clone(),
                    recoverable: false,
                },
            );
            return Err(AppError::Config(msg));
        }
    };
    let cmd_dir_str = cmd_dir.to_string_lossy().to_string();

    if let Err(e) = add_dir_to_user_path(&app, &cmd_dir_str).await {
        emit(
            &app,
            InstallEvent::Failed {
                stage_id: "path".into(),
                message: format!("追加 PATH 失败：{e}"),
                recoverable: true,
            },
        );
        return Err(e);
    }

    // Bash env var (best-effort: a real Git for Windows install always has bash)
    let bash_msg = if let Some(bash) = bash_path_opt.as_ref() {
        if let Err(e) = set_git_bash_env_var(&app, bash).await {
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "configure".into(),
                    message: format!("配置 CLAUDE_CODE_GIT_BASH_PATH 失败：{e}"),
                    recoverable: true,
                },
            );
            return Err(e);
        }
        bash.clone()
    } else {
        emit(
            &app,
            InstallEvent::Log {
                line: "[warn] 未在 Git 安装目录下找到 bin\\bash.exe，跳过 CLAUDE_CODE_GIT_BASH_PATH"
                    .into(),
            },
        );
        "（未设置——Git 安装目录下未找到 bash.exe）".to_string()
    };

    emit(
        &app,
        InstallEvent::Done {
            message: format!(
                "✓ Git 环境变量已配置：\n\n  • PATH 追加：{cmd_dir_str}\n  • CLAUDE_CODE_GIT_BASH_PATH：{bash_msg}\n\n\
                 ⚠️ ClaudeTool 当前进程持有旧的环境变量快照，需要\n\
                 **完全退出 ClaudeTool 再重新启动**才能识别 git 命令。"
            ),
        },
    );
    Ok(())
}

// =============================================================================
// Claude Code direct-download install (replaces `irm install.ps1 | iex`)
//
// Why we don't use Anthropic's PowerShell installer:
//   1. `claude.ai/install.ps1` is fronted by Cloudflare. On some user IPs CF
//      returns a JS-challenge HTML page; PowerShell's `irm` can't solve JS,
//      so the script-as-text becomes garbage and execution explodes.
//   2. The installer prints "Installation complete!" even when the second-stage
//      binary download fails ("× Installation failed"). PowerShell's exit code
//      is 0, so we (the parent) can't detect failure from the process state.
//   3. The installer sometimes forgets to add ~/.local/bin to user PATH and
//      delegates that to the human ("‼ Setup notes: ...").
//
// What we do instead: hit the same downloads.claude.ai endpoints that
// install.ps1 itself uses, but from Rust, with our 5-retry/proxy/progress
// pipeline plus SHA256 verification. downloads.claude.ai is served from
// Google Cloud Storage (`x-goog-*` response headers) with no Cloudflare
// challenge.
//
//   GET https://downloads.claude.ai/claude-code-releases/latest
//     → "<version>" (e.g. "2.1.116")
//   GET https://downloads.claude.ai/claude-code-releases/<v>/manifest.json
//     → JSON with platforms.win32-x64 = { binary, checksum (sha256), size }
//   GET https://downloads.claude.ai/claude-code-releases/<v>/win32-x64/claude.exe
//     → raw binary, ~247 MB
// =============================================================================

const CLAUDE_DOWNLOAD_BASE: &str = "https://downloads.claude.ai/claude-code-releases";

#[derive(Debug, Deserialize)]
struct ClaudeManifest {
    platforms: std::collections::HashMap<String, ClaudePlatformEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct ClaudePlatformEntry {
    binary: String,
    checksum: String,
    size: u64,
}

async fn fetch_latest_claude_version() -> Result<String> {
    let url = format!("{CLAUDE_DOWNLOAD_BASE}/latest");
    let v = net::client()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?
        .trim()
        .to_string();
    if v.is_empty() {
        return Err(AppError::Config(
            "Anthropic latest 端点返回空版本字符串".into(),
        ));
    }
    Ok(v)
}

async fn fetch_claude_win32_entry(version: &str) -> Result<ClaudePlatformEntry> {
    let url = format!("{CLAUDE_DOWNLOAD_BASE}/{version}/manifest.json");
    let manifest: ClaudeManifest = net::client()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    manifest
        .platforms
        .get("win32-x64")
        .cloned()
        .ok_or_else(|| {
            AppError::Config(format!(
                "manifest 里没有 win32-x64 条目（version={version}）"
            ))
        })
}

/// Verify a downloaded file's SHA256 matches the manifest's checksum.
/// Reads the file in 1MB chunks so we don't blow memory on the 250MB binary.
async fn verify_sha256(path: &std::path::Path, expected_hex: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual_hex = format!("{:x}", hasher.finalize());
    if actual_hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(AppError::Config(format!(
            "SHA256 不匹配（下载可能损坏或被篡改）。\n  期望: {expected_hex}\n  实际: {actual_hex}"
        )))
    }
}

/// Move the freshly-downloaded binary into %USERPROFILE%\.local\bin\<binary>.
/// Creates the directory if it doesn't exist. Overwrites any existing file.
async fn install_binary_to_local_bin(temp_path: &std::path::Path, binary_name: &str) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Config("could not resolve user home dir".into()))?;
    let bin_dir = home.join(".local").join("bin");
    tokio::fs::create_dir_all(&bin_dir).await?;
    let dest = bin_dir.join(binary_name);
    // Best-effort remove old file first; rename will fail across drives if a
    // stale file is open by the current process.
    let _ = tokio::fs::remove_file(&dest).await;
    // Cross-volume safe: copy + remove, not rename.
    tokio::fs::copy(temp_path, &dest).await?;
    let _ = tokio::fs::remove_file(temp_path).await;
    Ok(dest)
}

/// Install Claude Code by directly downloading the official binary from
/// downloads.claude.ai (the same endpoints Anthropic's install.ps1 uses,
/// but without the lying-on-success and CF-blocked PowerShell wrapper).
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
                line: "[DRY-RUN] skipping real download from downloads.claude.ai".into(),
            },
        );
        emit(
            &app,
            InstallEvent::Resolving {
                message: "解析最新版本号（dry-run）...".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Simulate streaming download of the ~250MB Win binary
        let total: u64 = 247 * 1024 * 1024;
        for i in 1..=6 {
            let downloaded = total * i / 6;
            emit(
                &app,
                InstallEvent::Downloading {
                    downloaded_bytes: downloaded,
                    total_bytes: Some(total),
                    message: format!(
                        "下载 claude.exe {:.1}MB / {:.1}MB（dry-run 模拟）",
                        downloaded as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ),
                },
            );
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        emit(
            &app,
            InstallEvent::Verifying {
                message: "（dry-run 跳过 SHA256 校验和最终验证）".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        emit(
            &app,
            InstallEvent::Done {
                message: "Dry-run 完成 ✓ — 实际运行会从 downloads.claude.ai 直拉二进制（绕过 CF + 不依赖 PS 脚本）。\n\
                          ⚠️ 想让 ReadinessWizard 消失请到「设置」关闭「模拟未安装 Claude Code」开关。"
                    .into(),
            },
        );
        return Ok(());
    }

    // Step 1: resolve latest version
    emit(
        &app,
        InstallEvent::Resolving {
            message: "查询最新 Claude Code 版本（downloads.claude.ai）...".into(),
        },
    );
    let version = match fetch_latest_claude_version().await {
        Ok(v) => v,
        Err(e) => {
            let msg = classify_download_error(&e, "拉取最新版本号");
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "resolve".into(),
                    message: msg.clone(),
                    recoverable: true,
                },
            );
            return Err(AppError::Config(msg));
        }
    };
    emit(
        &app,
        InstallEvent::Log {
            line: format!("[resolve] latest version = {version}"),
        },
    );

    // Step 2: fetch manifest and locate win32-x64 entry
    let entry = match fetch_claude_win32_entry(&version).await {
        Ok(e) => e,
        Err(e) => {
            let msg = classify_download_error(&e, "拉取 manifest.json");
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "manifest".into(),
                    message: msg.clone(),
                    recoverable: true,
                },
            );
            return Err(AppError::Config(msg));
        }
    };
    emit(
        &app,
        InstallEvent::Log {
            line: format!(
                "[manifest] win32-x64: binary={}, size={:.1}MB, sha256={}…",
                entry.binary,
                entry.size as f64 / 1_048_576.0,
                truncate(&entry.checksum, 16)
            ),
        },
    );

    // Step 3: stream-download claude.exe (5-retry pipeline already handles
    // proxy / chunk drops / CDN flakes).
    let download_url = format!("{CLAUDE_DOWNLOAD_BASE}/{version}/win32-x64/{}", entry.binary);
    emit(
        &app,
        InstallEvent::Downloading {
            downloaded_bytes: 0,
            total_bytes: Some(entry.size),
            message: format!("准备下载 claude.exe（{:.0}MB）...", entry.size as f64 / 1_048_576.0),
        },
    );
    let temp_path = match download_to_temp(
        &app,
        &download_url,
        Some(entry.size),
        "claude-code-installer.exe",
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            let msg = classify_download_error(&e, "下载 claude.exe");
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "download".into(),
                    message: msg.clone(),
                    recoverable: true,
                },
            );
            return Err(AppError::Config(msg));
        }
    };

    // Step 4: SHA256 verify (Anthropic's own script doesn't do this; we do)
    emit(
        &app,
        InstallEvent::Verifying {
            message: "校验 SHA256（防止下载被代理/CDN 篡改或截断）...".into(),
        },
    );
    if let Err(e) = verify_sha256(&temp_path, &entry.checksum).await {
        let msg = format!(
            "校验失败：{e}\n\n建议：换 VPN 节点重试（多半是中间代理压缩或截断了响应）。"
        );
        emit(
            &app,
            InstallEvent::Failed {
                stage_id: "checksum".into(),
                message: msg.clone(),
                recoverable: true,
            },
        );
        // Clean up the bad file so a retry doesn't reuse it.
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(AppError::Config(msg));
    }
    emit(
        &app,
        InstallEvent::Log {
            line: "[verify] SHA256 ✓".into(),
        },
    );

    // Step 5: move into ~/.local/bin/claude.exe
    emit(
        &app,
        InstallEvent::Installing {
            message: "安装到 %USERPROFILE%\\.local\\bin\\claude.exe ...".into(),
        },
    );
    let installed_path = match install_binary_to_local_bin(&temp_path, &entry.binary).await {
        Ok(p) => p,
        Err(e) => {
            let msg = format!(
                "无法写入 ~/.local/bin/{}：{e}\n\n常见原因：杀毒软件拦截写入；或目录被占用（请先关掉所有 claude 终端）。",
                entry.binary
            );
            emit(
                &app,
                InstallEvent::Failed {
                    stage_id: "install".into(),
                    message: msg.clone(),
                    recoverable: true,
                },
            );
            return Err(AppError::Config(msg));
        }
    };
    emit(
        &app,
        InstallEvent::Log {
            line: format!("[install] wrote {}", installed_path.display()),
        },
    );

    // Step 6: ensure ~/.local/bin is in user PATH (idempotent)
    if let Err(e) = ensure_local_bin_in_path(&app).await {
        emit(
            &app,
            InstallEvent::Log {
                line: format!("[path] 补全 PATH 失败（非致命，可手动添加）: {e}"),
            },
        );
    }

    // Step 7: real verification (file presence + best-effort version probe)
    emit(
        &app,
        InstallEvent::Verifying {
            message: "验证安装结果...".into(),
        },
    );
    if !installed_path.exists() {
        // This shouldn't happen — we just wrote it — but defend anyway.
        let msg = format!("安装文件意外消失：{}", installed_path.display());
        emit(
            &app,
            InstallEvent::Failed {
                stage_id: "verify".into(),
                message: msg.clone(),
                recoverable: false,
            },
        );
        return Err(AppError::Config(msg));
    }
    match crate::env_checker::check_claude_installed() {
        crate::env_checker::ClaudeStatus::Installed { version: v, .. } => {
            emit(
                &app,
                InstallEvent::Done {
                    message: format!(
                        "✓ Claude Code 已安装并验证：{v}\n\n安装位置：{}\n（在新终端中可直接 `claude`）",
                        installed_path.display()
                    ),
                },
            );
            Ok(())
        }
        crate::env_checker::ClaudeStatus::NotInstalled => {
            // File exists, PATH was added, but our current process holds a
            // pre-launch PATH snapshot — restart will pick it up.
            emit(
                &app,
                InstallEvent::Done {
                    message: format!(
                        "✓ Claude Code 已下载并安装：版本 {version}\n安装位置：{}\n\n\
                         ⚠️ ClaudeTool 当前进程持有旧的 PATH 快照，需要\n\
                         **完全退出 ClaudeTool 再重新启动**才能识别 claude 命令。",
                        installed_path.display()
                    ),
                },
            );
            Ok(())
        }
    }
}

/// Translate a low-level download/network error into a user-actionable Chinese
/// message, classifying common failure modes.
fn classify_download_error(err: &AppError, stage_label: &str) -> String {
    let raw = err.to_string();
    let lower = raw.to_lowercase();
    let hint = if lower.contains("just a moment")
        || lower.contains("_cf_chl")
        || lower.contains("cloudflare")
    {
        "你的代理出口 IP 被 Cloudflare 拦截。建议换 VPN 节点（避开数据中心 IP，首选住宅节点）后重试。"
    } else if lower.contains("connection closed")
        || lower.contains("econnreset")
        || lower.contains("connection reset")
        || lower.contains("broken pipe")
    {
        "连接被中途切断（多半是代理/VPN 抖动）。建议换节点或换协议（TLS/Trojan 比 SS 更稳）后重试。"
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "网络超时。建议确认代理是否在工作，或换更快的节点后重试。"
    } else if lower.contains("dns") || lower.contains("not found") || lower.contains("notfound") {
        "DNS 解析失败。请确认 ClaudeTool 设置里的代理地址正确，或检查 VPN 是否开启。"
    } else if lower.contains("ssl") || lower.contains("tls") || lower.contains("certificate") {
        "TLS/证书验证失败。可能代理是 HTTP-only 而目标是 HTTPS，或本机时间不准。"
    } else {
        "建议：检查代理设置、换 VPN 节点、或稍后重试。"
    };
    format!("{stage_label}失败：{raw}\n\n{hint}")
}
