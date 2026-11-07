use serde_json::{Value, json};

#[derive(Default)]
pub(super) struct Fixture {
    conflicted: bool,
    cleaned: bool,
}

impl Fixture {
    pub(super) fn handle(
        &mut self,
        method: &str,
        path: &str,
        payload: &[u8],
    ) -> Option<(&'static str, Value)> {
        let action = path.strip_prefix("/v1/app-providers/claude/environment/")?;
        assert_eq!(method, "POST");
        let request: Value = serde_json::from_slice(payload).unwrap();
        assert_eq!(request["targetHostId"], "fixture-host");
        let backup_id = "ab".repeat(32);
        Some(match action {
            "diagnose" => {
                let mut entries = vec![
                    json!({
                        "variable":"ANTHROPIC_INHERITED",
                        "source":{"kind":"process_inherited"},
                        "cleanupAvailable":false
                    }),
                    json!({
                        "variable":"ANTHROPIC_SYSTEM",
                        "source":{"kind":"system_shell","file":"profile","line":4},
                        "cleanupAvailable":false
                    }),
                ];
                entries.extend((0..22).filter(|i| !self.cleaned || *i != 0).map(|i| {
                    json!({
                        "variable":format!("ANTHROPIC_FIXTURE_{i}"),
                        "source":{"kind":"user_shell","file":"zshrc","line":i + 5},
                        "cleanupAvailable":true,
                        "cleanup":{"diagnosticId":format!("receipt-{i}"),"baseDigest":"fixture-base"}
                    })
                }));
                ("200 OK", json!({"appId":"claude","entries":entries}))
            }
            "clean" => {
                assert_eq!(
                    request,
                    json!({
                        "targetHostId":"fixture-host",
                        "selections":[{"diagnosticId":"receipt-0","baseDigest":"fixture-base"}]
                    })
                );
                if self.conflicted {
                    self.cleaned = true;
                    ("200 OK", json!({"backupId":backup_id,"removedDefinitions":1}))
                } else {
                    self.conflicted = true;
                    ("409 Conflict", json!(null))
                }
            }
            "restore" => {
                assert_eq!(request, json!({"targetHostId":"fixture-host","backupId":backup_id}));
                assert!(self.cleaned);
                self.cleaned = false;
                ("200 OK", json!({"items":[{"file":"zshrc","status":"restored"}]}))
            }
            _ => return None,
        })
    }
}
