use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        let districts = ["1", "2", "3"];
        let district = *prng.random_item(&districts);
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "0{}{}-{}-{}",
                district,
                prng.random_chars(3, 1),
                prng.random_chars(3, 4),
                prng.random_chars(3, 3)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "06{}-{}-{}",
                prng.random_chars(3, 1),
                prng.random_chars(3, 4),
                prng.random_chars(3, 3)
            )),
        );
        include_field(
            inc,
            user,
            "id",
            json!({ "name": "SID", "value": prng.random_chars(3, 9) }),
        );
    });
}
