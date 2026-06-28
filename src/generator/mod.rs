pub mod formats;
pub mod nat;
pub mod prng;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use formats::{format_csv, format_json, format_xml, format_yaml, ApiResponse, FormatOutput, InfoBlock};
use md5::Md5;
use md5::Digest as Md5Digest;
use nat::{full_nat_name, inject, NatDatasets};
use prng::Prng;
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{json, Map, Value};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::{Digest as Sha2Digest, Sha256};
use std::path::Path;
use unidecode::unidecode;

/// Unix timestamp (seconds) pinned in the original source as May 2022.
/// Used to bound DOB and registration date ranges.
const CONSTANT_TIME: i64 = 1_653_344_189;

/// All user fields in canonical output order.
const ORIGINAL_FIELDS: &[&str] = &[
    "gender", "name", "location", "email", "login", "registered", "dob", "phone",
    "cell", "id", "picture", "nat",
];

pub struct Generator {
    datasets: NatDatasets,
    version: &'static str,
}

#[derive(Debug, Default)]
pub struct GenerateOptions {
    /// Number of results to return (clamped to max_results; invalid → 1)
    pub results: Option<usize>,
    pub seed: Option<String>,
    pub page: Option<u32>,
    pub gender: Option<String>,
    /// Comma-separated nat codes, e.g. "US,FR"
    pub nat: Option<String>,
    /// Comma-separated fields to include
    pub inc: Option<String>,
    /// Comma-separated fields to exclude
    pub exc: Option<String>,
    /// Format: "json" | "pretty" | "prettyjson" | "xml" | "yaml" | "csv"
    pub fmt: Option<String>,
    /// Password spec: "upper,lower,number,8-16"
    pub password: Option<String>,
    pub lego: bool,
    pub noinfo: bool,
    /// JSONP callback name
    pub callback: Option<String>,
    pub max_results: usize,
}

impl Generator {
    pub fn new(version: &'static str) -> Self {
        Self {
            datasets: NatDatasets::default(),
            version,
        }
    }

    pub fn init(&mut self, data_dir: &Path) -> std::io::Result<()> {
        self.datasets = NatDatasets::load(data_dir)?;
        Ok(())
    }

    pub fn nat_codes(&self) -> Vec<String> {
        self.datasets.nat_codes()
    }

