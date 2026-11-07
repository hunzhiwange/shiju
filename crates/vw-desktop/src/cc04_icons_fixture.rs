//! Loopback responses for the explicit native icons window; no production data.
use serde_json::{Value, json};
use std::collections::BTreeMap;

const LIBRARY: &str = "/v1/agent-apps/custom-icons";
const IMAGE: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
const UPLOAD: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";

pub struct Fixture {
    rows: Vec<Value>,
    selections: BTreeMap<String, Value>,
}

fn icon(id: &str, name: &str, data: &str) -> Value {
    json!({"id":id,"name":name,"data":data,"createdAt":1})
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            rows: (1..=12)
                .map(|i| icon(&format!("icon-{i}"), &format!("中文图标 {i} · 动态素材"), IMAGE))
                .collect(),
            selections: BTreeMap::new(),
        }
    }

    pub fn handle(
        &mut self,
        method: &str,
        path: &str,
        id: &str,
        payload: &[u8],
    ) -> Option<(&'static str, Value)> {
        const MISSING: &str = "404 Not Found";
        let body = || serde_json::from_slice::<Value>(payload).unwrap();
        if path == LIBRARY {
            return match method {
                "GET" => Some(("200 OK", json!(self.rows))),
                "POST" => {
                    let request = body();
                    if request["data"] != UPLOAD
                        || request["name"].as_str().is_none_or(|s| s.trim().is_empty())
                    {
                        return Some((
                            "400 Bad Request",
                            json!({"code":"invalid_input","message":"Invalid icon input"}),
                        ));
                    }
                    if let Some(row) = self.rows.iter().find(|row| row["data"] == UPLOAD) {
                        return Some(("200 OK", json!({"icon":row,"existed":true})));
                    }
                    let row = icon("upload", request["name"].as_str().unwrap(), UPLOAD);
                    self.rows.insert(0, row.clone());
                    Some(("200 OK", json!({"icon":row,"existed":false})))
                }
                _ => None,
            };
        }
        if let Some(id) = path.strip_prefix("/v1/agent-apps/custom-icons/") {
            return match method {
                "PATCH" => match self.rows.iter_mut().find(|row| row["id"] == id) {
                    Some(row) => {
                        row["name"] = body()["name"].clone();
                        Some(("200 OK", row.clone()))
                    }
                    None => Some((MISSING, json!({"message":"Icon not found"}))),
                },
                "DELETE" => {
                    self.rows.retain(|row| row["id"] != id);
                    Some(("204 No Content", Value::Null))
                }
                _ => None,
            };
        }
        if !path.starts_with("/v1/app-providers/claude/") || !path.ends_with("/icon") {
            return None;
        }
        match method {
            "GET" => Some(("200 OK", json!({"data":self.selections.get(id)}))),
            "PUT" => {
                let request = body();
                match request["action"].as_str() {
                    Some("clear") => {
                        self.selections.remove(id);
                    }
                    Some("select") => {
                        let Some(row) = self.rows.iter().find(|row| row["id"] == request["iconId"])
                        else {
                            return Some((MISSING, json!({"message":"Icon not found"})));
                        };
                        self.selections.insert(id.into(), row["data"].clone());
                    }
                    _ => return Some(("400 Bad Request", json!({"message":"Invalid icon input"}))),
                }
                Some(("200 OK", json!({"data":self.selections.get(id)})))
            }
            _ => None,
        }
    }
}
