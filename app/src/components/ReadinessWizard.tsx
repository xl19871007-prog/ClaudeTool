import { openUrl } from '@tauri-apps/plugin-opener';
import { Modal } from '@/components/ui/Modal';
import { useEnv } from '@/store/env';
import { t } from '@/i18n/zh-CN';

const CLAUDE_DOWNLOAD_URL = 'https://claude.com/code';

export function ReadinessWizard() {
  const report = useEnv((s) => s.report);
  const loading = useEnv((s) => s.loading);
  const refresh = useEnv((s) => s.refresh);

  if (!report) return null;

  const claudeOk = report.claude.kind === 'installed';
  const networkOk = report.network.kind === 'ok' || report.network.kind === 'slow';
  if (claudeOk && networkOk) return null;

  const handleManualDownload = async () => {
    try {
      await openUrl(CLAUDE_DOWNLOAD_URL);
    } catch (err) {
      console.error('open url failed', err);
    }
  };

  return (
    <Modal
      open={true}
      onClose={() => {}}
      closeOnBackdropClick={false}
      ariaLabel={t.ready.wizardTitle}
    >
      <h2 className="text-lg font-semibold">{t.ready.wizardTitle}</h2>
      <p className="mt-1 text-sm text-muted-foreground">{t.ready.problemsDetected}</p>

      {!claudeOk && (
        <div className="mt-4 rounded border border-border p-3">
          <p className="text-sm font-medium text-destructive">{t.ready.claudeNotInstalled}</p>
          <p className="mt-1 text-xs text-muted-foreground">{t.ready.installNote}</p>
          <div className="mt-2 flex gap-2">
            <button
              onClick={handleManualDownload}
              className="rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90"
            >
              {t.ready.manualDownload}
            </button>
            <button
              onClick={() => void refresh()}
              disabled={loading}
              className="rounded border border-border px-3 py-1.5 text-sm hover:bg-muted disabled:opacity-50"
            >
              {t.ready.recheck}
            </button>
          </div>
        </div>
      )}

      {!networkOk && (
        <div className="mt-4 rounded border border-border p-3">
          <p className="text-sm font-medium text-warning">{t.ready.networkUnreachable}</p>
          <p className="mt-1 text-xs text-muted-foreground">{t.ready.networkNote}</p>
          <div className="mt-2 flex gap-2">
            <button
              onClick={() => void refresh()}
              disabled={loading}
              className="rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90 disabled:opacity-50"
            >
              {t.ready.retry}
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
