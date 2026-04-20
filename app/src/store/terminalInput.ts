import { create } from 'zustand';

interface InjectRequest {
  text: string;
  /** Bumped each call so the same text triggers re-render */
  nonce: number;
}

interface TerminalInputState {
  pending: InjectRequest | null;
  inject: (text: string) => void;
  consume: () => void;
}

export const useTerminalInput = create<TerminalInputState>((set, get) => ({
  pending: null,
  inject: (text) =>
    set({
      pending: { text, nonce: (get().pending?.nonce ?? 0) + 1 },
    }),
  consume: () => set({ pending: null }),
}));
