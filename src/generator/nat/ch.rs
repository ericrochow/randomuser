use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

/// Compute the EAN-13 check digit for a 12-digit string.
///
/// Digits are alternately weighted 1 and 3 starting from the first digit.
/// The check digit is `(10 - (sum % 10)) % 10`.
fn ean13_check(twelve_digits: &str) -> char {
    let s: u32 = twelve_digits.chars().enumerate().map(|(i, c)| {
        let d = c.to_digit(10).unwrap_or(0);
        if i % 2 == 0 { d } else { d * 3 }
    }).sum();
    char::from_digit((10 - s % 10) % 10, 10).unwrap()
}

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    // Swiss title override — extract gender before borrowing name mutably
    if inc.iter().any(|f| f == "name") {
        let gender = user
            .get("gender")
            .and_then(|g| g.as_str())
            .unwrap_or("male")
            .to_string();
        let title = match gender.as_str() {
            "male" => "Monsieur".to_string(),
            "nonbinary" => "Mx".to_string(),
            _ => {
                let female_titles = ["Mademoiselle", "Madame"];
                prng.random_item(&female_titles).to_string()
            }
        };
        if let Some(Value::Object(name)) = user.get_mut("name") {
            name.insert("title".to_string(), Value::String(title));
        }
    }

    // CH postcode: 1000–9999
    if inc.iter().any(|f| f == "location") {
        if let Some(Value::Object(loc)) = user.get_mut("location") {
            loc.insert("postcode".to_string(), json!(prng.range(1000, 9999)));
        }
    }

    with_picture_reorder(inc, user, |user| {
        let prefixes = ["075", "076", "077", "078", "079"];
        let prefix = prng.random_item(&prefixes);
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "{} {} {} {}",
                prefix,
                prng.random_chars(3, 3),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2)
            )),
        );
        let prefix2 = prng.random_item(&prefixes);
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "{} {} {} {}",
                prefix2,
                prng.random_chars(3, 3),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2)
            )),
        );
        let grp1 = prng.random_chars(3, 4);
        let grp2 = prng.random_chars(3, 4);
        let seq  = prng.random_chars(3, 1);
        let check = ean13_check(&format!("756{}{}{}", grp1, grp2, seq));
        include_field(
            inc,
            user,
            "id",
            json!({
                "name": "AVS",
                "value": format!("756.{}.{}.{}{}", grp1, grp2, seq, check)
            }),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::prng::Prng;

    fn run(seed: &str) -> Map<String, Value> {
        let mut prng = Prng::new();
        prng.seed_from_str(seed, 1);
        let mut user = Map::new();
        user.insert("gender".to_string(), json!("male"));
        user.insert("name".to_string(), json!({"title": "Mr", "first": "Hans", "last": "Müller"}));
        user.insert("picture".to_string(), json!({}));
        user.insert("location".to_string(), json!({"postcode": 12345}));
        let inc: Vec<String> = ["gender", "name", "phone", "cell", "id", "picture", "location"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        user
    }

    #[test]
    fn title_is_monsieur_for_male() {
        let user = run("ch_title");
        assert_eq!(user["name"]["title"], "Monsieur");
    }

    #[test]
    fn title_is_mx_for_nonbinary() {
        let mut prng = Prng::new();
        prng.seed_from_str("ch_nb", 1);
        let mut user = Map::new();
        user.insert("gender".to_string(), json!("nonbinary"));
        user.insert("name".to_string(), json!({"title": "Mx", "first": "Alex", "last": "Müller"}));
        user.insert("picture".to_string(), json!({}));
        user.insert("location".to_string(), json!({"postcode": 5000}));
        let inc: Vec<String> = ["gender", "name", "phone", "cell", "id", "picture", "location"]
            .iter().map(|s| s.to_string()).collect();
        inject(&inc, &mut user, &mut prng);
        assert_eq!(user["name"]["title"], "Mx");
    }

    #[test]
    fn title_is_female_title_for_female() {
        let mut prng = Prng::new();
        prng.seed_from_str("ch_female", 1);
        let mut user = Map::new();
        user.insert("gender".to_string(), json!("female"));
        user.insert("name".to_string(), json!({"title": "Mrs", "first": "Anna", "last": "Müller"}));
        user.insert("picture".to_string(), json!({}));
        user.insert("location".to_string(), json!({"postcode": 5000}));
        let inc: Vec<String> = ["gender", "name", "phone", "cell", "id", "picture", "location"]
            .iter().map(|s| s.to_string()).collect();
        inject(&inc, &mut user, &mut prng);
        let title = user["name"]["title"].as_str().unwrap();
        assert!(
            title == "Mademoiselle" || title == "Madame",
            "expected female title, got: {title}"
        );
    }

    #[test]
    fn postcode_in_range() {
        let user = run("ch_post");
        let pc = user["location"]["postcode"].as_i64().unwrap();
        assert!((1000..=9999).contains(&pc));
    }

    #[test]
    fn id_name_is_avs() {
        let user = run("ch_id");
        assert_eq!(user["id"]["name"], "AVS");
    }

    #[test]
    fn avs_check_digit_is_valid() {
        let user = run("ch_avs_check");
        let value = user["id"]["value"].as_str().unwrap();
        // Strip dots to get the 13 raw digits, then verify EAN-13 validity:
        // the weighted sum of all 13 digits must be divisible by 10.
        let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        assert_eq!(digits.len(), 13, "AVS must be 13 digits: {value}");
        let s: u32 = digits.chars().enumerate().map(|(i, c)| {
            let d = c.to_digit(10).unwrap();
            if i % 2 == 0 { d } else { d * 3 }
        }).sum();
        assert_eq!(s % 10, 0, "EAN-13 check digit invalid for AVS: {value}");
    }
}
