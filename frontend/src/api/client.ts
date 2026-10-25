import type {
  Account,
  BatchStatus,
  Instance,
  InstanceInput,
  LoaderKind,
  LoaderVersion,
  PluginInfo,
  Settings,
  SystemInfo,
  VersionList,
} from './types';

// API 地址与令牌来源（按优先级）：URL 参数（宿主注入）→ 会话存储 → Vite 环境变量 → 同源
const params = new URLSearchParams(window.location.search);

function pick(key: string, envValue: string | undefined, fallback: string): string {
  const value = params.get(key) ?? safeSession(`sr.${key}`) ?? envValue ?? fallback;
  try {
    sessionStorage.setItem(`sr.${key}`, value);
  } catch {
    /* 存储不可用时忽略 */
  }
  return value;
}

function safeSession(key: string): string | null {
  try {
    return sessionStorage.getItem(key);
  } catch {
    return null;
  }
}

const API_BASE = pick('api', import.meta.env.VITE_API_BASE, window.location.origin).replace(/\/$/, '');
const TOKEN = pick('token', import.meta.env.VITE_API_TOKEN, '');

// 令牌不留在地址栏
if (params.has('token')) window.history.replaceState(null, '', window.location.pathname);

export class ApiError extends Error {
  constructor(message: string, readonly status: number) {
    super(message);
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers: {
      'X-StarRaft-Token': TOKEN,
      ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new ApiError(err.error ?? `HTTP ${res.status}`, res.status);
  }
  if (res.status === 202 || res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export const api = {
  system: () => request<SystemInfo>('GET', '/api/system'),
  versions: (snapshots = false) => request<VersionList>('GET', `/api/versions?snapshots=${snapshots}`),
  loaderVersions: (loader: LoaderKind, mc: string) =>
    request<LoaderVersion[]>('GET', `/api/loaders/${loader}?mc=${encodeURIComponent(mc)}`),

  instances: () => request<Instance[]>('GET', '/api/instances'),
  createInstance: (input: InstanceInput) => request<Instance>('POST', '/api/instances', input),
  updateInstance: (id: string, input: InstanceInput) => request<Instance>('PUT', `/api/instances/${id}`, input),
  deleteInstance: (id: string) => request<void>('DELETE', `/api/instances/${id}`),
  installInstance: (id: string) => request<void>('POST', `/api/instances/${id}/install`),
  launchInstance: (id: string, accountId?: string) =>
    request<void>('POST', `/api/instances/${id}/launch`, { accountId }),
  stopInstance: (id: string) => request<void>('POST', `/api/instances/${id}/stop`),

  accounts: () => request<Account[]>('GET', '/api/accounts'),
  addOffline: (username: string) => request<Account>('POST', '/api/accounts/offline', { username }),
  loginMicrosoft: () => request<void>('POST', '/api/accounts/microsoft'),
  setDefaultAccount: (id: string) => request<void>('POST', `/api/accounts/${encodeURIComponent(id)}/default`),
  deleteAccount: (id: string) => request<void>('DELETE', `/api/accounts/${encodeURIComponent(id)}`),

  downloads: () => request<BatchStatus[]>('GET', '/api/downloads'),
  cancelDownload: (id: string) => request<void>('DELETE', `/api/downloads/${id}`),

  plugins: () => request<PluginInfo[]>('GET', '/api/plugins'),
  setPluginEnabled: (id: string, enabled: boolean) =>
    request<void>('PUT', `/api/plugins/${encodeURIComponent(id)}/enabled`, { enabled }),

  settings: () => request<Settings>('GET', '/api/settings'),
  saveSettings: (patch: Settings) => request<Settings>('PUT', '/api/settings', patch),
};

export function eventsUrl(): string {
  return `${API_BASE}/api/events?token=${encodeURIComponent(TOKEN)}`;
}
