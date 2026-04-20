import { create } from 'zustand';

interface WorkbenchState {
  cwd: string | null;
  setCwd: (cwd: string | null) => void;
}

export const useWorkbench = create<WorkbenchState>((set) => ({
  cwd: null,
  setCwd: (cwd) => set({ cwd }),
}));

export function pathBasename(p: string): string {
  return p.split(/[\\/]/).filter(Boolean).pop() ?? p;
}
