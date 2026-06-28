use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        let providers = ["66", "67", "68", "96", "97", "98", "99"];
        let prov = *prng.random_item(&providers);
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "(0{}) {}{}-{}",
                prov,
                prng.random_chars(4, 1),
                prng.random_chars(3, 2),
                prng.random_chars(3, 4)
            )),
        );

        let prov2 = *prng.random_item(&providers);
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "(0{}) {}{}-{}",
                prov2,
                prng.random_chars(4, 1),
                prng.random_chars(3, 2),
                prng.random_chars(3, 4)
            )),
        );

        include_field(
            inc,
            user,
            "id",
            json!({ "name": "", "value": Value::Null }),
        );
    });
}
