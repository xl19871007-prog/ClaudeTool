import { invoke } from '@tauri-apps/api/core';

export type ClaudeStatus =
  | { kind: 'installed'; version: string; path: string }
  | { kind: 'notInstalled' };

export type AuthStatus =
  | { kind: 'loggedIn'; account: string | null }
  | { kind: 'notLoggedIn' }
  | { kind: 'unknown' };

export type NetworkStatus =
  | { kind: 'ok'; latencyMs: number }
  | { kind: 'slow'; latencyMs: number }
  | { kind: 'unreachable'; error: string };

export interface UpdateInfo {
  current: string;
  latest: string | null;
  hasUpdate: boolean;
}

export type GitStatus =
  | {
      kind: 'installed';
      version: string;
      path: string;
      bashPath: string | null;
      /** False when discovered via registry (PATH wasn't updated). */
      inPath: boolean;
    }
  | { kind: 'notInstalled' };

export type GitBashEnvStatus =
  | { kind: 'configured'; path: string }
  | { kind: 'notConfigured' }
  | { kind: 'invalidPath'; path: string };

export interface EnvironmentReport {
  claude: ClaudeStatus;
  auth: AuthStatus;
  network: NetworkStatus;
  update: UpdateInfo;
  git: GitStatus;
  gitBashEnv: GitBashEnvStatus;
}

export interface ProxyConfig {
  http: string | null;
  https: string | null;
  noProxy: string | null;
}

export interface AppConfig {
  version: number;
  lastWorkdir: string | null;
  theme: string;
  suppressLoginPrompt: boolean;
  lastSeenVersion: string | null;
  proxy: ProxyConfig;
  debugForceClaudeMissing: boolean;
  debugForceGitMissing: boolean;
  debugDryRun: boolean;
}

export const setDebugFlag = (
  name: 'forceClaudeMissing' | 'forceGitMissing' | 'dryRun',
  value: boolean
) => invoke<AppConfig>('set_debug_flag', { name, value });

export const setProxy = (proxy: ProxyConfig) => invoke<AppConfig>('set_proxy', { proxy });

/** Build env vars suitable for child processes (Claude PTY, etc.). */
export function proxyToEnv(proxy: ProxyConfig | null | undefined): Record<string, string> {
  const env: Record<string, string> = {};
  if (!proxy) return env;
  if (proxy.http) {
    env.HTTP_PROXY = proxy.http;
    env.http_proxy = proxy.http;
  }
  if (proxy.https) {
    env.HTTPS_PROXY = proxy.https;
    env.https_proxy = proxy.https;
  }
  if (proxy.noProxy) {
    env.NO_PROXY = proxy.noProxy;
    env.no_proxy = proxy.noProxy;
  }
  return env;
}

export const checkEnvironment = () => invoke<EnvironmentReport>('check_environment');

export const getConfig = () => invoke<AppConfig>('get_config');

export const setSuppressLoginPrompt = (value: boolean) =>
  invoke<void>('set_suppress_login_prompt', { value });

export const setLastSeenVersion = (value: string) =>
  invoke<void>('set_last_seen_version', { value });
