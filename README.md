# randomuser

A Rust port of [RandomAPI/Randomuser.me-Node](https://github.com/RandomAPI/Randomuser.me-Node) — a seeded random user profile generator API. Generates realistic fake user data in JSON, XML, YAML, and CSV across 21 nationalities.

## Prerequisites

- [Rust](https://rustup.rs) 1.75 or later (uses the 2021 edition)

No other system dependencies are required.

## Building

```sh
# Debug build (fast compile, slower binary)
cargo build

# Optimised release build
cargo build --release

# With MongoDB stats persistence enabled
cargo build --release --features mongodb
```

## Running the tests

```sh
cargo test
```

This runs 127 tests: 97 unit tests embedded in each source module plus 30 integration tests in `tests/api.rs`.

## Configuration

All configuration is via environment variables. Every variable is optional; defaults are shown below.

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | TCP port to listen on |
| `DATA_DIR` | `data` | Path to the nationality data directory |
| `MAX_RESULTS` | `5000` | Maximum results allowed per request |
| `RATE_LIMIT` | `20000` | Max requests per IP per rate window |
| `RATE_WINDOW_SECS` | `300` | Rate-limit window length in seconds |
| `MONGODB_URI` | _(unset)_ | MongoDB connection string; requires `--features mongodb` at build time |
| `TRUSTED_PROXY` | `0` | Set to `1`, `true`, or `yes` to trust `X-Forwarded-For` / `X-Real-IP` headers |
| `BASE_URL` | _(unset)_ | Public URL of this deployment (e.g. `https://api.example.com`); sets the contact link and server entry in the OpenAPI spec |
| `RUST_LOG` | `randomuser=info` | Log filter (see below) |

### MongoDB (optional)

MongoDB stats persistence is an **opt-in compile-time feature**. To enable it, build with `--features mongodb`:

```sh
cargo build --release --features mongodb
```

When `MONGODB_URI` is set at runtime and the feature is compiled in, every API request is logged as a document in the `randomuser.requests` collection. If MongoDB is unreachable at startup, a warning is logged and the API continues serving normally — stats are simply not persisted.

If `MONGODB_URI` is set but the `mongodb` feature was not compiled in, the server logs a warning at startup and ignores the variable.

```sh
# Run with MongoDB stats enabled (binary must be built with --features mongodb)
MONGODB_URI=mongodb://localhost:27017 ./target/release/randomuser

# Run without MongoDB (default)
cargo run
```

Each request document has this shape:

```json
{
  "ts": "2026-06-29T12:00:00Z",
  "version": "1.4",
  "results": 5,
  "seed": "abc123",
  "page": 1,
  "nat": ["US", "GB"],
  "inc": ["name", "email"],
  "fmt": "json",
  "ip": "127.0.0.1"
}
```

### Rate limiting

Requests are rate-limited per client IP using a fixed sliding window. When a client exceeds `RATE_LIMIT` requests within `RATE_WINDOW_SECS` seconds, the server returns HTTP 429:

```json
{
  "error": "Whoa, ease up there cowboy. You've requested 20001 users in the last window. ..."
}
```

The rate-limiter map is capped at 1 000 000 distinct IPs. Expired entries are swept once per window by a background task; the cap also triggers a proactive sweep if reached between sweeps. Limits are tracked in memory and reset when the server restarts.

### Trusted proxy

When the server runs behind a reverse proxy (nginx, Caddy, etc.), set `TRUSTED_PROXY=1` so that rate limiting uses the real client IP from `X-Forwarded-For` rather than the proxy's address. Do **not** enable this unless the proxy is actually trusted — clients can spoof these headers to bypass rate limiting.

### Log verbosity

```sh
RUST_LOG=debug cargo run    # verbose (request tracing)
RUST_LOG=info  cargo run    # default (startup messages only)
RUST_LOG=warn  cargo run    # silent unless something goes wrong
```

## Running the server

The server must be started from the project root because it loads the `data/` directory at runtime using a relative path (override with `DATA_DIR`):

```sh
cd /path/to/randomuser
cargo run
```

On startup you will see (default build, no `--features mongodb`):

```
INFO randomuser: Loading generator data from "data" …
INFO randomuser: Loaded 21 nationalities: AU BR CA CH DE DK ES FI FR GB IE IN IR MX NL NO NZ RS TR UA US
INFO randomuser: Listening on http://0.0.0.0:3000
```

With `--features mongodb` compiled in and no `MONGODB_URI` set:

```
INFO randomuser: MongoDB stats disabled (set MONGODB_URI to enable)
INFO randomuser: Loading generator data from "data" …
INFO randomuser: Loaded 21 nationalities: AU BR CA CH DE DK ES FI FR GB IE IN IR MX NL NO NZ RS TR UA US
INFO randomuser: Listening on http://0.0.0.0:3000
```

The server listens on port **3000** by default.

---

## API

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api` | Generate users (latest version) |
| `GET` | `/api/` | Same as above |
| `GET` | `/api/1.4` | Versioned endpoint (currently identical behaviour) |
| `GET` | `/stats` | JSON snapshot of accumulated request counts |
| `GET` | `/stats/stream` | Server-Sent Events stream of live stats |
| `GET` | `/docs` | Interactive API documentation (Scalar UI) |

All `/api` parameters are passed as query string arguments. All are optional.

---

### Query parameters

#### `results`

Number of users to generate.

- **Type:** integer
- **Default:** `1`
- **Max:** `5000` (server default; configurable via `MAX_RESULTS`)
- Values above the maximum are clamped to the maximum rather than falling back to 1

```sh
curl "http://localhost:3000/api?results=10"
```

#### `seed`

Seed string for the random number generator. Given the same seed and page, the API always returns identical output. If omitted, a random seed is generated per request.

- **Type:** string (any value)
- **Default:** random

```sh
curl "http://localhost:3000/api?seed=foobar&results=5"
```

#### `page`

Page number for paginated use with a fixed seed. Each page produces a different, non-overlapping set of results.

- **Type:** positive integer
- **Default:** `1`

```sh
curl "http://localhost:3000/api?seed=foobar&results=5&page=2"
```

> The seed and page together uniquely identify a batch of results. Page 1 with seed `abc` always returns the same users; page 2 with seed `abc` returns the next batch.

#### `gender`

Filter results to a single gender.

- **Type:** `male` | `female` | `nonbinary`
- **Default:** random mix (approximately 47.5 % male, 47.5 % female, 5 % nonbinary)

```sh
curl "http://localhost:3000/api?gender=female&results=3"
curl "http://localhost:3000/api?gender=nonbinary&results=3"
```

#### `nat`

Comma-separated list of nationality codes to draw from. If omitted, all 21 nationalities are used. Unknown codes are silently ignored; if all codes are unknown the filter is dropped and all nationalities are used.

- **Type:** comma-separated string
- **Default:** all nationalities

Supported codes:

| Code | Country | Code | Country | Code | Country |
|------|---------|------|---------|------|---------|
| `AU` | Australia | `GB` | United Kingdom | `NO` | Norway |
| `BR` | Brazil | `IE` | Ireland | `NZ` | New Zealand |
| `CA` | Canada | `IN` | India | `RS` | Serbia |
| `CH` | Switzerland | `IR` | Iran | `TR` | Turkey |
| `DE` | Germany | `MX` | Mexico | `UA` | Ukraine |
| `DK` | Denmark | `NL` | Netherlands | `US` | United States |
| `ES` | Spain | `FR` | France | | |
| `FI` | Finland | | | | |

```sh
curl "http://localhost:3000/api?nat=us,gb,au&results=5"
```

#### `inc`

Comma-separated list of fields to **include**. Only the listed fields appear in each result object. Unknown field names are silently ignored.

- **Type:** comma-separated string
- **Default:** all fields

```sh
curl "http://localhost:3000/api?inc=name,email,nat"
```

#### `exc`

Comma-separated list of fields to **exclude**. Processed after `inc`; if both are given, `exc` removes fields from the `inc` result.

- **Type:** comma-separated string
- **Default:** none excluded

```sh
curl "http://localhost:3000/api?exc=picture,login"
```

Available field names: `gender`, `pronouns`, `name`, `location`, `email`, `login`, `registered`, `dob`, `phone`, `cell`, `id`, `picture`, `nat`

#### `fmt` / `format`

Output format. `fmt` and `format` are aliases.

- **Type:** string
- **Default:** `json`

| Value | Description | Content-Type |
|-------|-------------|-------------|
| `json` | Compact JSON | `application/json` |
| `pretty` or `prettyjson` | Indented JSON | `application/json` |
| `xml` | XML with `<?xml ...?>` declaration | `text/xml` |
| `yaml` | YAML | `text/x-yaml` |
| `csv` | CSV with dot-notation headers | `text/csv` |

Unknown values fall back to `json`.

```sh
curl "http://localhost:3000/api?fmt=xml&results=2"
curl "http://localhost:3000/api?fmt=csv&results=100"
curl "http://localhost:3000/api?fmt=pretty"
```

#### `password`

Custom password character set and length specification. If omitted, passwords are drawn from the built-in wordlist.

- **Type:** comma-separated list of charset names and an optional length spec
- **Default:** wordlist password

**Charset names:**

| Name | Characters |
|------|-----------|
| `upper` | `A–Z` |
| `lower` | `a–z` |
| `number` | `0–9` |
| `special` | `` !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~ `` |

**Length spec** (optional, must be the last element):

| Format | Meaning |
|--------|---------|
| `8` | Exactly 8 characters |
| `8-16` | Between 8 and 16 characters (random) |

Length is clamped to `[1, 64]`. If no length spec is given, the default is `8–64`.

```sh
# Lowercase + numbers, 8–12 chars
curl "http://localhost:3000/api?password=lower,number,8-12"

# All charsets, exactly 16 chars
curl "http://localhost:3000/api?password=upper,lower,number,special,16"
```

#### `noinfo`

When present (any value), omits the `info` block from the response.

```sh
curl "http://localhost:3000/api?noinfo=1"
```

#### `lego`

When present, all results use the `LEGO` nationality (LEGO-themed names and avatar pictures).

```sh
curl "http://localhost:3000/api?lego=1&results=3"
```

#### `callback`

JSONP callback name. Wraps the JSON response in `callbackName(...);`. Only applies when the output format is JSON. Must be a valid dot-separated JavaScript identifier (e.g. `MyApp.onData`); invalid names return HTTP 400.

```sh
curl "http://localhost:3000/api?callback=myHandler"
# → myHandler({...});
```

#### `dl` / `download`

When present, the response is served as a file download (`Content-Disposition: attachment`) with content type `application/octet-stream` instead of the format's native content type.

```sh
curl "http://localhost:3000/api?fmt=csv&results=500&dl=1" -o users.csv
```

---

### Response structure

#### JSON (default)

```json
{
  "results": [
    {
      "gender": "female",
      "pronouns": "she/her",
      "name": { "title": "Ms", "first": "Emily", "last": "Johnson" },
      "location": {
        "street": { "number": 4821, "name": "Maple Avenue" },
        "city": "Springfield",
        "state": "Illinois",
        "country": "United States",
        "postcode": 62704,
        "coordinates": { "latitude": "41.8781", "longitude": "-87.6298" },
        "timezone": { "offset": "-06:00", "description": "Central Time (US & Canada)" }
      },
      "email": "emily.johnson@example.com",
      "login": {
        "uuid": "a3f2e1d4-...",
        "username": "happybird472",
        "password": "letmein",
        "salt": "xK9mNp2q",
        "md5": "c4ca4238...",
        "sha1": "356a192b...",
        "sha256": "6b86b273..."
      },
      "registered": { "date": "2015-03-12T08:44:21.000Z", "age": 9 },
      "dob": { "date": "1988-07-24T14:32:05.000Z", "age": 36 },
      "phone": "(555) 867-5309",
      "cell": "(555) 012-3456",
      "id": { "name": "SSN", "value": "123-45-6789" },
      "picture": {
        "large": "https://randomuser.me/api/portraits/women/42.jpg",
        "medium": "https://randomuser.me/api/portraits/med/women/42.jpg",
        "thumbnail": "https://randomuser.me/api/portraits/thumb/women/42.jpg"
      },
      "nat": "US"
    }
  ],
  "info": {
    "seed": "a1b2c3d4e5f6g7h8",
    "results": 1,
    "page": 1,
    "version": "1.4"
  }
}
```

Fields appear in the canonical order shown above. The `info` block is omitted when `noinfo` is set.

#### Field reference

| Field | Type | Notes |
|-------|------|-------|
| `gender` | string | `"male"`, `"female"`, or `"nonbinary"` |
| `pronouns` | string | `"he/him"`, `"she/her"`, or `"they/them"` — matches `gender` |
| `name` | object | `title`, `first`, `last`; title is `Mx` for nonbinary users |
| `location` | object | `street` (number + name), `city`, `state`, `country`, `postcode`, `coordinates` (latitude + longitude), `timezone` (offset + description); coordinates and timezone are drawn from a per-nationality geographic bounding box |
| `email` | string | `firstname.lastname@example.com`; non-ASCII characters are transliterated |
| `login` | object | `uuid`, `username`, `password`, `salt`, `md5`, `sha1`, `sha256` |
| `registered` | object | `date` (RFC 3339), `age` (years) |
| `dob` | object | `date` (RFC 3339), `age` (years); always earlier than `registered` |
| `phone` | string | Formatted per nationality |
| `cell` | string | Formatted per nationality |
| `id` | object | `name` (ID type) and `value`; see nationality-specific IDs below |
| `picture` | object | `large`, `medium`, `thumbnail` — URLs pointing to randomuser.me CDN |
| `nat` | string | Two-letter nationality code |

#### Nationality-specific ID types

| Code | ID name | Format |
|------|---------|--------|
| `AU` | TFN | 9-digit Tax File Number |
| `BR` | CPF | `NNN.NNN.NNN-NN` (validated) |
| `CA` | SIN | 9-digit Social Insurance Number (Luhn-validated) |
| `CH` | AVS | `756.XXXX.XXXX.XY` where Y is an EAN-13 check digit |
| `DE` | SVNR | `DDXXXXYYMMGGPPC` format |
| `DK` | CPR | `DDMMYY-NNNN` |
| `ES` | DNI | `NNNNNNNL` |
| `FI` | HETU | `DDMMYYXNNNNC` (personal identity code, check character validated) |
| `FR` | INSEE | 13-digit social security number with 2-digit key |
| `GB` | NINO | `XX NNNNNN X` (National Insurance Number) |
| `IE` | PPS | `NNNNNNNXA` (post-2013) or `NNNNNNNX` (pre-2013) |
| `IN` | UIDAI | 12-digit Aadhaar number |
| `IR` | — | Empty (`name: ""`, `value: null`) |
| `MX` | NSS | `NN NN NN NNNN N` (IMSS social security number) |
| `NL` | BSN | 8-digit citizen service number |
| `NO` | FN | 11-digit fødselsnummer with validated check digits |
| `NZ` | — | Empty (`name: ""`, `value: null`) |
| `RS` | SID | 9-digit serial ID |
| `TR` | — | Empty (`name: ""`, `value: null`) |
| `UA` | — | Empty (`name: ""`, `value: null`) |
| `US` | SSN | `NNN-NN-NNNN` (validated; rejects known invalid patterns) |

#### Nationality-specific postcode formats

Most nationalities generate a 5-digit integer postcode. Exceptions:

| Code | Format | Example |
|------|--------|---------|
| `AU` | Integer 200–9999 | `2000` |
| `CA` | Letter-digit-letter space digit-letter-digit | `K1A 0A1` |
| `CH` | Integer 1000–9999 | `8001` |
| `NO` | Real postcode drawn from dataset | `0150` |

---

### Pagination example

Seed + page lets you page through a deterministic dataset:

```sh
# Page 1
curl "http://localhost:3000/api?seed=mydata&results=100&page=1"

# Page 2 — next 100 unique users, same seed
curl "http://localhost:3000/api?seed=mydata&results=100&page=2"
```

---

### Format examples

```sh
# Pretty-printed JSON
curl "http://localhost:3000/api?fmt=pretty&results=1"

# XML
curl "http://localhost:3000/api?fmt=xml&results=2"

# YAML
curl "http://localhost:3000/api?fmt=yaml&results=1"

# CSV — headers use dot notation for nested fields (e.g. name.first, location.city)
curl "http://localhost:3000/api?fmt=csv&results=50"

# CSV file download
curl "http://localhost:3000/api?fmt=csv&results=500&dl=1" -o users.csv
```

---

## Stats endpoints

### `GET /stats`

Returns a JSON snapshot of accumulated request counts since the server started.

```sh
curl http://localhost:3000/stats
```

```json
{
  "total_requests": 1042,
  "by_nat": {
    "US": 312,
    "GB": 198,
    "FR": 154
  }
}
```

### `GET /stats/stream`

Server-Sent Events stream that pushes an updated snapshot after every API request. The connection is kept alive with a comment every 15 seconds during quiet periods.

```sh
curl -N http://localhost:3000/stats/stream
```

```
event: stats
data: {"total_requests":1043,"by_nat":{"US":313,"GB":198,"FR":154}}

event: stats
data: {"total_requests":1044,"by_nat":{"US":313,"GB":198,"FR":155}}
```

Stats are in-memory only unless `MONGODB_URI` is configured and the `mongodb` Cargo feature is enabled. Counts reset on server restart.

---

## Interactive API docs

The server exposes a [Scalar](https://scalar.com) interactive documentation UI at `/docs`. It renders the full OpenAPI spec, lets you try requests live, and shows example responses for every endpoint and parameter.

```
http://localhost:3000/docs
```

---

## Project layout

```
randomuser/
├── src/
│   ├── main.rs               # Tokio + Axum server; wires state and routes
│   ├── lib.rs                # Module declarations
│   ├── config.rs             # Config struct; Config::from_env()
│   ├── generator/
│   │   ├── mod.rs            # Generator struct; generate() entry point
│   │   ├── prng.rs           # MT19937 wrapper; seeding, UUID, lat/lon
│   │   ├── formats.rs        # JSON / pretty / XML / YAML / CSV serialisers
│   │   └── nat/
│   │       ├── mod.rs        # Data loading; inject dispatch; shared helpers
│   │       ├── geo.rs        # Per-nationality bounding boxes and timezone lists
│   │       ├── au.rs         # Australia
│   │       ├── br.rs         # Brazil
│   │       └── ...           # One file per nationality
│   ├── stats/
│   │   ├── mod.rs            # StatEvent, LiveStats, RateLimiter, StatsHandle
│   │   └── mongo.rs          # Background MongoDB writer task (mongodb feature)
│   └── routes/
│       ├── api.rs            # AppState; handlers for /api and /api/:version
│       ├── openapi.rs        # utoipa schema types and Scalar docs router (/docs)
│       └── stats.rs          # Handlers for /stats and /stats/stream
├── tests/
│   └── api.rs                # 30 integration tests
├── data/                     # Nationality data files (names, cities, etc.)
│   ├── common/lists/         # Shared: passwords, timezones, titles, usernames
│   └── <NAT>/lists/          # Per-nationality: first names, last names, cities, states, streets
└── Cargo.toml
```

---

## Differences from the upstream Node.js implementation

This port corrects several bugs present in the original and adds features not in the original:

1. **Seed UTF-8 safety**: The original JS implementation sliced the seed string by byte index, which is safe in JS (UTF-16 strings). The Rust port ensures slicing occurs on a valid char boundary to avoid panicking on multi-byte UTF-8 seeds of length 18.

2. **FI HETU — invalid date**: The original called `new Date(dob)` where `dob` was a JSON object, producing `Invalid Date`. This port parses the ISO string from `dob.date`.

3. **FI HETU — wrong day-of-month**: The original used `getDay()` (day of week, 0–6) instead of `getDate()` (day of month, 1–31) when building the HETU date string.

4. **FI HETU — wrong check character**: The original computed the HETU check character using unpadded day and month values (e.g. `1` for January instead of `01`), producing wrong check characters for days < 10 and months < 10. This port zero-pads both before computing the check.

5. **DE / DK / FR — 3-digit year**: The original used JS `getYear()` which returns values like `104` for the year 2004. This port uses `year % 100` to correctly produce a 2-digit year.

6. **NO FNR — invalid check digits**: The original did not validate that K1 and K2 check digits are in range (must be 0–9; a value of 10 means the NNN is invalid and a new one should be picked). This port retries on out-of-range check digits.

7. **US SSN — unbounded retry loop**: The original could loop indefinitely generating SSNs. This port adds a retry ceiling with a deterministic fallback.

8. **CH AVS — missing check digit**: The original generated both digits of the last group randomly, producing numbers that fail EAN-13 validation. This port computes the correct check digit from the preceding 12 digits.

9. **Geo-accurate coordinates and timezones**: The original drew coordinates and timezone independently at random from global ranges. This port restricts coordinates to a per-nationality WGS 84 bounding box and selects the timezone from a list applicable to that country.

10. **Nonbinary gender and pronouns**: The original supported only male and female. This port adds nonbinary (≈ 5 % when unfiltered) with `they/them` pronouns and `Mx` title.
