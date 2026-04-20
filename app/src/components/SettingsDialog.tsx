import { useEffect, useState } from 'react';
import { Check, Save } from 'lucide-react';
import { Modal } from '@/components/ui/Modal';
import { useEnv } from '@/store/env';
import { setDebugFlag, setProxy, type AppConfig, type ProxyConfig } from '@/ipc/env';

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const config = useEnv((s) => s.config);
  const loadConfig = useEnv((s) => s.loadConfig);
  const refresh = useEnv((s) => s.refresh);

  const [local, setLocal] = useState<AppConfig | null>(config);
  const [proxyDraft, setProxyDraft] = useState<ProxyConfig>({
    http: config?.proxy.http ?? null,
    https: config?.proxy.https ?? null,
    noProxy: config?.proxy.noProxy ?? null,
  });
  const [proxySaved, setProxySaved] = useState(false);

  useEffect(() => {
    setLocal(config);
    if (config) {
      setProxyDraft({
        http: config.proxy.http,
        https: config.proxy.https,
        noProxy: config.proxy.noProxy,
      });
    }
  }, [config]);

  if (!local) return null;

  const handleToggle = async (
    name: 'forceClaudeMissing' | 'forceGitMissing' | 'dryRun',
    value: boolean
  ) => {
    try {
      const updated = await setDebugFlag(name, value);
      setLocal(updated);
      await loadConfig();
      // Re-run env detection so debug overrides take effect immediately.
      await refresh();
    } catch (err) {
      console.error('set debug flag failed', err);
    }
  };

  const handleProxyFieldChange = (key: keyof ProxyConfig, value: string) => {
    setProxyDraft((d) => ({ ...d, [key]: value || null }));
  };

  const handleSaveProxy = async () => {
    try {
      const updated = await setProxy(proxyDraft);
      setLocal(updated);
      await loadConfig();
      // Re-run env detection so the network probe uses the new proxy.
      await refresh();
      setProxySaved(true);
      setTimeout(() => setProxySaved(false), 2000);
    } catch (err) {
      console.error('set_proxy failed', err);
    }
  };

  const proxyDirty =
    (proxyDraft.http ?? '') !== (local.proxy.http ?? '') ||
    (proxyDraft.https ?? '') !== (local.proxy.https ?? '') ||
    (proxyDraft.noProxy ?? '') !== (local.proxy.noProxy ?? '');

  return (
    <Modal open={open} onClose={onClose} ariaLabel="设置">
      <div className="w-[520px] max-w-full">
        <h2 className="text-lg font-semibold">设置</h2>

        <section className="mt-4">
          <h3 className="text-sm font-medium">网络代理</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            如果你电脑用 VPN 但仍连不上 api.anthropic.com，多半是 VPN 走的「系统代理」模式
            ——不会自动覆盖 Claude / Git 这些命令行程序。在这里填入代理地址，本工具会把 HTTP_PROXY /
            HTTPS_PROXY 注入到所有由本工具启动的进程（claude / 安装器 / 网络检测）。
          </p>

          <div className="mt-3 space-y-2">
            <ProxyInput
              label="HTTPS 代理"
              placeholder="http://127.0.0.1:7897"
              value={proxyDraft.https ?? ''}
              onChange={(v) => handleProxyFieldChange('https', v)}
            />
            <ProxyInput
              label="HTTP 代理（可选，通常与 HTTPS 相同）"
              placeholder="http://127.0.0.1:7897"
              value={proxyDraft.http ?? ''}
              onChange={(v) => handleProxyFieldChange('http', v)}
            />
            <ProxyInput
              label="NO_PROXY 列表（可选，逗号分隔）"
              placeholder="localhost,127.0.0.1"
              value={proxyDraft.noProxy ?? ''}
              onChange={(v) => handleProxyFieldChange('noProxy', v)}
            />
          </div>

          <div className="mt-3 flex items-center gap-2">
            <button
              onClick={() => void handleSaveProxy()}
              disabled={!proxyDirty}
              className="flex items-center gap-1 rounded bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:opacity-90 disabled:opacity-40"
            >
              {proxySaved ? (
                <>
                  <Check className="h-3 w-3" />
                  已保存
                </>
              ) : (
                <>
                  <Save className="h-3 w-3" />
                  保存代理设置
                </>
              )}
            </button>
            <p className="text-[10px] text-muted-foreground">
              保存后立即对新启动的 claude / 安装器生效；当前活跃终端需重启或切文件夹刷新。
            </p>
          </div>
        </section>

        <section className="mt-6 border-t border-border pt-4">
          <h3 className="text-sm font-medium">调试模式</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            仅供开发与端到端测试使用。开启后会让本机也能走过 ReadinessWizard /
            安装流程，但不影响你已装的真实环境。
          </p>

          <div className="mt-3 space-y-2">
            <ToggleRow
              label="模拟未安装 Claude Code"
              hint="启用后启动检测会强制返回「未安装」，触发 ReadinessWizard"
              checked={local.debugForceClaudeMissing}
              onChange={(v) => void handleToggle('forceClaudeMissing', v)}
            />
            <ToggleRow
              label="模拟未安装 Git for Windows"
              hint="启用后 Git 检测强制返回「未安装」"
              checked={local.debugForceGitMissing}
              onChange={(v) => void handleToggle('forceGitMissing', v)}
            />
            <ToggleRow
              label="安装 Dry-Run 模式"
              hint="启用后「一键安装」按钮只走流程、emit 进度事件，但不真的下载/spawn 安装器"
              checked={local.debugDryRun}
              onChange={(v) => void handleToggle('dryRun', v)}
            />
          </div>
        </section>

        <div className="mt-6 flex justify-end">
          <button
            onClick={onClose}
            className="rounded bg-primary px-4 py-1.5 text-sm text-primary-foreground hover:opacity-90"
          >
            关闭
          </button>
        </div>
      </div>
    </Modal>
  );
}

function ProxyInput({
  label,
  placeholder,
  value,
  onChange,
}: {
  label: string;
  placeholder: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div>
      <label className="text-[11px] text-muted-foreground">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="mt-0.5 w-full rounded border border-input bg-background px-2 py-1 font-mono text-xs outline-none focus:ring-2 focus:ring-ring"
      />
    </div>
  );
}

function ToggleRow({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-3 rounded border border-border p-2 hover:bg-muted/50">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-0.5"
      />
      <div className="flex-1">
        <p className="text-sm">{label}</p>
        <p className="mt-0.5 text-[11px] text-muted-foreground">{hint}</p>
      </div>
    </label>
  );
}
