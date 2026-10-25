//! Minecraft 版本清单与版本 JSON

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Context;
use serde_json::Value;
use tokio::sync::Mutex;

use super::download::{DlTask, DownloadClient};

const OFFICIAL_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const BMCLAPI_MANIFEST: &str = "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";
const CACHE_TTL: Duration = Duration::from_secs(600);

pub struct VersionService {
    http: reqwest::Client,
    cache: Mutex<Option<(Instant, Value)>>,
}

impl VersionService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("StarRaftLauncher/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(20))
                .build()?,
            cache: Mutex::new(None),
        })
    }

    pub async fn manifest(&self, mirror: &str) -> anyhow::Result<Value> {
        let mut cache = self.cache.lock().await;
        if let Some((at, v)) = cache.as_ref() {
            if at.elapsed() < CACHE_TTL {
                return Ok(v.clone());
            }
        }
        let urls = if mirror == "bmclapi" { [BMCLAPI_MANIFEST, OFFICIAL_MANIFEST] } else { [OFFICIAL_MANIFEST, BMCLAPI_MANIFEST] };
        let mut last_err = None;
        for url in urls {
            match self.fetch_json(url).await {
                Ok(v) => {
                    *cache = Some((Instant::now(), v.clone()));
                    return Ok(v);
                }
                Err(e) => {
                    tracing::warn!("manifest {url} failed: {e:#}");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap()).context("无法获取版本清单")
    }

    async fn fetch_json(&self, url: &str) -> anyhow::Result<Value> {
        Ok(self.http.get(url).send().await?.error_for_status()?.json().await?)
    }

    /// 确保 versions/<id>/<id>.json 存在。已存在（例如手动放入的 Fabric 配置）则直接使用。
    pub async fn ensure_version_json(
        &self,
        dl: &DownloadClient,
        root: &Path,
        id: &str,
        mirror: &str,
    ) -> anyhow::Result<()> {
        let path = root.join("versions").join(id).join(format!("{id}.json"));
        if path.is_file() {
            return Ok(());
        }
        let manifest = self.manifest(mirror).await?;
        let entry = manifest["versions"]
            .as_array()
            .and_then(|vs| vs.iter().find(|v| v["id"] == id))
            .with_context(|| format!("版本清单中不存在 {id}"))?;

        let task = DlTask {
            url: entry["url"].as_str().context("版本条目缺少 url")?.to_string(),
            path: path.to_string_lossy().into_owned(),
            sha1: entry["sha1"].as_str().map(str::to_string),
            size: None,
        };
        dl.run(&format!("{id} 版本信息"), vec![task], mirror, 1).await?;
        Ok(())
    }
}
