use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::generator::{Generator, GenerateOptions};

pub type AppState = Arc<Generator>;

#[derive(Debug, Deserialize, Default)]
pub struct ApiQuery {
    pub results: Option<usize>,
    pub seed: Option<String>,
    pub page: Option<u32>,
    pub gender: Option<String>,
    pub nat: Option<String>,
    pub inc: Option<String>,
    pub exc: Option<String>,
    pub fmt: Option<String>,
    /// Alias for fmt
    pub format: Option<String>,
    pub password: Option<String>,
    /// Presence of key (any value) enables lego mode
    pub lego: Option<String>,
    /// Presence of key enables download mode
    pub dl: Option<String>,
    pub download: Option<String>,
    /// JSONP callback
    pub callback: Option<String>,
    /// Presence removes info block
    pub noinfo: Option<String>,
}

pub async fn handle(
    _version: Option<&str>,
    State(generator): State<AppState>,
    Query(q): Query<ApiQuery>,
    max_results: usize,
) -> Response {
    let fmt = q.fmt.or(q.format);
    let is_download = q.dl.is_some() || q.download.is_some();

    let out = generator.generate(GenerateOptions {
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
        max_results,
    });

    let mut headers = HeaderMap::new();
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));

    if is_download {
        headers.insert(
            "Content-Disposition",
            HeaderValue::from_str(&format!("attachment; filename=download.{}", out.ext))
                .unwrap_or(HeaderValue::from_static("attachment; filename=download.json")),
        );
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/octet-stream"),
        );
    } else {
        headers.insert(
            "Content-Type",
            HeaderValue::from_str(&format!("{}; charset=utf-8", out.content_type))
                .unwrap_or(HeaderValue::from_static("application/json; charset=utf-8")),
        );
    }

    (StatusCode::OK, headers, out.body).into_response()
}

/// Handler for `/api` (latest version)
pub async fn handle_latest(
    State(generator): State<AppState>,
    Query(q): Query<ApiQuery>,
) -> Response {
    handle(None, State(generator), Query(q), 5000).await
}

/// Handler for `/api/:version`
pub async fn handle_versioned(
    Path(version): Path<String>,
    State(generator): State<AppState>,
    Query(q): Query<ApiQuery>,
) -> Response {
    handle(Some(&version), State(generator), Query(q), 5000).await
}
