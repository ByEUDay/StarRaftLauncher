//! FrontEnd 使用的 HTTP API。所有 /api 路由需要令牌：
//! 请求头 `X-StarRaft-Token`，或查询参数 `?token=`（用于 EventSource）。

use std::convert::Infallible;

use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::core_ffi;
use crate::error::{ApiResult, AppError};
use crate::models::{Account, CreateInstance, Instance, UpdateInstance};
use crate::plugins::PluginInfo;
use crate::services::download::BatchStatus;
use crate::services::game;
use crate::services::loaders::{LoaderVersion, LOADERS};
use crate::state::Shared;
use crate::util::now;

pub fn router(state: Shared) -> Router {
    let api = Router::new()
        .route("/system", get(system))
        .route("/versions", get(versions))
        .route("/loaders/:loader", get(loader_versions))
        .route("/instances", get(list_instances).post(create_instance))
        .route("/instances/:id", get(get_instance).put(update_instance).delete(delete_instance))
        .route("/instances/:id/install", post(install_instance))
        .route("/instances/:id/launch", post(launch_instance))
        .route("/instances/:id/stop", post(stop_instance))
        .route("/accounts", get(list_accounts))
        .route("/accounts/offline", post(add_offline_account))
        .route("/accounts/microsoft", post(add_microsoft_account))
        .route("/accounts/:id/default", post(set_default_account))
        .route("/accounts/:id", delete(delete_account))
        .route("/downloads", get(list_downloads))
        .route("/downloads/:id", delete(cancel_download))
        .route("/plugins", get(list_plugins))
        .route("/plugins/:id/enabled", put(set_plugin_enabled))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/events", get(events))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state.clone());

    let mut app = Router::new().nest("/api", api);
    if let Some(dir) = &state.cfg.web_dir {
        app = app.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html"))));
    }
    // 开发模式下前端来自 Vite（不同端口），需要 CORS；安全性由令牌保证
    app.layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
}

