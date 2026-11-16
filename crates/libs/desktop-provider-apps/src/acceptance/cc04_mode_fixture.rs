use serde_json::{Value, json};

#[derive(Default)]
pub(super) struct Fixture {
    rejected: bool,
    direct: bool,
}

impl Fixture {
    pub(super) fn handle(
        &mut self,
        method: &str,
        path: &str,
        payload: &[u8],
    ) -> Option<(&'static str, Value)> {
        let action = path.strip_prefix("/v1/app-providers/claude/mode/")?;
        Some(match (method, action) {
            ("GET", "state?targetHostId=fixture-host&mode=direct") => {
                let members = (0..26).map(|i| format!("provider-{i}")).collect::<Vec<_>>();
                (
                    "200 OK",
                    json!({
                        "mode": if self.direct { "direct" } else { "proxy" },
                        "directProvider": "provider-0",
                        "inUseProvider": if self.direct { "provider-0" } else { "provider-1" },
                        "proxyRoute": "provider-1",
                        "attached": !self.direct,
                        "hasContract": true,
                        "stack": {"enabled":true,"active":!self.direct,"members":members},
                        "decision": null,
                        "routeRuntime": "unavailable"
                    }),
                )
            }
            ("POST", "apply") => {
                let request: Value = serde_json::from_slice(payload).unwrap();
                assert_eq!(
                    request,
                    json!({"targetHostId":"fixture-host","action":{"kind":"direct"}})
                );
                assert!(!self.direct, "不能在已恢复后再次提交");
                if self.rejected {
                    self.direct = true;
                    (
                        "200 OK",
                        json!({
                            "configurationChanged":true,"attached":false,"targetPublication":"unavailable"
                        }),
                    )
                } else {
                    self.rejected = true;
                    ("422 Unprocessable Entity", Value::Null)
                }
            }
            _ => panic!("unexpected provider mode request {method} {action}"),
        })
    }
}
