// Schema structs in this module are never constructed at runtime — they exist
// solely to drive utoipa's OpenAPI schema generation via proc macros.
#![allow(dead_code)]

use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

// ─── Response schema types ────────────────────────────────────────────────────
// These structs exist only to drive utoipa's schema generation. They mirror the
// actual JSON shape produced by the generator but are never instantiated at
// runtime.

/// Full API response envelope.
#[derive(utoipa::ToSchema)]
#[schema(example = json!({
    "results": [{
        "gender": "male",
        "pronouns": "he/him",
        "name": {"title": "Mr", "first": "John", "last": "Doe"},
        "email": "john.doe@example.com",
        "nat": "US"
    }],
    "info": {"seed": "abc123", "results": 1, "page": 1, "version": "1.4"}
}))]
pub struct RandomUserResponse {
    /// Array of generated user objects. Length equals the `results` query parameter.
    results: Vec<UserObject>,
    /// Request metadata. Absent when the `noinfo` query parameter is set.
    info: Option<InfoObject>,
}

/// A single generated user profile.
#[derive(utoipa::ToSchema)]
pub struct UserObject {
    /// Biological or self-identified gender.
    #[schema(example = "male")]
    gender: String,
    /// Grammatical pronouns matching the user's gender.
    #[schema(example = "he/him")]
    pronouns: String,
    name: NameObject,
    location: LocationObject,
    /// RFC 5321 email address. Always ends with `@example.com`.
    #[schema(example = "john.doe@example.com")]
    email: String,
    login: LoginObject,
    /// Date and age when the user registered.
    registered: DateAgeObject,
    /// Date of birth and current age.
    dob: DateAgeObject,
    /// Primary phone number (format varies by nationality).
    #[schema(example = "(555) 867-5309")]
    phone: String,
    /// Mobile/cell number (format varies by nationality).
    #[schema(example = "(555) 123-4567")]
    cell: String,
    id: IdObject,
    picture: PictureObject,
    /// ISO 3166-1 alpha-2 country code.
    #[schema(example = "US")]
    nat: String,
}

/// User's full name.
#[derive(utoipa::ToSchema)]
pub struct NameObject {
    /// Honorific / title (e.g. `Mr`, `Ms`, `Dr`, `Monsieur`, `Mx`).
    #[schema(example = "Mr")]
    title: String,
    #[schema(example = "John")]
    first: String,
    #[schema(example = "Doe")]
    last: String,
}

/// User's mailing address.
#[derive(utoipa::ToSchema)]
pub struct LocationObject {
    street: StreetObject,
    #[schema(example = "Springfield")]
    city: String,
    #[schema(example = "Illinois")]
    state: String,
    #[schema(example = "United States")]
    country: String,
    /// Postal code. Integer for most nationalities (e.g. `10001` for US), but
    /// a string for some (e.g. `"SW1A 2AA"` for GB, `"K1A 0A9"` for CA).
    #[schema(value_type = serde_json::Value, example = json!(10001))]
    postcode: serde_json::Value,
    coordinates: CoordinatesObject,
    timezone: TimezoneObject,
}

/// Street address components.
#[derive(utoipa::ToSchema)]
pub struct StreetObject {
    #[schema(example = 742)]
    number: u32,
    #[schema(example = "Evergreen Terrace")]
    name: String,
}

/// WGS 84 geographic coordinates, always within the user's nationality bounding box.
#[derive(utoipa::ToSchema)]
pub struct CoordinatesObject {
    #[schema(example = "40.7128")]
    latitude: String,
    #[schema(example = "-74.0060")]
    longitude: String,
}

/// UTC offset and descriptive label for the user's timezone.
#[derive(utoipa::ToSchema)]
pub struct TimezoneObject {
    /// ISO 8601 UTC offset string with zero-padded hours.
    #[schema(example = "-05:00")]
    offset: String,
    #[schema(example = "Eastern Time (US & Canada)")]
    description: String,
}

/// Hashed login credentials and a v4 UUID.
#[derive(utoipa::ToSchema)]
pub struct LoginObject {
    /// RFC 4122 v4 UUID.
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    uuid: String,
    #[schema(example = "greenfish482")]
    username: String,
    #[schema(example = "hunter2")]
    password: String,
    #[schema(example = "a4b3f1c9")]
    salt: String,
    #[schema(example = "6384e2b2184bcbf58eccadd21d2bf3bf")]
    md5: String,
    #[schema(example = "aa57264e3a7a8c3cfd9c4e3e7c1b7e9a3e7ab3e1")]
    sha1: String,
    #[schema(example = "3c9909afec25354d551dae21590bb26e38d53f2173b8d3dc3eee4c047e7ab1c1")]
    sha256: String,
}

