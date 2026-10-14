use std::future::IntoFuture;
use vela_core::config::Config;
use vela_server::{AppState, router};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vela_server=info,vela_imports=info".into()),
        )
        .init();
    let config = Config::from_env()?;
    let state = AppState::initialize(config.clone()).await?;
    let shutdown = state.shutdown.clone();
    let worker_shutdown = shutdown.clone();
    let imports = state.imports.clone();
    let mut worker = tokio::spawn(async move { imports.run(worker_shutdown).await });
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(address=%listener.local_addr()?,version=env!("CARGO_PKG_VERSION"),"Vela ready");
    let server_shutdown = shutdown.clone();
    let server=axum::serve(listener,router(state.clone()).into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(async move {
            #[cfg(unix)] {
                let mut terminate=tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("install SIGTERM handler");
                tokio::select! {_=tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{},_=server_shutdown.cancelled()=>{}}
            }
            #[cfg(not(unix))] {tokio::select!{_=tokio::signal::ctrl_c()=>{},_=server_shutdown.cancelled()=>{}}}
            server_shutdown.cancel();
        }).into_future();
    tokio::pin!(server);
    tokio::select! {
        result=&mut worker=> {
            let expected_stop=shutdown.is_cancelled();
            shutdown.cancel();
            match tokio::time::timeout(std::time::Duration::from_secs(30),&mut server).await {
                Ok(result)=>result?,
                Err(_)=>tracing::warn!("HTTP drain deadline reached"),
            }
            let _drained=state.drain_uploads().await?;
            state.pool.close().await;
            if expected_stop {result??; return Ok(());}
            anyhow::bail!("Import worker stopped unexpectedly: {result:?}");
        }
        result=&mut server=>{result?;}
    }
    shutdown.cancel();
    let worker_result = worker.await;
    let _drained = state.drain_uploads().await?;
    state.pool.close().await;
    worker_result??;
    Ok(())
}
