use super::StatEvent;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// BSON-serialisable representation of a single API request.
#[derive(Serialize)]
struct RequestDoc {
    ts: String,
    version: String,
    results: i64,
    seed: String,
    page: i64,
    nat: Vec<String>,
    inc: Vec<String>,
    fmt: String,
    ip: String,
}

/// Background task: connects to MongoDB and drains the stats channel,
/// inserting one document per request. If the connection fails, the
/// channel is drained and discarded so senders never block.
pub async fn run_writer(uri: String, mut rx: mpsc::Receiver<StatEvent>) {
    let client = match mongodb::Client::with_uri_str(&uri).await {
        Ok(c) => c,
        Err(e) => {
            warn!("MongoDB connection failed ({e}); request stats will not be persisted");
            while rx.recv().await.is_some() {}
            return;
        }
    };

    // Use a typed collection so the driver serialises RequestDoc directly to
    // BSON without an intermediate Document conversion.
    let collection = client
        .database("randomuser")
        .collection::<RequestDoc>("requests");

    info!("MongoDB stats writer connected");

    while let Some(event) = rx.recv().await {
        let doc = RequestDoc {
            ts: event.ts.to_rfc3339(),
            version: event.version,
            results: event.results as i64,
            seed: event.seed,
            page: event.page as i64,
            nat: event.nat,
            inc: event.inc,
            fmt: event.fmt,
            ip: event.ip.to_string(),
        };

        if let Err(e) = collection.insert_one(doc).await {
            warn!("MongoDB insert failed: {e}");
        }
    }
}