    pub fn generate(&self, opts: GenerateOptions) -> FormatOutput {
        let max = if opts.max_results == 0 { 5000 } else { opts.max_results };

        // Resolve result count
        let result_count = opts.results.filter(|&n| n >= 1 && n <= max).unwrap_or(1);

        // Resolve page
        let page = opts.page.unwrap_or(1).max(1);

        // Resolve and normalise format
        let fmt = opts.fmt.as_deref().unwrap_or("json").to_lowercase();
        let pretty = matches!(fmt.as_str(), "pretty" | "prettyjson");

        // Resolve gender filter
        let gender_filter = opts.gender.as_deref().map(|g| g.to_lowercase());
        let gender_filter = gender_filter
            .as_deref()
            .filter(|g| *g == "male" || *g == "female");

        // Resolve nat list
        let all_nats = self.datasets.nat_codes();
        let nat_filter: Option<Vec<String>> = if opts.lego {
            Some(vec!["LEGO".to_string()])
        } else {
            opts.nat.as_deref().map(|n| {
                n.split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    // Only allow known nat codes
                    .filter(|s| all_nats.contains(s) || s == "LEGO")
                    .collect::<Vec<_>>()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
        };

        // Resolve include/exclude lists
        let inc = build_inc(opts.inc.as_deref(), opts.exc.as_deref());

        // Generate (or use provided) seed
        let seed = opts.seed.unwrap_or_else(|| {
            // 16 random hex chars from OS entropy (before MT is seeded)
            rand::thread_rng()
                .sample_iter(Alphanumeric)
                .take(16)
                .map(|b| {
                    let c = b as char;
                    if c.is_ascii_alphanumeric() { c.to_lowercase().next().unwrap() } else { '0' }
                })
                .collect()
        });

        // Seed the PRNG
        let mut prng = Prng::new();
        prng.seed_from_str(&seed, page);

        // Generate users
        let mut results: Vec<Map<String, Value>> = Vec::with_capacity(result_count);
        for _ in 0..result_count {
            let user = self.gen_one_user(&mut prng, &inc, gender_filter, &nat_filter, &opts.password);
            results.push(user);
        }

        let info = if opts.noinfo {
            None
        } else {
            Some(InfoBlock {
                seed: seed.clone(),
                results: result_count,
                page,
                version: self.version,
            })
        };

        let resp = ApiResponse {
            results: &results,
            info,
        };

        let mut out = match fmt.as_str() {
            "xml" => format_xml(&resp),
            "yaml" => format_yaml(&resp),
            "csv" => format_csv(&resp),
            _ => format_json(&resp, pretty),
        };

        // JSONP wrapping
        if let Some(cb) = &opts.callback {
            if out.ext == "json" {
                out.body = format!("{}({});", cb, out.body);
            }
        }

        out
    }

    fn gen_one_user(
        &self,
        prng: &mut Prng,
        inc: &[String],
        gender_filter: Option<&str>,
        nat_filter: &Option<Vec<String>>,
        password_spec: &Option<String>,
    ) -> Map<String, Value> {
        let all_nats = self.datasets.nat_codes();

        // Pick nationality
        let nat = if opts_lego(nat_filter) {
            "LEGO".to_string()
        } else {
            match nat_filter {
                Some(nats) => prng.random_item(nats).clone(),
                None => all_nats[prng.range(0, (all_nats.len() - 1) as i64) as usize].clone(),
            }
        };

        // Pick gender
        let gender = match gender_filter {
            Some(g) => g.to_string(),
            None => {
                let choices = ["male", "female"];
                prng.random_item(&choices).to_string()
            }
        };

        // Pick name
        let (first, last) = self.random_name(&gender, &nat, prng);

        let mut user: Map<String, Value> = Map::new();

        // ── gender ─────────────────────────────────────────────────────────
        if inc_has(inc, "gender") {
            user.insert("gender".to_string(), json!(gender));
        }

        // ── name ───────────────────────────────────────────────────────────
        if inc_has(inc, "name") {
            // FR has its own title list; otherwise use common
            let title_list = if !self.datasets.nat_list(&nat, "title").is_empty() {
                self.datasets.nat_list(&nat, "title")
            } else {
                self.datasets.common_list("title")
            };
            let title = if gender == "male" {
                "Mr".to_string()
            } else {
                prng.random_item(title_list).clone()
            };
            user.insert(
                "name".to_string(),
                json!({ "title": title, "first": first, "last": last }),
            );
        }

        // ── location ───────────────────────────────────────────────────────
        if inc_has(inc, "location") {
            let city = prng.random_item(self.datasets.nat_list(&nat, "cities")).clone();
            let state = prng.random_item(self.datasets.nat_list(&nat, "states")).clone();
            let street_name = prng.random_item(self.datasets.nat_list(&nat, "street")).clone();
            let street_number = prng.range(1, 9999);
            let postcode = prng.range(10_000, 99_999); // default; inject may override
            let tz_str = prng.random_item(self.datasets.common_list("timezones")).clone();
            let timezone: Value = serde_json::from_str(&tz_str).unwrap_or(json!(null));
            let lat = prng.gen_latitude();
            let lon = prng.gen_longitude();

            user.insert(
                "location".to_string(),
                json!({
                    "street": { "number": street_number, "name": street_name },
                    "city": city,
                    "state": state,
                    "country": full_nat_name(&nat),
                    "postcode": postcode,
                    "coordinates": { "latitude": lat, "longitude": lon },
                    "timezone": timezone
                }),
            );
        }

        // ── email ──────────────────────────────────────────────────────────
        if inc_has(inc, "email") {
            let email = format!(
                "{}@example.com",
                unidecode(&format!("{}.{}", first, last))
                    .replace(' ', "")
                    .to_lowercase()
            );
            user.insert("email".to_string(), json!(email));
        }

        // ── login ──────────────────────────────────────────────────────────
        if inc_has(inc, "login") {
            let uuid = prng.gen_uuid();
            let u1 = prng.random_item(self.datasets.common_list("user1")).clone();
            let u2 = prng.random_item(self.datasets.common_list("user2")).clone();
            let num = prng.range(100, 999);
            let username = format!("{}{}{}", u1, u2, num);

            let salt = prng.random_chars(2, 8);
            let password = match password_spec {
                Some(spec) => self.gen_password(spec, prng),
                None => prng.random_item(self.datasets.common_list("passwords")).clone(),
            };

            let pw_salt = format!("{}{}", password, salt);
            let md5_hash = hex::encode(<Md5 as Md5Digest>::digest(pw_salt.as_bytes()));
            let sha1_hash = hex::encode(<Sha1 as Sha1Digest>::digest(pw_salt.as_bytes()));
            let sha256_hash = hex::encode(<Sha256 as Sha2Digest>::digest(pw_salt.as_bytes()));

            user.insert(
                "login".to_string(),
                json!({
                    "uuid": uuid,
                    "username": username,
                    "password": password,
                    "salt": salt,
                    "md5": md5_hash,
                    "sha1": sha1_hash,
                    "sha256": sha256_hash
                }),
            );
        }

        // ── registered ─────────────────────────────────────────────────────
        if inc_has(inc, "registered") {
            let reg_ms = prng.range(1_016_688_461_000, CONSTANT_TIME * 1000);
            let reg_dt = Utc
                .timestamp_millis_opt(reg_ms)
                .single()
                .unwrap_or_else(Utc::now);
            let age_years = age_years(reg_dt);
            user.insert(
                "registered".to_string(),
                json!({ "date": reg_dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true), "age": age_years }),
            );
        }

        // ── dob ────────────────────────────────────────────────────────────
        if inc_has(inc, "dob") {
            let dob_ms = prng.range(-800_000_000_000, CONSTANT_TIME * 1000 - 86_400_000 * 365 * 21);
            let dob_dt = Utc
                .timestamp_millis_opt(dob_ms)
                .single()
                .unwrap_or_else(Utc::now);
            let age_years = age_years(dob_dt);
            user.insert(
                "dob".to_string(),
                json!({ "date": dob_dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true), "age": age_years }),
            );
        }

        // ── picture (set before inject; inject will reorder it after id) ───
        if inc_has(inc, "picture") {
            let gender_text = if gender == "male" { "men" } else { "women" };
            let is_lego = nat == "LEGO";
            let id_max: i64 = if is_lego {
                9
            } else if gender == "male" {
                99
            } else {
                96
            };
            let pic_id = prng.range(0, id_max);
            let (g_dir, base_id) = if is_lego {
                ("lego", pic_id)
            } else {
                (gender_text, pic_id)
            };
            let base = "https://randomuser.me/api/";
            user.insert(
                "picture".to_string(),
                json!({
                    "large": format!("{}portraits/{}/{}.jpg", base, g_dir, base_id),
                    "medium": format!("{}portraits/med/{}/{}.jpg", base, g_dir, base_id),
                    "thumbnail": format!("{}portraits/thumb/{}/{}.jpg", base, g_dir, base_id)
                }),
            );
        }

        // ── nationality inject (phone, cell, id, picture reorder) ──────────
        inject(&nat, inc, &mut user, prng, &self.datasets);

        // ── nat ────────────────────────────────────────────────────────────
        if inc_has(inc, "nat") {
            user.insert("nat".to_string(), json!(nat));
        }

        user
    }

