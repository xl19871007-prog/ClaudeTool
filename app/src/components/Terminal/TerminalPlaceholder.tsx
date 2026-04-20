import { t } from '@/i18n/zh-CN';

export function TerminalPlaceholder() {
  return (
    <div className="flex h-full items-center justify-center bg-zinc-900 text-zinc-400">
      <div className="text-center">
        <p className="font-mono text-sm">{t.terminal.placeholderTitle}</p>
        <p className="mt-2 text-xs">{t.terminal.placeholderSubtitle}</p>
      </div>
    </div>
  );
}
