import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { Instance, InstanceInput, LoaderKind, LoaderVersion, VersionEntry } from '../api/types';
import { LOADER_NAMES } from '../api/types';
import { native } from '../api/native';
import { LoaderIcon } from './LoaderBadge';
import { Modal } from './ui';

interface Props {
  instance?: Instance;
  maxMemoryMb: number;
  onClose: () => void;
  onSaved: (inst: Instance) => void;
}

const LOADERS: { id: LoaderKind; hint: string }[] = [
  { id: 'vanilla', hint: '官方原版' },
  { id: 'fabric', hint: '轻量 · 更新快' },
  { id: 'quilt', hint: '兼容 Fabric' },
  { id: 'forge', hint: '经典模组生态' },
  { id: 'neoforge', hint: 'Forge 社区分支' },
];

export function InstanceForm({ instance, maxMemoryMb, onClose, onSaved }: Props) {
  const editing = Boolean(instance);
  const [name, setName] = useState(instance?.name ?? '');
  const [nameTouched, setNameTouched] = useState(editing);
  const [versionId, setVersionId] = useState(instance?.versionId ?? '');
  const [loader, setLoader] = useState<LoaderKind>(instance?.loader ?? 'vanilla');
  const [loaderVersion, setLoaderVersion] = useState(instance?.loaderVersion ?? '');
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersion[] | null>(null);
  const [loaderError, setLoaderError] = useState<string | null>(null);
  const [memoryMb, setMemoryMb] = useState<number | null>(instance?.memoryMb ?? null);
  const [javaPath, setJavaPath] = useState(instance?.javaPath ?? '');
  const [jvmArgs, setJvmArgs] = useState(instance?.jvmArgs ?? '');
  const [gameArgs, setGameArgs] = useState(instance?.gameArgs ?? '');
  const [snapshots, setSnapshots] = useState(false);
  const [versions, setVersions] = useState<VersionEntry[]>([]);
  const [versionError, setVersionError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (editing) return;
    let cancelled = false;
    setVersionError(null);
    api
      .versions(snapshots)
      .then((list) => {
        if (cancelled) return;
        setVersions(list.versions);
        setVersionId((current) => current || list.latest.release);
      })
      .catch((e) => !cancelled && setVersionError(e.message));
    return () => {
      cancelled = true;
    };
  }, [editing, snapshots]);

  // 切换游戏版本或加载器时，重新获取可用的加载器版本
  useEffect(() => {
    if (editing || loader === 'vanilla' || !versionId) {
      setLoaderVersions(null);
      return;
    }
    let cancelled = false;
    setLoaderVersions(null);
    setLoaderError(null);
    setLoaderVersion('');
    api
      .loaderVersions(loader, versionId)
      .then((list) => {
        if (cancelled) return;
        setLoaderVersions(list);
        const pick = list.find((v) => v.recommended) ?? list.find((v) => v.stable) ?? list[0];
        setLoaderVersion(pick?.id ?? '');
      })
      .catch((e) => !cancelled && setLoaderError(e.message));
    return () => {
      cancelled = true;
    };
  }, [editing, loader, versionId]);

  useEffect(() => {
    if (nameTouched || !versionId) return;
    setName(loader === 'vanilla' ? `Minecraft ${versionId}` : `${versionId} · ${LOADER_NAMES[loader]}`);
  }, [nameTouched, versionId, loader]);

  const needsLoaderVersion = !editing && loader !== 'vanilla';
  const canSubmit = !saving && name.trim() && versionId && (!needsLoaderVersion || loaderVersion);

  const submit = async () => {
    setSaving(true);
    setError(null);
    const input: InstanceInput = {
      name: name.trim(),
      versionId,
      loader,
      loaderVersion: loader === 'vanilla' ? null : loaderVersion,
      memoryMb,
      javaPath: javaPath.trim() || null,
      jvmArgs: jvmArgs.trim() || null,
      gameArgs: gameArgs.trim() || null,
      icon: instance?.icon ?? null,
    };
    try {
      const saved = editing ? await api.updateInstance(instance!.id, input) : await api.createInstance(input);
      onSaved(saved);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const browseJava = async () => {
    const picked = await native.pickFile('选择 Java 可执行文件').catch(() => null);
    if (picked) setJavaPath(picked);
  };

  const shortLoaderVersion = (id: string) => loaderVersions?.find((v) => v.id === id)?.version ?? id;

  return (
    <Modal
      title={editing ? `编辑 ${instance!.name}` : '新建实例'}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>取消</button>
          <button className="btn primary" disabled={!canSubmit} onClick={submit}>
            {saving ? '保存中…' : editing ? '保存' : '创建'}
          </button>
        </>
      }
    >
      <div className="form">
        {editing ? (
          <div className="form-summary">
            <LoaderIcon loader={loader} size={34} />
            <div>
              <div className="form-summary-title">Minecraft {versionId}</div>
              <div className="muted small">
                {LOADER_NAMES[loader]}
                {instance?.loaderVersion ? ` ${instance.loaderVersion}` : ''} · 版本与加载器创建后不可修改
              </div>
            </div>
          </div>
        ) : (
          <>
            <label className="field">
              <span>
                游戏版本
                <label className="inline-check">
                  <input type="checkbox" checked={snapshots} onChange={(e) => setSnapshots(e.target.checked)} /> 显示快照
                </label>
              </span>
              {versionError ? (
                <div className="error-text">无法获取版本列表：{versionError}</div>
              ) : (
                <select value={versionId} onChange={(e) => setVersionId(e.target.value)}>
                  {versions.length === 0 && <option>加载中…</option>}
                  {versions.map((v) => (
                    <option key={v.id} value={v.id}>
                      {v.id} {v.type !== 'release' ? `（${v.type}）` : ''}
                    </option>
                  ))}
                </select>
              )}
            </label>

            <div className="field">
              <span>模组加载器</span>
              <div className="loader-picker" role="radiogroup">
                {LOADERS.map((l) => (
                  <button
                    key={l.id}
                    role="radio"
                    aria-checked={loader === l.id}
                    className={`loader-option ${loader === l.id ? 'active' : ''}`}
                    onClick={() => setLoader(l.id)}
                  >
                    <LoaderIcon loader={l.id} size={30} />
                    <span className="loader-option-name">{LOADER_NAMES[l.id]}</span>
                    <span className="loader-option-hint">{l.hint}</span>
                  </button>
                ))}
              </div>
            </div>

            {loader !== 'vanilla' && (
              <label className="field">
                <span>{LOADER_NAMES[loader]} 版本</span>
                {loaderError ? (
                  <div className="error-text small">无法获取 {LOADER_NAMES[loader]} 版本：{loaderError}</div>
                ) : loaderVersions === null ? (
                  <div className="muted small loading-row"><span className="spinner small" /> 正在获取可用版本…</div>
                ) : loaderVersions.length === 0 ? (
                  <div className="notice small">{LOADER_NAMES[loader]} 暂不支持 Minecraft {versionId}，请换一个游戏版本。</div>
                ) : (
                  <select value={loaderVersion} onChange={(e) => setLoaderVersion(e.target.value)}>
                    {loaderVersions.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.version}
                        {v.recommended ? ' · 推荐' : ''}
                        {!v.stable ? ' · 测试版' : ''}
                      </option>
                    ))}
                  </select>
                )}
                {loaderVersion && loaderVersions && loaderVersions.length > 0 && (
                  <div className="muted tiny">将安装 {LOADER_NAMES[loader]} {shortLoaderVersion(loaderVersion)}，共 {loaderVersions.length} 个可选版本</div>
                )}
              </label>
            )}
          </>
        )}

        <label className="field">
          <span>名称</span>
          <input
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              setNameTouched(true);
            }}
            placeholder="我的生存世界"
          />
        </label>

        <label className="field">
          <span>
            最大内存
            <label className="inline-check">
              <input type="checkbox" checked={memoryMb === null} onChange={(e) => setMemoryMb(e.target.checked ? null : 4096)} />{' '}
              跟随全局设置
            </label>
          </span>
          {memoryMb !== null && (
            <div className="range-row">
              <input
                type="range"
                min={512}
                max={Math.max(1024, maxMemoryMb)}
                step={256}
                value={memoryMb}
                onChange={(e) => setMemoryMb(Number(e.target.value))}
              />
              <span className="range-value">{memoryMb} MB</span>
            </div>
          )}
        </label>

        <details className="advanced">
          <summary>高级选项</summary>
          <div className="form">
            <label className="field">
              <span>Java 路径（留空自动选择）</span>
              <div className="input-row">
                <input value={javaPath} onChange={(e) => setJavaPath(e.target.value)} placeholder="自动" />
                {native.available && <button className="btn ghost" onClick={browseJava}>浏览</button>}
              </div>
            </label>
            <label className="field">
              <span>额外 JVM 参数</span>
              <input value={jvmArgs} onChange={(e) => setJvmArgs(e.target.value)} placeholder="-XX:+UseZGC" />
            </label>
            <label className="field">
              <span>额外游戏参数</span>
              <input value={gameArgs} onChange={(e) => setGameArgs(e.target.value)} placeholder="--server mc.example.com" />
            </label>
          </div>
        </details>

        {error && <div className="error-text">{error}</div>}
      </div>
    </Modal>
  );
}
