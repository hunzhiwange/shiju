//! Plugin hook protocol implementation.
use anyhow::{Context, Result};
use serde_json::Value;

pub fn expand(value: &Value, event: &Value) -> Result<Value> {
    Ok(match value {
        Value::Object(map) => Value::Object(
            map.iter().map(|(k, v)| Ok((k.clone(), expand(v, event)?))).collect::<Result<_>>()?,
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| expand(v, event)).collect::<Result<_>>()?)
        }
        Value::String(text) => {
            let pattern = regex::Regex::new(r"\$\{([^{}]+)\}")?;
            let resolve = |path: &str| -> Result<Value> {
                let mut current = event;
                for part in path.split('.') {
                    current = current.get(part).context("missing MCP hook template field")?;
                }
                Ok(current.clone())
            };
            if let Some(capture) = pattern.captures(text)
                && capture.get(0).unwrap().as_str() == text
            {
                resolve(&capture[1])?
            } else {
                let mut output = String::new();
                let mut previous = 0;
                for capture in pattern.captures_iter(text) {
                    let matched = capture.get(0).unwrap();
                    output.push_str(&text[previous..matched.start()]);
                    let value = resolve(&capture[1])?;
                    output.push_str(
                        &value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()),
                    );
                    previous = matched.end();
                }
                output.push_str(&text[previous..]);
                Value::String(output)
            }
        }
        _ => value.clone(),
    })
}
