use super::{include_field, with_picture_reorder};
use crate::generator::prng::{pad, Prng};
use chrono::Datelike;
use serde_json::{json, Map, Value};

/// German Rentenversicherungsnummer (SVNR/RVN).
/// Format: PP DDMMYY I GGGG where:
///   PP  = 2-digit pension area code
///   DDMMYY = dob day/month/2-digit year
///   I   = first letter of last name (uppercase)
///   GG  = gender seq (00-49 male, 50-99 female) + 1 random digit
fn gen_svnr(dob: chrono::NaiveDate, last_name: &str, gender: &str, prng: &mut Prng) -> String {
    const PENSION_CODES: &[&str] = &[
        "02", "03", "04", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18",
        "19", "20", "21", "23", "24", "25", "26", "28", "29", "38", "39", "40", "42", "43",
        "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57",
        "58", "59", "60", "61", "62", "63", "64", "65", "66", "67", "68", "69", "70", "71",
        "72", "73", "74", "75", "76", "77", "78", "79", "80", "81", "82", "89",
    ];

    let pension = prng.random_item(PENSION_CODES);
    let day = pad(dob.day(), 2);
    let month = pad(dob.month(), 2);
    // JS getYear() returns year - 1900; use % 100 for 2-digit year
    let year = dob.year() % 100;
    let initial = last_name
        .chars()
        .next()
        .unwrap_or('X')
        .to_uppercase()
        .next()
        .unwrap_or('X');

    let gender_seq = if gender == "male" {
        pad(prng.range(0, 49), 2)
    } else {
        pad(prng.range(50, 99), 2)
    };
    let check_digit = prng.range(0, 9);

    format!("{pension} {day}{month}{year:02} {initial} {gender_seq}{check_digit}")
}

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "0{}-{}",
                prng.random_chars(3, 3),
                prng.random_chars(3, 7)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "017{}-{}",
                prng.random_chars(3, 1),
                prng.random_chars(3, 7)
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

            let last_name = user
                .get("name")
                .and_then(|n| n.get("last"))
                .and_then(|l| l.as_str())
                .unwrap_or("X");

            let gender = user
                .get("gender")
                .and_then(|g| g.as_str())
                .unwrap_or("male");

            let svnr = gen_svnr(dob_date, last_name, gender, prng);
            user.insert("id".to_string(), json!({ "name": "SVNR", "value": svnr }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::prng::Prng;

    #[test]
    fn svnr_format() {
        let dob = chrono::NaiveDate::from_ymd_opt(1985, 6, 15).unwrap();
        let mut prng = Prng::new();
        prng.seed_from_str("de_svnr", 1);
        let s = gen_svnr(dob, "Müller", "male", &mut prng);
        // PP DDMMYY I GGGG
        let parts: Vec<&str> = s.split(' ').collect();
        assert_eq!(parts.len(), 4, "expected 4 space-separated parts: {s}");
        assert_eq!(parts[1], "150685", "DDMMYY mismatch");
        assert_eq!(parts[2], "M", "initial mismatch");
    }

    #[test]
    fn id_name_is_svnr() {
        let mut prng = Prng::new();
        prng.seed_from_str("de_id", 1);
        let mut user = Map::new();
        user.insert("gender".to_string(), json!("male"));
        user.insert("name".to_string(), json!({"title": "Mr", "first": "Klaus", "last": "Braun"}));
        user.insert(
            "dob".to_string(),
            json!({"date": "1985-06-15T00:00:00.000Z", "age": 38}),
        );
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["gender", "name", "dob", "phone", "cell", "id", "picture"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        assert_eq!(user["id"]["name"], "SVNR");
    }
}
