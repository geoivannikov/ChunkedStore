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
