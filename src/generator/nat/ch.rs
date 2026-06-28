use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    // Swiss title override — extract gender before borrowing name mutably
    if inc.iter().any(|f| f == "name") {
        let gender = user
            .get("gender")
            .and_then(|g| g.as_str())
            .unwrap_or("male")
            .to_string();
        let title = if gender == "male" {
            "Monsieur".to_string()
        } else {
            let female_titles = ["Mademoiselle", "Madame"];
            prng.random_item(&female_titles).to_string()
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
        include_field(
            inc,
            user,
            "id",
            json!({
                "name": "AVS",
                "value": format!(
                    "756.{}.{}.{}",
                    prng.random_chars(3, 4),
                    prng.random_chars(3, 4),
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
}
