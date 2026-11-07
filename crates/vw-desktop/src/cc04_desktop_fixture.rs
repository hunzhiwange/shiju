//! Loopback-only responses for the explicit Desktop native window case.
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub struct Fixture {
    forms: BTreeMap<String, Value>,
}
impl Fixture {
    pub fn new() -> Self {
        let routes=(0..12).map(|i|(format!("claude-sonnet-5-r{i}"),json!({"model":format!("model-{i}"),"labelOverride":format!("中文动态显示名 {i}"),"supports1m":false}))).collect::<serde_json::Map<_,_>>();
        Self {
            forms: BTreeMap::from([(
                "provider-0".into(),
                json!({"name":"中文供应商 0","baseUrl":"https://fixture-api.example.test","official":false,"directEditable":false,"mappingEditable":true,"mapping":{"mode":"proxy","routes":routes}}),
            )]),
        }
    }
    pub fn handle(
        &mut self,
        path: &str,
        id: &str,
        payload: &[u8],
        basics: &mut BTreeMap<String, Value>,
    ) -> Option<(&'static str, Value)> {
        if !(path.ends_with("/form/read")
            || path.ends_with("/model-mapping")
            || path.ends_with("/defaults")
            || path.ends_with("/import-claude")
            || path.ends_with("/config")
            || path.ends_with("/switch"))
        {
            return None;
        }
        let value: Value = serde_json::from_slice(payload).unwrap();
        assert_eq!(value["targetHostId"], "fixture-host");
        let mut status = "200 OK";
        let result = if path.ends_with("/form/read") {
            self.forms.get(id).cloned().unwrap_or_else(|| {
                status = "404 Not Found";
                Value::Null
            })
        } else if path.ends_with("/defaults") {
            json!([{"routeId":"claude-sonnet-5","envKey":"ANTHROPIC_DEFAULT_SONNET_MODEL","supports1m":true},{"routeId":"claude-opus-5","envKey":"ANTHROPIC_DEFAULT_OPUS_MODEL","supports1m":true},{"routeId":"claude-haiku-4-5","envKey":"ANTHROPIC_DEFAULT_HAIKU_MODEL","supports1m":true},{"routeId":"claude-fable-5","envKey":"ANTHROPIC_DEFAULT_FABLE_MODEL","supports1m":true}])
        } else if path.ends_with("/import-claude") {
            json!({"imported":0})
        } else if path.ends_with("/model-mapping") {
            let mapping = &value["mapping"];
            let valid = mapping["routes"].as_object().unwrap().iter().all(|(id, row)| {
                row["model"].as_str().is_some_and(|s| {
                    !s.is_empty()
                        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
                }) && (mapping["mode"] != "direct" || row["model"] == *id)
            });
            if valid {
                self.forms.get_mut(id).unwrap()["mapping"] = mapping.clone();
                json!({"catalogSaved":true,"live":"not_attempted"})
            } else {
                status = "422 Unprocessable Entity";
                Value::Null
            }
        } else if path.ends_with("/switch") {
            json!({"catalogSaved":false,"live":"applied"})
        } else {
            let profile = &value["profile"];
            let config = &profile["config"];
            if config["kind"] != "official"
                && (!config["baseUrl"].as_str().unwrap_or_default().starts_with("https://")
                    || config["authToken"].as_str().unwrap_or_default().is_empty())
            {
                status = "422 Unprocessable Entity";
                Value::Null
            } else {
                basics.insert(
                    id.into(),
                    json!({"name":profile["name"],"category":null,"notes":null,"websiteUrl":null}),
                );
                self.forms.insert(id.into(),json!({"name":profile["name"],"baseUrl":config["baseUrl"].as_str().unwrap_or_default(),"official":config["kind"]=="official","directEditable":true,"mappingEditable":config["kind"]!="official","mapping":{"mode":"direct","routes":{}}}));
                json!({"catalogSaved":true,"live":if value["activation"]=="catalog_only" {"not_attempted"}else{"applied"}})
            }
        };
        Some((status, result))
    }
}
