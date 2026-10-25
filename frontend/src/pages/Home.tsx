import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { Page } from '../App';
import { LOADER_NAMES } from '../api/types';
import { GameConsole } from '../components/GameConsole';
import { LoaderChip, LoaderIcon } from '../components/LoaderBadge';
import { Cover, Scene } from '../components/Scene';
import { IconCalendar, IconClock, IconPlay, IconPlus, IconRepeat, IconStop } from '../components/icons';
import { Avatar, Empty, Stat, formatDuration, formatTime } from '../components/ui';
import { useAsync } from '../hooks/useAsync';
import { useLive } from '../state/events';

export function Home({ navigate }: { navigate: (p: Page) => void }) {
  const { state, toast } = useLive();
  const instances = useAsync(() => api.instances(), [state.revision]);
  const accounts = useAsync(() => api.accounts(), [state.revision]);
  const [selected, setSelected] = useState<string | null>(null);

  const list = instances.data ?? [];
  const account = accounts.data?.find((a) => a.isDefault) ?? accounts.data?.[0];

  useEffect(() => {
    if (list.length > 0 && !list.some((i) => i.id === selected)) setSelected(list[0].id);
  }, [list, selected]);

  const current = list.find((i) => i.id === selected);
  const running = current ? Boolean(state.running[current.id] || current.running) : false;
  const stage = current ? state.stages[current.id] : undefined;

  const launch = async () => {
    if (!current) return;
    try {
      if (running) await api.stopInstance(current.id);
      else await api.launchInstance(current.id, account?.id);
    } catch (e) {
      toast('error', e instanceof Error ? e.message : String(e));
    }
  };

  const hero = (
    <section className="hero">
      <Scene />
      <div className="hero-shade" />
      <div className="hero-content">
        <div className="hero-top">
          <div className="hero-kicker">{current ? '当前实例' : '欢迎来到 StarRaft'}</div>
          <button className="account-chip" onClick={() => navigate('accounts')}>
            {account ? (
              <>
                <Avatar name={account.username} size={30} />
                <div className="account-chip-text">
                  <div>{account.username}</div>
                  <div className="small">{account.kind === 'microsoft' ? '微软账户' : '离线账户'}</div>
                </div>
              </>
            ) : (
              <span>＋ 添加账户</span>
            )}
          </button>
        </div>

        {current ? (
          <>
            <h1 className="hero-title">{current.name}</h1>
            <div className="hero-chips">
              <span className="chip">Minecraft {current.versionId}</span>
              <LoaderChip loader={current.loader} version={current.loaderVersion} />
              {running ? <span className="chip live">运行中</span> : !current.installed && <span className="chip warn">未安装</span>}
            </div>
          </>
        ) : (
          <h1 className="hero-title">扬帆，向方块星海</h1>
        )}

        <div className="hero-bottom">
          {current && (
            <div className="hero-stats">
              <Stat icon={<IconClock />} label="游戏时长" value={formatDuration(current.playSeconds ?? 0)} />
              <Stat icon={<IconRepeat />} label="启动次数" value={current.launches ?? 0} />
              <Stat icon={<IconCalendar />} label="上次游玩" value={formatTime(current.lastPlayed)} />
            </div>
          )}
          {current ? (
            <button
              className={`play-btn ${running ? 'stop' : ''} ${stage ? 'busy' : ''}`}
              disabled={!account || Boolean(stage)}
              onClick={launch}
              title={!account ? '请先添加账户' : undefined}
            >
              {stage ? <span className="spinner" /> : running ? <IconStop size={22} /> : <IconPlay size={22} />}
              <span>
                <span className="play-label">{stage ? '请稍候' : running ? '停止游戏' : current.installed ? '开始游戏' : '安装并开始'}</span>
                <span className="play-sub">{stage ?? (running ? '结束游戏进程' : `Minecraft ${current.versionId}`)}</span>
              </span>
            </button>
          ) : (
            <button className="play-btn" onClick={() => navigate('instances')}>
              <IconPlus size={22} />
              <span>
                <span className="play-label">新建实例</span>
                <span className="play-sub">选择版本开始冒险</span>
              </span>
            </button>
          )}
        </div>
      </div>
    </section>
  );

  if (!instances.loading && list.length === 0) {
    return (
      <div className="page home">
        {hero}
        <Empty icon={<IconPlus size={26} />} title="还没有实例" hint="实例是一套独立的游戏版本、存档与设置" />
      </div>
    );
  }

  return (
    <div className="page home">
      {hero}

      <section>
        <div className="section-head">
          <h2>我的实例</h2>
          <button className="btn ghost small" onClick={() => navigate('instances')}>管理全部</button>
        </div>
        <div className="tiles">
          {list.slice(0, 7).map((inst) => (
            <button
              key={inst.id}
              className={`tile ${inst.id === selected ? 'selected' : ''}`}
              onClick={() => setSelected(inst.id)}
            >
              <Cover id={inst.id} />
              <div className="tile-body">
                <div className="tile-name">{inst.name}</div>
                <div className="tile-meta">
                  {inst.loader !== 'vanilla' && <LoaderIcon loader={inst.loader} size={15} />}
                  {inst.versionId}
                  {inst.loader !== 'vanilla' && ` · ${LOADER_NAMES[inst.loader]}`}
                  {(state.running[inst.id] || inst.running) && <span className="status-dot live" />}
                </div>
              </div>
            </button>
          ))}
          <button className="tile tile-new" onClick={() => navigate('instances')}>
            <IconPlus size={22} />
            <span>新建实例</span>
          </button>
        </div>
      </section>

      {current && <GameConsole instanceId={current.id} running={running} />}
    </div>
  );
}
