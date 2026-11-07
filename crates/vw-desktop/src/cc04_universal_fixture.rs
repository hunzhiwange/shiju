use serde_json::{Value, json};
use std::collections::BTreeMap;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId, app_providers::universal_sync::UniversalSyncRequest,
};

#[derive(Default)]
pub(crate) struct Fixture {
    rejected_once: bool,
}

impl Fixture {
    pub(crate) fn handle(
        &mut self,
        method: &str,
        path: &str,
        payload: &[u8],
        basics: &mut BTreeMap<String, Value>,
    ) -> Option<(&'static str, Value)> {
        if method == "GET" && path == "/v1/universal-providers" {
            let rows = ["跨应用中文供应商", "备用中文供应商"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                    let notes = (1..=6)
                        .map(|n| format!("第 {n} 项验收说明：在隔离目录中投影已保存的统一记录，逐项核对 Claude Code、Codex 与 Gemini CLI 的目录结果和原生状态。"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    json!({
                        "id": format!("fixture-{index}"),
                        "config": {
                            "name": name,
                            "providerType": "custom",
                            "apps": {"claude": true, "codex": false, "gemini": true},
                            "baseUrl": "https://fixture-api.example.test",
                            "notes": notes
                        },
                        "configured": true
                    })
                })
                .collect::<Vec<_>>();
            return Some(("200 OK", json!(rows)));
        }
        let id = path.strip_prefix("/v1/universal-providers/")?.strip_suffix("/sync")?;
        assert_eq!(method, "POST");
        assert!(matches!(id, "fixture-0" | "fixture-1"));
        let request: UniversalSyncRequest = serde_json::from_slice(payload).unwrap();
        assert!(!request.apps.is_empty());
        if !self.rejected_once {
            self.rejected_once = true;
            return Some(("400 Bad Request", json!({"message": "fixture validation failure"})));
        }
        let results = request
            .apps
            .into_iter()
            .map(|app| {
                let (provider_id, catalog) = match app {
                    AppId::Claude => {
                        let provider_id = format!("universal-claude-{id}");
                        basics.insert(provider_id.clone(), json!({"name": "投影后的中文供应商"}));
                        (Some(provider_id), json!({"status": "updated"}))
                    }
                    AppId::Gemini => {
                        (Some(format!("universal-gemini-{id}")), json!({"status": "updated"}))
                    }
                    AppId::Codex => (None, json!({"status": "failed", "code": "app_disabled"})),
                    _ => panic!("unexpected application in universal window fixture"),
                };
                json!({
                    "appId": app,
                    "providerId": provider_id,
                    "catalog": catalog,
                    "live": "not_attempted"
                })
            })
            .collect::<Vec<_>>();
        Some(("200 OK", json!(results)))
    }
}
