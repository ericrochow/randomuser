use super::{include_field, with_picture_reorder, NatDatasets};
use crate::generator::prng::{pad, Prng};
use chrono::Datelike;
use serde_json::{json, Map, Value};

fn check_digit(weights: &[i64], digits: &[i64]) -> i64 {
    let sum: i64 = weights.iter().zip(digits.iter()).map(|(w, d)| w * d).sum();
    let m = sum % 11;
    if m == 0 { 0 } else { 11 - m }
}

/// Norwegian fødselsnummer (11 digits): DDMMYY NNN K1 K2
///   NNN range by birth century:
///     2000–2099 → 500–999
///     1800–1899 → 500–749
///     1900–1999 → 000–499
///   NNN parity: odd = male, even = female
///   K1 and K2 must not both be 0.
fn gen_fnr(birth_date_str: &str, birth_year: i32, gender: &str, prng: &mut Prng) -> String {
    for _ in 0..200 {
        let no = {
            let (lo, hi) = if birth_year >= 2000 {
                (500i64, 999)
            } else if birth_year <= 1899 {
                (500i64, 749)
            } else {
                (0i64, 499)
            };
            let mut n = prng.range(lo, hi);
            // Adjust parity: odd=male, even=female
            let is_odd = n % 2 == 1;
            let is_male = gender == "male";
            if is_odd != is_male {
                if n > lo { n -= 1; } else { n += 1; }
            }
            n
        };

        let ten_digits_str = format!("{}{}", birth_date_str, pad(no, 3));
        let digits: Vec<i64> = ten_digits_str
            .chars()
            .map(|c| c.to_digit(10).unwrap() as i64)
            .collect();

        let k1 = check_digit(&[3, 7, 6, 1, 8, 9, 4, 5, 2], &digits);
        if k1 == 10 {
            continue;
        }

        let mut digits11 = digits.clone();
        digits11.push(k1);
        let k2 = check_digit(&[5, 4, 3, 2, 7, 6, 5, 4, 3, 2], &digits11);
        if k2 == 10 {
            continue;
        }

        if k1 == 0 && k2 == 0 {
            continue;
        }

        return format!("{}{}{}{}", birth_date_str, pad(no, 3), k1, k2);
    }
    // Fallback: practically unreachable with MT19937; use minimal NNN and
    // clamp any invalid check digit rather than looping forever.
    let nnn = if gender == "male" { 1i64 } else { 2i64 };
    let ten_str = format!("{}{}", birth_date_str, pad(nnn, 3));
    let digits: Vec<i64> = ten_str.chars().map(|c| c.to_digit(10).unwrap() as i64).collect();
    let k1 = check_digit(&[3, 7, 6, 1, 8, 9, 4, 5, 2], &digits).min(9);
    let mut d11 = digits;
    d11.push(k1);
    let k2 = check_digit(&[5, 4, 3, 2, 7, 6, 5, 4, 3, 2], &d11).min(9);
    format!("{}{}{}{}", birth_date_str, pad(nnn, 3), k1, k2)
}

pub fn inject(
    inc: &[String],
    user: &mut Map<String, Value>,
    prng: &mut Prng,
    datasets: &NatDatasets,
) {
    with_picture_reorder(inc, user, |user| {
        let phone_prefix = *prng.random_item(&[2i64, 3, 5, 6, 7, 8]);
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!("{}{}", phone_prefix, prng.random_chars(3, 7))),
        );

        let cell_prefix = *prng.random_item(&[4i64, 9]);
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!("{}{}", cell_prefix, prng.random_chars(3, 7))),
        );

        if inc.iter().any(|f| f == "id") {
            let dob_date = user
                .get("dob")
                .and_then(|d| d.get("date"))
                .and_then(|d| d.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.naive_utc().date())
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let birth_date_str = format!(
                "{}{}{}",
                pad(dob_date.day(), 2),
                pad(dob_date.month(), 2),
                pad(dob_date.year() % 100, 2)
            );
            let gender = user
                .get("gender")
                .and_then(|g| g.as_str())
                .unwrap_or("male");
            let fnr = gen_fnr(&birth_date_str, dob_date.year(), gender, prng);
            user.insert("id".to_string(), json!({ "name": "FN", "value": fnr }));
        }
    });

    // NO uses a real postcode from the dataset
    if inc.iter().any(|f| f == "location") {
        let post_codes = datasets.nat_list("NO", "post_codes");
        if !post_codes.is_empty() {
            if let Some(Value::Object(loc)) = user.get_mut("location") {
                let pc = prng.random_item(post_codes).clone();
                loc.insert("postcode".to_string(), Value::String(pc));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnr_length_is_11() {
        let mut prng = Prng::new();
        prng.seed_from_str("no_fnr", 1);
        let fnr = gen_fnr("150685", 1985, "male", &mut prng);
        assert_eq!(fnr.len(), 11, "FNR must be 11 digits: {fnr}");
    }

    #[test]
    fn fnr_k1_and_k2_not_both_zero() {
        let mut prng = Prng::new();
        prng.seed_from_str("no_fnr_zero", 1);
        for _ in 0..20 {
            let fnr = gen_fnr("010185", 1985, "female", &mut prng);
            let digits: Vec<i64> = fnr.chars().map(|c| c.to_digit(10).unwrap() as i64).collect();
            assert!(
                !(digits[9] == 0 && digits[10] == 0),
                "FNR must not have both k1=0 and k2=0: {fnr}"
            );
        }
    }

    #[test]
    fn fnr_check_digits_valid() {
        let mut prng = Prng::new();
        prng.seed_from_str("no_fnr_check", 1);
        let fnr = gen_fnr("150685", 1985, "male", &mut prng);
        let digits: Vec<i64> = fnr.chars().map(|c| c.to_digit(10).unwrap() as i64).collect();
        let k1 = check_digit(&[3, 7, 6, 1, 8, 9, 4, 5, 2], &digits[..9]);
        let k2 = check_digit(&[5, 4, 3, 2, 7, 6, 5, 4, 3, 2], &digits[..10]);
        assert_eq!(k1, digits[9], "k1 mismatch");
        assert_eq!(k2, digits[10], "k2 mismatch");
    }
}
