use super::{include_field, with_picture_reorder};
use crate::generator::prng::{pad, Prng};
use chrono::Datelike;
use serde_json::{json, Map, Value};

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        include_field(
            inc,
            user,
            "phone",
            Value::String(prng.random_chars(3, 8)),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(prng.random_chars(3, 8)),
        );

        if inc.iter().any(|f| f == "id") {
            let dob_date = user
                .get("dob")
                .and_then(|d| d.get("date"))
                .and_then(|d| d.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.naive_utc().date())
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            // JS getYear() = year - 1900; use % 100 for 2-digit year
            let year = dob_date.year() % 100;
            let cpr = format!(
                "{}{}{:02}-{}",
                pad(dob_date.day(), 2),
                pad(dob_date.month(), 2),
                year,
                prng.random_chars(3, 4)
            );
            user.insert("id".to_string(), json!({ "name": "CPR", "value": cpr }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpr_format() {
        let mut prng = Prng::new();
        prng.seed_from_str("dk_cpr", 1);
        let mut user = Map::new();
        user.insert(
            "dob".to_string(),
            json!({"date": "1985-06-15T00:00:00.000Z", "age": 38}),
        );
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        let val = user["id"]["value"].as_str().unwrap();
        // DDMMYY-XXXX
        let parts: Vec<&str> = val.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "150685");
        assert_eq!(parts[1].len(), 4);
    }
}