    fn random_name(&self, gender: &str, nat: &str, prng: &mut Prng) -> (String, String) {
        let first_list = if gender == "male" {
            self.datasets.nat_list(nat, "male_first")
        } else {
            self.datasets.nat_list(nat, "female_first")
        };
        let first = if first_list.is_empty() {
            "Jane".to_string()
        } else {
            prng.random_item(first_list).clone()
        };

        let last_list = self.datasets.nat_list(nat, "last");
        let last = if last_list.is_empty() {
            "Doe".to_string()
        } else {
            prng.random_item(last_list).clone()
        };

        (first, last)
    }

    fn gen_password(&self, spec: &str, prng: &mut Prng) -> String {
        if spec.is_empty() {
            return prng
                .random_item(self.datasets.common_list("passwords"))
                .clone();
        }

        let charsets = [
            ("special", " !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"),
            ("upper", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            ("lower", "abcdefghijklmnopqrstuvwxyz"),
            ("number", "0123456789"),
        ];

        let sections: Vec<&str> = spec.split(',').collect();
        let last_section = sections.last().copied().unwrap_or("");

        // Collect requested charsets (deduped)
        let mut charset = String::new();
        for &s in &sections {
            if let Some((_, chars)) = charsets.iter().find(|(name, _)| *name == s) {
                if !charset.contains(chars) {
                    charset.push_str(chars);
                }
            }
        }

        if charset.is_empty() {
            return prng
                .random_item(self.datasets.common_list("passwords"))
                .clone();
        }

        // Parse length spec (last element after all charset names)
        let (min, max) = parse_length(last_section);

        let charset_bytes = charset.as_bytes();
        let length = prng.range(min as i64, max as i64) as usize;
        (0..length)
            .map(|_| {
                let idx = prng.range(0, (charset_bytes.len() - 1) as i64) as usize;
                charset_bytes[idx] as char
            })
            .collect()
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn opts_lego(nat_filter: &Option<Vec<String>>) -> bool {
    nat_filter
        .as_ref()
        .map(|v| v.iter().all(|n| n == "LEGO"))
        .unwrap_or(false)
}

fn inc_has(inc: &[String], field: &str) -> bool {
    inc.iter().any(|f| f == field)
}

/// Build the final include list from `inc` and `exc` query params.
fn build_inc(inc_param: Option<&str>, exc_param: Option<&str>) -> Vec<String> {
    let mut inc: Vec<String> = inc_param
        .map(|s| {
            s.split(',')
                .map(|f| f.trim().to_lowercase())
                .filter(|f| !f.is_empty() && ORIGINAL_FIELDS.contains(&f.as_str()))
                .collect()
        })
        .unwrap_or_else(|| ORIGINAL_FIELDS.iter().map(|f| f.to_string()).collect());

    let exc: Vec<String> = exc_param
        .map(|s| {
            s.split(',')
                .map(|f| f.trim().to_lowercase())
                .filter(|f| !f.is_empty())
                .collect()
        })
        .unwrap_or_default();

    inc.retain(|f| !exc.contains(f));
    inc
}

fn parse_length(s: &str) -> (usize, usize) {
    let clamp = |n: usize| n.clamp(1, 64);
    if let Some(idx) = s.find('-') {
        let lo: usize = s[..idx].parse().unwrap_or(8);
        let hi: usize = s[idx + 1..].parse().unwrap_or(64);
        let lo = clamp(lo);
        let hi = clamp(hi).max(lo);
        (lo, hi)
    } else if let Ok(n) = s.parse::<usize>() {
        let n = clamp(n);
        (n, n)
    } else {
        (8, 64)
    }
}

fn age_years(dt: DateTime<Utc>) -> i64 {
    let now = Utc::now();
    let years = now.year() as i64 - dt.year() as i64;
    // Subtract 1 if birthday hasn't occurred yet this year
    if (now.month(), now.day()) < (dt.month(), dt.day()) {
        years - 1
    } else {
        years
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
    }

    fn make_generator() -> Generator {
        let mut g = Generator::new("1.4");
        g.init(&data_dir()).expect("data dir must exist");
        g
    }

    #[test]
    fn generate_returns_json_with_results() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(3),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        assert_eq!(v["results"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn same_seed_same_output() {
        let g = make_generator();
        let opts = || GenerateOptions {
            results: Some(2),
            seed: Some("testseed42".to_string()),
            max_results: 5000,
            ..Default::default()
        };
        let out1 = g.generate(opts());
        let out2 = g.generate(opts());
        assert_eq!(out1.body, out2.body);
    }

    #[test]
    fn different_seed_different_output() {
        let g = make_generator();
        let out1 = g.generate(GenerateOptions {
            results: Some(1),
            seed: Some("seed_alpha".to_string()),
            max_results: 5000,
            ..Default::default()
        });
        let out2 = g.generate(GenerateOptions {
            results: Some(1),
            seed: Some("seed_beta".to_string()),
            max_results: 5000,
            ..Default::default()
        });
        assert_ne!(out1.body, out2.body);
    }

    #[test]
    fn page_changes_output() {
        let g = make_generator();
        let out1 = g.generate(GenerateOptions {
            results: Some(1),
            seed: Some("paged".to_string()),
            page: Some(1),
            max_results: 5000,
            ..Default::default()
        });
        let out2 = g.generate(GenerateOptions {
            results: Some(1),
            seed: Some("paged".to_string()),
            page: Some(2),
            max_results: 5000,
            ..Default::default()
        });
        assert_ne!(out1.body, out2.body);
    }

    #[test]
    fn gender_filter_respected() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(50),
            gender: Some("female".to_string()),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        for user in v["results"].as_array().unwrap() {
            assert_eq!(user["gender"], "female");
        }
    }

    #[test]
    fn nat_filter_respected() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(50),
            nat: Some("US".to_string()),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        for user in v["results"].as_array().unwrap() {
            assert_eq!(user["nat"], "US");
        }
    }

    #[test]
    fn inc_filter_only_returns_specified_fields() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(5),
            inc: Some("name,email".to_string()),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        for user in v["results"].as_array().unwrap() {
            let keys: Vec<&str> = user.as_object().unwrap().keys().map(|s| s.as_str()).collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(sorted, vec!["email", "name"], "got keys: {:?}", keys);
        }
    }

    #[test]
    fn exc_filter_removes_specified_fields() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(5),
            exc: Some("picture,login".to_string()),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        for user in v["results"].as_array().unwrap() {
            assert!(user.get("picture").is_none(), "picture should be excluded");
            assert!(user.get("login").is_none(), "login should be excluded");
        }
    }

    #[test]
    fn email_has_no_spaces_and_is_lowercase() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(100),
            inc: Some("email".to_string()),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        for user in v["results"].as_array().unwrap() {
            let email = user["email"].as_str().unwrap();
            assert!(!email.contains(' '), "email has space: {email}");
            assert_eq!(email, email.to_lowercase(), "email not lowercase: {email}");
        }
    }

