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

/// Accumulated request statistics.
///
/// Returns a point-in-time JSON snapshot of all requests served since startup,
/// broken down by nationality code.
#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = 200, description = "Current request statistics",
         body = crate::stats::StatsSnapshot, content_type = "application/json"),
    ),
    tag = "Stats",
)]
pub async fn handle_stats_snapshot(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.stats.snapshot())
}

/// Live request statistics stream (Server-Sent Events).
///
/// Opens a persistent SSE connection. Each event is named `stats` and carries
/// a JSON payload with the same shape as `GET /stats`, emitted after every
/// request. A keep-alive comment is sent every 15 seconds.
///
/// Consume with `EventSource` in the browser or `curl -N /stats/stream`.
#[utoipa::path(
    get,
    path = "/stats/stream",
    responses(
        (status = 200, description = "SSE stream — each `stats` event is a StatsSnapshot JSON object",
         content_type = "text/event-stream"),
    ),
    tag = "Stats",
)]
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
