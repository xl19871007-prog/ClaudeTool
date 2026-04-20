import { create } from 'zustand';

type Panel = 'commands' | 'skills' | null;

interface PanelsState {
  open: Panel;
  toggle: (panel: Exclude<Panel, null>) => void;
  close: () => void;
}

export const usePanels = create<PanelsState>((set, get) => ({
  open: null,
  toggle: (panel) => set({ open: get().open === panel ? null : panel }),
  close: () => set({ open: null }),
}));
