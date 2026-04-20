import { useEffect, useRef, useState } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { spawn, type IPty } from 'tauri-pty';
import '@xterm/xterm/css/xterm.css';
import { useTerminalInput } from '@/store/terminalInput';
import { useEnv } from '@/store/env';
import { proxyToEnv } from '@/ipc/env';
import { t } from '@/i18n/zh-CN';

interface TerminalProps {
  cwd: string;
  args: string[];
  /** Bumped externally to remount/restart this Terminal */
  epoch: number;
}

export function Terminal({ cwd, args, epoch }: TerminalProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const ptyRef = useRef<IPty | null>(null);
  const termRef = useRef<XTerm | null>(null);
  const [exitInfo, setExitInfo] = useState<{ code: number } | null>(null);
  const [restartKey, setRestartKey] = useState(0);

  const pendingInput = useTerminalInput((s) => s.pending);
  const consumeInput = useTerminalInput((s) => s.consume);
  const proxy = useEnv((s) => s.config?.proxy ?? null);

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
    termRef.current = term;

    let pty: IPty | null = null;
    let resizeTimer: ReturnType<typeof setTimeout> | null = null;
    let disposed = false;

    try {
      // ADR-018: inject user-configured proxy env vars so Claude CLI can
      // reach Anthropic on systems where the OS VPN is sysproxy-only.
      const proxyEnv = proxyToEnv(proxy);
      pty = spawn('claude', args, {
        cwd,
        cols: term.cols,
        rows: term.rows,
        env: Object.keys(proxyEnv).length > 0 ? proxyEnv : undefined,
      });
      ptyRef.current = pty;

      pty.onData((data) => {
        term.write(data);
      });

      term.onData((data) => {
        pty?.write(data);
      });

      pty.onExit(({ exitCode }) => {
        if (!disposed) {
          setExitInfo({ code: exitCode });
          ptyRef.current = null;
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
      ptyRef.current = null;
      termRef.current = null;
      term.dispose();
    };
    // proxy intentionally excluded from deps: changing it should not auto-restart
    // an active claude session; user can hit "重启" or change folder if they want
    // the new proxy to take effect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cwd, args, epoch, restartKey]);

  // Consume injected input (from CommandPanel "试一试").
  // Writes to PTY without trailing newline, lets user verify before pressing Enter.
  useEffect(() => {
    if (!pendingInput) return;
    if (!ptyRef.current) {
      consumeInput();
      return;
    }
    try {
      ptyRef.current.write(pendingInput.text);
      termRef.current?.focus();
    } catch (err) {
      console.warn('PTY inject failed', err);
    }
    consumeInput();
  }, [pendingInput, consumeInput]);

  return (
    <div className="relative h-full w-full bg-zinc-900">
      <div ref={containerRef} className="h-full w-full p-2" />
      {exitInfo && (
        <div className="absolute inset-0 z-10 flex items-center justify-center bg-zinc-900/95 text-zinc-400">
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
      )}
    </div>
  );
}
