use serde_json::{Map, Value};

pub struct FormatOutput {
    pub body: String,
    pub ext: &'static str,
    pub content_type: &'static str,
    /// The number of results actually generated (after clamping); used by the
    /// route handler so the stats record does not re-derive the clamped count.
    pub resolved_results: usize,
}

/// Top-level response envelope.
pub struct ApiResponse<'a> {
    pub results: &'a [Map<String, Value>],
    pub info: Option<InfoBlock>,
}

pub struct InfoBlock {
    pub seed: String,
    pub results: usize,
    pub page: u32,
    pub version: &'static str,
}

impl<'a> ApiResponse<'a> {
    fn to_json_value(&self) -> Value {
        let results_val: Vec<Value> = self
            .results
            .iter()
            .map(|u| Value::Object(u.clone()))
            .collect();

        let mut map = Map::new();
        map.insert("results".to_string(), Value::Array(results_val));
        if let Some(info) = &self.info {
            let mut info_map = Map::new();
            info_map.insert("seed".to_string(), Value::String(info.seed.clone()));
            info_map.insert("results".to_string(), Value::Number(info.results.into()));
            info_map.insert("page".to_string(), Value::Number(info.page.into()));
            info_map.insert("version".to_string(), Value::String(info.version.to_string()));
            map.insert("info".to_string(), Value::Object(info_map));
        }
        Value::Object(map)
    }
}

pub fn format_json(resp: &ApiResponse, pretty: bool) -> FormatOutput {
    let v = resp.to_json_value();
    let body = if pretty {
        serde_json::to_string_pretty(&v).unwrap_or_default()
    } else {
        serde_json::to_string(&v).unwrap_or_default()
    };
    FormatOutput {
        body,
        ext: "json",
        content_type: "application/json",
        resolved_results: resp.results.len(),
    }
}

pub fn format_yaml(resp: &ApiResponse) -> FormatOutput {
    let v = resp.to_json_value();
    let body = serde_yml::to_string(&v).unwrap_or_default();
    FormatOutput {
        body,
        ext: "yaml",
        content_type: "text/x-yaml",
        resolved_results: resp.results.len(),
    }
}

/// Serialize a JSON Value recursively to XML elements under `parent_tag`.
fn value_to_xml(tag: &str, value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            out.push('<');
            out.push_str(tag);
            out.push('>');
            for (k, v) in map {
                value_to_xml(k, v, out);
            }
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        Value::Array(arr) => {
            for item in arr {
                value_to_xml(tag, item, out);
            }
        }
        Value::String(s) => {
            out.push('<');
            out.push_str(tag);
            out.push('>');
            out.push_str(&xml_escape(s));
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        Value::Number(n) => {
            out.push('<');
            out.push_str(tag);
            out.push('>');
            out.push_str(&n.to_string());
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        Value::Bool(b) => {
            out.push('<');
            out.push_str(tag);
            out.push('>');
            out.push_str(if *b { "true" } else { "false" });
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        Value::Null => {
            out.push('<');
            out.push_str(tag);
            out.push_str("/>");
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn format_xml(resp: &ApiResponse) -> FormatOutput {
    let v = resp.to_json_value();
    let mut body = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    value_to_xml("results", &v, &mut body);
    FormatOutput {
        body,
        ext: "xml",
        content_type: "text/xml",
        resolved_results: resp.results.len(),
    }
}

/// Flatten a JSON Value into dot-notation key/value pairs (for CSV header + row).
fn flatten_value(prefix: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_value(&key, v, out);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = format!("{}[{}]", prefix, i);
                flatten_value(&key, v, out);
            }
        }
        Value::String(s) => out.push((prefix.to_string(), s.clone())),
        Value::Number(n) => out.push((prefix.to_string(), n.to_string())),
        Value::Bool(b) => out.push((prefix.to_string(), b.to_string())),
        Value::Null => out.push((prefix.to_string(), String::new())),
    }
}

pub fn format_csv(resp: &ApiResponse) -> FormatOutput {
    if resp.results.is_empty() {
        return FormatOutput {
            body: String::new(),
            ext: "csv",
            content_type: "text/csv",
            resolved_results: 0,
        };
    }

    // Derive headers from the first result. All results share the same field
    // set because inc/exc is resolved once per request, so later rows will
    // never have columns that the header row lacks.
    let mut headers: Vec<String> = Vec::new();
    let mut pairs: Vec<(String, String)> = Vec::new();
    flatten_value("", &Value::Object(resp.results[0].clone()), &mut pairs);
    for (k, _) in &pairs {
        if !headers.contains(k) {
            headers.push(k.clone());
        }
    }

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(&headers).expect("in-memory csv write");

    for user in resp.results {
        let mut row_pairs: Vec<(String, String)> = Vec::new();
        flatten_value("", &Value::Object(user.clone()), &mut row_pairs);
        let row_map: std::collections::HashMap<String, String> =
            row_pairs.into_iter().collect();
        let row: Vec<String> = headers
            .iter()
            .map(|h| row_map.get(h).cloned().unwrap_or_default())
            .collect();
        wtr.write_record(&row).expect("in-memory csv write");
    }

    let body = String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default();
    FormatOutput {
        body,
        ext: "csv",
        content_type: "text/csv",
        resolved_results: resp.results.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_resp(results: &[Map<String, Value>]) -> ApiResponse<'_> {
        ApiResponse {
            results,
            info: Some(InfoBlock {
                seed: "testseed".to_string(),
                results: results.len(),
                page: 1,
                version: "1.4",
            }),
        }
    }

    fn one_user() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("gender".to_string(), json!("male"));
        m.insert("name".to_string(), json!({"title": "Mr", "first": "John", "last": "Doe"}));
        m
    }

    #[test]
    fn json_is_valid() {
        let u = one_user();
        let out = format_json(&sample_resp(&[u]), false);
        let parsed: Value = serde_json::from_str(&out.body).unwrap();
        assert!(parsed["results"].is_array());
        assert_eq!(parsed["info"]["version"], "1.4");
        assert_eq!(out.ext, "json");
        assert_eq!(out.content_type, "application/json");
    }

    #[test]
    fn json_pretty_starts_with_brace_newline() {
        let u = one_user();
        let out = format_json(&sample_resp(&[u]), true);
        assert!(out.body.starts_with("{\n  \"results\""));
    }

    #[test]
    fn yaml_contains_seed() {
        let u = one_user();
        let out = format_yaml(&sample_resp(&[u]));
        assert!(out.body.contains("testseed"));
        assert_eq!(out.content_type, "text/x-yaml");
    }

    #[test]
    fn xml_has_xml_decl() {
        let u = one_user();
        let out = format_xml(&sample_resp(&[u]));
        assert!(out.body.starts_with("<?xml"));
        assert!(out.body.contains("<results>"));
        assert_eq!(out.content_type, "text/xml");
    }

    #[test]
    fn csv_has_header_row() {
        let u = one_user();
        let out = format_csv(&sample_resp(&[u]));
        let lines: Vec<&str> = out.body.lines().collect();
        assert!(lines.len() >= 2, "CSV must have at least header + data row");
        assert!(lines[0].contains("gender"), "header must contain 'gender'");
        assert_eq!(out.content_type, "text/csv");
    }

    #[test]
    fn xml_escapes_ampersand() {
        let mut m = Map::new();
        m.insert("name".to_string(), json!("Tom & Jerry"));
        let out = format_xml(&sample_resp(&[m]));
        assert!(out.body.contains("Tom &amp; Jerry"));
    }
}
