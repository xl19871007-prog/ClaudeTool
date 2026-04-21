import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type InstallEvent =
  | { stage: 'started'; target: string }
  | { stage: 'resolving'; message: string }
  | {
      stage: 'downloading';
      downloadedBytes: number;
      totalBytes: number | null;
      message: string;
    }
  | { stage: 'installing'; message: string }
  | { stage: 'configuring'; message: string }
  | { stage: 'verifying'; message: string }
  | { stage: 'done'; message: string }
  | { stage: 'failed'; stageId: string; message: string; recoverable: boolean }
  | { stage: 'log'; line: string };

const EVENT_NAME = 'install-progress';

export const onInstallEvent = (handler: (e: InstallEvent) => void): Promise<UnlistenFn> =>
  listen<InstallEvent>(EVENT_NAME, (e) => handler(e.payload));

export const installGit = () => invoke<void>('install_git');
export const installClaudeCode = () => invoke<void>('install_claude_code');
export const repairGitEnv = () => invoke<void>('repair_git_env');
