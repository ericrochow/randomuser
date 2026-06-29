pub mod mongo;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tracing::warn;

// ─── Per-request event ────────────────────────────────────────────────────────

pub struct StatEvent {
    pub ts: DateTime<Utc>,
    pub version: String,
    pub results: usize,
    pub seed: String,
    pub page: u32,
    /// Nationality codes as supplied by the caller (may be empty).
    pub nat: Vec<String>,
    /// Include-field list resolved for this request.
    pub inc: Vec<String>,
    pub fmt: String,
    pub ip: IpAddr,
}

// ─── Live in-memory stats ─────────────────────────────────────────────────────

pub struct LiveStats {
    total: AtomicU64,
    /// Counts per nationality token. A request with nat=US,FR increments both
    /// "US" and "FR" independently; this reflects usage by nationality filter,
    /// not unique-user counts.
    by_nat: DashMap<String, AtomicU64>,
}

impl Default for LiveStats {
    fn default() -> Self {
        Self {
            total: AtomicU64::new(0),
            by_nat: DashMap::new(),
        }
    }
}

impl LiveStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, event: &StatEvent) {
        self.total.fetch_add(1, Ordering::Relaxed);
        for nat in &event.nat {
            self.by_nat
                .entry(nat.clone())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            total_requests: self.total.load(Ordering::Relaxed),
            by_nat: self
                .by_nat
                .iter()
                .map(|e| (e.key().clone(), e.value().load(Ordering::Relaxed)))
                .collect(),
        }
    }
}

/// Point-in-time snapshot of aggregated stats (serialisable for JSON + SSE).
#[derive(Debug, Clone, Serialize)]
pub struct StatsSnapshot {
    pub total_requests: u64,
    pub by_nat: HashMap<String, u64>,
}

// ─── Per-IP rate limiter ──────────────────────────────────────────────────────

/// Simple fixed-window per-IP rate limiter backed by a DashMap.
///
/// On each call to `check_and_increment`, the request count for the IP is
/// incremented. If the window has expired, the counter resets first. Returns
/// `true` if the request is within the configured limit.
///
/// Call `evict_expired` periodically (e.g. every `window` seconds) to remove
/// stale entries and prevent unbounded memory growth.
#[derive(Clone)]
pub struct RateLimiter {
    map: Arc<DashMap<IpAddr, (u64, Instant)>>,
    limit: u64,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: u64, window: Duration) -> Self {
        Self {
            map: Arc::new(DashMap::new()),
            limit,
            window,
        }
    }

    pub fn check_and_increment(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut entry = self.map.entry(ip).or_insert((0, now));
        if now.duration_since(entry.1) >= self.window {
            *entry = (1, now);
        } else {
            entry.0 += 1;
        }
        entry.0 <= self.limit
    }

    pub fn current_count(&self, ip: IpAddr) -> u64 {
        self.map.get(&ip).map(|e| e.0).unwrap_or(0)
    }

    /// Remove entries whose rate window has expired. Spawn a Tokio task that
    /// calls this on a fixed interval to bound memory use.
    pub fn evict_expired(&self) {
        let now = Instant::now();
        self.map.retain(|_, v| now.duration_since(v.1) < self.window);
    }
}

// ─── Stats handle (passed to every request handler) ──────────────────────────

#[derive(Clone)]
pub struct StatsHandle {
    live: Arc<LiveStats>,
    /// Present only when MongoDB is configured.
    mongo_tx: Option<mpsc::Sender<StatEvent>>,
    /// Broadcast channel for SSE clients.
    broadcast_tx: broadcast::Sender<StatsSnapshot>,
}

impl StatsHandle {
    /// Record a completed request: update in-memory counters, push SSE event,
    /// and forward to the MongoDB writer if one is running.
    pub fn record(&self, event: StatEvent) {
        self.live.record(&event);
        // Ignore send errors — fine if no SSE clients are connected.
        let _ = self.broadcast_tx.send(self.live.snapshot());
        if let Some(tx) = &self.mongo_tx {
            if tx.try_send(event).is_err() {
                warn!("MongoDB stats channel full; dropping event");
            }
        }
    }