async fn auth(State(s): State<Shared>, req: Request, next: Next) -> Result<Response, StatusCode> {
    let header_ok = req
        .headers()
        .get("x-starraft-token")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == s.cfg.token);
    let query_ok = req
        .uri()
        .query()
        .is_some_and(|q| q.split('&').any(|kv| kv.strip_prefix("token=") == Some(s.cfg.token.as_str())));
    if header_ok || query_ok {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// ---------------- system ----------------

async fn system(State(s): State<Shared>) -> ApiResult<Value> {
    let javas = tokio::task::spawn_blocking(core_ffi::detect_java).await.map_err(anyhow::Error::from)?;
    let (total, avail) = core_ffi::memory();
    Ok(Json(json!({
        "backendVersion": env!("CARGO_PKG_VERSION"),
        "coreVersion": core_ffi::version(),
        "os": core_ffi::os_info(),
        "memory": { "totalMb": total / 1024 / 1024, "availableMb": avail / 1024 / 1024 },
        "recommendedMemoryMb": core_ffi::recommended_memory_mb(),
        "javas": javas,
        "sidecarJava": s.cfg.sidecar_java,
        "dataDir": s.cfg.data_dir,
        "minecraftDir": s.cfg.mc_root,
        "instancesDir": s.cfg.instances_dir,
        "running": s.games.running_ids(),
    })))
}

#[derive(Deserialize)]
struct VersionsQuery {
    #[serde(default)]
    snapshots: bool,
}

async fn versions(State(s): State<Shared>, Query(q): Query<VersionsQuery>) -> ApiResult<Value> {
    let manifest = s.versions.manifest(&s.mirror()).await?;
    let list: Vec<Value> = manifest["versions"]
        .as_array()
        .map(|vs| vs.iter().filter(|v| q.snapshots || v["type"] == "release").cloned().collect())
        .unwrap_or_default();
    Ok(Json(json!({ "latest": manifest["latest"], "versions": list })))
}

#[derive(Deserialize)]
struct LoaderQuery {
    mc: String,
}

async fn loader_versions(
    State(s): State<Shared>,
    Path(loader): Path<String>,
    Query(q): Query<LoaderQuery>,
) -> ApiResult<Vec<LoaderVersion>> {
    if !LOADERS.contains(&loader.as_str()) {
        return Err(AppError::BadRequest(format!("不支持的加载器: {loader}")));
    }
    Ok(Json(s.loaders.versions(&loader, q.mc.trim(), &s.mirror()).await?))
}

// ---------------- instances ----------------

async fn list_instances(State(s): State<Shared>) -> ApiResult<Value> {
    let playtime = s.db.playtime()?;
    let running = s.games.running_ids();
    let list: Vec<Value> = s
        .db
        .list_instances()?
        .into_iter()
        .map(|i| {
            let (secs, launches) = playtime.get(&i.id).copied().unwrap_or((0, 0));
            let mut v = serde_json::to_value(&i).unwrap_or_default();
            v["playSeconds"] = json!(secs);
            v["launches"] = json!(launches);
            v["running"] = json!(running.contains(&i.id));
            v
        })
        .collect();
    Ok(Json(json!(list)))
}

async fn get_instance(State(s): State<Shared>, Path(id): Path<String>) -> ApiResult<Instance> {
    s.db.get_instance(&id)?.map(Json).ok_or_else(|| AppError::NotFound(format!("实例 {id} 不存在")))
}

async fn create_instance(State(s): State<Shared>, Json(req): Json<CreateInstance>) -> ApiResult<Instance> {
    let name = req.name.trim();
    if name.is_empty() || req.version_id.trim().is_empty() {
        return Err(AppError::BadRequest("名称和版本不能为空".into()));
    }
    let loader = req.loader.as_deref().unwrap_or("vanilla").to_lowercase();
    if !LOADERS.contains(&loader.as_str()) {
        return Err(AppError::BadRequest(format!("不支持的加载器: {loader}")));
    }
    let loader_version = req.loader_version.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    if loader != "vanilla" && loader_version.is_none() {
        return Err(AppError::BadRequest("请选择加载器版本".into()));
    }
    let inst = Instance {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: name.to_string(),
        version_id: req.version_id.trim().to_string(),
        loader_version: if loader == "vanilla" { None } else { loader_version },
        profile_id: None,
        loader,
        java_path: req.java_path.filter(|s| !s.is_empty()),
        memory_mb: req.memory_mb,
        jvm_args: req.jvm_args,
        game_args: req.game_args,
        icon: req.icon,
        installed: false,
        last_played: None,
        created_at: now(),
    };
    s.db.insert_instance(&inst)?;
    std::fs::create_dir_all(s.cfg.instance_dir(&inst.id)).map_err(anyhow::Error::from)?;
    s.events.emit("instance.changed", json!({ "id": inst.id }));
    Ok(Json(inst))
}

async fn update_instance(
    State(s): State<Shared>,
    Path(id): Path<String>,
    Json(req): Json<UpdateInstance>,
) -> ApiResult<Instance> {
    let mut inst = s.db.get_instance(&id)?.ok_or_else(|| AppError::NotFound(format!("实例 {id} 不存在")))?;
    inst.name = req.name.trim().to_string();
    inst.java_path = req.java_path.filter(|s| !s.is_empty());
    inst.memory_mb = req.memory_mb;
    inst.jvm_args = req.jvm_args;
    inst.game_args = req.game_args;
    inst.icon = req.icon;
    s.db.update_instance(&inst)?;
    s.events.emit("instance.changed", json!({ "id": id }));
    Ok(Json(inst))
}

async fn delete_instance(State(s): State<Shared>, Path(id): Path<String>) -> Result<StatusCode, AppError> {
    if s.games.is_running(&id) {
        return Err(AppError::Conflict("实例正在运行，无法删除".into()));
    }
    if !s.db.delete_instance(&id)? {
        return Err(AppError::NotFound(format!("实例 {id} 不存在")));
    }
    let dir = s.cfg.instance_dir(&id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(anyhow::Error::from)?;
    }
    s.events.emit("instance.changed", json!({ "id": id }));
    Ok(StatusCode::NO_CONTENT)
}

async fn install_instance(State(s): State<Shared>, Path(id): Path<String>) -> Result<StatusCode, AppError> {
    s.db.get_instance(&id)?.ok_or_else(|| AppError::NotFound(format!("实例 {id} 不存在")))?;
    let st = s.clone();
    tokio::spawn(async move {
        if let Err(e) = game::install(&st, &id).await {
            st.events.emit("task.error", json!({ "instanceId": id, "task": "install", "message": format!("{e:#}") }));
        }
    });
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LaunchRequest {
    account_id: Option<String>,
}

async fn launch_instance(
    State(s): State<Shared>,
    Path(id): Path<String>,
    body: Option<Json<LaunchRequest>>,
) -> Result<StatusCode, AppError> {
    if s.games.is_running(&id) {
        return Err(AppError::Conflict("该实例已在运行".into()));
    }
    let account_id = body.and_then(|Json(b)| b.account_id);
    let st = s.clone();
    tokio::spawn(async move {
        if let Err(e) = game::launch(&st, &id, account_id).await {
            st.events.emit("task.error", json!({ "instanceId": id, "task": "launch", "message": format!("{e:#}") }));
        }
    });
    Ok(StatusCode::ACCEPTED)
}

async fn stop_instance(State(s): State<Shared>, Path(id): Path<String>) -> Result<StatusCode, AppError> {
    if s.games.stop(&id) {
        Ok(StatusCode::ACCEPTED)
    } else {
        Err(AppError::NotFound("该实例未在运行".into()))
    }
}

// ---------------- accounts ----------------

async fn list_accounts(State(s): State<Shared>) -> ApiResult<Vec<Account>> {
    Ok(Json(s.db.list_accounts()?))
}

#[derive(Deserialize)]
struct OfflineRequest {
    username: String,
}

async fn add_offline_account(State(s): State<Shared>, Json(req): Json<OfflineRequest>) -> ApiResult<Account> {
    let result = s.accounts.offline(req.username.trim()).await.map_err(|e| AppError::BadRequest(format!("{e:#}")))?;
    let account = save_account(&s, result.into_account(now()))?;
    Ok(Json(account))
}

async fn add_microsoft_account(State(s): State<Shared>) -> Result<StatusCode, AppError> {
    let client_id = s
        .ms_client_id()
        .ok_or_else(|| AppError::BadRequest("请先在设置中填写微软 OAuth Client ID（Azure 应用注册）".into()))?;
    let st = s.clone();
    tokio::spawn(async move {
        match st.accounts.microsoft_login(&client_id).await {
            Ok(result) => match save_account(&st, result.into_account(now())) {
                Ok(acc) => st.events.emit("account.added", &acc),
                Err(e) => st.events.emit("account.error", json!({ "message": format!("{e:#}") })),
            },
            Err(e) => st.events.emit("account.error", json!({ "message": format!("{e:#}") })),
        }
    });
    Ok(StatusCode::ACCEPTED)
}

/// 保存账户；如果这是唯一账户则设为默认
fn save_account(s: &Shared, account: Account) -> anyhow::Result<Account> {
    s.db.upsert_account(&account)?;
    let accounts = s.db.list_accounts()?;
    if !accounts.iter().any(|a| a.is_default) {
        s.db.set_default_account(&account.id)?;
    }
    Ok(s.db.get_account(&account.id)?.unwrap_or(account))
}

async fn set_default_account(State(s): State<Shared>, Path(id): Path<String>) -> Result<StatusCode, AppError> {
    s.db.get_account(&id)?.ok_or_else(|| AppError::NotFound("账户不存在".into()))?;
    s.db.set_default_account(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_account(State(s): State<Shared>, Path(id): Path<String>) -> Result<StatusCode, AppError> {
    if !s.db.delete_account(&id)? {
        return Err(AppError::NotFound("账户不存在".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------- downloads / plugins / settings ----------------

async fn list_downloads(State(s): State<Shared>) -> ApiResult<Vec<BatchStatus>> {
    Ok(Json(s.downloader.batches()))
}

async fn cancel_download(State(s): State<Shared>, Path(id): Path<String>) -> Result<StatusCode, AppError> {
    s.downloader.cancel(&id).await?;
    Ok(StatusCode::ACCEPTED)
}

async fn list_plugins(State(s): State<Shared>) -> ApiResult<Vec<PluginInfo>> {
    Ok(Json(s.plugins.list()))
}

#[derive(Deserialize)]
struct EnabledRequest {
    enabled: bool,
}

async fn set_plugin_enabled(
    State(s): State<Shared>,
    Path(id): Path<String>,
    Json(req): Json<EnabledRequest>,
) -> Result<StatusCode, AppError> {
    if !s.plugins.set_enabled(&id, req.enabled) {
        return Err(AppError::NotFound("插件不存在或加载失败".into()));
    }
    s.db.set_plugin_enabled(&id, req.enabled)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_settings(State(s): State<Shared>) -> ApiResult<Map<String, Value>> {
    Ok(Json(s.db.settings()?))
}

async fn put_settings(State(s): State<Shared>, Json(req): Json<Map<String, Value>>) -> ApiResult<Map<String, Value>> {
    for (k, v) in &req {
        s.db.set_setting(k, v)?;
    }
    s.events.emit("settings.changed", &req);
    Ok(Json(s.db.settings()?))
}

// ---------------- events (SSE) ----------------

async fn events(State(s): State<Shared>) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let stream = BroadcastStream::new(s.events.subscribe())
        .filter_map(|msg| msg.ok())
        .map(|event| Ok(SseEvent::default().json_data(&event).unwrap_or_default()));
    Sse::new(stream).keep_alive(KeepAlive::default())
}
