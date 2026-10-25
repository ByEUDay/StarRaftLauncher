import { useEffect, useRef, useState } from 'react';
import { useLive } from '../state/events';
import { IconChevronDown, IconTerminal } from './icons';

export function GameConsole({ instanceId, running }: { instanceId: string; running: boolean }) {
  const { state, clearLog } = useLive();
  const lines = state.logs[instanceId] ?? [];
  const ref = useRef<HTMLDivElement>(null);
  const [follow, setFollow] = useState(true);
  const [open, setOpen] = useState(false);

  // 游戏启动或产生输出时自动展开
  useEffect(() => {
    if (running || lines.length > 0) setOpen(true);
  }, [running, lines.length > 0]);

  useEffect(() => {
    if (open && follow && ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [lines, follow, open]);

  const onScroll = () => {
    const el = ref.current;
    if (el) setFollow(el.scrollHeight - el.scrollTop - el.clientHeight < 40);
  };

  return (
    <section className={`console-card ${open ? 'open' : ''}`}>
      <button className="console-head" onClick={() => setOpen((o) => !o)}>
        <IconTerminal />
        <span>游戏日志</span>
        <span className={`status-dot ${running ? 'live' : ''}`} />
        <span className="muted small">{running ? '运行中' : lines.length ? `${lines.length} 行` : '暂无输出'}</span>
        <span className="grow" />
        {lines.length > 0 && (
          <span
            className="btn ghost small"
            role="button"
            onClick={(e) => {
              e.stopPropagation();
              clearLog(instanceId);
            }}
          >
            清空
          </span>
        )}
        <IconChevronDown className="chevron" />
      </button>
      {open && (
        <div className="console" ref={ref} onScroll={onScroll}>
          {lines.length === 0 ? (
            <div className="console-empty">启动游戏后，这里会实时显示游戏输出</div>
          ) : (
            lines.map((l, i) => (
              <div
                key={i}
                className={`log-line ${l.stream} ${/\b(ERROR|FATAL|Exception)\b/.test(l.line) ? 'err' : /\bWARN\b/.test(l.line) ? 'warn' : ''}`}
              >
                {l.line}
              </div>
            ))
          )}
        </div>
      )}
    </section>
  );
}
