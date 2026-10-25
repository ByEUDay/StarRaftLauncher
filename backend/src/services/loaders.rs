//! 模组加载器：版本列表与安装
//!
//! - Fabric / Quilt：官方 meta 直接提供带 inheritsFrom 的版本配置，下载即可
//! - Forge / NeoForge：下载安装器，由 Java 适配层解包并执行客户端处理器

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use super::download::{DlTask, DownloadClient};
use super::launcher::LauncherAdapter;
use crate::models::Instance;

pub const LOADERS: &[&str] = &["vanilla", "fabric", "quilt", "forge", "neoforge"];

const FABRIC_META: &str = "https://meta.fabricmc.net";
const FABRIC_META_BMCLAPI: &str = "https://bmclapi2.bangbang93.com/fabric-meta";
const QUILT_META: &str = "https://meta.quiltmc.org";
const FORGE_FILES: &str = "https://files.minecraftforge.net/net/minecraftforge/forge";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const NEOFORGE_API: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged";
const CACHE_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersion {
    /// 安装时使用的版本标识（Forge 为完整构件版本，如 1.20.1-47.4.28）
    pub id: String,
    /// 展示用版本号
    pub version: String,
    pub stable: bool,
    pub recommended: bool,
}

pub struct LoaderService {
    http: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, Value)>>,
}

impl LoaderService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("StarRaftLauncher/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(30))
                .build()?,
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// GET JSON，带 10 分钟缓存；依次尝试多个地址
    async fn get_json(&self, urls: &[String]) -> anyhow::Result<Value> {
        let key = urls[0].clone();
        if let Some((at, v)) = self.cache.lock().await.get(&key) {
            if at.elapsed() < CACHE_TTL {
                return Ok(v.clone());
            }
        }
        let mut last = None;
        for url in urls {
            match async { Ok::<Value, anyhow::Error>(self.http.get(url).send().await?.error_for_status()?.json().await?) }.await {
                Ok(v) => {
                    self.cache.lock().await.insert(key, (Instant::now(), v.clone()));
                    return Ok(v);
                }
                Err(e) => {
                    tracing::warn!("GET {url} failed: {e:#}");
                    last = Some(e);
                }
            }
        }
        Err(last.unwrap()).context("获取加载器版本列表失败")
    }

    fn fabric_urls(path: &str, mirror: &str) -> Vec<String> {
        let official = format!("{FABRIC_META}{path}");
        let mirrored = format!("{FABRIC_META_BMCLAPI}{path}");
        if mirror == "bmclapi" { vec![mirrored, official] } else { vec![official, mirrored] }
    }

    pub async fn versions(&self, loader: &str, mc: &str, mirror: &str) -> anyhow::Result<Vec<LoaderVersion>> {
        match loader {
            "fabric" => {
                let v = self.get_json(&Self::fabric_urls(&format!("/v2/versions/loader/{mc}"), mirror)).await?;
                Ok(meta_list(&v, |e| e["loader"]["stable"].as_bool().unwrap_or(false)))
            }
            "quilt" => {
                let v = self.get_json(&[format!("{QUILT_META}/v3/versions/loader/{mc}")]).await?;
                // Quilt meta 不保证顺序，排序后再标记推荐版本
                let mut list = meta_list(&v, |e| !e["loader"]["version"].as_str().unwrap_or("").contains('-'));
                list.sort_by(|a, b| compare_versions(&b.version, &a.version));
                list.iter_mut().for_each(|l| l.recommended = false);
                if let Some(first) = list.iter_mut().find(|l| l.stable) {
                    first.recommended = true;
                }
                Ok(list)
            }
            "forge" => {
                let meta = self.get_json(&[format!("{FORGE_FILES}/maven-metadata.json")]).await?;
                let promos = self.get_json(&[format!("{FORGE_FILES}/promotions_slim.json")]).await.unwrap_or(Value::Null);
                Ok(forge_list(&meta, &promos, mc))
            }
            "neoforge" => {
                let main = self.get_json(&[format!("{NEOFORGE_API}/neoforge")]).await?;
                let mut list = neoforge_list(&main, mc);
                if mc == "1.20.1" {
                    // 1.20.1 时期 NeoForge 仍以 net.neoforged:forge 发布
                    let legacy = self.get_json(&[format!("{NEOFORGE_API}/forge")]).await?;
                    list.extend(neoforge_legacy_list(&legacy, mc));
                }
                Ok(list)
            }
            "vanilla" => Ok(Vec::new()),
            other => bail!("不支持的加载器: {other}"),
        }
    }

    /// 安装加载器，返回生成的版本配置 ID。调用前原版本体须已安装（处理器需要客户端 jar）。
    pub async fn install(
        &self,
        inst: &Instance,
        root: &Path,
        dl: &DownloadClient,
        launcher: &LauncherAdapter,
        mirror: &str,
        concurrency: u32,
    ) -> anyhow::Result<String> {
        let mc = &inst.version_id;
        let version = inst.loader_version.as_deref().context("实例缺少加载器版本")?;
        match inst.loader.as_str() {
            "fabric" | "quilt" => {
                let path = format!("/v2/versions/loader/{mc}/{version}/profile/json");
                let urls = if inst.loader == "fabric" {
                    Self::fabric_urls(&path, mirror)
                } else {
                    vec![format!("{QUILT_META}/v3/versions/loader/{mc}/{version}/profile/json")]
                };
                let mut profile = None;
                for url in &urls {
                    match self.http.get(url).send().await.and_then(|r| r.error_for_status()) {
                        Ok(r) => {
                            profile = Some(r.json::<Value>().await?);
                            break;
                        }
                        Err(e) => tracing::warn!("GET {url} failed: {e:#}"),
                    }
                }
                let profile = profile.context("下载加载器配置失败")?;
                let id = profile["id"].as_str().context("加载器配置缺少 id")?.to_string();
                let dir = root.join("versions").join(&id);
                std::fs::create_dir_all(&dir)?;
                std::fs::write(dir.join(format!("{id}.json")), serde_json::to_vec_pretty(&profile)?)?;
                Ok(id)
            }
            "forge" | "neoforge" => {
                let url = installer_url(&inst.loader, mc, version);
                let file = url.rsplit('/').next().unwrap_or("installer.jar");
                let installer = root.join(".installers").join(file);
                dl.run(
                    &format!("{} · {} 安装器", inst.name, loader_name(&inst.loader)),
                    vec![DlTask { url, path: installer.to_string_lossy().into_owned(), sha1: None, size: None }],
                    mirror,
                    1,
                )
                .await?;

                let extracted = launcher.installer_extract(root, &installer).await?;
                dl.run(&format!("{} · 加载器依赖", inst.name), extracted.downloads, mirror, concurrency).await?;
                if extracted.processors > 0 {
                    launcher.installer_run(root, &installer, &inst.id).await?;
                }
                Ok(extracted.profile_id)
            }
            other => bail!("不支持的加载器: {other}"),
        }
    }
}

