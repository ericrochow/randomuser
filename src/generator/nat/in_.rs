use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        let prefix = *prng.random_item(&[7i64, 8, 9]);
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!("{}{}", prefix, prng.random_chars(3, 9))),
        );

        let prefix2 = *prng.random_item(&[7i64, 8, 9]);
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!("{}{}", prefix2, prng.random_chars(3, 9))),
        );

        include_field(
            inc,
            user,
            "id",
            json!({ "name": "UIDAI", "value": prng.random_chars(3, 12) }),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_starts_with_7_8_or_9() {
        let mut prng = Prng::new();
        prng.seed_from_str("in_phone", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        let phone = user["phone"].as_str().unwrap();
        assert!(
            phone.starts_with('7') || phone.starts_with('8') || phone.starts_with('9'),
            "phone must start with 7, 8, or 9: {phone}"
        );
    }

    #[test]
    fn uidai_is_12_digits() {
        let mut prng = Prng::new();
        prng.seed_from_str("in_uidai", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["id", "picture"].iter().map(|s| s.to_string()).collect();
        inject(&inc, &mut user, &mut prng);
        let val = user["id"]["value"].as_str().unwrap();
        assert_eq!(val.len(), 12);
        assert!(val.chars().all(|c| c.is_ascii_digit()));
    }
}
