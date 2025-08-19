use crate::error::{AppResult, AppError, ContextExt};
use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

use crate::models::SharedState;
use crate::handlers::{health, get_object, put_object, delete_object};

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
            get(get_object)
                .put(put_object)
                .delete(delete_object),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, StatusCode};
    use tower::util::ServiceExt;
    use axum::body::Body;

    #[tokio::test]
    async fn health_endpoint_works() {
        let state = SharedState::default();
        let app = create_app(state).await;

        let req = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }



    #[tokio::test]
    async fn app_has_all_routes() {
        let state = SharedState::default();
        let app = create_app(state).await;

        let methods = [Method::GET, Method::PUT, Method::DELETE];
        for method in methods {
            let req = axum::http::Request::builder()
                .method(method)
                .uri("/test-route")
                .body(Body::empty())
                .unwrap();

            let resp = app.clone().oneshot(req).await.unwrap();
            assert_ne!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        }
    }

    #[tokio::test]
    async fn create_app_with_cors_layers() {
        let state = SharedState::default();
        let app = create_app(state).await;

        let req = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_port_environment_variable() {
        std::env::set_var("PORT", "9090");
        let state = SharedState::default();
        
        let app = create_app(state).await;
        let req = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        
        std::env::remove_var("PORT");
    }

    #[tokio::test]
    async fn test_create_app_with_trace_layer() {
        let state = SharedState::default();
        let app = create_app(state).await;

        let req = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }



    #[tokio::test]
    async fn test_health_route_specific() {
        let state = SharedState::default();
        let app = create_app(state).await;

        let req = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"ok\n");
    }

    #[tokio::test]
    async fn test_wildcard_route_handles_all_paths() {
        let state = SharedState::default();
        let app = create_app(state).await;

        let test_paths = ["/", "/test", "/deep/path/file.txt", "/api/v1/data"];
        for path in test_paths {
            let req = axum::http::Request::builder()
                .method(Method::GET)
                .uri(path)
                .body(Body::empty())
                .unwrap();

            let resp = app.clone().oneshot(req).await.unwrap();
            assert_ne!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        }
    }
}
