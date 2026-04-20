import { useEffect } from 'react';
import { TopBar } from '@/components/layout/TopBar';
import { HistorySidebar } from '@/components/HistorySidebar/HistorySidebar';
import { Terminal } from '@/components/Terminal/Terminal';
import { TerminalPlaceholder } from '@/components/Terminal/TerminalPlaceholder';
import { useWorkbench, buildClaudeCliArgs } from '@/store/workbench';
import { useHistory } from '@/store/history';

export function Workbench() {
  const cwd = useWorkbench((s) => s.cwd);
  const sessionEpoch = useWorkbench((s) => s.sessionEpoch);
  const claudeArgs = useWorkbench((s) => s.claudeArgs);
  const loadHistory = useHistory((s) => s.load);
  const clearHistory = useHistory((s) => s.clear);

  useEffect(() => {
    if (cwd) {
      void loadHistory(cwd);
    } else {
      clearHistory();
    }
  }, [cwd, loadHistory, clearHistory]);

  const args = buildClaudeCliArgs(claudeArgs);

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <TopBar />
      <div className="flex flex-1 overflow-hidden">
        <HistorySidebar />
        <main className="flex-1 overflow-hidden">
          {cwd ? <Terminal cwd={cwd} args={args} epoch={sessionEpoch} /> : <TerminalPlaceholder />}
        </main>
      </div>
    </div>
  );
}