    #[test]
    fn dob_before_registered() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(50),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        for user in v["results"].as_array().unwrap() {
            let dob = user["dob"]["date"].as_str().unwrap();
            let reg = user["registered"]["date"].as_str().unwrap();
            assert!(dob < reg, "dob {dob} should be before registered {reg}");
        }
    }

    #[test]
    fn results_capped_at_max() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            results: Some(5001),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        assert_eq!(v["results"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn noinfo_removes_info_block() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            noinfo: true,
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        assert!(v.get("info").is_none());
    }

    #[test]
    fn lego_nat_returns_lego() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            lego: true,
            results: Some(5),
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        for user in v["results"].as_array().unwrap() {
            assert_eq!(user["nat"], "LEGO");
        }
    }

    #[test]
    fn info_version_is_correct() {
        let g = make_generator();
        let out = g.generate(GenerateOptions {
            max_results: 5000,
            ..Default::default()
        });
        let v: Value = serde_json::from_str(&out.body).unwrap();
        assert_eq!(v["info"]["version"], "1.4");
    }

    #[test]
    fn build_inc_respects_exc() {
        let inc = build_inc(None, Some("picture,login"));
        assert!(!inc.contains(&"picture".to_string()));
        assert!(!inc.contains(&"login".to_string()));
        assert!(inc.contains(&"email".to_string()));
    }

    #[test]
    fn build_inc_filters_unknown_fields() {
        let inc = build_inc(Some("email,fakeField"), None);
        assert!(inc.contains(&"email".to_string()));
        assert!(!inc.contains(&"fakefield".to_string()));
    }
}
