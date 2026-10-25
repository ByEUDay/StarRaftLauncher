//! Java 启动适配层（launcher/）的类型化封装

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::download::DlTask;
use super::sidecar::Sidecar;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
    pub version_id: String,
    pub java_major: u32,
    #[allow(dead_code)]
    pub main_class: String,
    pub downloads: Vec<DlTask>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchCommand {
    pub command: Vec<String>,
    pub working_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthInfo {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildParams {
    pub root: String,
    pub version_id: String,
    pub game_dir: String,
    pub java_path: String,
    pub memory_mb: u32,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub account: AuthInfo,
    pub launcher_name: String,
    pub launcher_version: String,
}

pub struct LauncherAdapter {
    rpc: Sidecar,
}

impl LauncherAdapter {
    pub fn new(rpc: Sidecar) -> Self {
        Self { rpc }
    }

    pub(super) fn rpc(&self) -> &Sidecar {
        &self.rpc
    }

    /// 解析客户端 jar、依赖库、natives 与资源索引的下载列表
    pub async fn resolve(&self, root: &Path, version_id: &str) -> anyhow::Result<Resolved> {
        self.rpc.call_as("resolve", json!({ "root": root, "versionId": version_id })).await
    }

    /// 读取资源索引，返回资源对象下载列表
    pub async fn resolve_assets(&self, root: &Path, version_id: &str) -> anyhow::Result<Vec<DlTask>> {
        self.rpc.call_as("resolveAssets", json!({ "root": root, "versionId": version_id })).await
    }

    /// 解压 natives、处理旧版虚拟资源目录
    pub async fn prepare(&self, root: &Path, version_id: &str, game_dir: &Path) -> anyhow::Result<()> {
        self.rpc
            .call("prepare", json!({ "root": root, "versionId": version_id, "gameDir": game_dir }))
            .await?;
        Ok(())
    }

    pub async fn build_command(&self, params: &BuildParams) -> anyhow::Result<LaunchCommand> {
        self.rpc.call_as("buildCommand", serde_json::to_value(params)?).await
    }

    pub async fn stop(&self) {
        self.rpc.stop().await;
    }
}
