use std::sync::Arc;

use serde_json::Value;

use crate::config::Config;
use crate::db::Db;
use crate::events::EventBus;
use crate::plugins::PluginHost;
use crate::services::account::AccountService;
use crate::services::download::DownloadClient;
use crate::services::game::GameManager;
use crate::services::launcher::LauncherAdapter;
use crate::services::loaders::LoaderService;
use crate::services::sidecar::Sidecar;
use crate::services::versions::VersionService;

pub struct AppState {
    pub cfg: Config,
    pub db: Db,
    pub events: EventBus,
    pub downloader: DownloadClient,
    pub versions: VersionService,
    pub loaders: LoaderService,
    pub launcher: LauncherAdapter,
    pub accounts: AccountService,
    pub plugins: PluginHost,
    pub games: GameManager,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub async fn init(cfg: Config) -> anyhow::Result<Shared> {
        for dir in [&cfg.data_dir, &cfg.mc_root, &cfg.instances_dir, &cfg.user_plugin_dir] {
            std::fs::create_dir_all(dir)?;
        }

        let db = Db::open(&cfg.data_dir.join("starraft.db"))?;
        let events = EventBus::new();
        let downloader = DownloadClient::start(&cfg, events.clone()).await?;
        let launcher = LauncherAdapter::new(Sidecar::jvm("launcher", &cfg.sidecar_java, &cfg.launcher_jar, events.clone()));
        let accounts = AccountService::new(Sidecar::jvm("account", &cfg.sidecar_java, &cfg.account_jar, events.clone()));
        let plugins = PluginHost::load(&cfg.plugin_dirs(), &db, events.clone());

        let state = Arc::new(Self {
            versions: VersionService::new()?,
            loaders: LoaderService::new()?,
            games: GameManager::default(),
            cfg,
            db,
            events,
            downloader,
            launcher,
            accounts,
            plugins,
        });
        state.plugins.emit("app.ready", serde_json::json!({ "version": env!("CARGO_PKG_VERSION") }));
        Ok(state)
    }

    pub async fn shutdown(&self) {
        self.games.stop_all();
        self.launcher.stop().await;
        self.accounts.stop().await;
        self.downloader.stop();
    }

    pub fn setting(&self, key: &str) -> Option<Value> {
        self.db.setting(key).ok().flatten().filter(|v| !v.is_null())
    }

    pub fn setting_str(&self, key: &str) -> Option<String> {
        self.setting(key).and_then(|v| v.as_str().map(str::to_string))
    }

    pub fn setting_u64(&self, key: &str) -> Option<u64> {
        self.setting(key).and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
    }

    pub fn mirror(&self) -> String {
        self.setting_str("mirror").unwrap_or_else(|| "bmclapi".into())
    }

    pub fn ms_client_id(&self) -> Option<String> {
        self.setting_str("msClientId")
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("STARRAFT_MS_CLIENT_ID").ok())
            .filter(|s| !s.is_empty())
    }
}
