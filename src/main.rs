use axum::{routing::get, Router};
use randomuser::{
    config::Config,
    generator::Generator,
    routes::api::{handle_latest, handle_versioned, AppState},
    routes::stats::{handle_stats_snapshot, handle_stats_stream},
    stats,
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

    let config = Config::from_env();

    if config.mongodb_uri.is_some() {
        info!("MongoDB stats enabled");
    } else {
        info!("MongoDB stats disabled (set MONGODB_URI to enable)");
    }
    if config.trusted_proxy {
        info!("Trusted-proxy mode enabled: real IP extracted from X-Forwarded-For");
    }

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

    let stats_handle = stats::create(config.mongodb_uri.as_deref());

    let limiter = randomuser::stats::RateLimiter::new(config.rate_limit, config.rate_window);

    // Sweep expired rate-limiter entries every full window so memory use stays
    // bounded regardless of how many unique source IPs have been seen.
    {
        let limiter_clone = limiter.clone();
        let window = config.rate_window;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(window);
            interval.tick().await; // skip the immediate first tick
            loop {
                interval.tick().await;
                limiter_clone.evict_expired();
            }
        });
    }

    let state = AppState {
        generator: Arc::new(gen),
        limiter,
        stats: stats_handle,
        max_results: config.max_results,
        trusted_proxy: config.trusted_proxy,
    };

    let app = Router::new()
        .route("/api", get(handle_latest))
        .route("/api/", get(handle_latest))
        .route("/api/:version", get(handle_versioned))
        .route("/stats", get(handle_stats_snapshot))
        .route("/stats/stream", get(handle_stats_stream))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("server failed");
}
