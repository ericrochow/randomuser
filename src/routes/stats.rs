use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures_util::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use super::api::AppState;

/// `GET /stats` — point-in-time JSON snapshot of accumulated request counts.
pub async fn handle_stats_snapshot(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.stats.snapshot())
}

/// `GET /stats/stream` — Server-Sent Events stream of live stats.
///
/// Each event carries a JSON object with the same shape as `GET /stats`.
/// A keep-alive comment is sent every 15 seconds so proxies and browsers
/// don't close the connection during quiet periods.
pub async fn handle_stats_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.stats.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(snapshot) => {
            let data = serde_json::to_string(&snapshot).unwrap_or_default();
            Some(Ok(Event::default().event("stats").data(data)))
        }
        // Lagged — receiver missed some broadcasts; skip and continue.
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
