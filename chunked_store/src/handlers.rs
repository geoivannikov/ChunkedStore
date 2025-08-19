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

use crate::models::{ChunkMsg, SharedState, content_type_for};

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

pub async fn cors_preflight(Path(path): Path<String>) -> impl IntoResponse {
    tracing::debug!(%path, "CORS preflight request");
    StatusCode::NO_CONTENT
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode as SC;
    use axum::body::{self, Body as AxumBody};
    use axum::Router;
    use axum::routing::get;
    use tower::util::ServiceExt;

    fn test_state() -> SharedState {
        std::sync::Arc::new(crate::models::AppState::default())
    }

    fn app(state: SharedState) -> Router {
        Router::new().route(
            "/{*path}",
            get(get_object).put(put_object).delete(delete_object),
        ).with_state(state)
    }

    #[tokio::test]
    async fn put_get_delete_happy_path() {
        let state = test_state();
        let app = app(state.clone());

        let req = axum::http::Request::builder()
            .method("PUT").uri("/foo.txt")
            .body(AxumBody::from("hello"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::CREATED);

        let req = axum::http::Request::builder()
            .method("GET").uri("/foo.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"hello");

        let req = axum::http::Request::builder()
            .method("DELETE").uri("/foo.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::NO_CONTENT);

        let req = axum::http::Request::builder()
            .method("GET").uri("/foo.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
    }

    #[tokio::test]
    async fn put_conflict_when_uploading() {
        let state = test_state();
        let app = app(state.clone());

        let put_req1 = axum::http::Request::builder()
            .method("PUT").uri("/conflict.txt")
            .body(AxumBody::from("first upload"))
            .unwrap();
        let resp1 = app.clone().oneshot(put_req1).await.unwrap();
        assert_eq!(resp1.status(), SC::CREATED);

        let put_req2 = axum::http::Request::builder()
            .method("PUT").uri("/conflict.txt")
            .body(AxumBody::from("second upload"))
            .unwrap();
        let resp2 = app.clone().oneshot(put_req2).await.unwrap();
        assert_eq!(resp2.status(), SC::CREATED);

        let get_req = axum::http::Request::builder()
            .method("GET").uri("/conflict.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"second upload");
    }

    #[tokio::test]
    async fn get_nonexistent_object() {
        let state = test_state();
        let app = app(state.clone());

        let req = axum::http::Request::builder()
            .method("GET").uri("/nonexistent.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_nonexistent_object() {
        let state = test_state();
        let app = app(state.clone());

        let req = axum::http::Request::builder()
            .method("DELETE").uri("/nonexistent.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
    }

    #[tokio::test]
    async fn put_empty_object() {
        let state = test_state();
        let app = app(state.clone());

        let req = axum::http::Request::builder()
            .method("PUT").uri("/empty.txt")
            .body(AxumBody::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::CREATED);

        let req = axum::http::Request::builder()
            .method("GET").uri("/empty.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.len(), 0);
    }

    #[tokio::test]
    async fn content_type_detection_in_handlers() {
        let state = test_state();
        let app = app(state.clone());

        let test_cases = [
            ("/video.mp4", "video/mp4"),
            ("/segment.m4s", "video/mp4"),
            ("/manifest.mpd", "application/dash+xml"),
            ("/data.bin", "application/octet-stream"),
        ];

        for (path, expected_ct) in test_cases {
            let req = axum::http::Request::builder()
                .method("PUT").uri(path)
                .body(AxumBody::from("test"))
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), SC::CREATED);

            let req = axum::http::Request::builder()
                .method("GET").uri(path)
                .body(AxumBody::empty()).unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), SC::OK);
            assert_eq!(resp.headers().get("content-type").unwrap(), expected_ct);
        }
    }

    #[tokio::test]
    async fn cache_control_headers() {
        let state = test_state();
        let app = app(state.clone());

        let req = axum::http::Request::builder()
            .method("PUT").uri("/cache-test.txt")
            .body(AxumBody::from("test"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::CREATED);

        let req = axum::http::Request::builder()
            .method("GET").uri("/cache-test.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::OK);
        assert_eq!(resp.headers().get("cache-control").unwrap(), "no-store");
    }

    #[tokio::test]
    async fn cors_preflight_handler() {
        let state = test_state();
        let app = Router::new()
            .route("/{*path}", get(get_object).put(put_object).delete(delete_object).options(cors_preflight))
            .with_state(state);

        let req = axum::http::Request::builder()
            .method("OPTIONS").uri("/test.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::NO_CONTENT);
    }

    #[tokio::test]
    async fn get_object_with_multiple_chunks() {
        let state = test_state();
        let app = app(state.clone());

        let req = axum::http::Request::builder()
            .method("PUT").uri("/multi.txt")
            .body(AxumBody::from("part1part2part3"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::CREATED);

        let req = axum::http::Request::builder()
            .method("GET").uri("/multi.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"part1part2part3");
    }

    #[tokio::test]
    async fn get_empty_object() {
        let state = test_state();
        let app = app(state.clone());

        let req = axum::http::Request::builder()
            .method("PUT").uri("/empty.txt")
            .body(AxumBody::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::CREATED);

        let req = axum::http::Request::builder()
            .method("GET").uri("/empty.txt")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.len(), 0);
    }

    #[tokio::test]
    async fn get_streaming_during_put() {
        use tokio::time::{sleep, Duration};
        let state = test_state();
        let app = app(state.clone());

        let chunks = vec![
            Bytes::from_static(b"one-"),
            Bytes::from_static(b"two-"),
            Bytes::from_static(b"three"),
        ];
        let stream = futures_util::stream::unfold((0usize, chunks.clone()), |(i, chunks)| async move {
            if i < chunks.len() {
                sleep(Duration::from_millis(20)).await;
                let next = chunks[i].clone();
                Some((Ok::<Bytes, std::io::Error>(next), (i + 1, chunks)))
            } else {
                None
            }
        });
        let put_req = axum::http::Request::builder()
            .method("PUT").uri("/streaming.bin")
            .body(AxumBody::from_stream(stream))
            .unwrap();

        let put_app = app.clone();
        let put_task = tokio::spawn(async move { put_app.oneshot(put_req).await.unwrap() });

        sleep(Duration::from_millis(5)).await;

        let get_req = axum::http::Request::builder()
            .method("GET").uri("/streaming.bin")
            .body(AxumBody::empty()).unwrap();
        let resp = app.clone().oneshot(get_req).await.unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"one-two-three");

        let put_resp = put_task.await.unwrap();
        assert_eq!(put_resp.status(), SC::CREATED);
    }

    #[tokio::test]
    async fn put_body_read_error_returns_400_and_cleans() {
        let state = test_state();
        let app = app(state.clone());

        let err_stream = futures_util::stream::iter(vec![
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"ok")),
            Err::<Bytes, std::io::Error>(std::io::Error::new(std::io::ErrorKind::Other, "boom")),
        ]);
        let req = axum::http::Request::builder()
            .method("PUT").uri("/err.bin")
            .body(AxumBody::from_stream(err_stream))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);

        let req = axum::http::Request::builder()
            .method("GET").uri("/err.bin")
            .body(AxumBody::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
    }

}
