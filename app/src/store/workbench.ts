import { create } from 'zustand';

export type ClaudeArgs =
  | { kind: 'new' }
  | { kind: 'continueLast' }
  | { kind: 'resume'; sessionId: string }
  | { kind: 'fork'; sessionId: string };

interface WorkbenchState {
  cwd: string | null;
  /** Bumped to force remount of Terminal (e.g. switching session) */
  sessionEpoch: number;
  /** Args for the next claude spawn */
  claudeArgs: ClaudeArgs;
  setCwd: (cwd: string | null) => void;
  startNew: () => void;
  startResume: (sessionId: string) => void;
  startFork: (sessionId: string) => void;
  startContinue: () => void;
}

export const useWorkbench = create<WorkbenchState>((set) => ({
  cwd: null,
  sessionEpoch: 0,
  claudeArgs: { kind: 'new' },
  setCwd: (cwd) =>
    set((s) => ({
      cwd,
      sessionEpoch: s.sessionEpoch + 1,
      claudeArgs: { kind: 'new' },
    })),
  startNew: () =>
    set((s) => ({
      sessionEpoch: s.sessionEpoch + 1,
      claudeArgs: { kind: 'new' },
    })),
  startResume: (sessionId) =>
    set((s) => ({
      sessionEpoch: s.sessionEpoch + 1,
      claudeArgs: { kind: 'resume', sessionId },
    })),
  startFork: (sessionId) =>
    set((s) => ({
      sessionEpoch: s.sessionEpoch + 1,
      claudeArgs: { kind: 'fork', sessionId },
    })),
  startContinue: () =>
    set((s) => ({
      sessionEpoch: s.sessionEpoch + 1,
      claudeArgs: { kind: 'continueLast' },
    })),
}));

export function pathBasename(p: string): string {
  return p.split(/[\\/]/).filter(Boolean).pop() ?? p;
}

export function buildClaudeCliArgs(args: ClaudeArgs): string[] {
  switch (args.kind) {
    case 'new':
      return [];
    case 'continueLast':
      return ['--continue'];
    case 'resume':
      return ['--resume', args.sessionId];
    case 'fork':
      return ['--resume', args.sessionId, '--fork-session'];
  }
}
