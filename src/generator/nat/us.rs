use super::{include_field, with_picture_reorder};
use crate::generator::prng::Prng;
use serde_json::{json, Map, Value};

/// Validate SSN against NANP rules.
fn check_ssn(ssn: &str) -> bool {
    // Must be XXX-XX-XXXX format
    let parts: Vec<&str> = ssn.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 3 || parts[1].len() != 2 || parts[2].len() != 4 {
        return false;
    }
    // Special invalid numbers
    if ssn == "219-09-9999" || ssn == "078-05-1120" {
        return false;
    }
    // Area number (first 3) must not be 000, 666, or 900-999
    let area: u32 = parts[0].parse().unwrap_or(0);
    if area == 0 || area == 666 || area >= 900 {
        return false;
    }
    // Group number (middle 2) must not be 00
    let group: u32 = parts[1].parse().unwrap_or(0);
    if group == 0 {
        return false;
    }
    // Serial number (last 4) must not be 0000
    let serial: u32 = parts[2].parse().unwrap_or(0);
    if serial == 0 {
        return false;
    }
    true
}

fn gen_ssn(prng: &mut Prng) -> String {
    for _ in 0..200 {
        let ssn = format!(
            "{}-{}-{}",
            prng.random_chars(3, 3),
            prng.random_chars(3, 2),
            prng.random_chars(3, 4)
        );
        if check_ssn(&ssn) {
            return ssn;
        }
    }
    "001-01-0001".to_string()
}

pub fn inject(inc: &[String], user: &mut Map<String, Value>, prng: &mut Prng) {
    with_picture_reorder(inc, user, |user| {
        // US phone: (NXX) NXX-XXXX where N is from "23456789" (mode 5)
        include_field(
            inc,
            user,
            "phone",
            Value::String(format!(
                "({}{}) {}{}-{}",
                prng.random_chars(5, 1),
                prng.random_chars(3, 2),
                prng.random_chars(5, 1),
                prng.random_chars(3, 2),
                prng.random_chars(3, 4)
            )),
        );
        include_field(
            inc,
            user,
            "cell",
            Value::String(format!(
                "({}{}) {}{}-{}",
                prng.random_chars(5, 1),
                prng.random_chars(3, 2),
                prng.random_chars(5, 1),
                prng.random_chars(3, 2),
                prng.random_chars(3, 4)
            )),
        );

        if inc.iter().any(|f| f == "id") {
            let ssn = gen_ssn(prng);
            user.insert("id".to_string(), json!({ "name": "SSN", "value": ssn }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssn_validation_rejects_invalid() {
        assert!(!check_ssn("000-12-3456")); // area 000
        assert!(!check_ssn("666-12-3456")); // area 666
        assert!(!check_ssn("900-12-3456")); // area >= 900
        assert!(!check_ssn("123-00-3456")); // group 00
        assert!(!check_ssn("123-12-0000")); // serial 0000
        assert!(!check_ssn("219-09-9999")); // special invalid
        assert!(!check_ssn("078-05-1120")); // special invalid
    }

    #[test]
    fn ssn_validation_accepts_valid() {
        assert!(check_ssn("123-45-6789"));
        assert!(check_ssn("001-01-0001"));
        assert!(check_ssn("899-99-9999"));
    }

    #[test]
    fn gen_ssn_is_always_valid() {
        let mut prng = Prng::new();
        prng.seed_from_str("us_ssn", 1);
        for _ in 0..100 {
            let ssn = gen_ssn(&mut prng);
            assert!(check_ssn(&ssn), "generated invalid SSN: {ssn}");
        }
    }

    #[test]
    fn phone_format() {
        let mut prng = Prng::new();
        prng.seed_from_str("us_phone", 1);
        let mut user = Map::new();
        user.insert("picture".to_string(), json!({}));
        let inc: Vec<String> = ["phone", "cell", "id", "picture"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        inject(&inc, &mut user, &mut prng);
        let phone = user["phone"].as_str().unwrap();
        // (NXX) NXX-XXXX
        assert!(phone.starts_with('('));
        assert_eq!(phone.len(), 14);
    }
}
