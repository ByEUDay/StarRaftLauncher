import { useState } from 'react';
import { native } from '../api/native';
import { useLive } from '../state/events';
import { Modal } from './ui';

export function DeviceCodeModal() {
  const { state, clearDeviceCode } = useLive();
  const [copied, setCopied] = useState(false);
  const code = state.deviceCode;
  if (!code) return null;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code.userCode);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  return (
    <Modal title="登录微软账户" onClose={clearDeviceCode}>
      <div className="device-code">
        <p>在浏览器中打开下面的地址，并输入代码完成授权：</p>
        <button className="link" onClick={() => native.openUrl(code.verificationUri)}>
          {code.verificationUri}
        </button>
        <div className="code-box" onClick={copy} title="点击复制">
          {code.userCode}
        </div>
        <p className="muted">{copied ? '已复制到剪贴板' : '点击代码复制'} · 授权完成后此窗口会自动关闭</p>
        <button className="btn primary" onClick={() => { void copy(); void native.openUrl(code.verificationUri); }}>
          复制代码并打开浏览器
        </button>
      </div>
    </Modal>
  );
}
