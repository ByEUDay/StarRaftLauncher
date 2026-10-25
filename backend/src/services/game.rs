//! 安装 / 启动 / 停止游戏的完整编排流程

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex;

use anyhow::{bail, Context};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;

use super::launcher::{AuthInfo, BuildParams};
use crate::core_ffi;
use crate::models::Account;
use crate::state::Shared;
use crate::util::{now, split_args};

/// 正在运行的游戏进程：instance_id -> 结束信号
#[derive(Default)]
pub struct GameManager {
    running: Mutex<HashMap<String, oneshot::Sender<()>>>,
    installing: Mutex<std::collections::HashSet<String>>,
}

impl GameManager {
    pub fn is_running(&self, id: &str) -> bool {
        self.running.lock().unwrap().contains_key(id)
    }

    pub fn running_ids(&self) -> Vec<String> {
        self.running.lock().unwrap().keys().cloned().collect()
    }

    pub fn stop(&self, id: &str) -> bool {
        match self.running.lock().unwrap().remove(id) {
            Some(tx) => tx.send(()).is_ok(),
            None => false,
        }
    }

    pub fn stop_all(&self) {
        for (_, tx) in self.running.lock().unwrap().drain() {
            let _ = tx.send(());
        }
    }
}

/// 安装期间的互斥守卫，防止同一实例被并发安装
struct InstallGuard<'a>(&'a GameManager, String);

impl Drop for InstallGuard<'_> {
    fn drop(&mut self) {
        self.0.installing.lock().unwrap().remove(&self.1);
    }
}

pub async fn install(s: &Shared, instance_id: &str) -> anyhow::Result<()> {
    if !s.games.installing.lock().unwrap().insert(instance_id.to_string()) {
        bail!("该实例正在安装中");
    }
    let _guard = InstallGuard(&s.games, instance_id.to_string());

    let inst = s.db.get_instance(instance_id)?.context("实例不存在")?;
    let root = &s.cfg.mc_root;
    let game_dir = s.cfg.instance_dir(&inst.id);
    let mirror = s.mirror();
    let concurrency = s.setting_u64("downloadConcurrency").unwrap_or(16) as u32;
    let stage = |name: &str| s.events.emit("install.stage", json!({ "instanceId": inst.id, "stage": name }));

    stage("version");
    s.versions.ensure_version_json(&s.downloader, root, &inst.version_id, &mirror).await?;

    stage("libraries");
    let resolved = s.launcher.resolve(root, &inst.version_id).await?;
    s.downloader
        .run(&format!("{} · 游戏本体与依赖库", inst.name), resolved.downloads, &mirror, concurrency)
        .await?;

    stage("assets");
    let assets = s.launcher.resolve_assets(root, &inst.version_id).await?;
    s.downloader.run(&format!("{} · 资源文件", inst.name), assets, &mirror, concurrency).await?;

    // 模组加载器：生成继承原版的版本配置，再补齐它额外需要的库
    let mut launch_version = inst.version_id.clone();
    if inst.loader != "vanilla" {
        stage("loader");
        let profile = s
            .loaders
            .install(&inst, root, &s.downloader, &s.launcher, &mirror, concurrency)
            .await
            .with_context(|| format!("安装 {} 失败", super::loaders::loader_name(&inst.loader)))?;
        s.db.set_profile(&inst.id, &profile)?;
        let resolved = s.launcher.resolve(root, &profile).await?;
        s.downloader
            .run(&format!("{} · 模组加载器依赖库", inst.name), resolved.downloads, &mirror, concurrency)
            .await?;
        launch_version = profile;
    }

    stage("natives");
    std::fs::create_dir_all(&game_dir)?;
    s.launcher.prepare(root, &launch_version, &game_dir).await?;

    s.db.mark_installed(&inst.id)?;
    s.events.emit("install.done", json!({ "instanceId": inst.id }));
    tracing::info!("instance {} ({}) installed", inst.name, inst.version_id);
    Ok(())
}

