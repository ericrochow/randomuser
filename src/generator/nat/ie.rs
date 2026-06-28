use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "0{}1-{}-{}",
                prng.range(1, 7),
                prng.random_chars(3, 3),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "081-{}-{}",
                prng.random_chars(3, 3),
                prng.random_chars(3, 4)
            )),
        );

        if inc.iter().any(|f| f == "id") {
            // PPS: 7 digits + T + optional A (if DOB >= 2013-01-01)
            let dob_ts = user
                .get("dob")
                .and_then(|d| d.as_str())  // legacy: dob was sometimes just a date string
                .or_else(|| {
                    user.get("dob")
                        .and_then(|d| d.get("date"))
                        .and_then(|d| d.as_str())
                })
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);

            // 2013-01-01 00:00:00 UTC = 1356998400
            let suffix = if dob_ts >= 1_356_998_400 { "TA" } else { "T" };
            let pps = format!("{}{}",  prng.random_chars(3, 7), suffix);
            user.insert("id".to_string(), json!({ "name": "PPS", "value": pps }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dob_date: &str, seed: &str) -> Map<String, Value> {
        let mut prng = Prng::new();
        prng.seed_from_str(seed, 1);
        let mut user = Map::new();
        user.insert("dob".to_string(), json!({"date": dob_date, "age": 30}));
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture", "dob"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        user
    }

    #[test]
    fn pps_pre_2013_has_no_a() {
        let user = run("1985-06-15T00:00:00.000Z", "ie_pps_old");
        let val = user["id"]["value"].as_str().unwrap();
        assert!(val.ends_with('T') && !val.ends_with("TA"), "pre-2013 PPS must end with T only: {val}");
    }

    #[test]
    fn pps_post_2013_has_a() {
        let user = run("2015-06-15T00:00:00.000Z", "ie_pps_new");
        let val = user["id"]["value"].as_str().unwrap();
        assert!(val.ends_with("TA"), "post-2013 PPS must end with TA: {val}");
    }
}
