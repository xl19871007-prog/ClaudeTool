import { Plus, Search } from 'lucide-react';
import { t } from '@/i18n/zh-CN';

export function HistorySidebar() {
  return (
    <aside className="flex w-60 flex-col border-r border-border bg-card">
      <div className="border-b border-border p-2">
        <button className="flex w-full items-center justify-center gap-1.5 rounded bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:opacity-90">
          <Plus className="h-4 w-4" />
          {t.history.newSession}
        </button>
      </div>
      <div className="border-b border-border p-2">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            placeholder={t.history.searchPlaceholder}
            className="w-full rounded border border-input bg-background py-1.5 pl-7 pr-2 text-sm outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        <div className="flex flex-col items-center justify-center py-8 text-center text-xs text-muted-foreground">
          <p>{t.history.emptyTitle}</p>
          <p className="mt-1">{t.history.emptySubtitle}</p>
        </div>
      </div>
    </aside>
  );
}
