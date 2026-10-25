import { useState } from 'react';
import { api } from '../api/client';
import { native } from '../api/native';
import type { Instance } from '../api/types';
import { InstanceForm } from '../components/InstanceForm';
import { LoaderChip, LoaderIcon } from '../components/LoaderBadge';
import { Cover } from '../components/Scene';
import { IconClock, IconDownload, IconEdit, IconFolder, IconGrid, IconPlay, IconPlus, IconStop, IconTrash } from '../components/icons';
import { Empty, PageHeader, formatDuration, formatMemory, formatTime } from '../components/ui';
import { useAsync } from '../hooks/useAsync';
import { useLive } from '../state/events';

export function Instances() {
  const { state, toast } = useLive();
  const instances = useAsync(() => api.instances(), [state.revision]);
  const system = useAsync(() => api.system(), []);
  const [editing, setEditing] = useState<Instance | 'new' | null>(null);

  const run = async (fn: () => Promise<unknown>, ok?: string) => {
    try {
      await fn();
      if (ok) toast('info', ok);
      await instances.reload();
    } catch (e) {
      toast('error', e instanceof Error ? e.message : String(e));
    }
  };

  const remove = (inst: Instance) => {
    if (window.confirm(`确定删除实例「${inst.name}」？存档、模组等文件会一并删除。`)) {
      void run(() => api.deleteInstance(inst.id), '实例已删除');
    }
  };

  const openFolder = (inst: Instance) => {
    if (!system.data) return;
    const sep = system.data.os.os === 'windows' ? '\\' : '/';
    native.openPath(`${system.data.instancesDir}${sep}${inst.id}`).catch((e) => toast('error', e.message));
  };

  const list = instances.data ?? [];

  return (
    <div className="page">
      <PageHeader
        title="实例"
        subtitle="每个实例拥有独立的游戏目录、存档与设置"
        actions={
          <button className="btn primary" onClick={() => setEditing('new')}>
            <IconPlus size={16} /> 新建实例
          </button>
        }
      />

      {instances.error && <div className="error-text">{instances.error}</div>}
      {!instances.loading && list.length === 0 && (
        <Empty icon={<IconGrid size={26} />} title="还没有实例" hint="点击右上角新建一个" />
      )}

      <div className="grid">
        {list.map((inst) => {
          const running = Boolean(state.running[inst.id] || inst.running);
          const stage = state.stages[inst.id];
          return (
            <article key={inst.id} className={`card instance-card ${running ? 'running' : ''}`}>
              <div className="instance-cover">
                <Cover id={inst.id} />
                <div className="instance-badges">
                  <span className="chip">{inst.versionId}</span>
                  {running ? <span className="chip live">运行中</span> : !inst.installed && <span className="chip warn">未安装</span>}
                </div>
                <div className="instance-letter"><LoaderIcon loader={inst.loader} size={46} /></div>
              </div>

              <div className="instance-body">
                <div className="instance-title">
                  <h3>{inst.name}</h3>
                  <LoaderChip loader={inst.loader} version={inst.loaderVersion} />
                </div>
                <div className="instance-meta">
                  <span><IconClock size={14} /> {formatDuration(inst.playSeconds ?? 0)}</span>
                  <span>上次 {formatTime(inst.lastPlayed)}</span>
                  <span>{inst.memoryMb ? formatMemory(inst.memoryMb) : '自动内存'}</span>
                </div>
                {stage && (
                  <div className="stage">
                    <span className="spinner small" /> {stage}…
                  </div>
                )}
              </div>

              <div className="instance-actions">
                {running ? (
                  <button className="btn danger" onClick={() => run(() => api.stopInstance(inst.id))}>
                    <IconStop size={15} /> 停止
                  </button>
                ) : (
                  <button className="btn primary" disabled={Boolean(stage)} onClick={() => run(() => api.launchInstance(inst.id))}>
                    <IconPlay size={15} /> 启动
                  </button>
                )}
                {!inst.installed && !running && (
                  <button className="btn" disabled={Boolean(stage)} onClick={() => run(() => api.installInstance(inst.id), '开始安装')}>
                    <IconDownload size={15} /> 安装
                  </button>
                )}
                <span className="grow" />
                <button className="icon-btn" title="编辑" onClick={() => setEditing(inst)}><IconEdit /></button>
                {native.available && (
                  <button className="icon-btn" title="打开游戏目录" onClick={() => openFolder(inst)}><IconFolder /></button>
                )}
                <button className="icon-btn danger" title="删除" disabled={running} onClick={() => remove(inst)}><IconTrash /></button>
              </div>
            </article>
          );
        })}
      </div>

      {editing && (
        <InstanceForm
          instance={editing === 'new' ? undefined : editing}
          maxMemoryMb={system.data?.memory.totalMb ?? 16384}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            void instances.reload();
          }}
        />
      )}
    </div>
  );
}
