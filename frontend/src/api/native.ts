// 与 C# 宿主（Photino）的原生能力桥；在浏览器中开发时自动降级

interface PhotinoExternal {
  sendMessage(message: string): void;
  receiveMessage(callback: (message: string) => void): void;
}

interface BridgeResponse {
  id: string;
  ok: boolean;
  result?: unknown;
  error?: string;
}

const external = (window as unknown as { external?: Partial<PhotinoExternal> }).external;
const bridge: PhotinoExternal | null =
  typeof external?.sendMessage === 'function' && typeof external?.receiveMessage === 'function'
    ? (external as PhotinoExternal)
    : null;

const pending = new Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
let seq = 0;

bridge?.receiveMessage((raw) => {
  let msg: BridgeResponse;
  try {
    msg = JSON.parse(raw);
  } catch {
    return;
  }
  const waiter = pending.get(msg.id);
  if (!waiter) return;
  pending.delete(msg.id);
  if (msg.ok) waiter.resolve(msg.result);
  else waiter.reject(new Error(msg.error ?? '原生调用失败'));
});

function call<T>(type: string, payload: Record<string, unknown> = {}): Promise<T> {
  if (!bridge) return Promise.reject(new Error('当前不在桌面宿主中运行'));
  const id = String(++seq);
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
    bridge.sendMessage(JSON.stringify({ id, type, payload }));
  });
}

export const native = {
  available: bridge !== null,
  pickFolder: (title?: string) => call<string | null>('pickFolder', { title }),
  pickFile: (title?: string) => call<string | null>('pickFile', { title }),
  openPath: (path: string) => call<void>('openPath', { path }),
  openUrl: async (url: string) => {
    if (bridge) return call<void>('openUrl', { url });
    window.open(url, '_blank', 'noopener');
  },
};