pub async fn launch(s: &Shared, instance_id: &str, account_id: Option<String>) -> anyhow::Result<()> {
    if s.games.is_running(instance_id) {
        bail!("该实例已在运行");
    }
    let inst = s.db.get_instance(instance_id)?.context("实例不存在")?;
    let account = match account_id {
        Some(id) => s.db.get_account(&id)?,
        None => s.db.default_account()?,
    }
    .context("请先在「账户」页面添加一个账户")?;
    let account = ensure_fresh_token(s, account).await?;

    let inst = if inst.installed {
        inst
    } else {
        install(s, instance_id).await?;
        s.db.get_instance(instance_id)?.context("实例不存在")?
    };

    s.events.emit("launch.stage", json!({ "instanceId": inst.id, "stage": "prepare" }));
    let resolved = s.launcher.resolve(&s.cfg.mc_root, inst.launch_version()).await?;
    let java = pick_java(s, inst.java_path.as_deref(), resolved.java_major).await?;
    let memory = inst
        .memory_mb
        .or_else(|| s.setting_u64("memoryMb").map(|m| m as u32))
        .unwrap_or_else(core_ffi::recommended_memory_mb);

    // 插件钩子：可修改 javaPath / memoryMb / jvmArgs / gameArgs，或设置 cancel
    let ctx = s.plugins.emit(
        "launch.before",
        json!({
            "instance": inst,
            "versionId": resolved.version_id,
            "javaMajor": resolved.java_major,
            "account": { "username": account.username, "uuid": account.uuid, "kind": account.kind },
            "javaPath": java,
            "memoryMb": memory,
            "jvmArgs": split_args(inst.jvm_args.as_deref().unwrap_or("")),
            "gameArgs": split_args(inst.game_args.as_deref().unwrap_or("")),
        }),
    );
    if ctx.get("cancel").and_then(Value::as_bool).unwrap_or(false) {
        let reason = ctx.get("cancelReason").and_then(Value::as_str).unwrap_or("被插件取消");
        bail!("启动已取消: {reason}");
    }

    let game_dir = s.cfg.instance_dir(&inst.id);
    let params = BuildParams {
        root: s.cfg.mc_root.to_string_lossy().into_owned(),
        version_id: inst.launch_version().to_string(),
        game_dir: game_dir.to_string_lossy().into_owned(),
        java_path: ctx.get("javaPath").and_then(Value::as_str).unwrap_or(&java).to_string(),
        memory_mb: ctx.get("memoryMb").and_then(Value::as_u64).map(|m| m as u32).unwrap_or(memory),
        jvm_args: string_list(&ctx["jvmArgs"]),
        game_args: string_list(&ctx["gameArgs"]),
        width: s.setting_u64("windowWidth").unwrap_or(854) as u32,
        height: s.setting_u64("windowHeight").unwrap_or(480) as u32,
        account: AuthInfo {
            username: account.username.clone(),
            uuid: account.uuid.clone(),
            access_token: account.access_token.clone().unwrap_or_else(|| "0".into()),
            user_type: if account.kind == "microsoft" { "msa".into() } else { "legacy".into() },
        },
        launcher_name: "StarRaftLauncher".into(),
        launcher_version: env!("CARGO_PKG_VERSION").into(),
    };
    let cmd = s.launcher.build_command(&params).await?;
    let (program, args) = cmd.command.split_first().context("启动命令为空")?;

    tracing::info!("launching {}: {} {} args", inst.name, program, args.len());
    let mut child = Command::new(program)
        .args(args)
        .current_dir(&cmd.working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("无法启动 Java: {program}"))?;

    let started = now();
    let session_id = s.db.start_session(&inst.id, &account.id, started)?;
    s.db.touch_instance(&inst.id, started)?;

    let (kill_tx, kill_rx) = oneshot::channel();
    s.games.running.lock().unwrap().insert(inst.id.clone(), kill_tx);
    s.events.emit("game.started", json!({ "instanceId": inst.id, "pid": child.id() }));

    if let Some(out) = child.stdout.take() {
        pipe_logs(s.clone(), inst.id.clone(), "stdout", out);
    }
    if let Some(err) = child.stderr.take() {
        pipe_logs(s.clone(), inst.id.clone(), "stderr", err);
    }

    let s = s.clone();
    let id = inst.id.clone();
    tokio::spawn(async move {
        let status = tokio::select! {
            status = child.wait() => status.ok(),
            _ = kill_rx => {
                let _ = child.kill().await;
                child.wait().await.ok()
            }
        };
        let code = status.and_then(|st| st.code());
        let ended = now();
        s.games.running.lock().unwrap().remove(&id);
        if let Err(e) = s.db.end_session(session_id, ended, code) {
            tracing::warn!("end_session failed: {e:#}");
        }
        s.events.emit("game.exit", json!({ "instanceId": id, "exitCode": code }));
        s.plugins.emit(
            "launch.after",
            json!({ "instanceId": id, "exitCode": code, "durationSecs": ended - started }),
        );
    });
    Ok(())
}

fn pipe_logs(s: Shared, id: String, stream: &'static str, reader: impl AsyncRead + Unpin + Send + 'static) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            s.events.emit("game.log", json!({ "instanceId": id, "stream": stream, "line": line }));
        }
    });
}

fn string_list(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// 微软账户令牌在 5 分钟内过期时自动刷新
async fn ensure_fresh_token(s: &Shared, account: Account) -> anyhow::Result<Account> {
    if account.kind != "microsoft" || account.expires_at.unwrap_or(0) > now() + 300 {
        return Ok(account);
    }
    let refresh = account.refresh_token.clone().context("登录已过期，请重新登录微软账户")?;
    let client_id = s.ms_client_id().context("未配置微软 OAuth Client ID")?;
    s.events.emit("account.refreshing", json!({ "id": account.id }));
    let mut fresh = s.accounts.microsoft_refresh(&client_id, &refresh).await?.into_account(account.created_at);
    fresh.is_default = account.is_default;
    s.db.upsert_account(&fresh)?;
    Ok(fresh)
}

/// 选择 Java：实例设置 > 全局设置 > 自动匹配（主版本相同优先，其次最接近的更高版本）
async fn pick_java(s: &Shared, instance_java: Option<&str>, required: u32) -> anyhow::Result<String> {
    let configured = instance_java
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .or_else(|| s.setting_str("javaPath").filter(|p| !p.is_empty() && p != "auto"));
    if let Some(path) = configured {
        if !Path::new(&path).exists() {
            bail!("设置的 Java 不存在: {path}");
        }
        return Ok(prefer_javaw(path));
    }

    let javas = tokio::task::spawn_blocking(core_ffi::detect_java).await?;
    let best = javas
        .iter()
        .filter(|j| j.major == required)
        .chain(javas.iter().filter(|j| j.major > required).min_by_key(|j| j.major))
        .next()
        .with_context(|| format!("未找到 Java {required}，请安装后在设置中指定"))?;
    Ok(prefer_javaw(best.path.clone()))
}

/// Windows 上用 javaw.exe 启动，避免弹出控制台窗口
fn prefer_javaw(path: String) -> String {
    if cfg!(windows) && path.to_lowercase().ends_with("java.exe") {
        let javaw = format!("{}w.exe", &path[..path.len() - 4]);
        if Path::new(&javaw).exists() {
            return javaw;
        }
    }
    path
}
