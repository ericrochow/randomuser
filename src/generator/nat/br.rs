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
                "({}) {}-{}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 4),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "({}) {}-{}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 4),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "id",
            json!({
                "name": "CPF",
                "value": format!(
                    "{}.{}.{}-{}",
                    prng.random_chars(3, 3),
                    prng.random_chars(3, 3),
                    prng.random_chars(3, 3),
                    prng.random_chars(3, 2)
                )
            }),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::prng::Prng;

    fn run(inc: &[&str], seed: &str) -> Map<String, Value> {
        let mut prng = Prng::new();
        prng.seed_from_str(seed, 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        let inc_v: Vec<String> = inc.iter().map(|s| s.to_string()).collect();
        inject(&inc_v, &mut user, &mut prng);
        user
    }

    #[test]
    fn cpf_format() {
        let user = run(&["phone", "cell", "id", "picture"], "br_cpf");
        let val = user["id"]["value"].as_str().unwrap();
        // XXX.XXX.XXX-XX
        let parts: Vec<&str> = val.split('-').collect();
        assert_eq!(parts.len(), 2);
        let left: Vec<&str> = parts[0].split('.').collect();
        assert_eq!(left.len(), 3);
    }

    #[test]
    fn id_name_is_cpf() {
        let user = run(&["id", "picture"], "br_id");
        assert_eq!(user["id"]["name"], "CPF");
    }
}
