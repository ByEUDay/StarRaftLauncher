//! StarRaftLauncher BackEnd —— 业务引擎
//!
//! 职责：对 FrontEnd 暴露 HTTP/SSE API，编排 Core(C++ FFI)、Download(Go)、
//! Launcher(Java)、Account(Kotlin)、Data(SQLite) 与 Plugins(Lua)。
//!
//! 与宿主的握手：启动后在 stdout 输出一行 `LISTEN <port>`，日志全部写 stderr。

mod api;
mod config;
mod core_ffi;
mod db;
mod error;
mod events;
mod models;
mod plugins;
mod services;
mod state;
mod util;

use std::io::Write;
use std::sync::Arc;

use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_writer(std::io::stderr)
        .init();

    let cfg = config::Config::load()?;
    tracing::info!(data_dir = %cfg.data_dir.display(), "starting StarRaft backend v{}", env!("CARGO_PKG_VERSION"));

    let state = state::AppState::init(cfg).await?;
    let app = api::router(state.clone());

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", state.cfg.port)).await?;
    let port = listener.local_addr()?.port();
    println!("LISTEN {port}");
    std::io::stdout().flush()?;
    tracing::info!("listening on http://127.0.0.1:{port}");

    let shutdown = Arc::new(Notify::new());
    if state.cfg.watch_stdin {
        // 宿主进程退出时 stdin 会关闭，随之退出，避免残留后台进程
        let s = shutdown.clone();
        tokio::spawn(async move {
            let mut sink = tokio::io::sink();
            let _ = tokio::io::copy(&mut tokio::io::stdin(), &mut sink).await;
            s.notify_one();
        });
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = shutdown.notified() => {}
            }
            tracing::info!("shutting down");
        })
        .await?;

    state.shutdown().await;
    Ok(())
}