pub fn loader_name(loader: &str) -> &'static str {
    match loader {
        "fabric" => "Fabric",
        "quilt" => "Quilt",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        _ => "原版",
    }
}

fn installer_url(loader: &str, mc: &str, version: &str) -> String {
    match loader {
        "forge" => format!("{FORGE_MAVEN}/{version}/forge-{version}-installer.jar"),
        // 1.20.1 的 NeoForge 版本形如 1.20.1-47.1.106，构件名为 forge
        _ if mc == "1.20.1" && version.starts_with("1.20.1-") => {
            format!("{NEOFORGE_MAVEN}/forge/{version}/forge-{version}-installer.jar")
        }
        _ => format!("{NEOFORGE_MAVEN}/neoforge/{version}/neoforge-{version}-installer.jar"),
    }
}

/// Fabric / Quilt meta 的 `[{loader: {version, ...}}]` 列表
fn meta_list(v: &Value, stable: impl Fn(&Value) -> bool) -> Vec<LoaderVersion> {
    let mut list: Vec<LoaderVersion> = v
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let version = e["loader"]["version"].as_str()?.to_string();
            Some(LoaderVersion { id: version.clone(), version, stable: stable(e), recommended: false })
        })
        .collect();
    if let Some(first) = list.iter_mut().find(|l| l.stable) {
        first.recommended = true;
    }
    list
}

fn forge_list(meta: &Value, promos: &Value, mc: &str) -> Vec<LoaderVersion> {
    let recommended = promos["promos"][format!("{mc}-recommended")].as_str();
    let prefix = format!("{mc}-");
    let mut list: Vec<LoaderVersion> = meta[mc]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .rev()
        .map(|full| {
            // 1.7.10-10.13.4.1614-1.7.10 → 10.13.4.1614
            let short = full.strip_prefix(&prefix).unwrap_or(full);
            let short = short.strip_suffix(&format!("-{mc}")).unwrap_or(short).to_string();
            LoaderVersion {
                recommended: Some(short.as_str()) == recommended,
                stable: true,
                id: full.to_string(),
                version: short,
            }
        })
        .collect();
    if !list.iter().any(|l| l.recommended) {
        if let Some(first) = list.first_mut() {
            first.recommended = true;
        }
    }
    list
}

