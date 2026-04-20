import { create } from 'zustand';
import { listSessions, onSessionRefined, type SessionMeta } from '@/ipc/history';
import type { UnlistenFn } from '@tauri-apps/api/event';

interface HistoryState {
  workdir: string | null;
  loading: boolean;
  sessions: SessionMeta[];
  query: string;
  setQuery: (q: string) => void;
  load: (workdir: string) => Promise<void>;
  clear: () => void;
}

let unlisten: UnlistenFn | null = null;

export const useHistory = create<HistoryState>((set, get) => ({
  workdir: null,
  loading: false,
  sessions: [],
  query: '',
  setQuery: (q) => set({ query: q }),
  load: async (workdir) => {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    set({ workdir, loading: true, sessions: [] });
    try {
      const sessions = await listSessions(workdir);
      set({ sessions, loading: false });
      unlisten = await onSessionRefined((payload) => {
        if (payload.workdir !== get().workdir) return;
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === payload.sessionId ? { ...sess, turnCount: payload.turnCount } : sess
          ),
        }));
      });
    } catch (err) {
      console.error('list_sessions failed', err);
      set({ loading: false });
    }
  },
  clear: () => {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    set({ workdir: null, sessions: [], query: '' });
  },
}));

export function filterSessions(sessions: SessionMeta[], query: string): SessionMeta[] {
  const q = query.trim().toLowerCase();
  if (!q) return sessions;
  return sessions.filter((s) => {
    const haystack = `${s.id} ${s.name ?? ''} ${s.summary ?? ''} ${s.firstPrompt}`.toLowerCase();
    return haystack.includes(q);
  });
}

export function formatRelative(updatedAtUnix: number): string {
  if (!updatedAtUnix) return '';
  const ms = updatedAtUnix * 1000;
  const diff = Date.now() - ms;
  const min = Math.floor(diff / 60000);
  if (min < 1) return '刚刚';
  if (min < 60) return `${min} 分钟前`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr} 小时前`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day} 天前`;
  return new Date(ms).toLocaleDateString('zh-CN');
}

export function sessionTitle(s: SessionMeta): string {
  return s.name || s.summary || s.firstPrompt || s.id;
}
