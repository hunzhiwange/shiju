use app_session_content::{
    cache::{Transcript, source_fingerprint},
    markdown::{MarkdownOptions, export_session_markdown},
    source::SourceRegistry,
};
use app_session_core::blocks::{ToolSource, tool_call_block, tool_result_block};
use serde_json::Value;
use vw_api_external_session::*;
use vw_api_transport::{ProjectId, ProjectScope};

#[test]
fn markdown_keeps_tool_previews_and_the_rest_of_the_turn_without_full_references() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let path = root.join("history.jsonl");
    std::fs::write(&path, "{}\n").unwrap();
    let identity = ExternalSessionIdentity {
        scope: ProjectScope::for_default_tenant(ProjectId::from("project")).unwrap(),
        target_host: "host".into(),
        instance_id: "instance".into(),
        provider: SessionProvider::Claude,
        session_id: ExternalSessionId("session".into()),
        source: SessionSource::File { relative_path: "history.jsonl".into() },
    };
    let mut registry = SourceRegistry::new(
        identity.scope.clone(),
        identity.target_host.clone(),
        identity.instance_id.clone(),
        identity.provider,
        vec![root],
    )
    .unwrap();
    registry
        .register(
            &SessionMeta {
                identity: identity.clone(),
                capabilities: SessionCapabilities {
                    read: true,
                    delete: false,
                    resume: false,
                    export_markdown: true,
                },
                title: None,
                summary: None,
                project_dir: None,
                created_at: None,
                last_active_at: None,
                source_path: None,
            },
            path,
        )
        .unwrap();
    let source = registry.resolve(&identity).unwrap();
    let transcript = Transcript::new(vec![
        SessionMessage { role: "user".into(), content: "question".into(), ..Default::default() },
        SessionMessage {
            role: "assistant".into(),
            blocks: vec![
                tool_call_block(
                    ToolSource::Generic,
                    "short-call",
                    "Example",
                    &Value::String("complete input\r\n".into()),
                    || None,
                ),
                tool_call_block(
                    ToolSource::Generic,
                    "call",
                    "Example",
                    &Value::String(format!("partial input {}", "x".repeat(500))),
                    || None,
                ),
                tool_result_block(
                    "call",
                    ToolStatus::Success,
                    &format!("partial output {}", "y".repeat(1500)),
                    || None,
                ),
            ],
            ..Default::default()
        },
        SessionMessage {
            role: "assistant".into(),
            content: "final reply".into(),
            ..Default::default()
        },
    ])
    .unwrap();
    let fingerprint = source_fingerprint(&source).unwrap();
    let markdown = export_session_markdown(
        &source,
        &transcript,
        &fingerprint,
        "Test session",
        None,
        &MarkdownOptions::default(),
    )
    .unwrap();

    for text in ["question", "complete input", "partial input", "partial output", "final reply"] {
        assert!(markdown.contains(text), "missing {text}");
    }
    assert_eq!(markdown.matches("工具内容仅保留预览，缺少全文。").count(), 2);
}
