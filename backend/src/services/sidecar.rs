//! JVM 侧车进程（Java 启动适配层 / Kotlin 账户模块）的 JSON-RPC 客户端。
//!
//! 协议：每行一个 JSON 对象（UTF-8，`\n` 分隔）
//!   请求   {"jsonrpc":"2.0","id":1,"method":"resolve","params":{...}}
//!   响应   {"jsonrpc":"2.0","id":1,"result":...} 或 {"id":1,"error":{"code":-32000,"message":"..."}}
//!   通知   {"jsonrpc":"2.0","method":"account.deviceCode","params":{...}}  -> 转发为前端事件
//!
//! 侧车按需懒启动；崩溃后下一次调用会自动重启。stdin 关闭即通知侧车退出。

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::{anyhow, bail, Context};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};

use crate::events::EventBus;

type Pending = Arc<StdMutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

struct Conn {
    child: Child,
    stdin: ChildStdin,
    pending: Pending,
}

pub struct Sidecar {
    name: &'static str,
    program: String,
    args: Vec<String>,
    events: EventBus,
    conn: Mutex<Option<Conn>>,
    next_id: AtomicU64,
}

impl Sidecar {
    pub fn jvm(name: &'static str, java: &str, jar: &std::path::Path, events: EventBus) -> Self {
        Self {
            name,
            program: java.to_string(),
            args: vec![
                "-Xmx256m".into(),
                "-Dfile.encoding=UTF-8".into(),
                "-Dstdout.encoding=UTF-8".into(),
                "-jar".into(),
                jar.to_string_lossy().into_owned(),
            ],
            events,
            conn: Mutex::new(None),
            next_id: AtomicU64::new(1),
        }
    }

    pub async fn call_as<T: DeserializeOwned>(&self, method: &str, params: Value) -> anyhow::Result<T> {
        let v = self.call(method, params).await?;
        serde_json::from_value(v).with_context(|| format!("{} 返回了无法解析的结果 ({method})", self.name))
    }

    pub async fn call(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let rx = {
            let mut guard = self.conn.lock().await;
            let alive = match guard.as_mut() {
                Some(c) => matches!(c.child.try_wait(), Ok(None)),
                None => false,
            };
            if !alive {
                *guard = Some(self.spawn().with_context(|| format!("无法启动 {} 模块 ({})", self.name, self.program))?);
            }
            let conn = guard.as_mut().expect("conn just ensured");

            let (tx, rx) = oneshot::channel();
            conn.pending.lock().unwrap().insert(id, tx);

            let mut line = serde_json::to_vec(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
            line.push(b'\n');
            if let Err(e) = conn.stdin.write_all(&line).await {
                conn.pending.lock().unwrap().remove(&id);
                return Err(e.into());
            }
            conn.stdin.flush().await?;
            rx
        };

        match rx.await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(msg)) => Err(anyhow!(msg)),
            Err(_) => bail!("{} 模块意外退出", self.name),
        }
    }

    fn spawn(&self) -> anyhow::Result<Conn> {
        tracing::info!("spawning sidecar {}: {} {:?}", self.name, self.program, self.args);
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().context("sidecar stdin")?;
        let stdout = child.stdout.take().context("sidecar stdout")?;
        let stderr = child.stderr.take().context("sidecar stderr")?;
        let pending: Pending = Default::default();

        let reader_pending = pending.clone();
        let events = self.events.clone();
        let name = self.name;
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                    tracing::debug!("[{name}] non-json stdout: {line}");
                    continue;
                };
                if let Some(id) = msg.get("id").and_then(Value::as_u64) {
                    let tx = reader_pending.lock().unwrap().remove(&id);
                    if let Some(tx) = tx {
                        let res = match msg.get("error") {
                            Some(err) if !err.is_null() => Err(err
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("未知错误")
                                .to_string()),
                            _ => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
                        };
                        let _ = tx.send(res);
                    }
                } else if let Some(method) = msg.get("method").and_then(Value::as_str) {
                    events.emit(method, msg.get("params").cloned().unwrap_or(Value::Null));
                }
            }
            // 进程退出：丢弃所有等待者，调用方会得到“意外退出”错误
            reader_pending.lock().unwrap().clear();
            tracing::warn!("sidecar {name} stdout closed");
        });

        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(target: "sidecar", "[{name}] {line}");
            }
        });

        Ok(Conn { child, stdin, pending })
    }

    pub async fn stop(&self) {
        if let Some(mut conn) = self.conn.lock().await.take() {
            drop(conn.stdin);
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), conn.child.wait()).await;
            let _ = conn.child.kill().await;
        }
    }
}
