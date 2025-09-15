use crate::error::{AppError, AppResult, ContextExt};
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

use crate::handlers::{delete_object, get_object, health, put_object};
use crate::models::SharedState;

pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("shutdown signal received");
}

pub async fn create_app(state: SharedState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route(
            "/{*path}",
            get(get_object).put(put_object).delete(delete_object),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub async fn run_server(state: SharedState) -> AppResult<()> {
    let port: u16 = std::env::var("PORT")
        .map(|s| s.parse())
        .unwrap_or(Ok(8080))?;

    let app = create_app(state).await;

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    info!(%addr, "starting server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;

    let server = axum::serve(listener, app);
    let res = server.with_graceful_shutdown(shutdown_signal()).await;

    if let Err(e) = res {
        error!(error = %e, "server error");
        return Err(AppError::Server(e.to_string()));
    }
    info!("server stopped");
    Ok(())
}
