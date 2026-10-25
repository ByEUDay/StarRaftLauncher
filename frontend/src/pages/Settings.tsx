import { useEffect, useState, type ReactNode } from 'react';
import { api } from '../api/client';
import { native } from '../api/native';
import type { Settings as SettingsData } from '../api/types';
import { IconChip, IconCup, IconFolder, IconGlobe, IconInfo, IconKey, IconMonitor } from '../components/icons';
import { PageHeader, Segmented, formatMemory } from '../components/ui';
import { useAsync } from '../hooks/useAsync';
import { useLive } from '../state/events';

function Section({ icon, title, desc, children }: { icon: ReactNode; title: string; desc?: string; children: ReactNode }) {
  return (
    <section className="card settings-section">
      <div className="settings-aside">
        <span className="settings-icon">{icon}</span>
        <div>
          <h3>{title}</h3>
          {desc && <p className="muted small">{desc}</p>}
        </div>
      </div>
      <div className="settings-body form">{children}</div>
    </section>
  );
}

export function Settings() {
  const { toast } = useLive();
  const system = useAsync(() => api.system(), []);
  const loaded = useAsync(() => api.settings(), []);
  const [form, setForm] = useState<SettingsData>({});
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (loaded.data) setForm(loaded.data);
  }, [loaded.data]);

  const dirty = loaded.data !== undefined && JSON.stringify(form) !== JSON.stringify(loaded.data);
  const set = <K extends keyof SettingsData>(key: K, value: SettingsData[K]) => setForm((f) => ({ ...f, [key]: value }));

  const save = async () => {
    setSaving(true);
    try {
      loaded.setData(await api.saveSettings(form));
      toast('success', '设置已保存');
    } catch (e) {
      toast('error', e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const sys = system.data;
  const totalMb = sys?.memory.totalMb ?? 8192;
  const javaPath = form.javaPath ?? 'auto';
  const knownJava = javaPath === 'auto' || sys?.javas.some((j) => j.path === javaPath);
  const autoMemory = form.memoryMb == null;

  return (
    <div className="page">
      <PageHeader
        title="设置"
        actions={
          <button className="btn primary" disabled={saving || !dirty} onClick={save}>
            {saving ? '保存中…' : dirty ? '保存更改' : '已保存'}
          </button>
        }
      />

      <Section icon={<IconCup />} title="Java 运行时" desc="实例可单独覆盖。自动模式按游戏版本要求匹配。">
        <div className="input-row">
          <select value={knownJava ? javaPath : '__custom'} onChange={(e) => e.target.value !== '__custom' && set('javaPath', e.target.value)}>
            <option value="auto">自动选择</option>
            {sys?.javas.map((j) => (
              <option key={j.path} value={j.path}>Java {j.major} · {j.version} — {j.path}</option>
            ))}
            {!knownJava && <option value="__custom">{javaPath}</option>}
          </select>
          {native.available && (
            <button
              className="btn"
              onClick={async () => {
                const p = await native.pickFile('选择 Java 可执行文件').catch(() => null);
                if (p) set('javaPath', p);
              }}
            >
              浏览…
            </button>
          )}
        </div>
        {sys && (
          <div className="java-list">
            {sys.javas.length === 0 ? (
              <span className="error-text small">未检测到 Java，请安装 Java 17 或 21（推荐 Eclipse Temurin）。</span>
            ) : (
              sys.javas.map((j) => (
                <span key={j.path} className="chip" title={j.path}>Java {j.version}</span>
              ))
            )}
          </div>
        )}
      </Section>

      <Section icon={<IconChip />} title="内存" desc={`物理内存 ${formatMemory(totalMb)}，当前可用 ${formatMemory(sys?.memory.availableMb ?? 0)}`}>
        <Segmented
          value={autoMemory ? 'auto' : 'manual'}
          options={[
            { value: 'auto', label: `自动（${formatMemory(sys?.recommendedMemoryMb ?? 4096)}）` },
            { value: 'manual', label: '手动' },
          ]}
          onChange={(v) => set('memoryMb', v === 'auto' ? null : sys?.recommendedMemoryMb ?? 4096)}
        />
        {!autoMemory && (
          <div className="range-row">
            <input type="range" min={512} max={totalMb} step={256} value={form.memoryMb ?? 4096} onChange={(e) => set('memoryMb', Number(e.target.value))} />
            <span className="range-value">{formatMemory(form.memoryMb ?? 4096)}</span>
          </div>
        )}
      </Section>

      <Section icon={<IconGlobe />} title="下载" desc="BMCLAPI 在中国大陆更快，失败时自动回退官方源。">
        <Segmented
          value={form.mirror ?? 'bmclapi'}
          options={[
            { value: 'bmclapi', label: 'BMCLAPI 镜像' },
            { value: 'official', label: 'Mojang 官方' },
          ]}
          onChange={(v) => set('mirror', v)}
        />
        <label className="field">
          <span>并发连接数</span>
          <div className="range-row">
            <input type="range" min={1} max={64} value={form.downloadConcurrency ?? 16} onChange={(e) => set('downloadConcurrency', Number(e.target.value))} />
            <span className="range-value">{form.downloadConcurrency ?? 16}</span>
          </div>
        </label>
      </Section>

      <Section icon={<IconMonitor />} title="游戏窗口" desc="游戏启动时的初始分辨率。">
        <div className="input-row">
          <label className="field grow">
            <span>宽度</span>
            <input type="number" min={320} value={form.windowWidth ?? 854} onChange={(e) => set('windowWidth', Number(e.target.value))} />
          </label>
          <span className="times">×</span>
          <label className="field grow">
            <span>高度</span>
            <input type="number" min={240} value={form.windowHeight ?? 480} onChange={(e) => set('windowHeight', Number(e.target.value))} />
          </label>
        </div>
      </Section>

      <Section icon={<IconKey />} title="微软登录" desc="Azure 应用注册的 Client ID，需支持个人账户并通过 Minecraft API 审核。">
        <input
          className="mono"
          value={form.msClientId ?? ''}
          onChange={(e) => set('msClientId', e.target.value)}
          placeholder="00000000-0000-0000-0000-000000000000"
        />
      </Section>

      {sys && (
        <Section icon={<IconInfo />} title="关于" desc="StarRaft Launcher · 9 种语言协作构建">
          <dl className="about">
            <dt>系统</dt><dd>{sys.os.os} · {sys.os.arch}</dd>
            <dt>版本</dt><dd>BackEnd v{sys.backendVersion} · Core v{sys.coreVersion}</dd>
            <dt>侧车 Java</dt>
            <dd className="mono"><span className="truncate" title={sys.sidecarJava}>{sys.sidecarJava}</span></dd>
            <dt>数据目录</dt>
            <dd className="mono">
              <span className="truncate" title={sys.dataDir}>{sys.dataDir}</span>
              {native.available && (
                <button className="icon-btn" title="在访达中打开" onClick={() => native.openPath(sys.dataDir)}><IconFolder /></button>
              )}
            </dd>
          </dl>
        </Section>
      )}
    </div>
  );
}
