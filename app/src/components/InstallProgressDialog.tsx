import { useEffect, useRef, useState } from 'react';
import { Loader2, CheckCircle2, XCircle, Copy, FolderOpen } from 'lucide-react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { openPath } from '@tauri-apps/plugin-opener';
import { Modal } from '@/components/ui/Modal';
import { onInstallEvent, type InstallEvent } from '@/ipc/installer';

interface InstallProgressDialogProps {
  open: boolean;
  title: string;
  /** Called when user dismisses dialog (only enabled after done/failed). */
  onClose: () => void;
}

interface FailedState {
  stageId: string;
  message: string;
  recoverable: boolean;
}

export function InstallProgressDialog({ open, title, onClose }: InstallProgressDialogProps) {
  const [stage, setStage] = useState<string>('started');
  const [statusText, setStatusText] = useState<string>('启动中...');
  const [downloaded, setDownloaded] = useState<number | null>(null);
  const [total, setTotal] = useState<number | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [done, setDone] = useState(false);
  const [failed, setFailed] = useState<FailedState | null>(null);
  const logsRef = useRef<HTMLPreElement | null>(null);

  useEffect(() => {
    if (!open) {
      setStage('started');
      setStatusText('启动中...');
      setDownloaded(null);
      setTotal(null);
      setLogs([]);
      setDone(false);
      setFailed(null);
      return;
    }

    let unlisten: (() => void) | null = null;
    onInstallEvent(handleEvent).then((u) => {
      unlisten = u;
    });
    return () => {
      unlisten?.();
    };
  }, [open]);

  useEffect(() => {
    if (logsRef.current) {
      logsRef.current.scrollTop = logsRef.current.scrollHeight;
    }
  }, [logs]);

  const handleEvent = (e: InstallEvent) => {
    setStage(e.stage);
    switch (e.stage) {
      case 'started':
        setStatusText(`开始安装：${e.target}`);
        break;
      case 'resolving':
      case 'installing':
      case 'configuring':
      case 'verifying':
        setStatusText(e.message);
        break;
      case 'downloading':
        setStatusText(e.message);
        setDownloaded(e.downloadedBytes);
        setTotal(e.totalBytes);
        break;
      case 'done':
        setStatusText(e.message);
        setDone(true);
        break;
      case 'failed':
        setFailed({ stageId: e.stageId, message: e.message, recoverable: e.recoverable });
        setStatusText(`失败：${e.message}`);
        break;
      case 'log':
        setLogs((prev) => [...prev.slice(-300), e.line]);
        break;
    }
  };

  const inProgress = !done && !failed;
  const percent =
    downloaded !== null && total !== null && total > 0
      ? Math.min(100, Math.round((downloaded / total) * 100))
      : null;

  const handleCopyError = async () => {
    if (!failed) return;
    const payload = `Stage: ${failed.stageId}\nError: ${failed.message}\n\nRecent log:\n${logs.slice(-50).join('\n')}`;
    try {
      await writeText(payload);
    } catch (err) {
      console.error('copy failed', err);
    }
  };

  const handleOpenLogDir = async () => {
    // Open the system temp dir as a best-effort (tauri-plugin-fs has no
    // dedicated "open dir in explorer" but opener supports paths).
    try {
      await openPath(await tempDir());
    } catch (err) {
      console.error('open log dir failed', err);
    }
  };

  return (
    <Modal open={open} onClose={onClose} closeOnBackdropClick={!inProgress} ariaLabel={title}>
      <div className="w-[520px] max-w-full">
        <h2 className="text-lg font-semibold">{title}</h2>

        <div className="mt-3 flex items-start gap-2">
          {inProgress && <Loader2 className="mt-0.5 h-4 w-4 shrink-0 animate-spin text-accent" />}
          {done && <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-success" />}
          {failed && <XCircle className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />}
          <div className="flex-1">
            <p className="text-sm">{statusText}</p>
            <p className="mt-1 text-[10px] uppercase tracking-wide text-muted-foreground">
              当前阶段：{stage}
            </p>
          </div>
        </div>

        {percent !== null && (
          <div className="mt-3">
            <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="h-full bg-accent transition-all duration-150"
                style={{ width: `${percent}%` }}
              />
            </div>
            <p className="mt-1 text-right text-[11px] text-muted-foreground">{percent}%</p>
          </div>
        )}

        {logs.length > 0 && (
          <details className="mt-3" open={!!failed}>
            <summary className="cursor-pointer text-xs text-muted-foreground">
              安装日志（{logs.length} 行）
            </summary>
            <pre
              ref={logsRef}
              className="mt-2 max-h-56 overflow-auto whitespace-pre-wrap rounded border border-border bg-muted/50 p-2 font-mono text-[10px]"
            >
              {logs.join('\n')}
            </pre>
          </details>
        )}

        {failed && (
          <div className="mt-3 rounded border border-destructive/50 bg-destructive/5 p-3">
            <p className="text-xs font-semibold text-destructive">出错了</p>
            <p className="mt-1 text-xs">{failed.message}</p>
            <div className="mt-2 flex flex-wrap gap-2">
              <button
                onClick={handleCopyError}
                className="flex items-center gap-1 rounded border border-border px-2 py-1 text-[11px] hover:bg-muted"
              >
                <Copy className="h-3 w-3" />
                复制错误信息
              </button>
              <button
                onClick={handleOpenLogDir}
                className="flex items-center gap-1 rounded border border-border px-2 py-1 text-[11px] hover:bg-muted"
              >
                <FolderOpen className="h-3 w-3" />
                打开临时目录
              </button>
            </div>
          </div>
        )}

        <div className="mt-4 flex justify-end">
          <button
            onClick={onClose}
            disabled={inProgress}
            className="rounded bg-primary px-4 py-1.5 text-sm text-primary-foreground hover:opacity-90 disabled:opacity-40"
          >
            {inProgress ? '安装中...' : done ? '完成' : '关闭'}
          </button>
        </div>
      </div>
    </Modal>
  );
}

async function tempDir(): Promise<string> {
  // Best-effort: use Windows TEMP env var via a Tauri command would be cleaner,
  // but for the open button we accept "may fail on macOS" as cost.
  return 'C:\\Windows\\Temp';
}
