//! A small JSON-schema check for final answers (`run --output-schema`, GRAFT-0101).
//! Supports what scripts usually need: `type`, `required`, `properties`, `items`,
//! `enum`. Anything else in the schema is ignored rather than rejected.

use serde_json::Value;

/// Parse the answer (a bare JSON object, optionally inside a ```json fence) and
/// list what doesn't match `schema`. Empty = valid.
pub fn check_answer(answer: &str, schema: &Value) -> Vec<String> {
    let text = answer.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .and_then(|t| t.strip_suffix("```"))
        .map_or(text, str::trim);
    match serde_json::from_str::<Value>(text) {
        Ok(v) => {
            let mut problems = Vec::new();
            check(&v, schema, "$", &mut problems);
            problems
        }
        Err(e) => vec![format!("the answer is not valid JSON ({e})")],
    }
}

/// The answer as compact JSON, if it parses (fences removed).
pub fn normalise(answer: &str) -> Option<String> {
    let text = answer.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .and_then(|t| t.strip_suffix("```"))
        .map_or(text, str::trim);
    serde_json::from_str::<Value>(text).ok().map(|v| v.to_string())
}

fn check(v: &Value, schema: &Value, at: &str, problems: &mut Vec<String>) {
    if problems.len() >= 10 {
        return;
    }
    if let Some(t) = schema.get("type").and_then(Value::as_str) {
        let ok = match t {
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "integer" => v.is_i64() || v.is_u64(),
            "number" => v.is_number(),
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            _ => true,
        };
        if !ok {
            problems.push(format!("{at} should be {t}"));
            return;
        }
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array)
        && !options.contains(v)
    {
        problems.push(format!("{at} must be one of {}", Value::Array(options.clone())));
    }
    if let Some(obj) = v.as_object() {
        for key in schema.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            if !obj.contains_key(key) {
                problems.push(format!("{at}.{key} is required"));
            }
        }
        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            for (key, sub) in props {
                if let Some(child) = obj.get(key) {
                    check(child, sub, &format!("{at}.{key}"), problems);
                }
            }
        }
    }
    if let (Some(items), Some(arr)) = (schema.get("items"), v.as_array()) {
        for (i, child) in arr.iter().enumerate() {
            check(child, items, &format!("{at}[{i}]"), problems);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn checks_answers_against_a_schema() {
        let schema = json!({"type": "object", "required": ["summary", "files"], "properties": {
            "summary": {"type": "string"},
            "files": {"type": "array", "items": {"type": "string"}},
            "risk": {"enum": ["low", "high"]}
        }});
        assert!(check_answer(r#"{"summary": "ok", "files": ["a.rs"]}"#, &schema).is_empty());
        assert!(check_answer("```json\n{\"summary\": \"ok\", \"files\": []}\n```", &schema).is_empty());
        let p = check_answer(r#"{"files": [1], "risk": "medium"}"#, &schema);
        assert!(p.iter().any(|x| x.contains("$.summary is required")), "{p:?}");
        assert!(p.iter().any(|x| x.contains("$.files[0] should be string")), "{p:?}");
        assert!(p.iter().any(|x| x.contains("$.risk must be one of")), "{p:?}");
        assert!(check_answer("Here you go: {}", &schema)[0].contains("not valid JSON"));
        assert_eq!(normalise("```json\n{\"a\": 1}\n```").as_deref(), Some(r#"{"a":1}"#));
    }
}
