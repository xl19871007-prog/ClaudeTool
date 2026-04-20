import { useEffect, useMemo, useState } from 'react';
import { Copy, ExternalLink, ChevronLeft, Check, Play, Search } from 'lucide-react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { Drawer } from '@/components/ui/Drawer';
import { usePanels } from '@/store/panels';
import { useSkills } from '@/store/skills';
import { useWorkbench } from '@/store/workbench';
import { useTerminalInput } from '@/store/terminalInput';
import type { SkillMeta, SkillSource } from '@/ipc/skills';

const SOURCE_LABEL: Record<SkillSource, string> = {
  user: '用户级',
  project: '项目级',
  plugin: '来自插件',
  recommend: '推荐插件',
};

interface InstallSteps {
  marketplaceAdd: string;
  pluginInstall: string;
}

/**
 * Build the two-step slash commands to install a recommended plugin
 * inside a Claude Code session. Each command must be sent separately
 * (slash commands cannot be chained with `&&`). The `marketplace add`
 * step is idempotent — re-running on an already-registered marketplace
 * just prints "already on disk" with no side effects.
 */
function buildInstallSteps(skill: SkillMeta): InstallSteps | null {
  if (!skill.marketplaceId || !skill.marketplaceAddArg) return null;
  return {
    marketplaceAdd: `/plugin marketplace add ${skill.marketplaceAddArg}`,
    pluginInstall: `/plugin install ${skill.name}@${skill.marketplaceId}`,
  };
}

export function SkillsPanel() {
  const open = usePanels((s) => s.open === 'skills');
  const close = usePanels((s) => s.close);
  const cwd = useWorkbench((s) => s.cwd);

  const report = useSkills((s) => s.report);
  const loading = useSkills((s) => s.loading);
  const load = useSkills((s) => s.load);
  const selected = useSkills((s) => s.selected);
  const selectedMd = useSkills((s) => s.selectedMd);
  const loadingMd = useSkills((s) => s.loadingMd);
  const select = useSkills((s) => s.select);

  const [tab, setTab] = useState<'installed' | 'recommend'>('installed');
  const [query, setQuery] = useState('');

  useEffect(() => {
    if (open) {
      void load(cwd);
    }
  }, [open, cwd, load]);

  const installed = report?.installed ?? [];
  const recommended = report?.recommended ?? [];

  const filterSkills = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return (list: SkillMeta[]) => list;
    return (list: SkillMeta[]) =>
      list.filter((s) => {
        const bundled = (s.bundledSkills ?? [])
          .map((b) => `${b.name} ${b.descriptionZh}`)
          .join(' ');
        const haystack =
          `${s.name} ${s.description} ${s.pluginName ?? ''} ${s.category ?? ''} ${s.marketplaceOwnerLabel ?? ''} ${bundled}`.toLowerCase();
        return haystack.includes(q);
      });
  }, [query]);

  const filteredInstalled = filterSkills(installed);
  const filteredRecommended = filterSkills(recommended);

  if (selected) {
    return (
      <Drawer open={open} onClose={close} title={selected.name} widthClass="w-[560px]">
        <div className="border-b border-border p-3">
          <button
            onClick={() => void select(null)}
            className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <ChevronLeft className="h-3 w-3" />
            返回列表
          </button>
        </div>
        <div className="p-4">
          <SkillDetail skill={selected} md={selectedMd} loadingMd={loadingMd} />
        </div>
      </Drawer>
    );
  }

  const activeList = tab === 'installed' ? filteredInstalled : filteredRecommended;
  const totalCount = tab === 'installed' ? installed.length : recommended.length;

  return (
    <Drawer open={open} onClose={close} title="Skills" widthClass="w-[520px]">
      <div className="flex border-b border-border">
        <TabBtn active={tab === 'installed'} onClick={() => setTab('installed')}>
          已装 ({installed.length})
        </TabBtn>
        <TabBtn active={tab === 'recommend'} onClick={() => setTab('recommend')}>
          推荐 ({recommended.length})
        </TabBtn>
      </div>
      <div className="border-b border-border p-3">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索：plugin 名 / 描述 / bundled skill..."
            className="w-full rounded border border-input bg-background py-1.5 pl-7 pr-2 text-sm outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
        {query && (
          <p className="mt-1 text-[10px] text-muted-foreground">
            匹配 {activeList.length} / {totalCount} 条
          </p>
        )}
      </div>
      <div className="p-3">
        {loading && <p className="py-8 text-center text-xs text-muted-foreground">正在扫描...</p>}
        {!loading && tab === 'installed' && installed.length === 0 && (
          <p className="py-8 text-center text-xs text-muted-foreground">
            本机还没有装任何 Skill
            <br />
            可在「推荐」Tab 看官方仓库的 plugin 列表
          </p>
        )}
        {!loading && tab === 'recommend' && recommended.length === 0 && (
          <p className="py-8 text-center text-xs text-muted-foreground">
            推荐列表为空（你已装完所有种子里的 plugin）
          </p>
        )}
        {!loading && totalCount > 0 && activeList.length === 0 && (
          <p className="py-8 text-center text-xs text-muted-foreground">没有匹配的 skill</p>
        )}
        {!loading && tab === 'installed' && activeList.length > 0 && (
          <ul className="space-y-2">
            {activeList.map((s) => (
              <SkillCard key={s.id} skill={s} onClick={() => void select(s)} />
            ))}
          </ul>
        )}
        {!loading && tab === 'recommend' && activeList.length > 0 && (
          <>
            <p className="mb-2 text-[11px] text-muted-foreground">
              来自 Anthropic 官方 marketplace。每个 plugin 含一个或多个 skill。
            </p>
            <ul className="space-y-2">
              {activeList.map((s) => (
                <SkillCard key={s.id} skill={s} onClick={() => void select(s)} />
              ))}
            </ul>
          </>
        )}
      </div>
    </Drawer>
  );
}

