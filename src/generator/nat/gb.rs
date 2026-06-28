use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

fn gen_phone(prng: &mut Prng) -> String {
    // 18 different phone formats (matching original)
    let choice = prng.range(0, 17) as usize;
    match choice {
        0 => format!("01{} {}", prng.random_chars(3, 3), prng.random_chars(3, 5)),
        1 => format!("01{} {}", prng.random_chars(3, 3), prng.random_chars(3, 6)),
        2 => format!(
            "011{}{} {} {}",
            prng.random_chars(3, 1),
            prng.random_chars(3, 3),
            prng.random_chars(3, 3),
            prng.random_chars(3, 4)
        ),
        3 => format!(
            "01{}1 {} {}",
            prng.random_chars(3, 1),
            prng.random_chars(3, 3),
            prng.random_chars(3, 4)
        ),
        4 => format!("013873 {}", prng.random_chars(3, 5)),
        5 => format!("015242 {}", prng.random_chars(3, 5)),
        6 => format!("015394 {}", prng.random_chars(3, 5)),
        7 => format!("015395 {}", prng.random_chars(3, 5)),
        8 => format!("015396 {}", prng.random_chars(3, 5)),
        9 => format!("016973 {}", prng.random_chars(3, 5)),
        10 => format!("016974 {}", prng.random_chars(3, 5)),
        11 => format!("016977 {}", prng.random_chars(3, 4)),
        12 => format!("016977 {}", prng.random_chars(3, 5)),
        13 => format!("017683 {}", prng.random_chars(3, 5)),
        14 => format!("017684 {}", prng.random_chars(3, 5)),
        15 => format!("017687 {}", prng.random_chars(3, 5)),
        16 => format!("019467 {}", prng.random_chars(3, 5)),
        _ => format!(
            "02{} {} {}",
            prng.random_chars(3, 1),
            prng.random_chars(3, 4),
            prng.random_chars(3, 4)
        ),
    }
}

fn code_char(prng: &mut Prng) -> char {
    const CODE: &[u8] = b"ABDEFGHJLNPQRSTUWXYZ";
    CODE[prng.range(0, 19) as usize] as char
}

fn digit_str(prng: &mut Prng) -> String {
    prng.random_chars(3, 1)
}

fn gen_postcode(prng: &mut Prng) -> String {
    let choice = prng.range(0, 5) as usize;
    match choice {
        0 => {
            let (a, b, c, d, e) = (prng.random_chars(4, 1), digit_str(prng), digit_str(prng), code_char(prng), code_char(prng));
            format!("{a}{b} {c}{d}{e}")
        }
        1 => {
            let (a, b, c, d, e) = (prng.random_chars(4, 2), digit_str(prng), digit_str(prng), code_char(prng), code_char(prng));
            format!("{a}{b} {c}{d}{e}")
        }
        2 => {
            let (a, b, c, d, e) = (prng.random_chars(4, 1), prng.random_chars(3, 2), digit_str(prng), code_char(prng), code_char(prng));
            format!("{a}{b} {c}{d}{e}")
        }
        3 => {
            let (a, b, c, d, e) = (prng.random_chars(4, 2), prng.random_chars(3, 2), digit_str(prng), code_char(prng), code_char(prng));
            format!("{a}{b} {c}{d}{e}")
        }
        4 => {
            let (a, b, c, d, e, f) = (prng.random_chars(4, 2), digit_str(prng), prng.random_chars(4, 1), digit_str(prng), code_char(prng), code_char(prng));
            format!("{a}{b}{c} {d}{e}{f}")
        }
        _ => {
            let (a, b, c, d, e, f) = (prng.random_chars(4, 1), digit_str(prng), prng.random_chars(4, 1), digit_str(prng), code_char(prng), code_char(prng));
            format!("{a}{b}{c} {d}{e}{f}")
        }
    }
}

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        if inc.iter().any(|f| f == "phone") {
            let phone = gen_phone(prng);
            user.insert("phone".to_string(), Value::String(phone));
        }

        include_field(
            inc,
            user,
            "cell",
            Value::String(format!("07{} {}", prng.random_chars(3, 3), prng.random_chars(3, 6))),
        );

        if inc.iter().any(|f| f == "location") {
            if let Some(Value::Object(loc)) = user.get_mut("location") {
                let postcode = gen_postcode(prng);
                loc.insert("postcode".to_string(), Value::String(postcode));
            }
        }

        if inc.iter().any(|f| f == "id") {
            // NINO: two letters + space + 2 + space + 2 + space + 2 + space + 1 letter (uppercase)
            let nino_1 = b"abceghjklmnoprstwxyz"; // 20 chars
            let nino_2 = b"abceghjklmnprstwxyz";  // 19 chars
            let n1 = nino_1[prng.range(0, 19) as usize] as char;
            let n2 = nino_2[prng.range(0, 18) as usize] as char;
            let nino = format!(
                "{}{} {} {} {} {}",
                n1, n2,
                prng.random_chars(3, 2),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2),
                prng.random_chars(4, 1)
            )
            .to_uppercase();
            user.insert("id".to_string(), json!({ "name": "NINO", "value": nino }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nino_format() {
        let mut prng = Prng::new();
        prng.seed_from_str("gb_nino", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        user.insert("location".to_string(), json!({"postcode": "SW1A 1AA"}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture", "location"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        let nino = user["id"]["value"].as_str().unwrap();
        let parts: Vec<&str> = nino.split(' ').collect();
        assert_eq!(parts.len(), 5, "NINO must have 5 space-separated parts: {nino}");
        assert_eq!(parts[0].len(), 2);
        assert!(parts[0].chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn cell_starts_with_07() {
        let mut prng = Prng::new();
        prng.seed_from_str("gb_cell", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        user.insert("location".to_string(), json!({"postcode": "x"}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture", "location"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        assert!(user["cell"].as_str().unwrap().starts_with("07"));
    }
}