/// NeoForge 版本号到 Minecraft 版本：
/// 21.1.77 → 1.21.1，21.0.5 → 1.21；26.3.0.99 → 26.3，26.1.2.5 → 26.1.2
pub fn neoforge_mc_version(version: &str) -> Option<String> {
    let base = version.split(['-', '+']).next()?;
    let parts: Vec<u32> = base.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    match parts.as_slice() {
        [major, ..] if *major == 0 => None,
        [major, minor, patch, _build] if *major >= 25 => Some(if *patch == 0 {
            format!("{major}.{minor}")
        } else {
            format!("{major}.{minor}.{patch}")
        }),
        [major, minor, _build] if *major < 25 => Some(if *minor == 0 { format!("1.{major}") } else { format!("1.{major}.{minor}") }),
        _ => None,
    }
}

fn neoforge_list(v: &Value, mc: &str) -> Vec<LoaderVersion> {
    let mut list: Vec<LoaderVersion> = v["versions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| x.as_str())
        .filter(|ver| !ver.contains("+snapshot") && neoforge_mc_version(ver).as_deref() == Some(mc))
        .rev()
        .map(|ver| LoaderVersion {
            id: ver.to_string(),
            version: ver.to_string(),
            stable: !ver.contains('-'),
            recommended: false,
        })
        .collect();
    if let Some(first) = list.iter_mut().find(|l| l.stable) {
        first.recommended = true;
    }
    list
}

fn neoforge_legacy_list(v: &Value, mc: &str) -> Vec<LoaderVersion> {
    let prefix = format!("{mc}-");
    v["versions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| x.as_str())
        .filter(|ver| ver.starts_with(&prefix))
        .rev()
        .map(|ver| LoaderVersion {
            id: ver.to_string(),
            version: ver.trim_start_matches(&prefix).to_string(),
            stable: true,
            recommended: false,
        })
        .collect()
}

/// 语义化比较：数字段逐段比较，正式版大于同号预发布版
fn compare_versions(a: &str, b: &str) -> Ordering {
    let split = |s: &str| {
        let (main, pre) = s.split_once('-').map_or((s, None), |(m, p)| (m, Some(p.to_string())));
        let nums: Vec<u64> = main.split('.').map(|p| p.parse().unwrap_or(0)).collect();
        (nums, pre)
    };
    let (na, pa) = split(a);
    let (nb, pb) = split(b);
    na.cmp(&nb).then_with(|| match (pa, pb) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => numbers(&x).cmp(&numbers(&y)).then_with(|| x.cmp(&y)),
    })
}

/// 预发布标签中的数字序列：beta.10 → [10]，rc2 → [2]
fn numbers(s: &str) -> Vec<u64> {
    s.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extracted {
    pub profile_id: String,
    pub processors: u32,
    pub downloads: Vec<DlTask>,
}

impl LauncherAdapter {
    pub async fn installer_extract(&self, root: &Path, installer: &Path) -> anyhow::Result<Extracted> {
        self.rpc().call_as("installer.extract", json!({ "root": root, "installer": installer })).await
    }

    pub async fn installer_run(&self, root: &Path, installer: &Path, tag: &str) -> anyhow::Result<()> {
        self.rpc()
            .call("installer.run", json!({ "root": root, "installer": installer, "tag": tag }))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_version_mapping() {
        assert_eq!(neoforge_mc_version("21.1.265").as_deref(), Some("1.21.1"));
        assert_eq!(neoforge_mc_version("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(neoforge_mc_version("20.4.80-beta").as_deref(), Some("1.20.4"));
        assert_eq!(neoforge_mc_version("26.3.0.99").as_deref(), Some("26.3"));
        assert_eq!(neoforge_mc_version("26.1.2.17").as_deref(), Some("26.1.2"));
        assert_eq!(neoforge_mc_version("0.25w14craftmine.3-beta"), None);
    }

    #[test]
    fn quilt_ordering() {
        let mut v = vec!["0.20.0-beta.9", "0.24.0", "0.20.0-beta.10", "0.20.0", "0.27.1"];
        v.sort_by(|a, b| compare_versions(b, a));
        assert_eq!(v, vec!["0.27.1", "0.24.0", "0.20.0", "0.20.0-beta.10", "0.20.0-beta.9"]);
    }

    #[test]
    fn forge_list_marks_recommended() {
        let meta = json!({ "1.20.1": ["1.20.1-47.4.27", "1.20.1-47.4.28"] });
        let promos = json!({ "promos": { "1.20.1-recommended": "47.4.27" } });
        let list = forge_list(&meta, &promos, "1.20.1");
        assert_eq!(list[0].id, "1.20.1-47.4.28");
        assert_eq!(list[0].version, "47.4.28");
        assert!(list[1].recommended && !list[0].recommended);
        assert!(installer_url("forge", "1.20.1", &list[0].id).ends_with("/1.20.1-47.4.28/forge-1.20.1-47.4.28-installer.jar"));
        assert!(installer_url("neoforge", "1.21.1", "21.1.265").ends_with("/neoforge/21.1.265/neoforge-21.1.265-installer.jar"));
    }
}