function TabBtn({
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
      className={`flex-1 px-3 py-2 text-xs ${
        active
          ? 'border-b-2 border-primary font-medium'
          : 'text-muted-foreground hover:text-foreground'
      }`}
    >
      {children}
    </button>
  );
}

function SkillCard({ skill, onClick }: { skill: SkillMeta; onClick: () => void }) {
  const bundledCount = skill.bundledSkills?.length ?? 0;
  const tagParts = [
    SOURCE_LABEL[skill.source],
    skill.pluginName,
    skill.marketplaceOwnerLabel,
    bundledCount > 0 ? `${bundledCount} 个 skill` : null,
    skill.category,
  ].filter(Boolean);
  return (
    <li>
      <button
        onClick={onClick}
        className="block w-full rounded border border-border bg-background p-3 text-left hover:bg-muted"
      >
        <div className="flex items-start justify-between gap-2">
          <p className="text-sm font-medium">{skill.name}</p>
          <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 text-[10px] text-muted-foreground">
            {tagParts.join(' · ')}
          </span>
        </div>
        <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{skill.description}</p>
      </button>
    </li>
  );
}

function SkillDetail({
  skill,
  md,
  loadingMd,
}: {
  skill: SkillMeta;
  md: string | null;
  loadingMd: boolean;
}) {
  const cwd = useWorkbench((s) => s.cwd);
  const inject = useTerminalInput((s) => s.inject);
  const closePanel = usePanels((s) => s.close);
  const [copied, setCopied] = useState(false);

  const steps = buildInstallSteps(skill);

  const handleCopyAll = async () => {
    if (!steps) return;
    try {
      await writeText(`${steps.marketplaceAdd}\n${steps.pluginInstall}`);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('clipboard write failed', err);
    }
  };

  const injectStep = (cmd: string) => {
    inject(cmd);
    closePanel();
  };

  return (
    <div className="space-y-3 text-sm">
      <div>
        <p className="text-base font-semibold">{skill.name}</p>
        <p className="mt-1 text-xs text-muted-foreground">
          {SOURCE_LABEL[skill.source]}
          {skill.pluginName ? ` · ${skill.pluginName}` : ''}
          {skill.category ? ` · ${skill.category}` : ''}
        </p>
      </div>
      <p className="text-sm">{skill.description}</p>

      {skill.bundledSkills && skill.bundledSkills.length > 0 && (
        <div className="rounded border border-border bg-background p-3">
          <p className="text-xs font-medium">
            该 plugin 包含的 skill ({skill.bundledSkills.length})
          </p>
          <ul className="mt-2 space-y-1.5">
            {skill.bundledSkills.map((bs) => (
              <li key={bs.name} className="flex items-start gap-2 text-[11px]">
                <code className="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-foreground">
                  {bs.name}
                </code>
                <span className="text-muted-foreground">{bs.descriptionZh}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {!skill.installed && steps && (
        <div className="rounded border border-border bg-background p-3">
          <p className="text-xs font-medium">如何安装（在 Claude 会话里分两步执行）</p>

          <div className="mt-2 space-y-1">
            <p className="text-[11px] text-muted-foreground">第 1 步：注册 marketplace</p>
            <code className="block whitespace-pre-wrap break-all rounded bg-muted p-2 font-mono text-[11px]">
              {steps.marketplaceAdd}
            </code>
            <button
              onClick={() => injectStep(steps.marketplaceAdd)}
              disabled={!cwd}
              className="mt-1 flex items-center gap-1 rounded bg-primary px-2 py-1 text-[11px] text-primary-foreground hover:opacity-90 disabled:opacity-40"
              title={cwd ? '写入终端，按回车执行' : '需要先选个文件夹打开终端'}
            >
              <Play className="h-3 w-3" />
              在终端里执行第 1 步
            </button>
          </div>

          <div className="mt-3 space-y-1">
            <p className="text-[11px] text-muted-foreground">第 2 步：安装 plugin</p>
            <code className="block whitespace-pre-wrap break-all rounded bg-muted p-2 font-mono text-[11px]">
              {steps.pluginInstall}
            </code>
            <button
              onClick={() => injectStep(steps.pluginInstall)}
              disabled={!cwd}
              className="mt-1 flex items-center gap-1 rounded bg-primary px-2 py-1 text-[11px] text-primary-foreground hover:opacity-90 disabled:opacity-40"
              title={cwd ? '写入终端，按回车执行' : '需要先选个文件夹打开终端'}
            >
              <Play className="h-3 w-3" />
              在终端里执行第 2 步
            </button>
          </div>

          <div className="mt-3 border-t border-border pt-2">
            <button
              onClick={handleCopyAll}
              className="flex items-center gap-1 rounded border border-border px-2 py-1 text-[11px] hover:bg-muted"
            >
              {copied ? (
                <>
                  <Check className="h-3 w-3 text-success" />
                  已复制两步命令
                </>
              ) : (
                <>
                  <Copy className="h-3 w-3" />
                  复制两步命令
                </>
              )}
            </button>
          </div>

          <p className="mt-2 text-[11px] text-muted-foreground">
            提示：上面是 <code className="font-mono">/plugin</code> 形式的 slash 命令， 可在 Claude
            会话里直接执行；不要拼成一行。Windows 用户还需先装好 Git for Windows 并设{' '}
            <code className="mx-1 font-mono">CLAUDE_CODE_GIT_BASH_PATH</code>
            指向 bash.exe（M4 起本工具会自动处理）。
          </p>
        </div>
      )}

      {skill.installed && (
        <div className="rounded border border-border bg-background p-3">
          <div className="flex items-center justify-between">
            <p className="text-xs font-medium">SKILL.md</p>
            <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
              <ExternalLink className="h-3 w-3" />
              <span className="font-mono">{skill.path}</span>
            </span>
          </div>
          {loadingMd ? (
            <p className="mt-2 text-xs text-muted-foreground">读取中...</p>
          ) : md ? (
            <pre className="mt-2 max-h-96 overflow-auto whitespace-pre-wrap rounded bg-muted p-2 font-mono text-[11px]">
              {md}
            </pre>
          ) : (
            <p className="mt-2 text-xs text-muted-foreground">无法读取 SKILL.md</p>
          )}
        </div>
      )}
    </div>
  );
}
