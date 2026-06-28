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
                "(6{}) {} {}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 3),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "(6{}) {} {}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 3),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "id",
            json!({
                "name": "NSS",
                "value": format!(
                    "{} {} {} {} {}",
                    prng.random_chars(3, 2),
                    prng.random_chars(3, 2),
                    prng.random_chars(3, 2),
                    prng.random_chars(3, 4),
                    prng.random_chars(3, 1)
                )
            }),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_starts_with_6() {
        let mut prng = Prng::new();
        prng.seed_from_str("mx_phone", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        assert!(user["phone"].as_str().unwrap().starts_with("(6"));
    }
}
