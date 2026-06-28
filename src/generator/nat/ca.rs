use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

/// Validate a 9-digit SIN using the Luhn-like algorithm from the original.
fn check_sin(sin: &str) -> bool {
    let check = [1i32, 2, 1, 2, 1, 2, 1, 2, 1];
    let sum: i32 = sin
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let digit = c.to_digit(10).unwrap() as i32;
            let mut res = digit * check[i];
            if res >= 10 {
                res = res / 10 + res % 10;
            }
            res
        })
        .sum();
    sum % 10 == 0
}

fn gen_sin(prng: &mut Prng) -> String {
    loop {
        let candidate = prng.random_chars(3, 9);
        if check_sin(&candidate) {
            return candidate;
        }
    }
}

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "{}{} {}{}-{}",
                prng.random_chars(4, 1),
                prng.random_chars(3, 2),
                prng.random_chars(4, 1),
                prng.random_chars(3, 2),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "{}{} {}{}-{}",
                prng.random_chars(4, 1),
                prng.random_chars(3, 2),
                prng.random_chars(4, 1),
                prng.random_chars(3, 2),
                prng.random_chars(3, 4)
            )),
        );

        if inc.iter().any(|f| f == "id") {
            let sin = gen_sin(prng);
            user.insert("id".to_string(), json!({ "name": "SIN", "value": sin }));
        }
    });

    // CA postcode: ANA NAN format (letter-digit-letter digit-letter-digit)
    if inc.iter().any(|f| f == "location") {
        if let Some(Value::Object(loc)) = user.get_mut("location") {
            const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
            let l = |prng: &mut Prng| LETTERS[prng.range(0, 25) as usize] as char;
            let d = |prng: &mut Prng| prng.random_chars(3, 1);
            let postcode = format!(
                "{}{}{} {}{}{}",
                l(prng),
                d(prng),
                l(prng),
                d(prng),
                l(prng),
                d(prng)
            );
            loc.insert("postcode".to_string(), Value::String(postcode));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_validation() {
        // Known-good SIN: 046 454 286
        assert!(check_sin("046454286"));
        // Bad SIN
        assert!(!check_sin("123456789"));
    }

    #[test]
    fn gen_sin_is_valid() {
        let mut prng = Prng::new();
        prng.seed_from_str("ca_sin", 1);
        for _ in 0..10 {
            let sin = gen_sin(&mut prng);
            assert!(check_sin(&sin), "generated invalid SIN: {sin}");
        }
    }

    #[test]
    fn postcode_format() {
        let mut prng = Prng::new();
        prng.seed_from_str("ca_post", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        user.insert("location".to_string(), json!({"postcode": "00000"}));
        let inc: Vec<String> = ["id", "phone", "cell", "picture", "location"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        let pc = user["location"]["postcode"].as_str().unwrap();
        // A1A 1A1 — 3 chars, space, 3 chars
        assert_eq!(pc.len(), 7);
        assert_eq!(&pc[3..4], " ");
    }
}
