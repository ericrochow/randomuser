//! Integration tests for the API generator.
//!
//! These tests instantiate a real Generator (with real data files) and drive
//! it through the HTTP-like generate() interface, verifying the output
//! matches the contract documented in the original spec/ test suite.

use randomuser::generator::{GenerateOptions, Generator};
use serde_json::Value;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn gen() -> Generator {
    let mut g = Generator::new("1.4");
    g.init(&data_dir()).expect("data dir must exist");
    g
}

fn generate(opts: GenerateOptions) -> Value {
    let out = gen().generate(opts);
    serde_json::from_str(&out.body).expect("output must be valid JSON")
}

// ─── General contract ─────────────────────────────────────────────────────────

#[test]
fn returns_200_with_results_array() {
    let v = generate(GenerateOptions {
        max_results: 5000,
        ..Default::default()
    });
    assert!(v["results"].is_array());
    assert_eq!(v["results"].as_array().unwrap().len(), 1);
}

#[test]
fn info_block_has_correct_version() {
    let v = generate(GenerateOptions {
        max_results: 5000,
        ..Default::default()
    });
    assert_eq!(v["info"]["version"], "1.4");
}

#[test]
fn same_seed_returns_identical_output() {
    let seed = "integration_seed_42";
    let opts = || GenerateOptions {
        results: Some(5),
        seed: Some(seed.to_string()),
        max_results: 5000,
        ..Default::default()
    };
    let out1 = gen().generate(opts());
    let out2 = gen().generate(opts());
    assert_eq!(out1.body, out2.body);
}

#[test]
fn different_page_with_same_seed_differs() {
    let seed = "paged_seed";
    let make = |page: u32| GenerateOptions {
        results: Some(3),
        seed: Some(seed.to_string()),
        page: Some(page),
        max_results: 5000,
        ..Default::default()
    };
    let out1 = gen().generate(make(1));
    let out2 = gen().generate(make(10));
    assert_ne!(out1.body, out2.body);
}

// ─── Result count ─────────────────────────────────────────────────────────────

#[test]
fn fetch_5000_results() {
    let v = generate(GenerateOptions {
        results: Some(5000),
        max_results: 5000,
        ..Default::default()
    });
    assert_eq!(v["results"].as_array().unwrap().len(), 5000);
}

#[test]
fn result_count_above_max_gives_1() {
    let v = generate(GenerateOptions {
        results: Some(5001),
        max_results: 5000,
        ..Default::default()
    });
    assert_eq!(v["results"].as_array().unwrap().len(), 1);
}

// ─── Field include/exclude ────────────────────────────────────────────────────

