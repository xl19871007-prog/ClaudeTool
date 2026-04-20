import { Folder, BookOpen, Package, Settings } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { StatusDot } from '@/components/ui/StatusDot';
import { useWorkbench, pathBasename } from '@/store/workbench';
import { useEnv } from '@/store/env';
import { usePanels } from '@/store/panels';
import { t } from '@/i18n/zh-CN';

export function TopBar() {
  const cwd = useWorkbench((s) => s.cwd);
  const setCwd = useWorkbench((s) => s.setCwd);
  const report = useEnv((s) => s.report);
  const togglePanel = usePanels((s) => s.toggle);
  const activePanel = usePanels((s) => s.open);

  const handleSelectFolder = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: t.topbar.changeFolder,
      });
      if (typeof selected === 'string' && selected.length > 0) {
        setCwd(selected);
      }
    } catch (err) {
      console.error('Folder dialog failed', err);
    }
  };

  const claudeStatus: 'unknown' | 'ok' | 'error' = !report
    ? 'unknown'
    : report.claude.kind === 'installed'
      ? 'ok'
      : 'error';
  const claudeLabel =
    report?.claude.kind === 'installed'
      ? report.claude.version.split(/\s+/)[0] || '已装'
      : report?.claude.kind === 'notInstalled'
        ? t.topbar.claudeMissing
        : '...';

  const networkStatus: 'unknown' | 'ok' | 'slow' | 'error' = (() => {
    if (!report) return 'unknown';
    switch (report.network.kind) {
      case 'ok':
        return 'ok';
      case 'slow':
        return 'slow';
      case 'unreachable':
        return 'error';
    }
  })();
  const networkLabel = (() => {
    if (!report) return '...';
    switch (report.network.kind) {
      case 'ok':
        return t.topbar.networkOk;
      case 'slow':
        return t.topbar.networkSlow;
      case 'unreachable':
        return t.topbar.networkUnreachable;
    }
  })();

  const hasUpdate = report?.update.hasUpdate ?? false;

  return (
    <header className="flex h-12 items-center gap-3 border-b border-border bg-card px-3">
      <button
        onClick={handleSelectFolder}
        className="flex max-w-xs items-center gap-1.5 rounded px-2 py-1 hover:bg-muted"
        title={cwd ?? t.topbar.selectFolder}
      >
        <Folder className="h-4 w-4 shrink-0" />
        <span className="truncate text-sm">{cwd ? pathBasename(cwd) : t.topbar.selectFolder}</span>
      </button>

      <div className="ml-3 flex items-center gap-3 text-xs text-muted-foreground">
        <div className="flex items-center gap-1">
          <StatusDot status={claudeStatus} />
          <span>
            {t.topbar.claudeStatus}: {claudeLabel}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <StatusDot status={networkStatus} />
          <span>
            {t.topbar.network}: {networkLabel}
          </span>
        </div>
      </div>

      <div className="ml-auto flex items-center gap-1">
        <button
          onClick={() => togglePanel('commands')}
          className={`flex items-center gap-1 rounded px-2 py-1 text-sm hover:bg-muted ${
            activePanel === 'commands' ? 'bg-muted' : ''
          }`}
          aria-label={t.topbar.commands}
          aria-pressed={activePanel === 'commands'}
        >
          <BookOpen className="h-4 w-4" />
          <span>{t.topbar.commands}</span>
        </button>
        <button
          onClick={() => togglePanel('skills')}
          className={`flex items-center gap-1 rounded px-2 py-1 text-sm hover:bg-muted ${
            activePanel === 'skills' ? 'bg-muted' : ''
          }`}
          aria-label={t.topbar.skills}
          aria-pressed={activePanel === 'skills'}
        >
          <Package className="h-4 w-4" />
          <span>{t.topbar.skills}</span>
        </button>
        <button
          className="relative rounded p-2 hover:bg-muted"
          aria-label={
            hasUpdate ? `${t.topbar.settings} · ${t.topbar.updateAvailable}` : t.topbar.settings
          }
          title={hasUpdate ? t.topbar.updateAvailable : t.topbar.settings}
        >
          <Settings className="h-4 w-4" />
          {hasUpdate && (
            <span className="absolute right-1 top-1 h-2 w-2 rounded-full bg-destructive" />
          )}
        </button>
      </div>
    </header>
  );
}