/// An ISO 8601 date with a computed age in whole years.
#[derive(utoipa::ToSchema)]
pub struct DateAgeObject {
    /// RFC 3339 timestamp.
    #[schema(example = "1985-03-15T00:00:00.000Z")]
    date: String,
    #[schema(example = 38)]
    age: i64,
}

/// National identification document.
#[derive(utoipa::ToSchema)]
pub struct IdObject {
    /// Document type abbreviation (e.g. `SSN`, `NINO`, `HETU`).
    #[schema(example = "SSN")]
    name: String,
    /// Document value. `null` for LEGO users which have no real-world equivalent.
    #[schema(example = "123-45-6789", nullable)]
    value: Option<String>,
}

/// Portrait picture URLs at three sizes.
#[derive(utoipa::ToSchema)]
pub struct PictureObject {
    #[schema(example = "https://randomuser.me/api/portraits/men/75.jpg")]
    large: String,
    #[schema(example = "https://randomuser.me/api/portraits/med/men/75.jpg")]
    medium: String,
    #[schema(example = "https://randomuser.me/api/portraits/thumb/men/75.jpg")]
    thumbnail: String,
}

/// Response envelope metadata.
#[derive(utoipa::ToSchema)]
pub struct InfoObject {
    /// The seed used to generate this response (auto-generated if not supplied).
    #[schema(example = "abc123def456")]
    seed: String,
    /// Number of user objects in this response.
    #[schema(example = 1)]
    results: usize,
    /// Page number (for seeded pagination).
    #[schema(example = 1)]
    page: u32,
    /// API version string.
    #[schema(example = "1.4")]
    version: String,
}

/// Error response body.
#[derive(utoipa::ToSchema)]
pub struct ErrorResponse {
    #[schema(example = "Invalid callback name")]
    error: String,
}

// ─── OpenAPI spec ─────────────────────────────────────────────────────────────

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Random User API",
        version = "1.4",
        description = "Generate random, realistic-looking user profiles for testing and prototyping.\n\n\
            All query parameters are optional. Responses are deterministic when `seed` is provided.\n\n\
            **Output formats:** `json` (default) · `pretty` · `xml` · `yaml` · `csv` — controlled by the `fmt` parameter.\n\n\
            **Nationalities:** AU BR CA CH DE DK ES FI FR GB IE IN IR MX NL NO NZ RS TR UA US",
        license(name = "Apache-2.0", url = "https://www.apache.org/licenses/LICENSE-2.0"),
    ),
    paths(
        crate::routes::api::handle_latest,
        crate::routes::api::handle_versioned,
        crate::routes::stats::handle_stats_snapshot,
        crate::routes::stats::handle_stats_stream,
    ),
    components(
        schemas(
            RandomUserResponse,
            UserObject,
            NameObject,
            LocationObject,
            StreetObject,
            CoordinatesObject,
            TimezoneObject,
            LoginObject,
            DateAgeObject,
            IdObject,
            PictureObject,
            InfoObject,
            ErrorResponse,
            crate::stats::StatsSnapshot,
        )
    ),
    tags(
        (name = "Generate", description = "Random user profile generation"),
        (name = "Stats",    description = "Live request statistics"),
    ),
)]
pub struct ApiDoc;

// ─── Router ───────────────────────────────────────────────────────────────────

/// Returns an Axum router that serves the Scalar interactive docs at `/docs`.
///
/// `base_url` — public URL of this deployment; sets the OpenAPI server entry and
/// contact URL so Scalar try-it requests target the right host.
///
/// `site_name` — display name for the API title and contact; defaults to
/// `"Random User API"` when unset.
pub fn docs_router<S: Clone + Send + Sync + 'static>(
    base_url: Option<&str>,
    site_name: Option<&str>,
) -> Router<S> {
    let mut spec = ApiDoc::openapi();

    if let Some(name) = site_name {
        spec.info.title = name.to_string();
    }

    if base_url.is_some() || site_name.is_some() {
        let contact = utoipa::openapi::ContactBuilder::new()
            .name(site_name.map(str::to_string))
            .url(base_url.map(str::to_string))
            .build();
        spec.info.contact = Some(contact);
    }

    if let Some(url) = base_url {
        spec.servers = Some(vec![
            utoipa::openapi::ServerBuilder::new()
                .url(url)
                .description(Some("This deployment"))
                .build(),
        ]);
    }

    Router::new().merge(Scalar::with_url("/docs", spec))
}
