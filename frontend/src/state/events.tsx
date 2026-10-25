import { createContext, useContext, useEffect, useReducer, useRef, type ReactNode } from 'react';
import { eventsUrl } from '../api/client';
import type { BatchStatus, DeviceCode, ServerEvent } from '../api/types';

// 汇总后端通过 SSE 推送的实时状态
export interface Toast {
  id: number;
  kind: 'error' | 'info' | 'success';
  message: string;
}

export interface LogLine {
  stream: 'stdout' | 'stderr' | 'plugin';
  line: string;
}

interface LiveState {
  connected: boolean;
  downloads: Record<string, BatchStatus>;
  logs: Record<string, LogLine[]>;
  running: Record<string, boolean>;
  stages: Record<string, string>;
  pluginLogs: { plugin: string; message: string; at: number }[];
  deviceCode: DeviceCode | null;
  toasts: Toast[];
  revision: number; // 实例/账户等数据变化时递增，页面据此刷新
}

type Action =
  | { type: 'connected'; value: boolean }
  | { type: 'event'; event: ServerEvent }
  | { type: 'toast'; toast: Omit<Toast, 'id'> }
  | { type: 'dismiss'; id: number }
  | { type: 'clearDeviceCode' }
  | { type: 'clearLog'; instanceId: string };

const MAX_LOG_LINES = 1000;
let toastSeq = 0;

const initial: LiveState = {
  connected: false,
  downloads: {},
  logs: {},
  running: {},
  stages: {},
  pluginLogs: [],
  deviceCode: null,
  toasts: [],
  revision: 0,
};

function appendLog(logs: LiveState['logs'], id: string, line: LogLine) {
  const prev = logs[id] ?? [];
  const next = prev.length >= MAX_LOG_LINES ? prev.slice(prev.length - MAX_LOG_LINES + 1) : prev.slice();
  next.push(line);
  return { ...logs, [id]: next };
}

function withToast(state: LiveState, kind: Toast['kind'], message: string): LiveState {
  return { ...state, toasts: [...state.toasts.slice(-4), { id: ++toastSeq, kind, message }] };
}

const STAGE_LABELS: Record<string, string> = {
  version: '获取版本信息',
  libraries: '下载游戏本体与依赖库',
  assets: '下载资源文件',
  loader: '安装模组加载器',
  natives: '解压本地库',
  prepare: '准备启动参数',
};

function reducer(state: LiveState, action: Action): LiveState {
  switch (action.type) {
    case 'connected':
      return { ...state, connected: action.value };
    case 'toast':
      return withToast(state, action.toast.kind, action.toast.message);
    case 'dismiss':
      return { ...state, toasts: state.toasts.filter((t) => t.id !== action.id) };
    case 'clearDeviceCode':
      return { ...state, deviceCode: null };
    case 'clearLog':
      return { ...state, logs: { ...state.logs, [action.instanceId]: [] } };
    case 'event':
      return applyEvent(state, action.event);
  }
}

function applyEvent(state: LiveState, { kind, data }: ServerEvent): LiveState {
  switch (kind) {
    case 'download.progress':
      return { ...state, downloads: { ...state.downloads, [data.id]: data } };
    case 'install.stage':
    case 'launch.stage':
      return { ...state, stages: { ...state.stages, [data.instanceId]: STAGE_LABELS[data.stage] ?? data.stage } };
    case 'loader.progress':
      if (!data.instanceId) return state;
      return {
        ...state,
        stages: { ...state.stages, [data.instanceId]: `运行安装处理器 ${data.index}/${data.total}` },
      };
    case 'install.done': {
      const stages = { ...state.stages };
      delete stages[data.instanceId];
      return withToast({ ...state, stages, revision: state.revision + 1 }, 'success', '安装完成');
    }
    case 'game.started': {
      const stages = { ...state.stages };
      delete stages[data.instanceId];
      return {
        ...state,
        stages,
        running: { ...state.running, [data.instanceId]: true },
        logs: { ...state.logs, [data.instanceId]: [] },
        revision: state.revision + 1,
      };
    }
    case 'game.log':
      return { ...state, logs: appendLog(state.logs, data.instanceId, { stream: data.stream, line: data.line }) };
    case 'game.exit': {
      const running = { ...state.running };
      delete running[data.instanceId];
      const next = { ...state, running, revision: state.revision + 1 };
      return data.exitCode === 0 || data.exitCode === null
        ? next
        : withToast(next, 'error', `游戏异常退出（退出码 ${data.exitCode}）`);
    }
    case 'task.error': {
      const stages = { ...state.stages };
      delete stages[data.instanceId];
      return withToast({ ...state, stages }, 'error', data.message);
    }
    case 'account.deviceCode':
      return { ...state, deviceCode: data };
    case 'account.added':
      return withToast({ ...state, deviceCode: null, revision: state.revision + 1 }, 'success', `已添加账户 ${data.username}`);
    case 'account.error':
      return withToast({ ...state, deviceCode: null }, 'error', data.message);
    case 'plugin.log':
      return { ...state, pluginLogs: [...state.pluginLogs.slice(-199), { ...data, at: Date.now() }] };
    case 'plugin.error':
      return withToast(state, 'error', `插件 ${data.plugin} 出错：${data.message}`);
    case 'instance.changed':
    case 'settings.changed':
      return { ...state, revision: state.revision + 1 };
    default:
      return state;
  }
}

interface LiveContext {
  state: LiveState;
  toast: (kind: Toast['kind'], message: string) => void;
  dismiss: (id: number) => void;
  clearDeviceCode: () => void;
  clearLog: (instanceId: string) => void;
}

const Ctx = createContext<LiveContext | null>(null);

export function LiveProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initial);
  const retry = useRef(0);

  useEffect(() => {
    let source: EventSource | null = null;
    let timer: number | undefined;
    let closed = false;

    const connect = () => {
      source = new EventSource(eventsUrl());
      source.onopen = () => {
        retry.current = 0;
        dispatch({ type: 'connected', value: true });
      };
      source.onmessage = (e) => {
        try {
          dispatch({ type: 'event', event: JSON.parse(e.data) });
        } catch {
          /* 忽略无法解析的事件 */
        }
      };
      source.onerror = () => {
        dispatch({ type: 'connected', value: false });
        source?.close();
        if (closed) return;
        const delay = Math.min(10_000, 500 * 2 ** retry.current++);
        timer = window.setTimeout(connect, delay);
      };
    };
    connect();

    return () => {
      closed = true;
      source?.close();
      window.clearTimeout(timer);
    };
  }, []);

  // 提示自动消失
  useEffect(() => {
    if (state.toasts.length === 0) return;
    const first = state.toasts[0];
    const t = window.setTimeout(() => dispatch({ type: 'dismiss', id: first.id }), first.kind === 'error' ? 8000 : 4000);
    return () => window.clearTimeout(t);
  }, [state.toasts]);

  const value: LiveContext = {
    state,
    toast: (kind, message) => dispatch({ type: 'toast', toast: { kind, message } }),
    dismiss: (id) => dispatch({ type: 'dismiss', id }),
    clearDeviceCode: () => dispatch({ type: 'clearDeviceCode' }),
    clearLog: (instanceId) => dispatch({ type: 'clearLog', instanceId }),
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useLive(): LiveContext {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error('useLive must be used inside <LiveProvider>');
  return ctx;
}
