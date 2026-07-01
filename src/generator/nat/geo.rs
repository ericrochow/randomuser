use crate::generator::prng::Prng;
use serde_json::{json, Value};

/// Per-nationality geographic bounding box and applicable timezones.
///
/// Coordinates are stored as integers scaled by 10 000, the same unit used by
/// `Prng::range`, to keep arithmetic exact and avoid floating-point in the
/// struct definition.
pub struct NatGeo {
    /// Southern boundary (latitude × 10 000).
    lat_min: i64,
    /// Northern boundary (latitude × 10 000).
    lat_max: i64,
    /// Western boundary (longitude × 10 000).
    lon_min: i64,
    /// Eastern boundary (longitude × 10 000).
    lon_max: i64,
    /// Timezone entries applicable to this country: (UTC offset, description).
    timezones: &'static [(&'static str, &'static str)],
}

/// Return the geographic metadata for a nationality code, or `None` for nats
/// without a meaningful geographic footprint (e.g. "LEGO").
fn nat_geo(nat: &str) -> Option<&'static NatGeo> {
    static AU: NatGeo = NatGeo {
        lat_min: -390_000, lat_max: -180_000,
        lon_min: 1_130_000, lon_max: 1_540_000,
        timezones: &[
            ("+08:00", "Perth"),
            ("+09:30", "Darwin"),
            ("+10:00", "Brisbane, Canberra, Melbourne, Sydney"),
        ],
    };
    static BR: NatGeo = NatGeo {
        lat_min: -330_000, lat_max:   50_000,
        lon_min: -730_000, lon_max: -340_000,
        timezones: &[
            ("-05:00", "Bogota, Lima, Quito, Rio Branco"),
            ("-04:00", "Georgetown, La Paz, Manaus, San Juan"),
            ("-03:00", "Brasilia, Buenos Aires, Georgetown"),
        ],
    };
    static CA: NatGeo = NatGeo {
        lat_min:   420_000, lat_max:   700_000,
        lon_min: -1_410_000, lon_max: -520_000,
        timezones: &[
            ("-08:00", "Pacific Time (US & Canada)"),
            ("-07:00", "Mountain Time (US & Canada)"),
            ("-06:00", "Central Time (US & Canada)"),
            ("-05:00", "Eastern Time (US & Canada)"),
            ("-04:00", "Atlantic Time (Canada)"),
            ("-03:30", "Newfoundland"),
        ],
    };
    static CH: NatGeo = NatGeo {
        lat_min:  450_000, lat_max:  480_000,
        lon_min:   60_000, lon_max:  110_000,
        timezones: &[
            ("+01:00", "Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna"),
        ],
    };
    static DE: NatGeo = NatGeo {
        lat_min:  470_000, lat_max:  550_000,
        lon_min:   60_000, lon_max:  150_000,
        timezones: &[
            ("+01:00", "Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna"),
        ],
    };
    static DK: NatGeo = NatGeo {
        lat_min:  540_000, lat_max:  580_000,
        lon_min:   80_000, lon_max:  150_000,
        timezones: &[
            ("+01:00", "Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna"),
        ],
    };
    static ES: NatGeo = NatGeo {
        lat_min:  360_000, lat_max:  440_000,
        lon_min:  -90_000, lon_max:   40_000,
        timezones: &[
            ("+01:00", "Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna"),
        ],
    };
    static FI: NatGeo = NatGeo {
        lat_min:  600_000, lat_max:  700_000,
        lon_min:  200_000, lon_max:  320_000,
        timezones: &[
            ("+02:00", "Helsinki, Kyiv, Riga, Sofia, Tallinn, Vilnius"),
        ],
    };
    static FR: NatGeo = NatGeo {
        lat_min:  420_000, lat_max:  510_000,
        lon_min:  -50_000, lon_max:   90_000,
        timezones: &[
            ("+01:00", "Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna"),
        ],
    };
    static GB: NatGeo = NatGeo {
        lat_min:  500_000, lat_max:  610_000,
        lon_min:  -80_000, lon_max:   20_000,
        timezones: &[
            ("+00:00", "Dublin, Edinburgh, Lisbon, London"),
        ],
    };
    static IE: NatGeo = NatGeo {
        lat_min:  510_000, lat_max:  550_000,
        lon_min: -100_000, lon_max:  -60_000,
        timezones: &[
            ("+00:00", "Dublin, Edinburgh, Lisbon, London"),
        ],
    };
    static IN: NatGeo = NatGeo {
        lat_min:   80_000, lat_max:  370_000,
        lon_min:  680_000, lon_max:  970_000,
        timezones: &[
            ("+05:30", "Chennai, Kolkata, Mumbai, New Delhi"),
        ],
    };
    static IR: NatGeo = NatGeo {
        lat_min:  250_000, lat_max:  400_000,
        lon_min:  440_000, lon_max:  630_000,
        timezones: &[
            ("+03:30", "Tehran"),
        ],
    };
    static MX: NatGeo = NatGeo {
        lat_min:  150_000, lat_max:   320_000,
        lon_min: -1_170_000, lon_max: -870_000,
        timezones: &[
            ("-07:00", "Mountain Time (US & Canada)"),
            ("-06:00", "Central Time (US & Canada)"),
        ],
    };
    static NL: NatGeo = NatGeo {
        lat_min:  510_000, lat_max:  530_000,
        lon_min:   30_000, lon_max:   70_000,
        timezones: &[
            ("+01:00", "Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna"),
        ],
    };
    static NO: NatGeo = NatGeo {
        lat_min:  580_000, lat_max:  710_000,
        lon_min:   40_000, lon_max:  310_000,
        timezones: &[
            ("+01:00", "Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna"),
        ],
    };
    static NZ: NatGeo = NatGeo {
        lat_min: -470_000, lat_max: -340_000,
        lon_min: 1_660_000, lon_max: 1_780_000,
        timezones: &[
            ("+12:00", "Auckland, Wellington"),
        ],
    };
    static RS: NatGeo = NatGeo {
        lat_min:  420_000, lat_max:  460_000,
        lon_min:  190_000, lon_max:  230_000,
        timezones: &[
            ("+01:00", "Belgrade, Bratislava, Budapest, Ljubljana, Prague"),
        ],
    };
    static TR: NatGeo = NatGeo {
        lat_min:  360_000, lat_max:  420_000,
        lon_min:  260_000, lon_max:  450_000,
        timezones: &[
            ("+03:00", "Istanbul"),
        ],
    };
    static UA: NatGeo = NatGeo {
        lat_min:  440_000, lat_max:  520_000,
        lon_min:  220_000, lon_max:  400_000,
        timezones: &[
            ("+02:00", "Helsinki, Kyiv, Riga, Sofia, Tallinn, Vilnius"),
        ],
    };
    static US: NatGeo = NatGeo {
        lat_min:   250_000, lat_max:   490_000,
        lon_min: -1_240_000, lon_max: -670_000,
        timezones: &[
            ("-10:00", "Hawaii"),
            ("-09:00", "Alaska"),
            ("-08:00", "Pacific Time (US & Canada)"),
            ("-07:00", "Mountain Time (US & Canada)"),
            ("-06:00", "Central Time (US & Canada)"),
            ("-05:00", "Eastern Time (US & Canada)"),
        ],
    };
    match nat {
        "AU" => Some(&AU),
        "BR" => Some(&BR),
        "CA" => Some(&CA),
        "CH" => Some(&CH),
        "DE" => Some(&DE),
        "DK" => Some(&DK),
        "ES" => Some(&ES),
        "FI" => Some(&FI),
        "FR" => Some(&FR),
        "GB" => Some(&GB),
        "IE" => Some(&IE),
        "IN" => Some(&IN),
        "IR" => Some(&IR),
        "MX" => Some(&MX),
        "NL" => Some(&NL),
        "NO" => Some(&NO),
        "NZ" => Some(&NZ),
        "RS" => Some(&RS),
        "TR" => Some(&TR),
        "UA" => Some(&UA),
        "US" => Some(&US),
        _    => None,
    }
}

