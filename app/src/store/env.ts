import { create } from 'zustand';
import { type AppConfig, type EnvironmentReport, checkEnvironment, getConfig } from '@/ipc/env';

interface EnvState {
  loading: boolean;
  report: EnvironmentReport | null;
  config: AppConfig | null;
  refresh: () => Promise<void>;
  loadConfig: () => Promise<void>;
}

export const useEnv = create<EnvState>((set) => ({
  loading: false,
  report: null,
  config: null,
  refresh: async () => {
    set({ loading: true });
    try {
      const report = await checkEnvironment();
      set({ report, loading: false });
    } catch (err) {
      console.error('check_environment failed', err);
      set({ loading: false });
    }
  },
  loadConfig: async () => {
    try {
      const config = await getConfig();
      set({ config });
    } catch (err) {
      console.error('get_config failed', err);
    }
  },
}));
