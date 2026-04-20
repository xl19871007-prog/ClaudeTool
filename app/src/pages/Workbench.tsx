import { TopBar } from '@/components/layout/TopBar';
import { HistorySidebar } from '@/components/HistorySidebar/HistorySidebar';
import { Terminal } from '@/components/Terminal/Terminal';
import { TerminalPlaceholder } from '@/components/Terminal/TerminalPlaceholder';
import { useWorkbench } from '@/store/workbench';

export function Workbench() {
  const cwd = useWorkbench((s) => s.cwd);

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <TopBar />
      <div className="flex flex-1 overflow-hidden">
        <HistorySidebar />
        <main className="flex-1 overflow-hidden">
          {cwd ? <Terminal cwd={cwd} /> : <TerminalPlaceholder />}
        </main>
      </div>
    </div>
  );
}
