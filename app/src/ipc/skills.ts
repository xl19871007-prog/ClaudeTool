import { invoke } from '@tauri-apps/api/core';

export type SkillSource = 'user' | 'project' | 'plugin' | 'recommend';

export interface BundledSkillView {
  name: string;
  descriptionZh: string;
}

export interface SkillMeta {
  id: string;
  name: string;
  description: string;
  source: SkillSource;
  pluginName: string | null;
  path: string;
  installed: boolean;
  category: string | null;
  /** For recommended plugins: the skills bundled inside. Null for standalone installed skills. */
  bundledSkills: BundledSkillView[] | null;
  /** For recommended plugins: marketplace registry id (e.g. "anthropic-agent-skills"). */
  marketplaceId: string | null;
  /** For recommended plugins: arg for `claude plugin marketplace add <arg>` (e.g. "anthropics/skills"). */
  marketplaceAddArg: string | null;
  /** For recommended plugins: e.g. "Anthropic 官方". */
  marketplaceOwnerLabel: string | null;
}

export interface SkillsReport {
  installed: SkillMeta[];
  recommended: SkillMeta[];
}

export const listSkills = (workdir: string | null) =>
  invoke<SkillsReport>('list_skills', { workdir });

export const readSkillMd = (path: string) => invoke<string>('read_skill_md', { path });
