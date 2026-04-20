import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface SessionMeta {
  id: string;
  name: string | null;
  summary: string | null;
  firstPrompt: string;
  cwd: string;
  turnCount: number;
  createdAt: string | null;
  /** Unix epoch seconds, UTC */
  updatedAtUnix: number;
  bytes: number;
}

export interface SessionRefinePayload {
  workdir: string;
  sessionId: string;
  turnCount: number;
}

export const listSessions = (workdir: string) =>
  invoke<SessionMeta[]>('list_sessions', { workdir });

export const onSessionRefined = (
  handler: (payload: SessionRefinePayload) => void
): Promise<UnlistenFn> =>
  listen<SessionRefinePayload>('session-refined', (e) => handler(e.payload));
