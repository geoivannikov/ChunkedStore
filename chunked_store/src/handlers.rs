use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use bytes::Bytes;
use futures_util::{stream, StreamExt};
use std::convert::Infallible;
use tracing::{info, warn};

use crate::models::{content_type_for, ChunkMsg, SharedState};

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

pub async fn get_object(
    State(state): State<SharedState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
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

    let ct = content_type_for(&path);

    if is_complete {
        let bytes = match chunks.len() {
            0 => Bytes::new(),
            1 => chunks[0].clone(),
            _ => {
                let total: usize = chunks.iter().map(|c| c.len()).sum();
                let mut v = Vec::with_capacity(total);
                for c in &chunks {
                    v.extend_from_slice(c);
                }
                Bytes::from(v)
            }
        };
        tracing::info!(%path, size = bytes.len(), "GET: complete");
        let mut resp = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(bytes))
            .unwrap();
        let headers = resp.headers_mut();
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(ct));
        return resp;
    }

    tracing::info!(%path, "GET: streaming (in-progress)");

    let historical = stream::iter(chunks.into_iter()).map(Ok::<Bytes, Infallible>);
    let live = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(ChunkMsg::Data(b)) => Some((Ok::<Bytes, Infallible>(b), rx)),
            Ok(ChunkMsg::Done) | Ok(ChunkMsg::Abort) => None,
            Err(_) => None,
        }
    });

    let body_stream = historical.chain(live);

    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from_stream(body_stream))
        .unwrap();
    let headers = resp.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(ct));
    resp
}

pub async fn put_object(
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
        store.insert(path.clone(), crate::models::ChunkedObject::new());
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
                    let mut obj = crate::models::ChunkedObject::new();
                    obj.add_chunk(bytes.clone());
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

pub async fn delete_object(
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
