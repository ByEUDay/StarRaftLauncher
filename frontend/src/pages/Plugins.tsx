import { api } from '../api/client';
import { hashString } from '../components/Scene';
import { IconSparkle, IconTerminal } from '../components/icons';
import { Empty, PageHeader } from '../components/ui';
import { useAsync } from '../hooks/useAsync';
import { useLive } from '../state/events';

export function Plugins() {
  const { state, toast } = useLive();
  const plugins = useAsync(() => api.plugins(), []);

  const toggle = async (id: string, enabled: boolean) => {
    try {
      await api.setPluginEnabled(id, enabled);
      await plugins.reload();
    } catch (e) {
      toast('error', e instanceof Error ? e.message : String(e));
    }
  };

  const list = plugins.data ?? [];

  return (
    <div className="page">
      <PageHeader title="插件" subtitle="Lua 5.4 沙箱插件，在游戏启动前后调整参数。用户插件放在数据目录的 plugins 文件夹中。" />
      {!plugins.loading && list.length === 0 && <Empty icon={<IconSparkle size={26} />} title="没有已加载的插件" />}

      <div className="plugin-grid">
        {list.map((p) => {
          const hue = hashString(p.id) % 360;
          return (
            <div key={p.id} className={`card plugin-card ${p.enabled ? '' : 'disabled'}`}>
              <div className="plugin-icon" style={{ background: `linear-gradient(135deg, hsl(${hue} 75% 62%), hsl(${(hue + 40) % 360} 70% 48%))` }}>
                {p.name.slice(0, 1)}
              </div>
              <div className="grow">
                <div className="plugin-name">
                  {p.name} <span className="muted small">v{p.version}</span>
                </div>
                <div className="muted small">{p.description || p.id}</div>
                {p.author && <div className="muted tiny">作者 {p.author}</div>}
                {p.error && <div className="error-text small mono">{p.error}</div>}
              </div>
              <label className="switch" title={p.error ? '加载失败' : p.enabled ? '已启用' : '已停用'}>
                <input type="checkbox" checked={p.enabled} disabled={Boolean(p.error)} onChange={(e) => toggle(p.id, e.target.checked)} />
                <span />
              </label>
            </div>
          );
        })}
      </div>

      <section className="console-card open">
        <div className="console-head static">
          <IconTerminal />
          <span>插件日志</span>
          <span className="muted small">{state.pluginLogs.length} 条</span>
        </div>
        <div className="console short">
          {state.pluginLogs.length === 0 ? (
            <div className="console-empty">插件输出会显示在这里</div>
          ) : (
            state.pluginLogs.map((l, i) => (
              <div key={i} className="log-line">
                <span className="log-time">{new Date(l.at).toLocaleTimeString('zh-CN')}</span>
                <span className="log-tag">{l.plugin}</span>
                {l.message}
              </div>
            ))
          )}
        </div>
      </section>
    </div>
  );
}
