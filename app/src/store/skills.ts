import { create } from 'zustand';
import { listSkills, readSkillMd, type SkillMeta, type SkillsReport } from '@/ipc/skills';

interface SkillsState {
  loading: boolean;
  report: SkillsReport | null;
  selected: SkillMeta | null;
  selectedMd: string | null;
  loadingMd: boolean;
  load: (workdir: string | null) => Promise<void>;
  select: (skill: SkillMeta | null) => Promise<void>;
}

export const useSkills = create<SkillsState>((set) => ({
  loading: false,
  report: null,
  selected: null,
  selectedMd: null,
  loadingMd: false,
  load: async (workdir) => {
    set({ loading: true });
    try {
      const report = await listSkills(workdir);
      set({ report, loading: false });
    } catch (err) {
      console.error('list_skills failed', err);
      set({ loading: false });
    }
  },
  select: async (skill) => {
    set({ selected: skill, selectedMd: null, loadingMd: !!skill });
    if (!skill || !skill.installed) {
      set({ loadingMd: false });
      return;
    }
    try {
      const md = await readSkillMd(skill.path);
      set({ selectedMd: md, loadingMd: false });
    } catch (err) {
      console.error('read_skill_md failed', err);
      set({ loadingMd: false });
    }
  },
}));
