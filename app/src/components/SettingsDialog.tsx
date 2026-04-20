import { useEffect, useState } from 'react';
import { Modal } from '@/components/ui/Modal';
import { useEnv } from '@/store/env';
import { setDebugFlag, type AppConfig } from '@/ipc/env';

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const config = useEnv((s) => s.config);
  const loadConfig = useEnv((s) => s.loadConfig);
  const refresh = useEnv((s) => s.refresh);

  const [local, setLocal] = useState<AppConfig | null>(config);

  useEffect(() => {
    setLocal(config);
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

  return (
    <Modal open={open} onClose={onClose} ariaLabel="设置">
      <div className="w-[480px] max-w-full">
        <h2 className="text-lg font-semibold">设置</h2>

        <section className="mt-4">
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
