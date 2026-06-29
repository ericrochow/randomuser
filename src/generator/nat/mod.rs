use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fs,
    io,
    path::Path,
};

pub mod au;
pub mod br;
pub mod ca;
pub mod ch;
pub mod de;
pub mod dk;
pub mod es;
pub mod fi;
pub mod fr;
pub mod gb;
pub mod ie;
pub mod in_;
pub mod ir;
pub mod lego;
pub mod mx;
pub mod nl;
pub mod no;
pub mod nz;
pub mod rs;
pub mod tr;
pub mod ua;
pub mod us;

/// All loaded list files for one nationality (or common), keyed by filename stem.
pub type NatLists = HashMap<String, Vec<String>>;

/// Full dataset: common + per-nat lists.
#[derive(Default)]
pub struct NatDatasets {
    /// "common" → list_name → lines
    pub common: NatLists,
    /// "AU", "FR", … → list_name → lines
    pub by_nat: HashMap<String, NatLists>,
}

impl NatDatasets {
    /// Load every file under `data_dir`. Subdirectory name = nat code,
    /// files inside `<nat>/lists/` = list name (stem).
    pub fn load(data_dir: &Path) -> std::io::Result<Self> {
        let mut ds = NatDatasets::default();

        for entry in fs::read_dir(data_dir)? {
            let entry = entry?;
            let nat = entry.file_name().to_string_lossy().into_owned();
            let lists_dir = entry.path().join("lists");
            if !lists_dir.is_dir() {
                continue;
            }
            let lists = load_lists(&lists_dir)?;
            if nat == "common" {
                ds.common = lists;
            } else {
                ds.by_nat.insert(nat, lists);
            }
        }
        Ok(ds)
    }

    pub fn nat_list<'a>(&'a self, nat: &str, key: &str) -> &'a [String] {
        self.by_nat
            .get(nat)
            .and_then(|m| m.get(key))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn common_list<'a>(&'a self, key: &str) -> &'a [String] {
        self.common.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Verify that all list files required at request time are present.
    /// Called once from `Generator::init` so missing data fails at startup
    /// rather than producing a confusing panic inside a request handler.
    pub fn validate_required_lists(&self) -> io::Result<()> {
        const COMMON_REQUIRED: &[&str] = &["timezones", "user1", "user2", "passwords"];
        const NAT_REQUIRED: &[&str] = &["cities", "states", "street", "male_first", "female_first", "last"];

        for key in COMMON_REQUIRED {
            if self.common_list(key).is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("required common list '{}' is missing or empty", key),
                ));
            }
        }
        for (nat, lists) in &self.by_nat {
            for key in NAT_REQUIRED {
                if lists.get(*key).map(|v| v.is_empty()).unwrap_or(true) {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("required list '{}' for nat '{}' is missing or empty", key, nat),
                    ));
                }
            }
        }
        Ok(())
    }

    /// All nationality codes (excludes "common" and "LEGO").
    pub fn nat_codes(&self) -> Vec<String> {
        let mut codes: Vec<_> = self
            .by_nat
            .keys()
            .filter(|k| *k != "LEGO")
            .cloned()
            .collect();
        codes.sort();
        codes
    }
}

fn load_lists(lists_dir: &Path) -> std::io::Result<NatLists> {
    let mut map = HashMap::new();
    for entry in fs::read_dir(lists_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let stem = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            // normalise "NINO - 1st letter" → "nino_1st_letter" for stable access
            .replace(' ', "_")
            .replace('-', "_")
            .to_lowercase();
        let content = fs::read_to_string(&path)?;
        let lines: Vec<String> = content
            .lines()
            .map(|l| l.to_string())
            .filter(|l| !l.is_empty())
            .collect();
        map.insert(stem, lines);
    }
    Ok(map)
}

// ─── Shared inject helpers ────────────────────────────────────────────────────

/// Conditionally set `field` on `user` when the field is in `inc`.
pub fn include_field(inc: &[String], user: &mut Map<String, Value>, field: &str, value: Value) {
    if inc.iter().any(|f| f == field) {
        user.insert(field.to_string(), value);
    }
}

/// Country code → full English country name.
pub fn full_nat_name(code: &str) -> &'static str {
    match code {
        "AU" => "Australia",
        "BR" => "Brazil",
        "CA" => "Canada",
        "CH" => "Switzerland",
        "DE" => "Germany",
        "DK" => "Denmark",
        "ES" => "Spain",
        "FI" => "Finland",
        "FR" => "France",
        "GB" => "United Kingdom",
        "IE" => "Ireland",
        "IN" => "India",
        "IR" => "Iran",
        "MX" => "Mexico",
        "NL" => "Netherlands",
        "NO" => "Norway",
        "NZ" => "New Zealand",
        "RS" => "Serbia",
        "TR" => "Turkey",
        "UA" => "Ukraine",
        "US" => "United States",
        "LEGO" => "LEGO",
        _ => "Unknown",
    }
}

/// Dispatch inject for `nat`. The inject receives the full `user` map (already
/// containing all inc-filtered base fields), the `inc` list, the PRNG, and
/// the datasets. Each inject may mutate phone/cell/id/location and reorders picture.
pub fn inject(
    nat: &str,
    inc: &[String],
    user: &mut Map<String, Value>,
    prng: &mut super::prng::Prng,
    datasets: &NatDatasets,
) {
    match nat {
        "AU" => au::inject(inc, user, prng),
        "BR" => br::inject(inc, user, prng),
        "CA" => ca::inject(inc, user, prng),
        "CH" => ch::inject(inc, user, prng),
        "DE" => de::inject(inc, user, prng),
        "DK" => dk::inject(inc, user, prng),
        "ES" => es::inject(inc, user, prng),
        "FI" => fi::inject(inc, user, prng),
        "FR" => fr::inject(inc, user, prng),
        "GB" => gb::inject(inc, user, prng),
        "IE" => ie::inject(inc, user, prng),
        "IN" => in_::inject(inc, user, prng),
        "IR" => ir::inject(inc, user, prng),
        "LEGO" => lego::inject(inc, user, prng),
        "MX" => mx::inject(inc, user, prng),
        "NL" => nl::inject(inc, user, prng),
        "NO" => no::inject(inc, user, prng, datasets),
        "NZ" => nz::inject(inc, user, prng),
        "RS" => rs::inject(inc, user, prng),
        "TR" => tr::inject(inc, user, prng),
        "UA" => ua::inject(inc, user, prng),
        "US" => us::inject(inc, user, prng),
        _ => {}
    }
}

// ─── Shared picture-reorder helper ───────────────────────────────────────────

/// Saves and removes the picture field, calls `f`, then re-inserts picture.
/// This ensures picture always appears after phone/cell/id in the output.
pub fn with_picture_reorder<F>(
    inc: &[String],
    user: &mut Map<String, Value>,
    f: F,
) where
    F: FnOnce(&mut Map<String, Value>),
{
    let pic = user.shift_remove("picture");
    f(user);
    if inc.iter().any(|f| f == "picture") {
        if let Some(p) = pic {
            user.insert("picture".to_string(), p);
        }
    }
}
