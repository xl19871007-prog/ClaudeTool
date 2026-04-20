import { useEffect, useState } from 'react';
import { Copy, ExternalLink, ChevronLeft } from 'lucide-react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { Drawer } from '@/components/ui/Drawer';
import { usePanels } from '@/store/panels';
import { useSkills } from '@/store/skills';
import { useWorkbench } from '@/store/workbench';
import type { SkillMeta, SkillSource } from '@/ipc/skills';

const SOURCE_LABEL: Record<SkillSource, string> = {
  user: '用户级',
  project: '项目级',
  plugin: '插件',
  recommend: '推荐',
};

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

  useEffect(() => {
    if (open) {
      void load(cwd);
    }
  }, [open, cwd, load]);

  const installed = report?.installed ?? [];
  const recommended = report?.recommended ?? [];

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
      <div className="p-3">
        {loading && <p className="py-8 text-center text-xs text-muted-foreground">正在扫描...</p>}
        {!loading && tab === 'installed' && installed.length === 0 && (
          <p className="py-8 text-center text-xs text-muted-foreground">
            本机还没有装任何 Skill
            <br />
            可在「推荐」Tab 看官方仓库的 Skill 列表
          </p>
        )}
        {!loading && tab === 'recommend' && recommended.length === 0 && (
          <p className="py-8 text-center text-xs text-muted-foreground">
            推荐列表为空（你已装完所有种子里的 Skill）
          </p>
        )}
        {!loading && tab === 'installed' && installed.length > 0 && (
          <ul className="space-y-2">
            {installed.map((s) => (
              <SkillCard key={s.id} skill={s} onClick={() => void select(s)} />
            ))}
          </ul>
        )}
        {!loading && tab === 'recommend' && recommended.length > 0 && (
          <ul className="space-y-2">
            {recommended.map((s) => (
              <SkillCard key={s.id} skill={s} onClick={() => void select(s)} />
            ))}
          </ul>
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
  return (
    <li>
      <button
        onClick={onClick}
        className="block w-full rounded border border-border bg-background p-3 text-left hover:bg-muted"
      >
        <div className="flex items-start justify-between gap-2">
          <p className="text-sm font-medium">{skill.name}</p>
          <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 text-[10px] text-muted-foreground">
            {SOURCE_LABEL[skill.source]}
            {skill.pluginName ? ` · ${skill.pluginName}` : ''}
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
  const handleCopyInstall = async () => {
    if (!skill.repoPath) return;
    const cmd = `claude plugin marketplace add anthropics/skills && claude plugin install ${skill.name}@anthropics/skills`;
    try {
      await writeText(cmd);
    } catch (err) {
      console.error('clipboard write failed', err);
    }
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

      {!skill.installed && skill.repoPath && (
        <div className="rounded border border-border bg-background p-3">
          <p className="text-xs font-medium">如何安装</p>
          <code className="mt-2 block whitespace-pre-wrap break-all rounded bg-muted p-2 font-mono text-[11px]">
            claude plugin marketplace add anthropics/skills{'\n'}claude plugin install {skill.name}
            @anthropics/skills
          </code>
          <button
            onClick={handleCopyInstall}
            className="mt-2 flex items-center gap-1 rounded bg-primary px-2 py-1 text-[11px] text-primary-foreground hover:opacity-90"
          >
            <Copy className="h-3 w-3" />
            复制安装命令
          </button>
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
