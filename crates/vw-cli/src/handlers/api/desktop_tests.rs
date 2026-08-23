//! `api desktop` 专用路由与宿主动作保护测试。

use std::sync::mpsc::TryRecvError;

use clap::Parser;
use serde_json::{Value, json};

use super::session_tests::{assert_request, run_command, spawn_server};
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_desktop_commands_cover_preferences_skills_and_guarded_host_actions() {
    let (port, requests) = spawn_server(vec![Value::Null; 16]);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str(), "--directory", "/repo path"];
    let pref = br#"{"compact":true}"#;
    let content = br#"{"content":"do not execute"}"#;
    let project = br#"{"model":"gpt","auto_model":true,"acp_agent":"codex"}"#;
    let guarded = [
        (&["desktop", "preferences-patch", "--input", "-"][..], pref.as_slice()),
        (&["desktop", "tool-put", "sql/notes", "--input", "-"][..], content.as_slice()),
        (&["desktop", "skill-create"][..], b"".as_slice()),
        (&["desktop", "skill-install", "rust"][..], b"".as_slice()),
        (&["desktop", "skill-enable", "rust", "--enabled", "true"][..], b"".as_slice()),
        (&["desktop", "skill-delete", "rust"][..], b"".as_slice()),
        (&["desktop", "open", "/repo/file", "finder"][..], b"".as_slice()),
        (&["desktop", "reveal", "/repo/file"][..], b"".as_slice()),
        (&["desktop", "service", "status"][..], b"".as_slice()),
        (&["desktop", "project-preferences-put", "--input", "-"][..], project.as_slice()),
    ];
    for (command, input) in guarded {
        let error = run_command(&common, command, input).await.expect_err("host mutation requires --yes");
        assert!(error.contains("--yes"));
        assert!(error.contains(&format!("http://127.0.0.1:{port}")));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }
    assert!(Cli::try_parse_from(["shiju", "api", "desktop", "service", "shell"]).is_err());

    let reads = [
        (&["desktop", "preferences-get"][..], "/v1/desktop/preferences"),
        (&["desktop", "tool-get", "sql/notes"][..], "/v1/desktop/tool-content/sql%2Fnotes"),
        (&["desktop", "skills"][..], "/v1/desktop/skills?project_path=%2Frepo+path"),
        (&["desktop", "skill-detail", "rust"][..], "/v1/desktop/skills/detail?skill_id=rust&project_path=%2Frepo+path"),
        (&["desktop", "external-apps"][..], "/v1/desktop/external-apps"),
        (&["desktop", "project-preferences-get"][..], "/v1/desktop/project-preferences?project_path=%2Frepo+path"),
    ];
    for (command, path) in reads {
        let _ = run_command(&common, command, b"").await;
        assert_request(&requests, "GET", path, Value::Null);
    }

    let yes = ["--host", "127.0.0.1", "--port", port.as_str(), "--directory", "/repo path", "--yes"];
    let writes = [
        (&["desktop", "preferences-patch", "--input", "-"][..], pref.as_slice(), "PATCH", "/v1/desktop/preferences", json!({"compact":true})),
        (&["desktop", "tool-put", "sql/notes", "--input", "-"][..], content.as_slice(), "PUT", "/v1/desktop/tool-content/sql%2Fnotes", json!({"content":"do not execute"})),
        (&["desktop", "skill-create"][..], b"".as_slice(), "POST", "/v1/desktop/skills/create", json!({"project_path":"/repo path"})),
        (&["desktop", "skill-install", "rust"][..], b"".as_slice(), "POST", "/v1/desktop/skills/install-built-in", json!({"project_path":"/repo path","skill_id":"rust"})),
        (&["desktop", "skill-enable", "rust", "--enabled", "true"][..], b"".as_slice(), "POST", "/v1/desktop/skills/set-enabled", json!({"project_path":"/repo path","skill_id":"rust","enabled":true})),
        (&["desktop", "skill-delete", "rust"][..], b"".as_slice(), "POST", "/v1/desktop/skills/delete", json!({"project_path":"/repo path","skill_id":"rust"})),
        (&["desktop", "open", "/repo/file", "finder"][..], b"".as_slice(), "POST", "/v1/desktop/external-apps/open", json!({"path":"/repo/file","target":"finder"})),
        (&["desktop", "reveal", "/repo/file"][..], b"".as_slice(), "POST", "/v1/desktop/external-path/reveal", json!({"path":"/repo/file"})),
        (&["desktop", "service", "status"][..], b"".as_slice(), "POST", "/v1/desktop/service/status", json!({})),
        (&["desktop", "project-preferences-put", "--input", "-"][..], project.as_slice(), "PUT", "/v1/desktop/project-preferences?project_path=%2Frepo+path", json!({"model":"gpt","auto_model":true,"acp_agent":"codex"})),
    ];
    for (command, input, method, path, body) in writes {
        let _ = run_command(&yes, command, input).await;
        assert_request(&requests, method, path, body);
    }
}
