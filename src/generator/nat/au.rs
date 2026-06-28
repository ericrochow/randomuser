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
                "0{}-{}{}{}{}",
                prng.range(0, 9),
                prng.range(0, 9),
                prng.random_chars(3, 3),
                "-",
                prng.random_chars(3, 4)
            )),
        );

        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "04{}-{}-{}",
                prng.random_chars(3, 2),
                prng.random_chars(3, 3),
                prng.random_chars(3, 3)
            )),
        );

        include_field(
            inc,
            user,
            "id",
            json!({ "name": "TFN", "value": prng.random_chars(3, 9) }),
        );
    });

    // AU uses a different postcode range: 200–9999
    if inc.iter().any(|f| f == "location") {
        if let Some(Value::Object(loc)) = user.get_mut("location") {
            loc.insert("postcode".to_string(), json!(prng.range(200, 9999)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::prng::Prng;
    use serde_json::Map;

    fn make_user(inc: &[&str], prng: &mut Prng) -> Map<String, Value> {
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({"large":"x","medium":"y","thumbnail":"z"}));
        user.insert("location".to_string(), json!({"postcode": 12345}));
        let inc_v: Vec<String> = inc.iter().map(|s| s.to_string()).collect();
        inject(&inc_v, &mut user, prng);
        user
    }

    #[test]
    fn phone_present_when_included() {
        let mut prng = Prng::new();
        prng.seed_from_str("au_test", 1);
        let inc = ["phone", "cell", "id", "picture", "location"];
        let user = make_user(&inc, &mut prng);
        assert!(user.get("phone").is_some());
        assert!(user.get("cell").is_some());
        assert!(user.get("id").is_some());
    }

    #[test]
    fn phone_absent_when_excluded() {
        let mut prng = Prng::new();
        prng.seed_from_str("au_test2", 1);
        let user = make_user(&["id", "picture", "location"], &mut prng);
        assert!(user.get("phone").is_none());
    }

    #[test]
    fn postcode_in_au_range() {
        let mut prng = Prng::new();
        prng.seed_from_str("au_post", 1);
        let inc = ["phone", "cell", "id", "picture", "location"];
        let user = make_user(&inc, &mut prng);
        let pc = user["location"]["postcode"].as_i64().unwrap();
        assert!((200..=9999).contains(&pc));
    }

    #[test]
    fn picture_is_last_before_nat() {
        let mut prng = Prng::new();
        prng.seed_from_str("au_order", 1);
        let inc = ["phone", "cell", "id", "picture", "location"];
        let user = make_user(&inc, &mut prng);
        let keys: Vec<&str> = user.keys().map(|s| s.as_str()).collect();
        let pic_pos = keys.iter().position(|k| *k == "picture").unwrap();
        let id_pos = keys.iter().position(|k| *k == "id").unwrap();
        assert!(pic_pos > id_pos, "picture must come after id");
    }

    #[test]
    fn id_name_is_tfn() {
        let mut prng = Prng::new();
        prng.seed_from_str("au_id", 1);
        let inc = ["id", "picture", "location"];
        let user = make_user(&inc, &mut prng);
        assert_eq!(user["id"]["name"], "TFN");
    }
}
