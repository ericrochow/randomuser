use axum::{routing::get, Router};
use randomuser::{
    config::Config,
    generator::Generator,
    routes::api::{handle_latest, handle_versioned, AppState},
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{compression::CompressionLayer, cors::CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "randomuser=info,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::default();

    info!("Loading generator data from {:?} …", config.data_dir);
    let mut gen = Generator::new("1.4");
    gen.init(&config.data_dir)
        .expect("failed to load data directory");

    let nat_codes = gen.nat_codes();
    info!(
        "Loaded {} nationalities: {}",
        nat_codes.len(),
        nat_codes.join(" ")
    );

    let state: AppState = Arc::new(gen);

    let app = Router::new()
        .route("/api", get(handle_latest))
        .route("/api/", get(handle_latest))
        .route("/api/:version", get(handle_versioned))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server failed");
}
