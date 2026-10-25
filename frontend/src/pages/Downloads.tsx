import { api } from '../api/client';
import type { BatchStatus } from '../api/types';
import { IconAlert, IconCheck, IconDownload, IconX } from '../components/icons';
import { Empty, PageHeader, ProgressBar, formatBytes } from '../components/ui';
import { useAsync } from '../hooks/useAsync';
import { useLive } from '../state/events';

const STATE_LABEL: Record<string, string> = {
  running: '下载中',
  done: '已完成',
  failed: '失败',
  cancelled: '已取消',
};

function StateIcon({ state }: { state: string }) {
  if (state === 'running') return <span className="spinner small" />;
  if (state === 'done') return <IconCheck size={16} />;
  if (state === 'failed') return <IconAlert size={16} />;
  return <IconX size={16} />;
}

export function Downloads() {
  const { state, toast } = useLive();
  const initial = useAsync(() => api.downloads(), []);

  // 合并首次加载的数据与实时推送
  const merged: Record<string, BatchStatus> = {};
  for (const b of initial.data ?? []) merged[b.id] = b;
  Object.assign(merged, state.downloads);
  const batches = Object.values(merged).sort((a, b) => b.id.localeCompare(a.id));

  return (
    <div className="page">
      <PageHeader title="下载" subtitle="Go 下载引擎并发执行 · BMCLAPI 镜像加速 · SHA-1 校验" />
      {initial.error && <div className="error-text">{initial.error}</div>}
      {batches.length === 0 && (
        <Empty icon={<IconDownload size={26} />} title="暂无下载任务" hint="安装实例时会在这里显示进度" />
      )}
      <div className="list">
        {batches.map((b) => {
          const useBytes = b.bytesTotal > 0;
          const value = useBytes ? b.bytesDone : b.completed + b.failed;
          const max = useBytes ? b.bytesTotal : b.total;
          const pct = max > 0 ? Math.round((value / max) * 100) : 0;
          return (
            <div key={b.id} className={`card download-card ${b.state}`}>
              <div className="row">
                <div className={`download-state ${b.state}`}><StateIcon state={b.state} /></div>
                <div className="grow">
                  <div className="download-title">{b.label || b.id}</div>
                  <div className="muted small">
                    {STATE_LABEL[b.state] ?? b.state} · {b.completed}/{b.total} 个文件
                    {useBytes && ` · ${formatBytes(b.bytesDone)} / ${formatBytes(b.bytesTotal)}`}
                    {b.failed > 0 && <span className="danger-text"> · {b.failed} 个失败</span>}
                  </div>
                </div>
                <div className="download-pct">{pct}%</div>
                {b.state === 'running' && (
                  <button className="btn ghost small" onClick={() => api.cancelDownload(b.id).catch((e) => toast('error', e.message))}>
                    取消
                  </button>
                )}
              </div>
              <ProgressBar
                value={value}
                max={max}
                active={b.state === 'running'}
                tone={b.state === 'failed' ? 'danger' : b.state === 'done' ? 'success' : 'accent'}
              />
              {b.errors.length > 0 && (
                <details className="errors">
                  <summary className="small">错误详情</summary>
                  <ul>{b.errors.map((e, i) => <li key={i} className="small mono">{e}</li>)}</ul>
                </details>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
