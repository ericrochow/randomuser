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
                "({})-{}-{}",
                prng.random_chars(3, 3),
                prng.random_chars(3, 3),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "({})-{}-{}",
                prng.random_chars(3, 3),
                prng.random_chars(3, 3),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "id",
            json!({ "name": "BSN", "value": prng.random_chars(3, 8) }),
        );
    });
}
