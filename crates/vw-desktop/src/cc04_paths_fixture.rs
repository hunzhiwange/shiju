use serde_json::{Value, json};

#[derive(Default)]
pub(super) struct Fixture {
    saved: Option<String>,
}

impl Fixture {
    pub(super) fn handle(
        &mut self,
        method: &str,
        path: &str,
        payload: &[u8],
    ) -> Option<(&'static str, Value)> {
        let query = path.strip_prefix("/v1/app-providers/claude/config/directory")?;
        match (method, query) {
            ("GET", "?targetHostId=fixture-host") => {}
            ("PUT", "") => {
                let request: Value = serde_json::from_slice(payload).unwrap();
                assert_eq!(request["targetHostId"], "fixture-host");
                match request["action"]["kind"].as_str() {
                    Some("set") => {
                        let directory = request["action"]["directory"].as_str().unwrap();
                        if !directory.starts_with("~/") {
                            return Some(("422 Unprocessable Entity", Value::Null));
                        }
                        self.saved = Some(directory.replacen('~', "/隔离主机/示例用户", 1));
                    }
                    Some("reset") => self.saved = None,
                    _ => panic!("unexpected directory operation"),
                }
            }
            _ => panic!("unexpected directory request {method} {path}"),
        }
        let directory = self.saved.as_deref().unwrap_or("/隔离主机/示例用户/.claude");
        let files = (0..22)
            .map(|i| {
                json!({
                    "path": format!("{directory}/示例配置-{i:02}.json"),
                    "state": if i == 0 { "present" } else { "absent" },
                })
            })
            .collect::<Vec<_>>();
        Some((
            "200 OK",
            json!({
                "directoryOverride": self.saved,
                "paths": {"appId": "claude", "directory": directory, "dataDirectory": directory, "files": files},
            }),
        ))
    }
}
