use serde_json::{Value, json};
use vw_gateway_client::vw_api_types::app_providers::snippets::{
    ConfigSnippet, SnippetExtract, SnippetSource, SnippetWrite,
};

pub(crate) struct Fixture {
    saved: ConfigSnippet,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self { saved: ConfigSnippet { content: Some(r#"{"theme":"初始中文片段"}"#.into()) } }
    }

    pub(crate) fn handle(
        &mut self,
        method: &str,
        path: &str,
        payload: &[u8],
    ) -> Option<(&'static str, Value)> {
        let suffix = path.strip_prefix("/v1/app-providers/claude/config/snippet")?;
        let snippet = match (method, suffix) {
            ("GET", "?targetHostId=fixture-host") => self.saved.clone(),
            ("POST", "/extract") => {
                let request: SnippetExtract = serde_json::from_slice(payload).unwrap();
                assert_eq!(request.target_host_id, "fixture-host");
                let SnippetSource::Saved { provider_id } = request.source else {
                    panic!("expected saved provider source")
                };
                let options =
                    (1..=22).map(|index| format!("第 {index} 项中文动态设置")).collect::<Vec<_>>();
                ConfigSnippet {
                    content: Some(
                        serde_json::to_string_pretty(&json!({
                            "theme": format!("提取自 {provider_id}"),
                            "options": options,
                        }))
                        .unwrap(),
                    ),
                }
            }
            ("POST", "/preview") | ("PUT", "") => {
                let request: SnippetWrite = serde_json::from_slice(payload).unwrap();
                assert_eq!(request.target_host_id, "fixture-host");
                let content = if request.content.trim().is_empty() {
                    None
                } else {
                    let Ok(Value::Object(fields)) = serde_json::from_str(&request.content) else {
                        return Some(("400 Bad Request", json!({"message": "invalid snippet"})));
                    };
                    Some(serde_json::to_string_pretty(&fields).unwrap())
                };
                let snippet = ConfigSnippet { content };
                if method == "PUT" {
                    self.saved = snippet.clone();
                }
                snippet
            }
            _ => panic!("unexpected snippet fixture request"),
        };
        Some(("200 OK", json!(snippet)))
    }
}
