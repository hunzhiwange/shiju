//! Loopback responses for the explicit endpoints window; no user configuration access.
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Fixture {
    rows: BTreeMap<String, Vec<Value>>,
    options: BTreeMap<String, Value>,
}
impl Fixture {
    pub fn handle(
        &mut self,
        method: &str,
        path: &str,
        id: &str,
        payload: &[u8],
    ) -> Option<(&'static str, Value)> {
        if path.ends_with("/endpoints") {
            let rows=self.rows.entry(id.into()).or_insert_with(||(0..8).map(|i|json!({"url":format!("https://fixture-{i}.example/v1"),"kind":"baseUrl","addedAt":42+i,"lastUsed":null})).collect());
            if method == "POST" {
                let body: Value = serde_json::from_slice(payload).unwrap();
                let url = body["url"].as_str().unwrap_or_default();
                if !url.starts_with("https://") || url.contains(' ') {
                    return Some(("422 Unprocessable Entity", Value::Null));
                }
                match body["action"].as_str() {
                    Some("add") => {
                        if !rows
                            .iter()
                            .any(|r| r["url"] == body["url"] && r["kind"] == body["kind"])
                        {
                            rows.insert(0,json!({"url":url,"kind":body["kind"],"addedAt":100,"lastUsed":null}));
                        }
                    }
                    Some("remove") => {
                        rows.retain(|r| r["url"] != body["url"] || r["kind"] != body["kind"])
                    }
                    Some("markUsed") => {
                        for row in rows
                            .iter_mut()
                            .filter(|r| r["url"] == body["url"] && r["kind"] == body["kind"])
                        {
                            row["lastUsed"] = json!(12345);
                        }
                    }
                    _ => return Some(("400 Bad Request", Value::Null)),
                }
            }
            return Some(("200 OK", json!(rows)));
        }
        if path.ends_with("/request-options") || path.ends_with("/request-options/read") {
            let body: Value = serde_json::from_slice(payload).unwrap();
            assert_eq!(body["targetHostId"], "fixture-host");
            let options = self.options.entry(id.into()).or_insert(json!({}));
            if method == "PUT" {
                let incoming = &body["options"];
                if incoming["localProxyRequestOverrides"]["headers"]
                    .as_object()
                    .is_some_and(|v| v.keys().any(|k| k.eq_ignore_ascii_case("authorization")))
                    || incoming["requestHeaders"]
                        .as_object()
                        .is_some_and(|v| v.keys().any(|k| k.contains(' ')))
                {
                    return Some(("400 Bad Request", Value::Null));
                }
                for (k, v) in incoming.as_object().unwrap() {
                    options[k] = v.clone();
                }
            }
            return Some((
                "200 OK",
                json!({"headersConfigured":options["requestHeaders"].as_object().is_some_and(|v|!v.is_empty()),"overridesConfigured":options["localProxyRequestOverrides"].as_object().is_some_and(|v|v.values().any(|v|v.as_object().is_some_and(|v|!v.is_empty()))),"application":"not_applied"}),
            ));
        }
        None
    }
}
