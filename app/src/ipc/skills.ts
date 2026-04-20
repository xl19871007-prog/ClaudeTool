import { invoke } from '@tauri-apps/api/core';

export type SkillSource = 'user' | 'project' | 'plugin' | 'recommend';

export interface SkillMeta {
  id: string;
  name: string;
  description: string;
  source: SkillSource;
  pluginName: string | null;
  path: string;
  installed: boolean;
  category: string | null;
  repoPath: string | null;
}

export interface SkillsReport {
  installed: SkillMeta[];
  recommended: SkillMeta[];
}

export const listSkills = (workdir: string | null) =>
  invoke<SkillsReport>('list_skills', { workdir });

export const readSkillMd = (path: string) => invoke<string>('read_skill_md', { path });
