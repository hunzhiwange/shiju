//! Resident third-party Responses repair, before protocol observation.
use axum::body::{Body, Bytes};
use futures_util::StreamExt;
use serde_json::Value;
use std::collections::BTreeMap;

const LIMIT: usize = 4 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Repair {
    arguments: BTreeMap<String, String>,
    held: Vec<(String, String, Value)>,
    retained: usize,
}

impl Repair {
    pub(super) fn push(&mut self, raw: String) -> Result<String, std::io::Error> {
        let data = raw
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(|s| s.strip_prefix(' ').unwrap_or(s))
            .collect::<Vec<_>>()
            .join("\n");
        let event = serde_json::from_str::<Value>(&data).ok();
        let kind = event.as_ref().and_then(|v| v["type"].as_str()).unwrap_or("");
        if kind == "response.function_call_arguments.delta" {
            let value = event.as_ref().unwrap();
            let id = value["item_id"].as_str().ok_or_else(invalid)?;
            let delta = value["delta"].as_str().ok_or_else(invalid)?;
            // Account for IDs as well as payload; empty deltas cannot grow the map unboundedly.
            self.retain(id.len().saturating_add(delta.len()))?;
            self.arguments.entry(id.into()).or_default().push_str(delta);
            return Ok(raw);
        }
        let end = event.as_ref().and_then(|value| {
            let (id, arguments) = match kind {
                "response.function_call_arguments.done" => (&value["item_id"], &value["arguments"]),
                "response.output_item.done" if value["item"]["type"] == "function_call" => {
                    (&value["item"]["id"], &value["item"]["arguments"])
                }
                _ => return None,
            };
            arguments.as_str().is_none_or(str::is_empty).then(|| id.as_str()).flatten()
        });
        if let Some(id) = end {
            let id = id.to_owned();
            self.retain(raw.len().saturating_mul(2).saturating_add(id.len()))?;
            self.held.push((id, raw, event.unwrap()));
            return Ok(String::new());
        }
        // Never flush speculative tool completions on a failed upstream terminal.
        if matches!(kind, "error" | "response.failed" | "response.incomplete") {
            self.held.clear();
            self.arguments.clear();
            self.retained = 0;
            return Ok(raw);
        }
        let mut out = String::new();
        for (id, original, mut held) in std::mem::take(&mut self.held) {
            let arguments =
                self.arguments.get(&id).filter(|s| !s.is_empty()).map(String::as_str).or_else(
                    || {
                        if kind != "response.completed" {
                            return None;
                        }
                        event.as_ref()?["response"]["output"]
                            .as_array()?
                            .iter()
                            .find(|item| item["id"].as_str() == Some(&id))?["arguments"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                    },
                );
            if let Some(arguments) = arguments {
                if held["type"] == "response.output_item.done" {
                    held["item"]["arguments"] = arguments.into();
                } else {
                    held["arguments"] = arguments.into();
                }
                // Preserve event/id/retry fields; only the JSON data field changes.
                for line in original.lines().filter(|line| !line.starts_with("data:")) {
                    if !line.is_empty() {
                        out.push_str(line);
                        out.push('\n');
                    }
                }
                out.push_str("data: ");
                out.push_str(&held.to_string());
                out.push_str("\n\n");
            } else {
                out.push_str(&original);
            }
        }
        out.push_str(&raw);
        if kind == "response.completed" {
            self.arguments.clear();
            self.retained = 0;
        }
        Ok(out)
    }

    fn retain(&mut self, bytes: usize) -> Result<(), std::io::Error> {
        self.retained = self.retained.saturating_add(bytes);
        if self.retained > LIMIT {
            return Err(invalid());
        }
        Ok(())
    }
}

fn invalid() -> std::io::Error {
    std::io::Error::other("Invalid or oversized Responses late arguments")
}

pub(crate) fn body(body: Body) -> Body {
    Body::from_stream(futures_util::stream::unfold(
        (
            body.into_data_stream(),
            super::sse_framing::Lines::default(),
            String::new(),
            Repair::default(),
            false,
        ),
        |(mut stream, mut lines, mut block, mut repair, done)| async move {
            if done {
                return None;
            }
            loop {
                let bytes = match stream.next().await {
                    Some(Ok(bytes)) => bytes,
                    Some(Err(_)) => {
                        return Some((Err(invalid()), (stream, lines, block, repair, true)));
                    }
                    None => return None, // Downstream observer rejects missing terminal; do not flush held ends.
                };
                let result = (|| {
                    let mut output = String::new();
                    for line in lines.push(&bytes, LIMIT).map_err(|_| invalid())? {
                        if block.len().saturating_add(line.len()).saturating_add(1) > LIMIT {
                            return Err(invalid());
                        }
                        block.push_str(&line);
                        block.push('\n');
                        if line.is_empty() {
                            output.push_str(&repair.push(std::mem::take(&mut block))?);
                        }
                    }
                    Ok(output)
                })();
                match result {
                    Ok(output) if output.is_empty() => continue,
                    Ok(output) => {
                        return Some((
                            Ok(Bytes::from(output)),
                            (stream, lines, block, repair, false),
                        ));
                    }
                    Err(error) => return Some((Err(error), (stream, lines, block, repair, true))),
                }
            }
        },
    ))
}
