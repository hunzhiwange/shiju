//! Offline acceptance against the installed, hash-verified Claude adapter.

use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{
    AcpAgentConfig, AcpClient, AcpMessageDirection, FullAccessSupport,
    NonInteractivePermissionPolicy, PermissionMode, PromptRequest, SessionStrategy,
};
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

#[tokio::test]
#[ignore = "requires Python 3 and VW_ACP_CLAUDE_ACCEPTANCE_ENTRY pointing to fixed Claude ACP 0.26.0"]
async fn acp_desktop_approval_policy_real_claude_falls_back() {
    let command = std::env::var("VW_ACP_CLAUDE_ACCEPTANCE_ENTRY")
        .expect("set VW_ACP_CLAUDE_ACCEPTANCE_ENTRY to the fixed Claude npm entry");
    assert!(
        super::verified_entry(&command, &[]),
        "Claude acceptance requires the reviewed runtime"
    );
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let workspace = root_path.join("workspace");
    let config = root_path.join("config");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&config).unwrap();
    std::fs::write(workspace.join("keep.txt"), "keep").unwrap();
    let settings = config.join("settings.json");
    std::fs::write(
        &settings,
        json!({"permissions": {"defaultMode": "acceptEdits", "deny": ["Edit(./organization.txt)"]}})
            .to_string(),
    )
    .unwrap();

    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/acp_claude_approval_fixture.py");
    let mut fixture = Command::new("python3")
        .arg(script)
        .arg(&workspace)
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut fixture_stdout = BufReader::new(fixture.stdout.take().unwrap()).lines();
    let endpoint = timeout(Duration::from_secs(10), fixture_stdout.next_line())
        .await
        .expect("loopback fixture should start")
        .unwrap()
        .unwrap();

    let messages = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&messages);
    let client = AcpClient::new(
        "claude",
        AcpAgentConfig {
            command,
            args: Vec::new(),
            env: [
                ("CLAUDE_CONFIG_DIR", config.display().to_string()),
                ("ANTHROPIC_BASE_URL", endpoint),
                ("ANTHROPIC_API_KEY", "shiju-offline-fixture".into()),
                ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1".into()),
                ("VW_ACP_LAUNCH_REFERENCE", "claude-approval-fixture".into()),
                ("VW_ACP_CLAUDE_SETTINGS", settings.display().to_string()),
            ]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
        },
    )
    .with_desktop_workspace(Some(workspace.clone()))
    .with_permission_mode(PermissionMode::ApproveReads)
    .with_non_interactive_permissions(Some(NonInteractivePermissionPolicy::Deny))
    .with_acp_message_callback(Some(Arc::new(move |direction, message| {
        let wire = serde_json::to_value(message).unwrap();
        if wire.get("method").is_some() || wire["result"].get("configOptions").is_some() {
            observed.lock().unwrap().push((direction, wire));
        }
    })));

    let run = timeout(Duration::from_secs(60), async {
        let initial = client.create_session(&workspace).await?;
        let result = client
            .run_prompt(
                PromptRequest::new(workspace.clone(), "Run the deterministic approval fixture")
                    .with_session_strategy(SessionStrategy::ResumeOrLoad(initial.session_id)),
                &mut |_| {},
            )
            .await?;
        let version = client.agent_version();
        let info = client.session_capabilities(&result.session_id);
        let policy = client.session_access_policy(&result.session_id)?;
        Ok::<_, crate::AcpError>((result, version, info, policy))
    })
    .await;
    let close = client.close().await;
    fixture.kill().await.unwrap();
    close.unwrap();

    let (result, version, info, policy) =
        run.expect("Claude approval acceptance timed out").unwrap();
    assert_eq!(version.as_deref(), Some("0.26.0"));
    assert!(result.deltas.concat().contains("FALLBACK_FIXTURE_DONE"));
    assert!(mode_is(info.unwrap().config_options.as_deref(), "default"));
    assert_eq!(policy.permission_mode, PermissionMode::ApproveReads);
    assert_eq!(policy.support, FullAccessSupport::Unavailable);
    assert!(!policy.full_access_requested);

    let messages = messages.lock().unwrap();
    let new_id = messages
        .iter()
        .find(|(direction, wire)| {
            *direction == AcpMessageDirection::Outbound && wire["method"] == "session/new"
        })
        .map(|(_, wire)| &wire["id"])
        .unwrap();
    assert!(
        messages.iter().any(|(direction, wire)| {
            *direction == AcpMessageDirection::Inbound
                && &wire["id"] == new_id
                && wire["result"]["configOptions"].as_array().is_some_and(|options| {
                    options.iter().any(|option| {
                        option["id"] == "mode" && option["currentValue"] == "acceptEdits"
                    })
                })
        }),
        "native startup should advertise acceptEdits before the host demotes it"
    );
    assert!(messages.iter().any(|(direction, wire)| {
        *direction == AcpMessageDirection::Outbound
            && wire["method"] == "session/set_config_option"
            && wire["params"]["configId"] == "mode"
            && wire["params"]["value"] == "default"
    }));
    assert!(!messages.iter().any(|(_, wire)| wire["method"] == "fs/write_text_file"));
    let permission_inputs: Vec<&Value> = messages
        .iter()
        .filter(|(_, wire)| wire["method"] == "session/request_permission")
        .map(|(_, wire)| &wire["params"]["toolCall"]["rawInput"])
        .collect();
    assert!(
        permission_inputs
            .iter()
            .any(|input| input["file_path"] == json!(workspace.join("native.txt")))
    );
    assert!(
        !permission_inputs
            .iter()
            .any(|input| input["file_path"] == json!(workspace.join("organization.txt")))
    );
    for command in ["touch command.txt", "rm keep.txt"] {
        assert!(permission_inputs.iter().any(|input| input["command"] == command), "{command}");
    }
    assert!(client.permission_stats().denied >= 3);
    for path in ["native.txt", "organization.txt", ".protected", "command.txt", "../outside.txt"] {
        assert!(!workspace.join(path).exists(), "unexpected native side effect at {path}");
    }
    assert_eq!(std::fs::read_to_string(workspace.join("keep.txt")).unwrap(), "keep");
}

fn mode_is(options: Option<&[SessionConfigOption]>, expected: &str) -> bool {
    options.into_iter().flatten().any(|option| {
        option.id.0.as_ref() == "mode"
            && matches!(&option.kind, SessionConfigKind::Select(select) if select.current_value.0.as_ref() == expected)
    })
}
