use std::env;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::core_ffi;

/// 运行时配置。全部来自环境变量（由 C# 宿主或 scripts/dev.sh 设置），并提供发布目录下的默认值：
///
/// ```text
/// <STARRAFT_HOME>/bin/srbackend    <- 本程序
/// <STARRAFT_HOME>/bin/srdl         <- Go 下载引擎
/// <STARRAFT_HOME>/lib/*.jar        <- Java / Kotlin 侧车
/// <STARRAFT_HOME>/web/             <- FrontEnd 构建产物
/// <STARRAFT_HOME>/plugins/         <- 内置 Lua 插件
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub token: String,
    pub watch_stdin: bool,
    pub data_dir: PathBuf,
    pub mc_root: PathBuf,
    pub instances_dir: PathBuf,
    pub web_dir: Option<PathBuf>,
    pub builtin_plugin_dir: PathBuf,
    pub user_plugin_dir: PathBuf,
    pub download_bin: PathBuf,
    pub launcher_jar: PathBuf,
    pub account_jar: PathBuf,
    pub sidecar_java: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let home = match env::var_os("STARRAFT_HOME") {
            Some(h) => PathBuf::from(h),
            None => default_home()?,
        };
        let data_dir = match env::var_os("STARRAFT_DATA_DIR") {
            Some(d) => PathBuf::from(d),
            None => dirs::data_dir().context("无法确定数据目录")?.join("StarRaftLauncher"),
        };

        let token = env::var("STARRAFT_TOKEN").unwrap_or_else(|_| {
            let t = crate::util::random_hex(24);
            tracing::warn!("STARRAFT_TOKEN 未设置，已生成临时令牌: {t}");
            t
        });

        let web_dir = env_path("STARRAFT_WEB_DIR").or_else(|| Some(home.join("web"))).filter(|p| p.join("index.html").is_file());

        Ok(Self {
            port: env::var("STARRAFT_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(0),
            token,
            watch_stdin: env::var("STARRAFT_WATCH_STDIN").map(|v| v == "1").unwrap_or(false),
            mc_root: env_path("STARRAFT_MC_ROOT").unwrap_or_else(|| data_dir.join("minecraft")),
            instances_dir: data_dir.join("instances"),
            user_plugin_dir: data_dir.join("plugins"),
            builtin_plugin_dir: env_path("STARRAFT_PLUGIN_DIR").unwrap_or_else(|| home.join("plugins")),
            download_bin: env_path("STARRAFT_DOWNLOAD_BIN").unwrap_or_else(|| home.join("bin").join(exe("srdl"))),
            launcher_jar: env_path("STARRAFT_LAUNCHER_JAR").unwrap_or_else(|| home.join("lib").join("launcher-adapter.jar")),
            account_jar: env_path("STARRAFT_ACCOUNT_JAR").unwrap_or_else(|| home.join("lib").join("account.jar")),
            sidecar_java: env::var("STARRAFT_SIDECAR_JAVA").unwrap_or_else(|_| pick_sidecar_java()),
            web_dir,
            data_dir,
        })
    }

    pub fn plugin_dirs(&self) -> Vec<PathBuf> {
        vec![self.builtin_plugin_dir.clone(), self.user_plugin_dir.clone()]
    }

    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.instances_dir.join(id)
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn default_home() -> anyhow::Result<PathBuf> {
    let exe = env::current_exe()?;
    let bin_dir = exe.parent().context("无效的可执行文件路径")?;
    Ok(bin_dir.parent().unwrap_or(Path::new(".")).to_path_buf())
}

fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}

/// 侧车（Java 适配层 / Kotlin 账户模块）需要 Java 17+，优先使用 Core 探测到的最新版本
fn pick_sidecar_java() -> String {
    core_ffi::detect_java()
        .into_iter()
        .filter(|j| j.major >= 17)
        .max_by_key(|j| j.major)
        .map(|j| j.path)
        .unwrap_or_else(|| "java".into())
}
