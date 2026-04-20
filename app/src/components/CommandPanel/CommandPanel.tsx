import { useMemo, useState } from 'react';
import { Search, Play } from 'lucide-react';
import { Drawer } from '@/components/ui/Drawer';
import { usePanels } from '@/store/panels';
import { useTerminalInput } from '@/store/terminalInput';
import { useWorkbench } from '@/store/workbench';
import commandsData from '@/data/commands.zh-CN.json';

interface CommandEntry {
  id: string;
  category: string;
  command: string;
  purpose: string;
  example: string;
  tip: string;
}

interface CommandsFile {
  version: number;
  recordedAt: string;
  categories: Record<string, string>;
  commands: CommandEntry[];
}

const data = commandsData as CommandsFile;
const ALL_CATEGORIES = Object.entries(data.categories);

export function CommandPanel() {
  const open = usePanels((s) => s.open === 'commands');
  const close = usePanels((s) => s.close);
  const inject = useTerminalInput((s) => s.inject);
  const cwd = useWorkbench((s) => s.cwd);

  const [query, setQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<string | 'all'>('all');

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return data.commands.filter((c) => {
      if (activeCategory !== 'all' && c.category !== activeCategory) return false;
      if (!q) return true;
      const haystack = `${c.command} ${c.purpose} ${c.tip} ${c.example}`.toLowerCase();
      return haystack.includes(q);
    });
  }, [query, activeCategory]);

  const handleTry = (cmd: string) => {
    inject(cmd);
    close();
  };

  return (
    <Drawer open={open} onClose={close} title="命令大全" widthClass="w-[520px]">
      <div className="space-y-2 border-b border-border p-3">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜命令、关键词..."
            className="w-full rounded border border-input bg-background py-1.5 pl-7 pr-2 text-sm outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
        <div className="flex flex-wrap gap-1">
          <CategoryChip active={activeCategory === 'all'} onClick={() => setActiveCategory('all')}>
            全部 ({data.commands.length})
          </CategoryChip>
          {ALL_CATEGORIES.map(([id, label]) => {
            const count = data.commands.filter((c) => c.category === id).length;
            return (
              <CategoryChip
                key={id}
                active={activeCategory === id}
                onClick={() => setActiveCategory(id)}
              >
                {label} ({count})
              </CategoryChip>
            );
          })}
        </div>
      </div>

      <div className="p-3">
        {filtered.length === 0 ? (
          <p className="py-8 text-center text-xs text-muted-foreground">没有匹配的命令</p>
        ) : (
          <ul className="space-y-3">
            {filtered.map((c) => (
              <li key={c.id} className="rounded border border-border bg-background p-3 text-sm">
                <div className="flex items-start justify-between gap-2">
                  <code className="font-mono text-[13px] text-foreground break-all">
                    {c.command}
                  </code>
                  <button
                    onClick={() => handleTry(c.command)}
                    disabled={!cwd}
                    className="flex shrink-0 items-center gap-1 rounded bg-primary px-2 py-1 text-[11px] text-primary-foreground hover:opacity-90 disabled:opacity-40"
                    title={cwd ? '把命令写入终端（不自动回车）' : '需要先选个文件夹打开终端'}
                  >
                    <Play className="h-3 w-3" />
                    试一试
                  </button>
                </div>
                <p className="mt-2 text-xs">{c.purpose}</p>
                <p className="mt-1 text-[11px] text-muted-foreground">
                  示例：<code className="font-mono">{c.example}</code>
                </p>
                <p className="mt-1 text-[11px] text-muted-foreground">💡 {c.tip}</p>
              </li>
            ))}
          </ul>
        )}
        <p className="pt-4 text-center text-[10px] text-muted-foreground">
          数据更新于 {data.recordedAt}
        </p>
      </div>
    </Drawer>
  );
}

function CategoryChip({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`rounded-full px-2.5 py-0.5 text-[11px] ${
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-muted text-muted-foreground hover:bg-muted/80'
      }`}
    >
      {children}
    </button>
  );
}
