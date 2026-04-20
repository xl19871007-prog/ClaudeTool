import { useState } from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Modal } from '@/components/ui/Modal';
import { useEnv } from '@/store/env';
import { setSuppressLoginPrompt } from '@/ipc/env';
import { t } from '@/i18n/zh-CN';

const CLAUDE_AI_URL = 'https://claude.ai/login';

export function LoginPromptDialog() {
  const report = useEnv((s) => s.report);
  const config = useEnv((s) => s.config);
  const loadConfig = useEnv((s) => s.loadConfig);
  const [dismissed, setDismissed] = useState(false);
  const [dontAskAgain, setDontAskAgain] = useState(false);

  const claudeInstalled = report?.claude.kind === 'installed';
  const notLoggedIn = report?.auth.kind === 'notLoggedIn';
  const suppressed = config?.suppressLoginPrompt ?? false;
  const shouldShow = !dismissed && !suppressed && claudeInstalled && notLoggedIn;

  if (!shouldShow) return null;

  const handleDismiss = async () => {
    if (dontAskAgain) {
      try {
        await setSuppressLoginPrompt(true);
        await loadConfig();
      } catch (err) {
        console.error('set_suppress_login_prompt failed', err);
      }
    }
    setDismissed(true);
  };

  const handleOpenClaudeAi = async () => {
    try {
      await openUrl(CLAUDE_AI_URL);
    } catch (err) {
      console.error('open url failed', err);
    }
  };

  return (
    <Modal open={true} onClose={handleDismiss} ariaLabel={t.login.title}>
      <h2 className="text-lg font-semibold">{t.login.title}</h2>
      <p className="mt-2 text-sm text-muted-foreground">{t.login.message}</p>

      <label className="mt-4 flex items-center gap-2 text-xs text-muted-foreground">
        <input
          type="checkbox"
          checked={dontAskAgain}
          onChange={(e) => setDontAskAgain(e.target.checked)}
        />
        {t.login.dontAskAgain}
      </label>

      <div className="mt-4 flex gap-2">
        <button
          onClick={handleOpenClaudeAi}
          className="rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90"
        >
          {t.login.openClaudeAi}
        </button>
        <button
          onClick={handleDismiss}
          className="rounded border border-border px-3 py-1.5 text-sm hover:bg-muted"
        >
          {t.login.alreadyLoggedIn}
        </button>
      </div>
    </Modal>
  );
}
