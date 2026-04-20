import { useState } from 'react';
import { Download, ExternalLink, RefreshCw, Settings } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Modal } from '@/components/ui/Modal';
import { useEnv } from '@/store/env';
import { installGit, installClaudeCode } from '@/ipc/installer';
import { InstallProgressDialog } from '@/components/InstallProgressDialog';
import { SettingsDialog } from '@/components/SettingsDialog';
import { t } from '@/i18n/zh-CN';

const CLAUDE_DOWNLOAD_URL = 'https://claude.com/code';
const GIT_DOWNLOAD_URL = 'https://git-scm.com/download/win';

type ActiveInstall = 'git' | 'claude' | null;

export function ReadinessWizard() {
  const report = useEnv((s) => s.report);
  const loading = useEnv((s) => s.loading);
  const refresh = useEnv((s) => s.refresh);

  const [activeInstall, setActiveInstall] = useState<ActiveInstall>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  if (!report) return null;

  const claudeOk = report.claude.kind === 'installed';
  const gitOk = report.git.kind === 'installed';
  const gitBashOk =
    !gitOk || report.gitBashEnv.kind === 'configured' || report.gitBashEnv.kind === 'invalidPath';
  const networkOk = report.network.kind === 'ok' || report.network.kind === 'slow';
  if (claudeOk && gitOk && gitBashOk && networkOk) return null;

  const networkRequiredButMissing = !networkOk;

  const handleOpenUrl = async (url: string) => {
    try {
      await openUrl(url);
    } catch (err) {
      console.error('open url failed', err);
    }
  };

  const handleInstallGit = async () => {
    setActiveInstall('git');
    try {
      await installGit();
    } catch (err) {
      console.error('install_git failed', err);
    }
    // Refresh env after install (regardless of success — UI re-evaluates).
    await refresh();
  };

  const handleInstallClaude = async () => {
    setActiveInstall('claude');
    try {
      await installClaudeCode();
    } catch (err) {
      console.error('install_claude_code failed', err);
    }
    await refresh();
  };

  return (
    <>
      <Modal
        open={!activeInstall}
        onClose={() => {}}
        closeOnBackdropClick={false}
        ariaLabel={t.ready.wizardTitle}
      >
        <div className="w-[520px] max-w-full">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-lg font-semibold">{t.ready.wizardTitle}</h2>
              <p className="mt-1 text-sm text-muted-foreground">{t.ready.problemsDetected}</p>
            </div>
            <button
              onClick={() => setSettingsOpen(true)}
              className="flex shrink-0 items-center gap-1 rounded border border-border px-2 py-1 text-xs hover:bg-muted"
              title="打开设置（配代理 / 调试开关）"
            >
              <Settings className="h-3 w-3" />
              设置
            </button>
          </div>

          {!networkOk && (
            <ProblemRow
              tone="warning"
              title="网络无法访问 Anthropic"
              description="若已开 VPN 仍不通：VPN 多半走「系统代理」模式，不覆盖命令行程序。点右侧「配置代理」填入 Clash/V2Ray 的 HTTPS 代理地址即可。"
              actions={
                <>
                  <button
                    onClick={() => setSettingsOpen(true)}
                    className="flex items-center gap-1 rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90"
                  >
                    <Settings className="h-3 w-3" />
                    配置代理
                  </button>
                  <button
                    onClick={() => void refresh()}
                    disabled={loading}
                    className="flex items-center gap-1 rounded border border-border px-3 py-1.5 text-sm hover:bg-muted disabled:opacity-50"
                  >
                    <RefreshCw className="h-3 w-3" />
                    我已开 VPN，重试
                  </button>
                </>
              }
            />
          )}

          {!gitOk && (
            <ProblemRow
              tone="destructive"
              title="未检测到 Git for Windows"
              description="Claude Code 在 Windows 上的 plugin 命令依赖 git-bash。本工具会自动下载安装并配置环境变量。"
              actions={
                <>
                  <button
                    onClick={() => void handleInstallGit()}
                    disabled={networkRequiredButMissing}
                    className="flex items-center gap-1 rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90 disabled:opacity-40"
                    title={
                      networkRequiredButMissing ? '请先解决网络问题' : '下载并安装 Git for Windows'
                    }
                  >
                    <Download className="h-3 w-3" />
                    一键安装 Git
                  </button>
                  <button
                    onClick={() => void handleOpenUrl(GIT_DOWNLOAD_URL)}
                    className="flex items-center gap-1 rounded border border-border px-3 py-1.5 text-sm hover:bg-muted"
                  >
                    <ExternalLink className="h-3 w-3" />
                    手动下载
                  </button>
                </>
              }
            />
          )}

          {gitOk && report.gitBashEnv.kind === 'notConfigured' && (
            <ProblemRow
              tone="warning"
              title="CLAUDE_CODE_GIT_BASH_PATH 未配置"
              description={
                report.git.kind === 'installed' && report.git.bashPath
                  ? `检测到 git-bash 在 ${report.git.bashPath}。点「一键安装 Git」会重设环境变量；或手动 setx。`
                  : '需要设置环境变量指向 git-bash.exe。Claude Code plugin 命令才能在 Win 上运行。'
              }
              actions={
                <button
                  onClick={() => void handleInstallGit()}
                  className="flex items-center gap-1 rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90"
                  title="重新跑安装流程会顺带配置环境变量"
                >
                  <RefreshCw className="h-3 w-3" />
                  自动配置
                </button>
              }
            />
          )}

          {!claudeOk && (
            <ProblemRow
              tone="destructive"
              title={t.ready.claudeNotInstalled}
              description="本工具会调用 Anthropic 官方 PowerShell 脚本（irm https://claude.ai/install.ps1 | iex）安装到 ~/.local/bin。"
              actions={
                <>
                  <button
                    onClick={() => void handleInstallClaude()}
                    disabled={networkRequiredButMissing}
                    className="flex items-center gap-1 rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90 disabled:opacity-40"
                  >
                    <Download className="h-3 w-3" />
                    一键安装 Claude Code
                  </button>
                  <button
                    onClick={() => void handleOpenUrl(CLAUDE_DOWNLOAD_URL)}
                    className="flex items-center gap-1 rounded border border-border px-3 py-1.5 text-sm hover:bg-muted"
                  >
                    <ExternalLink className="h-3 w-3" />
                    手动下载
                  </button>
                </>
              }
            />
          )}

          <div className="mt-4 flex justify-end">
            <button
              onClick={() => void refresh()}
              disabled={loading}
              className="flex items-center gap-1 rounded border border-border px-3 py-1.5 text-xs hover:bg-muted disabled:opacity-50"
            >
              <RefreshCw className="h-3 w-3" />
              重新检测
            </button>
          </div>
        </div>
      </Modal>

      <InstallProgressDialog
        open={activeInstall !== null}
        title={
          activeInstall === 'git'
            ? '安装 Git for Windows'
            : activeInstall === 'claude'
              ? '安装 Claude Code'
              : ''
        }
        onClose={() => setActiveInstall(null)}
      />

      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </>
  );
}

function ProblemRow({
  tone,
  title,
  description,
  actions,
}: {
  tone: 'destructive' | 'warning';
  title: string;
  description: string;
  actions: React.ReactNode;
}) {
  const titleClass = tone === 'destructive' ? 'text-destructive' : 'text-warning';
  return (
    <div className="mt-4 rounded border border-border p-3">
      <p className={`text-sm font-medium ${titleClass}`}>{title}</p>
      <p className="mt-1 text-xs text-muted-foreground">{description}</p>
      <div className="mt-2 flex flex-wrap gap-2">{actions}</div>
    </div>
  );
}
