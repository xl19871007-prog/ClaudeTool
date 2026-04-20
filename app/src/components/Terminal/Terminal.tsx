import { useEffect, useRef, useState } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { spawn, type IPty } from 'tauri-pty';
import '@xterm/xterm/css/xterm.css';
import { t } from '@/i18n/zh-CN';

interface TerminalProps {
  cwd: string;
}

export function Terminal({ cwd }: TerminalProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [exitInfo, setExitInfo] = useState<{ code: number } | null>(null);
  const [restartKey, setRestartKey] = useState(0);

  useEffect(() => {
    setExitInfo(null);
    if (!containerRef.current) return;

    const term = new XTerm({
      fontFamily: '"Cascadia Code", "JetBrains Mono", Consolas, "Courier New", monospace',
      fontSize: 14,
      theme: {
        background: '#18181b',
        foreground: '#f4f4f5',
        cursor: '#f4f4f5',
        selectionBackground: '#3f3f46',
      },
      cursorBlink: true,
      allowProposedApi: true,
      scrollback: 5000,
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());

    term.open(containerRef.current);
    fitAddon.fit();

    let pty: IPty | null = null;
    let resizeTimer: ReturnType<typeof setTimeout> | null = null;
    let disposed = false;

    try {
      pty = spawn('claude', [], {
        cwd,
        cols: term.cols,
        rows: term.rows,
      });

      pty.onData((data) => {
        term.write(data);
      });

      term.onData((data) => {
        pty?.write(data);
      });

      pty.onExit(({ exitCode }) => {
        if (!disposed) {
          setExitInfo({ code: exitCode });
        }
      });
    } catch (err) {
      term.write(`\r\n\x1b[31m启动 Claude 失败：${String(err)}\x1b[0m\r\n`);
      console.error('PTY spawn failed', err);
    }

    const handleResize = () => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        fitAddon.fit();
        if (pty) {
          try {
            pty.resize(term.cols, term.rows);
          } catch (err) {
            console.warn('PTY resize failed', err);
          }
        }
      }, 100);
    };
    window.addEventListener('resize', handleResize);

    return () => {
      disposed = true;
      window.removeEventListener('resize', handleResize);
      if (resizeTimer) clearTimeout(resizeTimer);
      try {
        pty?.kill();
      } catch {
        // ignore
      }
      term.dispose();
    };
  }, [cwd, restartKey]);

  if (exitInfo) {
    return (
      <div className="flex h-full items-center justify-center bg-zinc-900 text-zinc-400">
        <div className="text-center">
          <p className="font-mono text-sm">
            {t.terminal.exitedTitle}（exit code: {exitInfo.code}）
          </p>
          <button
            onClick={() => setRestartKey((k) => k + 1)}
            className="mt-3 rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90"
          >
            {t.terminal.restart}
          </button>
        </div>
      </div>
    );
  }

  return <div ref={containerRef} className="h-full w-full bg-zinc-900 p-2" />;
}
