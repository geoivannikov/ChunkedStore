use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use anyhow::Context;
use bytes::Bytes;
use tokio::sync::Mutex;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::signal;
use tracing::{error, info, warn};
use tower_http::trace::TraceLayer;

#[derive(Clone, Default)]
struct AppState {
    store: Arc<Mutex<HashMap<String, Bytes>>>,
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let state: SharedState = Arc::new(AppState::default());

    let app = Router::new()
        .route("/healthz", get(health))
        .route("/{*path}", get(get_object).put(put_object).delete(delete_object))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    info!(%addr, "starting server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;

    let server = axum::serve(listener, app);
    let res = server
        .with_graceful_shutdown(shutdown_signal())
        .await;

    if let Err(e) = res {
        error!(error = %e, "server error");
        return Err(e.into());
    }
    info!("server stopped");
    Ok(())
}

async fn shutdown_signal() {
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

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

async fn get_object(State(state): State<SharedState>, Path(path): Path<String>) -> impl IntoResponse {
    let store = state.store.lock().await;
    if let Some(bytes) = store.get(&path) {
        tracing::info!(path, "object retrieved");
        return Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(bytes.clone()))
            .unwrap();
    }
    tracing::warn!(path, "object not found");
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Object not found\n"))
        .unwrap()
}

async fn put_object(State(state): State<SharedState>, Path(path): Path<String>, body: Body) -> impl IntoResponse {
    match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => {
            let mut store = state.store.lock().await;
            store.insert(path.clone(), bytes);
            tracing::info!(path, "stored object");
            (StatusCode::CREATED, format!("Object stored at {}\n", path))
        }
        Err(e) => {
            tracing::warn!(error = %e, path, "failed to read request body");
            (StatusCode::BAD_REQUEST, "Failed to read body\n".to_string())
        }
    }
}

async fn delete_object(State(state): State<SharedState>, Path(path): Path<String>) -> impl IntoResponse {
    let mut store = state.store.lock().await;
    if store.remove(&path).is_some() {
        tracing::info!(path, "object deleted");
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap();
    }
    tracing::warn!(path, "object not found");
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Object not found\n"))
        .unwrap()
}