/// Generate a `(latitude, longitude, timezone)` triple appropriate for `nat`.
///
/// When geo metadata exists for the nationality, coordinates are drawn from
/// that country's bounding box and the timezone is chosen from the country's
/// applicable timezone list. For unrecognised nats (e.g. "LEGO") the function
/// falls back to a globally random coordinate pair and a random entry from
/// `fallback_timezones`.
pub fn gen_location(nat: &str, prng: &mut Prng, fallback_timezones: &[String]) -> (String, String, Value) {
    if let Some(geo) = nat_geo(nat) {
        // Timezone is picked first to preserve the original PRNG call order
        // (tz → lat → lon), keeping seed-reproducibility across the refactor.
        let (offset, description) = *prng.random_item(geo.timezones);
        let lat = format!("{:.4}", prng.range(geo.lat_min, geo.lat_max) as f64 / 10_000.0);
        let lon = format!("{:.4}", prng.range(geo.lon_min, geo.lon_max) as f64 / 10_000.0);
        let timezone = json!({ "offset": offset, "description": description });
        (lat, lon, timezone)
    } else {
        let tz_str = prng.random_item(fallback_timezones).clone();
        let timezone = serde_json::from_str(&tz_str).unwrap_or(json!(null));
        (prng.gen_latitude(), prng.gen_longitude(), timezone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::prng::Prng;

    const ALL_NATS: &[&str] = &[
        "AU", "BR", "CA", "CH", "DE", "DK", "ES", "FI", "FR", "GB",
        "IE", "IN", "IR", "MX", "NL", "NO", "NZ", "RS", "TR", "UA", "US",
    ];

    fn prng() -> Prng {
        let mut p = Prng::new();
        p.seed_from_str("geo_test", 1);
        p
    }

    #[test]
    fn all_nats_have_geo_metadata() {
        for nat in ALL_NATS {
            assert!(nat_geo(nat).is_some(), "missing geo for {nat}");
        }
    }

    #[test]
    fn lego_returns_none() {
        assert!(nat_geo("LEGO").is_none());
    }

    #[test]
    fn coordinates_within_bounding_box() {
        let mut p = prng();
        for nat in ALL_NATS {
            let geo = nat_geo(nat).unwrap();
            for _ in 0..10 {
                let lat: f64 = p.range(geo.lat_min, geo.lat_max) as f64 / 10_000.0;
                let lon: f64 = p.range(geo.lon_min, geo.lon_max) as f64 / 10_000.0;
                assert!(
                    lat >= geo.lat_min as f64 / 10_000.0 && lat <= geo.lat_max as f64 / 10_000.0,
                    "{nat} lat {lat} out of bounds"
                );
                assert!(
                    lon >= geo.lon_min as f64 / 10_000.0 && lon <= geo.lon_max as f64 / 10_000.0,
                    "{nat} lon {lon} out of bounds"
                );
                // consume timezone pick
                p.random_item(geo.timezones);
            }
        }
    }

    #[test]
    fn gen_location_produces_valid_json_timezone() {
        let mut p = prng();
        let fallback: Vec<String> = vec![
            r#"{"offset":"+00:00","description":"UTC"}"#.to_string(),
        ];
        for nat in ALL_NATS {
            let (_, _, tz) = gen_location(nat, &mut p, &fallback);
            assert!(tz["offset"].is_string(), "{nat} timezone offset missing");
            assert!(tz["description"].is_string(), "{nat} timezone description missing");
        }
    }
}
