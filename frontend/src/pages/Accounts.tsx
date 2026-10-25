import { useState } from 'react';
import { api } from '../api/client';
import { IconStar, IconTrash, IconUser, MicrosoftLogo } from '../components/icons';
import { Avatar, Empty, PageHeader, formatTime } from '../components/ui';
import { useAsync } from '../hooks/useAsync';
import { useLive } from '../state/events';

export function Accounts() {
  const { state, toast } = useLive();
  const accounts = useAsync(() => api.accounts(), [state.revision]);
  const [username, setUsername] = useState('');
  const [busy, setBusy] = useState(false);

  const act = async (fn: () => Promise<unknown>, ok?: string) => {
    setBusy(true);
    try {
      await fn();
      if (ok) toast('success', ok);
      await accounts.reload();
    } catch (e) {
      toast('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const addOffline = () =>
    act(async () => {
      await api.addOffline(username.trim());
      setUsername('');
    }, '离线账户已添加');

  const list = accounts.data ?? [];

  return (
    <div className="page">
      <PageHeader title="账户" subtitle="使用微软正版账户或离线账户进入游戏" />

      <div className="two-col">
        <section className="card add-card microsoft">
          <div className="add-card-icon"><MicrosoftLogo size={26} /></div>
          <h3>微软账户</h3>
          <p className="muted small">通过设备码在浏览器中授权，密码不会经过启动器。需要在设置中配置 Azure 应用 Client ID。</p>
          <button className="btn primary block" disabled={busy} onClick={() => act(() => api.loginMicrosoft(), '请在弹出的窗口中完成授权')}>
            使用微软账户登录
          </button>
        </section>

        <section className="card add-card">
          <div className="add-card-icon offline"><IconUser size={24} /></div>
          <h3>离线账户</h3>
          <p className="muted small">无需联网验证，可用于单人游戏与离线模式服务器。</p>
          <div className="input-row">
            <input
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="玩家名，3–16 位字母、数字或下划线"
              onKeyDown={(e) => e.key === 'Enter' && username.trim() && addOffline()}
            />
            <button className="btn" disabled={busy || !username.trim()} onClick={addOffline}>添加</button>
          </div>
        </section>
      </div>

      <div className="section-head">
        <h2>已保存的账户</h2>
        <span className="muted small">{list.length} 个</span>
      </div>
      {!accounts.loading && list.length === 0 && <Empty icon={<IconUser size={26} />} title="还没有账户" hint="添加一个账户后即可开始游戏" />}
      <div className="account-grid">
        {list.map((a) => (
          <div key={a.id} className={`card account-card ${a.isDefault ? 'selected' : ''}`}>
            <Avatar name={a.username} size={52} />
            <div className="grow">
              <div className="account-name">
                {a.username}
                {a.isDefault && <span className="chip accent">默认</span>}
              </div>
              <div className="muted small">
                {a.kind === 'microsoft' ? '微软账户' : '离线账户'}
                {a.kind === 'microsoft' && a.expiresAt ? ` · 令牌有效至 ${formatTime(a.expiresAt)}` : ''}
              </div>
              <div className="muted tiny mono">{a.uuid}</div>
            </div>
            <div className="account-actions">
              {!a.isDefault && (
                <button className="icon-btn" title="设为默认" disabled={busy} onClick={() => act(() => api.setDefaultAccount(a.id))}>
                  <IconStar />
                </button>
              )}
              <button
                className="icon-btn danger"
                title="移除"
                disabled={busy}
                onClick={() => window.confirm(`移除账户 ${a.username}？`) && act(() => api.deleteAccount(a.id), '账户已移除')}
              >
                <IconTrash />
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