#[test]
fn include_fields_returns_only_those_fields() {
    let v = generate(GenerateOptions {
        results: Some(10),
        inc: Some("name,email".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let mut keys: Vec<&str> = user.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        keys.sort();
        assert_eq!(keys, vec!["email", "name"]);
    }
}

#[test]
fn exclude_fields_removes_them() {
    let v = generate(GenerateOptions {
        results: Some(10),
        exc: Some("picture,login".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        assert!(user.get("picture").is_none());
        assert!(user.get("login").is_none());
        assert!(user.get("email").is_some(), "other fields must still be present");
    }
}

// ─── Nationality ──────────────────────────────────────────────────────────────

const ALL_NATS: &[&str] = &[
    "AU", "BR", "CA", "CH", "DE", "DK", "ES", "FI", "FR", "GB", "IE", "IN", "IR", "MX", "NL",
    "NO", "NZ", "RS", "TR", "UA", "US",
];

#[test]
fn single_nat_filter_respected() {
    for nat in ALL_NATS {
        let v = generate(GenerateOptions {
            results: Some(10),
            nat: Some(nat.to_string()),
            max_results: 5000,
            ..Default::default()
        });
        for user in v["results"].as_array().unwrap() {
            assert_eq!(
                user["nat"].as_str().unwrap(),
                *nat,
                "nat mismatch for {nat}"
            );
        }
    }
}

#[test]
fn multiple_nat_filter_only_returns_requested_nats() {
    let requested = ["US", "FR", "DE"];
    let v = generate(GenerateOptions {
        results: Some(100),
        nat: Some(requested.join(",")),
        max_results: 5000,
        ..Default::default()
    });
    let result_nats: std::collections::HashSet<&str> = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["nat"].as_str().unwrap())
        .collect();
    for nat in &result_nats {
        assert!(requested.contains(nat), "unexpected nat {nat}");
    }
}

#[test]
fn invalid_nat_falls_back_to_random() {
    let v = generate(GenerateOptions {
        results: Some(10),
        nat: Some("BLAH".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let nat = user["nat"].as_str().unwrap();
        assert!(ALL_NATS.contains(&nat), "got unexpected nat {nat}");
    }
}

#[test]
fn lego_nat_works() {
    let v = generate(GenerateOptions {
        results: Some(5),
        lego: true,
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        assert_eq!(user["nat"], "LEGO");
    }
}

// ─── Gender ───────────────────────────────────────────────────────────────────

#[test]
fn gender_filter_male() {
    let v = generate(GenerateOptions {
        results: Some(50),
        gender: Some("male".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        assert_eq!(user["gender"], "male");
    }
}

#[test]
fn gender_filter_female() {
    let v = generate(GenerateOptions {
        results: Some(50),
        gender: Some("female".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        assert_eq!(user["gender"], "female");
    }
}

// ─── Email ────────────────────────────────────────────────────────────────────

#[test]
fn email_has_no_spaces() {
    let v = generate(GenerateOptions {
        results: Some(500),
        inc: Some("email".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let email = user["email"].as_str().unwrap();
        assert!(!email.contains(' '), "email has space: {email}");
    }
}

#[test]
fn email_is_lowercase() {
    let v = generate(GenerateOptions {
        results: Some(500),
        inc: Some("email".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let email = user["email"].as_str().unwrap();
        assert_eq!(email, email.to_lowercase(), "email not lowercase: {email}");
    }
}

#[test]
fn email_ends_with_example_com() {
    let v = generate(GenerateOptions {
        results: Some(50),
        inc: Some("email".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let email = user["email"].as_str().unwrap();
        assert!(email.ends_with("@example.com"), "bad email domain: {email}");
    }
}

// ─── Login ────────────────────────────────────────────────────────────────────

#[test]
fn login_has_all_required_fields() {
    let v = generate(GenerateOptions {
        results: Some(10),
        inc: Some("login".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let login = &user["login"];
        assert!(login["uuid"].is_string());
        assert!(login["username"].is_string());
        assert!(login["password"].is_string());
        assert!(login["salt"].is_string());
        assert!(login["md5"].is_string());
        assert!(login["sha1"].is_string());
        assert!(login["sha256"].is_string());
    }
}

#[test]
fn password_generation_with_custom_spec() {
    let v = generate(GenerateOptions {
        results: Some(50),
        inc: Some("login".to_string()),
        password: Some("upper,lower,number,8-16".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let pwd = user["login"]["password"].as_str().unwrap();
        assert!(
            (8..=16).contains(&pwd.len()),
            "password length out of range: {pwd}"
        );
        assert!(
            pwd.chars()
                .any(|c| c.is_ascii_uppercase() || c.is_ascii_lowercase() || c.is_ascii_digit()),
            "password has no expected chars: {pwd}"
        );
    }
}

// ─── DOB / Registered ────────────────────────────────────────────────────────

#[test]
fn dob_precedes_registered() {
    let v = generate(GenerateOptions {
        results: Some(100),
        max_results: 5000,
        ..Default::default()
    });
    for user in v["results"].as_array().unwrap() {
        let dob = user["dob"]["date"].as_str().unwrap();
        let reg = user["registered"]["date"].as_str().unwrap();
        assert!(dob < reg, "dob must precede registered: dob={dob} reg={reg}");
    }
}

// ─── Output formats ───────────────────────────────────────────────────────────

#[test]
fn fmt_json_returns_valid_json() {
    let out = gen().generate(GenerateOptions {
        fmt: Some("json".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    assert_eq!(out.content_type, "application/json");
    serde_json::from_str::<Value>(&out.body).expect("must be valid JSON");
}

#[test]
fn fmt_pretty_returns_indented_json() {
    let out = gen().generate(GenerateOptions {
        fmt: Some("pretty".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    assert!(out.body.starts_with("{\n  \"results\""), "pretty output: {}", &out.body[..50]);
}

#[test]
fn fmt_xml_has_xml_declaration() {
    let out = gen().generate(GenerateOptions {
        fmt: Some("xml".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    assert_eq!(out.content_type, "text/xml");
    assert!(out.body.starts_with("<?xml"));
}

#[test]
fn fmt_yaml_returns_yaml() {
    let out = gen().generate(GenerateOptions {
        fmt: Some("yaml".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    assert_eq!(out.content_type, "text/x-yaml");
    // YAML should contain some key we can recognize
    assert!(out.body.contains("results:") || out.body.contains("results:\n"));
}

#[test]
fn fmt_csv_has_header_and_data_rows() {
    let out = gen().generate(GenerateOptions {
        fmt: Some("csv".to_string()),
        results: Some(5),
        max_results: 5000,
        ..Default::default()
    });
    assert_eq!(out.content_type, "text/csv");
    let lines: Vec<&str> = out.body.lines().collect();
    assert!(lines.len() >= 6, "expected header + 5 data rows");
    assert!(lines[0].contains("gender") || lines[0].contains("name"));
}

#[test]
fn fmt_invalid_falls_back_to_json() {
    let out = gen().generate(GenerateOptions {
        fmt: Some("blahblah".to_string()),
        max_results: 5000,
        ..Default::default()
    });
    serde_json::from_str::<Value>(&out.body).expect("invalid fmt must fall back to JSON");
}

// ─── noinfo ───────────────────────────────────────────────────────────────────

#[test]
fn noinfo_removes_info_block() {
    let v = generate(GenerateOptions {
        noinfo: true,
        max_results: 5000,
        ..Default::default()
    });
    assert!(v.get("info").is_none(), "info block must be absent with noinfo");
    assert!(v["results"].is_array());
}
