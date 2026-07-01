use super::{include_field, with_picture_reorder};
use crate::generator::prng::{pad, Prng};
use chrono::Datelike;
use serde_json::{json, Map, Value};

/// Finnish personal identity code (HETU): DDMMYYCnnnK
///   DD     = day of month (01–31)
///   MM     = month (01–12)
///   YY     = 2-digit year
///   C      = century marker ('+' <1900, '-' 1900–1999, 'A' 2000+)
///   nnn    = individual number (odd=male, even=female)
///   K      = check character
///
/// The original inject has two bugs we fix here:
///   1. Used `getDay()` (weekday) instead of `getDate()` (day of month).
///   2. Used `new Date(dob)` where dob was an object, giving Invalid Date.
const CHECKSUM: &[u8] = b"0123456789ABCDEFHJKLMNPRSTUVWXY";

fn gen_hetu(dob: chrono::NaiveDate, gender: &str, prng: &mut Prng) -> String {

    let day = dob.day();
    let month = dob.month();
    let full_year = dob.year();
    let year = full_year % 100;

    let century = if full_year < 1900 {
        '+'
    } else if full_year < 2000 {
        '-'
    } else {
        'A'
    };

    // Last digit of individual number: even=female, odd=male
    let even_digits = [0u8, 2, 4, 6, 8];
    let odd_digits = [1u8, 3, 5, 7, 9];
    let last_digit = if gender == "male" {
        *prng.random_item(&odd_digits)
    } else {
        *prng.random_item(&even_digits)
    };
    let nnn = format!("{}{}", prng.random_chars(3, 2), last_digit);

    // HETU check: 9-digit number DDMMYYNNN — all fields must be zero-padded.
    let check_input: u32 = format!("{:02}{:02}{:02}{}", day, month, year, nnn)
        .parse()
        .unwrap_or(0);
    let cc = CHECKSUM[(check_input % 31) as usize] as char;

    format!("{}{}{:02}{}{}{}", pad(day, 2), pad(month, 2), year, century, nnn, cc)
}

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "0{}-{}-{}",
                prng.range(2, 9),
                prng.random_chars(3, 3),
                prng.random_chars(3, 3)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "04{}-{}-{}-{}",
                prng.range(0, 9),
                prng.random_chars(3, 3),
                prng.random_chars(3, 2),
                prng.random_chars(3, 2)
            )),
        );

        if inc.iter().any(|f| f == "id") {
            let dob_date = user
                .get("dob")
                .and_then(|d| d.get("date"))
                .and_then(|d| d.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.naive_utc().date())
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let gender = user
                .get("gender")
                .and_then(|g| g.as_str())
                .unwrap_or("male");

            let hetu = gen_hetu(dob_date, gender, prng);
            user.insert("id".to_string(), json!({ "name": "HETU", "value": hetu }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hetu_length() {
        let dob = chrono::NaiveDate::from_ymd_opt(1985, 6, 15).unwrap();
        let mut prng = Prng::new();
        prng.seed_from_str("fi_hetu", 1);
        let h = gen_hetu(dob, "male", &mut prng);
        // DDMMYYC + 3 digits + 1 check = 11
        assert_eq!(h.len(), 11, "HETU must be 11 chars: {h}");
    }

    #[test]
    fn hetu_century_marker() {
        let mut prng = Prng::new();
        prng.seed_from_str("fi_century", 1);
        let dob_1985 = chrono::NaiveDate::from_ymd_opt(1985, 1, 1).unwrap();
        let h = gen_hetu(dob_1985, "male", &mut prng);
        assert_eq!(&h[6..7], "-");

        prng.seed_from_str("fi_century2", 1);
        let dob_2005 = chrono::NaiveDate::from_ymd_opt(2005, 1, 1).unwrap();
        let h2 = gen_hetu(dob_2005, "female", &mut prng);
        assert_eq!(&h2[6..7], "A");
    }

    #[test]
    fn hetu_check_char_correct_for_single_digit_day_and_month() {
        // day=1, month=6 — previously the check was computed from "1685NNN"
        // (7 chars) instead of "010685NNN" (9 chars), giving the wrong char.
        let dob = chrono::NaiveDate::from_ymd_opt(1985, 6, 1).unwrap();
        let mut prng = Prng::new();
        prng.seed_from_str("fi_pad", 1);
        let h = gen_hetu(dob, "male", &mut prng);
        // Verify the check character by recomputing it from the HETU string.
        let digits_str = format!("{}{}", &h[..6], &h[7..10]);
        let check_num: u32 = digits_str.parse().unwrap();
        let expected_cc = CHECKSUM[(check_num % 31) as usize] as char;
        assert_eq!(
            h.chars().last().unwrap(),
            expected_cc,
            "check char wrong for single-digit day/month HETU: {h}"
        );
    }

    #[test]
    fn hetu_gender_parity() {
        let dob = chrono::NaiveDate::from_ymd_opt(1980, 3, 20).unwrap();
        let mut prng = Prng::new();
        prng.seed_from_str("fi_gender", 1);
        // For male, individual number last digit must be odd
        let h = gen_hetu(dob, "male", &mut prng);
        let nnn_last: u32 = h[9..10].parse().unwrap();
        assert_eq!(nnn_last % 2, 1, "male HETU must have odd individual number");
    }
}