    /// Subscribe to the live stats broadcast stream (used by SSE endpoint).
    pub fn subscribe(&self) -> broadcast::Receiver<StatsSnapshot> {
        self.broadcast_tx.subscribe()
    }

    /// Return a point-in-time snapshot (used by the JSON stats endpoint).
    pub fn snapshot(&self) -> StatsSnapshot {
        self.live.snapshot()
    }
}

// ─── Constructor ─────────────────────────────────────────────────────────────

/// Build a StatsHandle and, if `mongodb_uri` is Some, spawn the background
/// writer task. Must be called inside a Tokio runtime.
pub fn create(mongodb_uri: Option<&str>) -> StatsHandle {
    let live = Arc::new(LiveStats::new());
    // Capacity 64: if all clients are slow they'll miss some events, which
    // is fine — we don't want to backpressure request handlers.
    let (broadcast_tx, _) = broadcast::channel(64);

    let mongo_tx = mongodb_uri.map(|uri| {
        // Bounded channel: drops events with a warning rather than growing
        // without bound when MongoDB is slow.
        let (tx, rx) = mpsc::channel(4096);
        let uri = uri.to_string();
        tokio::spawn(mongo::run_writer(uri, rx));
        tx
    });

    StatsHandle {
        live,
        mongo_tx,
        broadcast_tx,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn dummy_event(nat: &str) -> StatEvent {
        StatEvent {
            ts: Utc::now(),
            version: "1.4".to_string(),
            results: 1,
            seed: "test".to_string(),
            page: 1,
            nat: vec![nat.to_string()],
            inc: vec!["name".to_string()],
            fmt: "json".to_string(),
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        }
    }

    #[test]
    fn live_stats_accumulate() {
        let stats = LiveStats::new();
        stats.record(&dummy_event("US"));
        stats.record(&dummy_event("US"));
        stats.record(&dummy_event("GB"));

        let snap = stats.snapshot();
        assert_eq!(snap.total_requests, 3);
        assert_eq!(snap.by_nat["US"], 2);
        assert_eq!(snap.by_nat["GB"], 1);
    }

    #[test]
    fn live_stats_default_is_empty() {
        let stats = LiveStats::default();
        let snap = stats.snapshot();
        assert_eq!(snap.total_requests, 0);
        assert!(snap.by_nat.is_empty());
    }

    #[test]
    fn rate_limiter_allows_within_limit() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.check_and_increment(ip));
        assert!(limiter.check_and_increment(ip));
        assert!(limiter.check_and_increment(ip));
        assert!(!limiter.check_and_increment(ip)); // 4th request exceeds limit
    }

    #[test]
    fn rate_limiter_resets_after_window() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let limiter = RateLimiter::new(1, Duration::from_millis(1));
        assert!(limiter.check_and_increment(ip));
        assert!(!limiter.check_and_increment(ip)); // over limit

        std::thread::sleep(Duration::from_millis(5));
        assert!(limiter.check_and_increment(ip)); // window reset
    }

    #[test]
    fn rate_limiter_tracks_ips_independently() {
        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4));
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        assert!(limiter.check_and_increment(ip1));
        assert!(!limiter.check_and_increment(ip1));
        assert!(limiter.check_and_increment(ip2)); // ip2 has its own counter
    }

    #[test]
    fn rate_limiter_evict_removes_expired_entries() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        let limiter = RateLimiter::new(100, Duration::from_millis(1));
        limiter.check_and_increment(ip);
        assert_eq!(limiter.map.len(), 1);

        std::thread::sleep(Duration::from_millis(5));
        limiter.evict_expired();
        assert_eq!(limiter.map.len(), 0);
    }
}
