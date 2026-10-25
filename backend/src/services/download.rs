//! Go 下载引擎（srdl）客户端。srdl 作为本地 HTTP 守护进程运行，见 download/README.md。

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::config::Config;
use crate::events::EventBus;

const TOKEN_HEADER: &str = "X-StarRaft-Token";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DlTask {
    pub url: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BatchStatus {
    pub id: String,
    pub label: String,
    pub state: String,
    pub total: u64,
    pub completed: u64,
    pub failed: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub errors: Vec<String>,
}

pub struct DownloadClient {
    base: String,
    token: String,
    http: reqwest::Client,
    events: EventBus,
    child: Mutex<Option<Child>>,
    batches: Mutex<HashMap<String, BatchStatus>>,
}

impl DownloadClient {
    pub async fn start(cfg: &Config, events: EventBus) -> anyhow::Result<Self> {
        let token = crate::util::random_hex(16);
        let mut child = Command::new(&cfg.download_bin)
            .args(["--port", "0"])
            .env("STARRAFT_DL_TOKEN", &token)
            .stdin(Stdio::piped()) // 保持打开；srdl 在 stdin EOF 时自行退出
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("无法启动下载引擎: {}", cfg.download_bin.display()))?;

        let mut lines = BufReader::new(child.stdout.take().context("srdl stdout")?).lines();
        let port: u16 = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(line) = lines.next_line().await? {
                if let Some(p) = line.strip_prefix("LISTEN ") {
                    return Ok::<u16, anyhow::Error>(p.trim().parse()?);
                }
            }
            bail!("下载引擎在握手前退出")
        })
        .await
        .context("下载引擎握手超时")??;

        tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::info!(target: "srdl", "{line}");
                }
            });
        }

        tracing::info!("download engine ready on port {port}");
        Ok(Self {
            base: format!("http://127.0.0.1:{port}"),
            token,
            http: reqwest::Client::builder().no_proxy().build()?,
            events,
            child: Mutex::new(Some(child)),
            batches: Mutex::new(HashMap::new()),
        })
    }

    /// 提交一批任务并等待完成，期间以 `download.progress` 事件推送进度
    pub async fn run(&self, label: &str, tasks: Vec<DlTask>, mirror: &str, concurrency: u32) -> anyhow::Result<BatchStatus> {
        if tasks.is_empty() {
            return Ok(BatchStatus { label: label.into(), state: "done".into(), ..Default::default() });
        }

        let created: Value = self
            .http
            .post(format!("{}/v1/batches", self.base))
            .header(TOKEN_HEADER, &self.token)
            .json(&json!({ "label": label, "tasks": tasks, "mirror": mirror, "concurrency": concurrency }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let id = created["id"].as_str().context("下载引擎返回无效批次 ID")?.to_string();

        loop {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let status: BatchStatus = self
                .http
                .get(format!("{}/v1/batches/{id}", self.base))
                .header(TOKEN_HEADER, &self.token)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            self.batches.lock().unwrap().insert(id.clone(), status.clone());
            self.events.emit("download.progress", &status);

            match status.state.as_str() {
                "running" | "pending" => continue,
                "done" => return Ok(status),
                "cancelled" => bail!("下载已取消"),
                _ => bail!(
                    "{} 个文件下载失败: {}",
                    status.failed,
                    status.errors.iter().take(3).cloned().collect::<Vec<_>>().join("; ")
                ),
            }
        }
    }

    pub async fn cancel(&self, id: &str) -> anyhow::Result<()> {
        self.http
            .delete(format!("{}/v1/batches/{id}", self.base))
            .header(TOKEN_HEADER, &self.token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub fn batches(&self) -> Vec<BatchStatus> {
        self.batches.lock().unwrap().values().cloned().collect()
    }

    pub fn stop(&self) {
        // 丢弃 Child（kill_on_drop）同时关闭其 stdin
        self.child.lock().unwrap().take();
    }
}
