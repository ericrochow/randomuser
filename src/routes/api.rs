use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use crate::generator::{is_safe_callback, GenerateOptions, Generator};
use crate::stats::{RateLimiter, StatEvent, StatsHandle};

/// Shared state cloned into every request handler.
#[derive(Clone)]
pub struct AppState {
    pub generator: Arc<Generator>,
    pub limiter: RateLimiter,
    pub stats: StatsHandle,
    pub max_results: usize,
    /// When true, extract the real client IP from X-Forwarded-For / X-Real-IP.
    /// Enable only when the server sits behind a trusted reverse proxy.
    pub trusted_proxy: bool,
}

#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ApiQuery {
    /// Number of results to return. Clamped to `[1, max_results]` (server default 5 000).
    #[param(example = 10, minimum = 1)]
    pub results: Option<usize>,
    /// Seed for reproducible output. The same seed + page always produces the same users.
    #[param(example = "abc123")]
    pub seed: Option<String>,
    /// Page number for paginated, seeded requests (default 1).
    #[param(example = 1, minimum = 1)]
    pub page: Option<u32>,
    /// Filter results by gender. Accepted values: `male`, `female`, `nonbinary`.
    #[param(example = "male")]
    pub gender: Option<String>,
    /// Comma-separated nationality codes to restrict results to, e.g. `US,FR,DE`.
    /// Accepts AU BR CA CH DE DK ES FI FR GB IE IN IR MX NL NO NZ RS TR UA US.
    #[param(example = "US,GB")]
    pub nat: Option<String>,
    /// Comma-separated field names to include. All fields are returned when omitted.
    #[param(example = "name,email,location")]
    pub inc: Option<String>,
    /// Comma-separated field names to exclude.
    #[param(example = "login,picture")]
    pub exc: Option<String>,
    /// Output format. One of `json` (default), `pretty`, `xml`, `yaml`, `csv`.
    #[param(example = "json")]
    pub fmt: Option<String>,
    /// Alias for `fmt`.
    pub format: Option<String>,
    /// Password character-set spec. Comma-separated tokens from
    /// `upper` `lower` `number` `special`, optionally followed by a length range `8-16`.
    #[param(example = "upper,lower,number,8-16")]
    pub password: Option<String>,
    /// Set to any value to request LEGO-themed user pictures.
    pub lego: Option<String>,
    /// Set to any value to trigger a file-download (`Content-Disposition: attachment`) response.
    pub dl: Option<String>,
    /// Alias for `dl`.
    pub download: Option<String>,
    /// JSONP callback function name. Must be a valid dot-separated JS identifier (e.g. `MyApp.cb`).
    #[param(example = "MyApp.onData")]
    pub callback: Option<String>,
    /// Set to any value to omit the `info` block from the response.
    pub noinfo: Option<String>,
}

/// Generate random user data.
///
/// Returns one or more randomly generated user profiles. All parameters are optional.
/// Use `seed` + `page` for reproducible, paginated datasets. The `fmt` parameter
/// switches the response body between JSON, XML, YAML, and CSV — only JSON is shown here.
#[utoipa::path(
    get,
    path = "/api",
    params(ApiQuery),
    responses(
        (status = 200, description = "Random user data in the requested format",
         body = crate::routes::openapi::RandomUserResponse, content_type = "application/json"),
        (status = 400, description = "Invalid JSONP callback name",
         body = crate::routes::openapi::ErrorResponse),
        (status = 429, description = "Rate limit exceeded",
         body = crate::routes::openapi::ErrorResponse),
    ),
    tag = "Generate",
)]
pub async fn handle_latest(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ApiQuery>,
) -> Response {
    handle(state, addr, headers, q).await
}

/// Generate random user data (versioned endpoint).
///
/// Identical to `GET /api` but requires an explicit version in the path.
/// Returns 404 if the version does not match the running API version (`1.4`).
#[utoipa::path(
    get,
    path = "/api/{version}",
    params(
        ApiQuery,
        ("version" = String, Path, description = "API version — only `1.4` is currently valid",
         example = "1.4"),
    ),
    responses(
        (status = 200, description = "Random user data in the requested format",
         body = crate::routes::openapi::RandomUserResponse, content_type = "application/json"),
        (status = 400, description = "Invalid JSONP callback name",
         body = crate::routes::openapi::ErrorResponse),
        (status = 404, description = "Unknown API version",
         body = crate::routes::openapi::ErrorResponse),
        (status = 429, description = "Rate limit exceeded",
         body = crate::routes::openapi::ErrorResponse),
    ),
    tag = "Generate",
)]
pub async fn handle_versioned(
    Path(version): Path<String>,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ApiQuery>,
) -> Response {
    if version != state.generator.version() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("API version '{}' not found", version) })),
        )
            .into_response();
    }
    handle(state, addr, headers, q).await
}

