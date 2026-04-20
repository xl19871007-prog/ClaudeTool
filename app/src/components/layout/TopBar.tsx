import { Folder, BookOpen, Package, Settings } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { StatusDot } from '@/components/ui/StatusDot';
import { useWorkbench, pathBasename } from '@/store/workbench';
import { t } from '@/i18n/zh-CN';

export function TopBar() {
  const cwd = useWorkbench((s) => s.cwd);
  const setCwd = useWorkbench((s) => s.setCwd);

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
          <StatusDot status="unknown" />
          <span>{t.topbar.claudeStatus}: ?</span>
        </div>
        <div className="flex items-center gap-1">
          <StatusDot status="unknown" />
          <span>{t.topbar.network}</span>
        </div>
      </div>

      <div className="ml-auto flex items-center gap-1">
        <button
          className="flex items-center gap-1 rounded px-2 py-1 text-sm hover:bg-muted"
          aria-label={t.topbar.commands}
        >
          <BookOpen className="h-4 w-4" />
          <span>{t.topbar.commands}</span>
        </button>
        <button
          className="flex items-center gap-1 rounded px-2 py-1 text-sm hover:bg-muted"
          aria-label={t.topbar.skills}
        >
          <Package className="h-4 w-4" />
          <span>{t.topbar.skills}</span>
        </button>
        <button className="rounded p-2 hover:bg-muted" aria-label={t.topbar.settings}>
          <Settings className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}
