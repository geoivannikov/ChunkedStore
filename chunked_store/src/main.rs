use chunked_store::server;

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

    let state = chunked_store::models::SharedState::default();
    server::run_server(state).await
}
