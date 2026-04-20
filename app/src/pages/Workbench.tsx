import { TopBar } from '@/components/layout/TopBar';
import { HistorySidebar } from '@/components/HistorySidebar/HistorySidebar';
import { TerminalPlaceholder } from '@/components/Terminal/TerminalPlaceholder';

export function Workbench() {
  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <TopBar />
      <div className="flex flex-1 overflow-hidden">
        <HistorySidebar />
        <main className="flex-1 overflow-hidden">
          <TerminalPlaceholder />
        </main>
      </div>
    </div>
  );
}