async fn handle(
    state: AppState,
    addr: SocketAddr,
    headers: HeaderMap,
    q: ApiQuery,
) -> Response {
    let ip = real_ip(&headers, addr.ip(), state.trusted_proxy);

    if let Some(count) = state.limiter.check_and_increment(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": format!(
                    "Whoa, ease up there cowboy. You've requested {} users in the last \
                     window. Help us keep this service free and spare some bandwidth \
                     for other users please :)",
                    count
                )
            })),
        )
            .into_response();
    }

    // Validate JSONP callback before it reaches the generator.
    if let Some(cb) = &q.callback {
        if !is_safe_callback(cb) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid callback name" })),
            )
                .into_response();
        }
    }

    let fmt = q.fmt.or(q.format);
    let is_download = q.dl.is_some() || q.download.is_some();

    // Capture for stats before values are moved into GenerateOptions.
    let stat_nat: Vec<String> = q
        .nat
        .as_deref()
        .map(|n| n.split(',').map(|s| s.trim().to_uppercase()).collect())
        .unwrap_or_default();
    let stat_inc: Vec<String> = q
        .inc
        .as_deref()
        .map(|i| i.split(',').map(|s| s.trim().to_lowercase()).collect())
        .unwrap_or_default();
    let stat_fmt = fmt.clone().unwrap_or_else(|| "json".to_string());
    let stat_seed = q.seed.clone().unwrap_or_default();
    let stat_page = q.page.unwrap_or(1);

    let out = state.generator.generate(GenerateOptions {
        results: q.results,
        seed: q.seed,
        page: q.page,
        gender: q.gender,
        nat: q.nat,
        inc: q.inc,
        exc: q.exc,
        fmt: fmt.clone(),
        password: q.password,
        lego: q.lego.is_some(),
        noinfo: q.noinfo.is_some(),
        callback: q.callback,
        max_results: state.max_results,
    });

    // Fire-and-forget — never blocks the response.
    state.stats.record(StatEvent {
        ts: Utc::now(),
        version: state.generator.version().to_string(),
        // Use the count the generator actually produced rather than re-deriving it.
        results: out.resolved_results,
        seed: stat_seed,
        page: stat_page,
        nat: stat_nat,
        inc: stat_inc,
        fmt: stat_fmt,
        ip,
    });

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));

    if is_download {
        resp_headers.insert(
            "Content-Disposition",
            HeaderValue::from_str(&format!("attachment; filename=download.{}", out.ext))
                .unwrap_or(HeaderValue::from_static("attachment; filename=download.json")),
        );
        resp_headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/octet-stream"),
        );
    } else {
        resp_headers.insert(
            "Content-Type",
            HeaderValue::from_str(&format!("{}; charset=utf-8", out.content_type))
                .unwrap_or(HeaderValue::from_static("application/json; charset=utf-8")),
        );
    }

    (StatusCode::OK, resp_headers, out.body).into_response()
}

/// Resolve the real client IP address.
///
/// When `trusted_proxy` is true the handler trusts the first IP in
/// `X-Forwarded-For` (or `X-Real-IP` as a fallback). This should only be
/// enabled when the server is guaranteed to be behind a single trusted proxy;
/// otherwise a client can spoof these headers to bypass rate limiting.
fn real_ip(headers: &HeaderMap, peer: IpAddr, trusted_proxy: bool) -> IpAddr {
    if !trusted_proxy {
        return peer;
    }
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<IpAddr>().ok())
        })
        .unwrap_or(peer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_ip_returns_peer_without_trusted_proxy() {
        let peer: IpAddr = "1.2.3.4".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("9.9.9.9"),
        );
        assert_eq!(real_ip(&headers, peer, false), peer);
    }

    #[test]
    fn real_ip_reads_x_forwarded_for_when_trusted() {
        let peer: IpAddr = "1.2.3.4".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("9.9.9.9, 10.0.0.1"),
        );
        let ip = real_ip(&headers, peer, true);
        assert_eq!(ip, "9.9.9.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn real_ip_falls_back_to_x_real_ip() {
        let peer: IpAddr = "1.2.3.4".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("5.6.7.8"));
        let ip = real_ip(&headers, peer, true);
        assert_eq!(ip, "5.6.7.8".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn real_ip_falls_back_to_peer_on_missing_headers() {
        let peer: IpAddr = "1.2.3.4".parse().unwrap();
        let ip = real_ip(&HeaderMap::new(), peer, true);
        assert_eq!(ip, peer);
    }
}
