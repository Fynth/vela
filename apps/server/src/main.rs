use std::future::IntoFuture;
use vela_core::config::Config;
use vela_server::{AppState, router};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vela_server=info,vela_imports=info,vela_scanner=info".into()),
        )
        .init();
    let config = Config::from_env()?;
    let state = AppState::initialize(config.clone()).await?;
    // Do not start jobs if this process cannot acquire its HTTP listener.
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let shutdown = state.shutdown.clone();
    let mut workers = tokio::task::JoinSet::new();
    let imports = state.imports.clone();
    let token = shutdown.clone();
    workers.spawn(async move { ("Import", imports.run(token).await) });
    let scanner = state.scanner.clone();
    let token = shutdown.clone();
    workers.spawn(async move { ("Scanner", scanner.run(token).await) });
    let analysis = state.analysis.clone();
    let token = shutdown.clone();
    workers.spawn(async move { ("Audio analysis", analysis.run(token).await) });
    let listening = state.listening.clone();
    let token = shutdown.clone();
    workers.spawn(async move {
        let result: anyhow::Result<()> = async {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => return Ok(()),
                    _ = interval.tick() => { listening.prune().await?; }
                }
            }
        }
        .await;
        ("Listening retention", result)
    });
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
    let mut failure: Option<anyhow::Error> = None;
    tokio::select! {
        result=workers.join_next()=> {
            let expected_stop=shutdown.is_cancelled();
            shutdown.cancel();
            match result {
                Some(Ok((name,_))) if !expected_stop=>failure=Some(anyhow::anyhow!("{name} worker stopped unexpectedly")),
                Some(Ok((_,Err(_))))=>failure=Some(anyhow::anyhow!("Background worker failed during shutdown")),
                Some(Err(_))=>failure=Some(anyhow::anyhow!("Background worker task failed")),
                None=>failure=Some(anyhow::anyhow!("Background worker supervision ended unexpectedly")),
                _=>{},
            }
            match tokio::time::timeout(std::time::Duration::from_secs(30),&mut server).await {
                Ok(Err(error)) if failure.is_none()=>failure=Some(error.into()),
                Err(_)=>tracing::warn!("HTTP drain deadline reached"),
                _=>{},
            }
        }
        result=&mut server=>{if let Err(error)=result {failure=Some(error.into());}}
    }
    shutdown.cancel();
    while let Some(result) = workers.join_next().await {
        match result {
            Ok((_, Err(_))) if failure.is_none() => {
                failure = Some(anyhow::anyhow!("Background worker failed during shutdown"))
            }
            Err(_) if failure.is_none() => {
                failure = Some(anyhow::anyhow!("Background worker task failed"))
            }
            _ => {}
        }
    }
    let _drained = state.drain_uploads().await?;
    state.pool.close().await;
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}
