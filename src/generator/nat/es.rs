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
                "9{}-{}-{}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 3),
                prng.random_chars(3, 3)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "6{}-{}-{}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 3),
                prng.random_chars(3, 3)
            )),
        );
        include_field(
            inc,
            user,
            "id",
            json!({
                "name": "DNI",
                "value": format!("{}-{}", prng.random_chars(3, 8), prng.random_chars(4, 1))
            }),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dni_format() {
        let mut prng = Prng::new();
        prng.seed_from_str("es_dni", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        let val = user["id"]["value"].as_str().unwrap();
        let parts: Vec<&str> = val.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 1);
        // DNI letter is uppercase
        assert!(parts[1].chars().all(|c| c.is_ascii_uppercase()));
    }
}
