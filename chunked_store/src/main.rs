use chunked_store::error::AppResult;
use chunked_store::server;

fn setup_app() -> AppResult<chunked_store::models::SharedState> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("chunked_store=debug".parse().unwrap()),
        )
        .with_target(false)
        .compact()
        .init();

    Ok(chunked_store::models::SharedState::default())
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let state = setup_app()?;
    server::run_server(state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_app_creates_state() {
        let state = chunked_store::models::SharedState::default();
        assert_eq!(state.store.lock().await.len(), 0);
    }

    #[test]
    fn test_setup_app_initializes_tracing() {
        let result = setup_app();
        assert!(result.is_ok());
        tracing::info!("Test log message");
    }

    #[tokio::test]
    async fn test_main_flow() {
        let state = chunked_store::models::SharedState::default();
        let app = server::create_app(state).await;
        let _service = app.into_make_service();
    }
}
