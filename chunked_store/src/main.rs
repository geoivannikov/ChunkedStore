use std::{
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    sync::Arc,
};

use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::Bytes;
use futures_util::{stream, StreamExt};
use tokio::{
    signal,
    sync::{broadcast, Mutex},
};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

#[derive(Clone, Debug)]
enum ChunkMsg {
    Data(Bytes),
    Done,
    Abort,
}

#[derive(Clone)]
struct ChunkedObject {
    chunks: Vec<Bytes>,
    is_complete: bool,
    notifier: broadcast::Sender<ChunkMsg>,
}

impl ChunkedObject {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            chunks: Vec::new(),
            is_complete: false,
            notifier: tx,
        }
    }

    fn add_chunk(&mut self, chunk: Bytes) {
        self.chunks.push(chunk.clone());
        let _ = self.notifier.send(ChunkMsg::Data(chunk));
    }

    fn complete(&mut self) {
        self.is_complete = true;
        let _ = self.notifier.send(ChunkMsg::Done);
    }

    fn abort(&mut self) {
        let _ = self.notifier.send(ChunkMsg::Abort);
    }

    fn subscribe(&self) -> broadcast::Receiver<ChunkMsg> {
        self.notifier.subscribe()
    }

    fn into_all_bytes(&self) -> Bytes {
        match self.chunks.len() {
            0 => Bytes::new(),
            1 => self.chunks[0].clone(),
            _ => {
                let total: usize = self.chunks.iter().map(|c| c.len()).sum();
                let mut v = Vec::with_capacity(total);
                for c in &self.chunks {
                    v.extend_from_slice(c);
                }
                Bytes::from(v)
            }
        }
    }
}

#[derive(Clone, Default)]
struct AppState {
    store: Arc<Mutex<HashMap<String, ChunkedObject>>>,
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("chunked_store=debug".parse().unwrap()),
        )
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
        .route(
            "/{*path}",
            get(get_object).put(put_object).delete(delete_object),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    info!(%addr, "starting server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;

    let server = axum::serve(listener, app);
    let res = server.with_graceful_shutdown(shutdown_signal()).await;

    if let Err(e) = res {
        error!(error = %e, "server error");
        return Err(e.into());
    }
    info!("server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { let _ = signal::ctrl_c().await; };

    #[cfg(unix)]
    let terminate = async {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
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
    tracing::debug!(%path, "GET: start");

    let (chunks, is_complete, rx) = {
        let store = state.store.lock().await;
        if let Some(obj) = store.get(&path) {
            (obj.chunks.clone(), obj.is_complete, obj.subscribe())
        } else {
            tracing::warn!(%path, "GET: not found");
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Object not found\n"))
                .unwrap();
        }
    };

    if is_complete {
        let bytes = {
            if chunks.len() == 1 {
                chunks[0].clone()
            } else {
                let total: usize = chunks.iter().map(|c| c.len()).sum();
                let mut v = Vec::with_capacity(total);
                for c in &chunks {
                    v.extend_from_slice(c);
                }
                Bytes::from(v)
            }
        };
        tracing::info!(%path, size = bytes.len(), "GET: complete");
        let mut resp = Response::builder().status(StatusCode::OK).body(Body::from(bytes)).unwrap();
        resp.headers_mut().insert("Cache-Control", HeaderValue::from_static("no-store"));
        return resp;
    }

    tracing::info!(%path, "GET: streaming (in-progress)");

    let historical = stream::iter(chunks.into_iter()).map(Ok::<Bytes, Infallible>);

    let live = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(ChunkMsg::Data(b)) => Some((Ok::<Bytes, Infallible>(b), rx)),
            Ok(ChunkMsg::Done) | Ok(ChunkMsg::Abort) => None,
            Err(_lagged) => {
                None
            }
        }
    });

    let body_stream = historical.chain(live);

    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from_stream(body_stream))
        .unwrap();
    resp.headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("no-store"));
    resp
}

async fn put_object(
    State(state): State<SharedState>,
    Path(path): Path<String>,
    body: Body,
) -> impl IntoResponse {
    tracing::debug!(%path, "PUT: start");

    {
        let mut store = state.store.lock().await;
        if let Some(existing) = store.get(&path) {
            if !existing.is_complete {
                tracing::warn!(%path, "PUT: conflict (already uploading)");
                return (
                    StatusCode::CONFLICT,
                    "Another upload in progress for this path\n",
                );
            }
        }
        store.insert(path.clone(), ChunkedObject::new());
    }

    let mut stream = body.into_data_stream();
    let mut total = 0usize;

    while let Some(next) = stream.next().await {
        match next {
            Ok(bytes) => {
                total += bytes.len();
                let mut store = state.store.lock().await;
                if let Some(obj) = store.get_mut(&path) {
                    obj.add_chunk(bytes);
                } else {
                    let mut obj = ChunkedObject::new();
                    obj.add_chunk(Bytes::new());
                    store.insert(path.clone(), obj);
                }
            }
            Err(e) => {
                tracing::error!(%e, %path, "PUT: read error");
                let mut store = state.store.lock().await;
                if let Some(obj) = store.get_mut(&path) {
                    obj.abort();
                }
                store.remove(&path);
                return (StatusCode::BAD_REQUEST, "Failed to read body\n");
            }
        }
    }

    {
        let mut store = state.store.lock().await;
        if let Some(obj) = store.get_mut(&path) {
            obj.complete();
        }
    }

    info!(%path, %total, "PUT: stored (streaming)");
    (StatusCode::CREATED, "Object stored successfully\n")
}

async fn delete_object(
    State(state): State<SharedState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    tracing::debug!(%path, "DELETE: start");

    let mut store = state.store.lock().await;
    if let Some(mut obj) = store.remove(&path) {
        obj.abort();
        info!(%path, "DELETE: ok");
        Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap()
    } else {
        warn!(%path, "DELETE: not found");
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Object not found\n"))
            .unwrap()
    }
}
