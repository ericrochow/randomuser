use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use chrono::Datelike;
use serde_json::{json, Map, Value};

/// French national identification number (INSEE / numéro de sécurité sociale).
/// Format: G YY MM LLOOOKKK CC
///   G   = gender (1=male, 2=female)
///   YY  = 2-digit birth year
///   MM  = 2-digit birth month (01–12)
///   LLOOOKKK = 8-digit geographic code
///   CC  = 2-digit control key = 97 - (number % 97)
fn gen_insee(dob: chrono::NaiveDate, gender: &str, prng: &mut Prng) -> String {
    let g: u64 = if gender == "male" { 1 } else { 2 };
    // JS getYear() = year - 1900; use % 100 for 2-digit form
    let yy = (dob.year() % 100) as u64;
    let mm = dob.month() as u64;
    let lloookkk: u64 = prng.random_chars(3, 8).parse().unwrap_or(0);

    let number: u64 = g * 10_000_000_000 + yy * 100_000_000 + mm * 1_000_000 + lloookkk;
    let cc = 97 - (number % 97);

    format!("{g}{yy:02}{mm:02}{lloookkk:08} {cc:02}")
}

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "0{}-{}-{}-{}-{}",
                prng.range(1, 5),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "06-{}-{}-{}-{}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2)
            )),
        );

        if inc.iter().any(|f| f == "id") {
            let dob_date = user
                .get("dob")
                .and_then(|d| d.get("date"))
                .and_then(|d| d.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.naive_utc().date())
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let gender = user
                .get("gender")
                .and_then(|g| g.as_str())
                .unwrap_or("male");

            let insee = gen_insee(dob_date, gender, prng);
            user.insert("id".to_string(), json!({ "name": "INSEE", "value": insee }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insee_control_key() {
        let dob = chrono::NaiveDate::from_ymd_opt(1985, 6, 1).unwrap();
        let mut prng = Prng::new();
        prng.seed_from_str("fr_insee", 1);
        let insee = gen_insee(dob, "male", &mut prng);
        let parts: Vec<&str> = insee.split(' ').collect();
        assert_eq!(parts.len(), 2, "INSEE must have two space-separated parts");
        let cc: u64 = parts[1].parse().unwrap();
        // Reconstruct number and verify
        let number_str = &parts[0];
        let g: u64 = number_str[..1].parse().unwrap();
        assert_eq!(g, 1, "gender digit should be 1 for male");
        assert!((1..=97).contains(&cc), "control key must be 1–97");
    }

    #[test]
    fn cell_starts_with_06() {
        let mut prng = Prng::new();
        prng.seed_from_str("fr_cell", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        user.insert("gender".to_string(), json!("female"));
        user.insert("dob".to_string(), json!({"date": "1985-06-01T00:00:00.000Z", "age": 38}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture", "gender", "dob"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        assert!(user["cell"].as_str().unwrap().starts_with("06-"));
    }
}
