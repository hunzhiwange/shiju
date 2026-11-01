//! Desktop validation for the primitive values accepted by MCP forms.
pub(crate) fn valid(form: &vw_shared::question::McpForm, answers: &[Vec<String>]) -> bool {
    let Some(fields) = form.schema["properties"].as_object() else { return false };
    if answers.len() != fields.len() + 1 {
        return false;
    }
    if answers.last().is_some_and(|v| v.as_slice() == ["decline"]) {
        return true;
    }
    if !answers.last().is_some_and(|v| v.as_slice() == ["accept"]) {
        return false;
    }
    for ((name, field), answer) in fields.iter().zip(answers) {
        if answer.is_empty() {
            if form.schema["required"].as_array().is_some_and(|xs| xs.iter().any(|v| v == name)) {
                return false;
            }
            continue;
        }
        let [value] = answer.as_slice() else { return false };
        match field["type"].as_str() {
            Some("string") => {
                let len = value.chars().count() as u64;
                if field["minLength"].as_u64().is_some_and(|n| len < n)
                    || field["maxLength"].as_u64().is_some_and(|n| len > n)
                    || field["enum"].as_array().is_some_and(|xs| !xs.iter().any(|v| v == value))
                {
                    return false;
                }
            }
            Some("boolean") => {
                if value != "true" && value != "false" {
                    return false;
                }
            }
            Some(kind @ ("number" | "integer")) => {
                let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value) else {
                    return false;
                };
                if kind == "integer" && !parsed.is_i64() && !parsed.is_u64() {
                    return false;
                }
                let Some(number) = parsed.as_f64() else { return false };
                if field["minimum"].as_f64().is_some_and(|n| number < n)
                    || field["maximum"].as_f64().is_some_and(|n| number > n)
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}
