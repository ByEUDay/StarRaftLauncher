import { useState, type ReactNode } from 'react';
import { DeviceCodeModal } from './components/DeviceCodeModal';
import {
  IconAlert,
  IconCheck,
  IconDownload,
  IconGrid,
  IconHome,
  IconInfo,
  IconSliders,
  IconSparkle,
  IconUser,
  LogoMark,
} from './components/icons';
import { Accounts } from './pages/Accounts';
import { Downloads } from './pages/Downloads';
import { Home } from './pages/Home';
import { Instances } from './pages/Instances';
import { Plugins } from './pages/Plugins';
import { Settings } from './pages/Settings';
import { useLive } from './state/events';

export type Page = 'home' | 'instances' | 'accounts' | 'downloads' | 'plugins' | 'settings';

const NAV: { id: Page; label: string; icon: ReactNode }[] = [
  { id: 'home', label: '主页', icon: <IconHome /> },
  { id: 'instances', label: '实例', icon: <IconGrid /> },
  { id: 'accounts', label: '账户', icon: <IconUser /> },
  { id: 'downloads', label: '下载', icon: <IconDownload /> },
  { id: 'plugins', label: '插件', icon: <IconSparkle /> },
  { id: 'settings', label: '设置', icon: <IconSliders /> },
];

const TOAST_ICON = { error: <IconAlert />, success: <IconCheck />, info: <IconInfo /> };

export function App() {
  const [page, setPage] = useState<Page>('home');
  const { state, dismiss } = useLive();
  const activeDownloads = Object.values(state.downloads).filter((d) => d.state === 'running').length;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <LogoMark />
          <div>
            <div className="brand-name">StarRaft</div>
            <div className="brand-sub">Minecraft 启动器</div>
          </div>
        </div>
        <nav>
          {NAV.map((n) => (
            <button key={n.id} className={`nav-item ${page === n.id ? 'active' : ''}`} onClick={() => setPage(n.id)}>
              <span className="nav-icon">{n.icon}</span>
              <span className="nav-label">{n.label}</span>
              {n.id === 'downloads' && activeDownloads > 0 && <span className="nav-badge">{activeDownloads}</span>}
            </button>
          ))}
        </nav>
        <div className={`conn ${state.connected ? 'ok' : ''}`}>
          <span className="conn-dot" />
          <span className="conn-label">{state.connected ? '引擎在线' : '正在连接…'}</span>
        </div>
      </aside>

      <main className="content">
        <div className="page-anim" key={page}>
          {page === 'home' && <Home navigate={setPage} />}
          {page === 'instances' && <Instances />}
          {page === 'accounts' && <Accounts />}
          {page === 'downloads' && <Downloads />}
          {page === 'plugins' && <Plugins />}
          {page === 'settings' && <Settings />}
        </div>
      </main>

      <DeviceCodeModal />

      <div className="toasts" aria-live="polite">
        {state.toasts.map((t) => (
          <div key={t.id} className={`toast ${t.kind}`} onClick={() => dismiss(t.id)}>
            <span className="toast-icon">{TOAST_ICON[t.kind]}</span>
            <span>{t.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
