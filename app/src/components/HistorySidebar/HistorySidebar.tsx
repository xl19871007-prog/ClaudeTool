import { Plus, Search, RotateCw, GitBranch } from 'lucide-react';
import { useHistory, filterSessions, formatRelative, sessionTitle } from '@/store/history';
import { useWorkbench } from '@/store/workbench';
import { t } from '@/i18n/zh-CN';

export function HistorySidebar() {
  const cwd = useWorkbench((s) => s.cwd);
  const startNew = useWorkbench((s) => s.startNew);
  const startResume = useWorkbench((s) => s.startResume);
  const startFork = useWorkbench((s) => s.startFork);
  const startContinue = useWorkbench((s) => s.startContinue);

  const sessions = useHistory((s) => s.sessions);
  const loading = useHistory((s) => s.loading);
  const query = useHistory((s) => s.query);
  const setQuery = useHistory((s) => s.setQuery);

  const filtered = filterSessions(sessions, query);
  const hasSessions = sessions.length > 0;

  return (
    <aside className="flex w-60 flex-col border-r border-border bg-card">
      <div className="space-y-1 border-b border-border p-2">
        <button
          onClick={startNew}
          disabled={!cwd}
          className="flex w-full items-center justify-center gap-1.5 rounded bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:opacity-90 disabled:opacity-50"
        >
          <Plus className="h-4 w-4" />
          {t.history.newSession}
        </button>
        {hasSessions && (
          <button
            onClick={startContinue}
            className="flex w-full items-center justify-center gap-1.5 rounded border border-border px-3 py-1.5 text-xs hover:bg-muted"
            title={t.history.continueLastTip}
          >
            <RotateCw className="h-3 w-3" />
            {t.history.continueLast}
          </button>
        )}
      </div>

      {hasSessions && (
        <div className="border-b border-border p-2">
          <div className="relative">
            <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t.history.searchPlaceholder}
              className="w-full rounded border border-input bg-background py-1.5 pl-7 pr-2 text-sm outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
        </div>
      )}

      <div className="flex-1 overflow-y-auto p-2">
        {!cwd && <EmptyState title={t.history.emptyTitle} subtitle={t.history.emptySubtitle} />}
        {cwd && loading && (
          <div className="py-6 text-center text-xs text-muted-foreground">{t.history.loading}</div>
        )}
        {cwd && !loading && !hasSessions && (
          <EmptyState title={t.history.noSessionsTitle} subtitle={t.history.noSessionsSubtitle} />
        )}
        {cwd && !loading && hasSessions && filtered.length === 0 && (
          <div className="py-6 text-center text-xs text-muted-foreground">{t.history.noMatch}</div>
        )}
        {cwd && !loading && (
          <ul className="space-y-1">
            {filtered.map((session) => (
              <li key={session.id}>
                <div className="group rounded p-2 text-left hover:bg-muted">
                  <button
                    onClick={() => startResume(session.id)}
                    className="block w-full text-left"
                    title={session.firstPrompt}
                  >
                    <p className="line-clamp-2 text-sm leading-snug">{sessionTitle(session)}</p>
                    <p className="mt-1 text-[11px] text-muted-foreground">
                      {formatRelative(session.updatedAtUnix)}
                      {session.turnCount > 0 && (
                        <>
                          <span className="mx-1">·</span>
                          {session.turnCount} {t.history.turns}
                        </>
                      )}
                    </p>
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      startFork(session.id);
                    }}
                    className="mt-1 hidden items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground group-hover:flex"
                    title={t.history.forkTip}
                  >
                    <GitBranch className="h-3 w-3" />
                    {t.history.fork}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </aside>
  );
}

function EmptyState({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-8 text-center text-xs text-muted-foreground">
      <p>{title}</p>
      <p className="mt-1">{subtitle}</p>
    </div>
  );
}
