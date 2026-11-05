use super::VibeWindowBackend;
use vw_acp::server::{BackendMountRequest, SessionBackend};
use vw_acp::{SessionMountSelection, SessionSkillMount};

#[tokio::test]
async fn acp_server_skill_overlay_accepts_native_directory() {
    let project = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("SKILL.md"), "---\nname: overlay_probe\ndescription: inspect overlay fixture\n---\nRead references/probe.txt on demand.\n").unwrap();
    std::fs::create_dir(source.path().join("references")).unwrap();
    std::fs::write(source.path().join("references/probe.txt"), "overlay attachment").unwrap();
    let backend = VibeWindowBackend::default();
    let mounted = backend
        .create_session_with_mounts(
            project.path().to_path_buf(),
            BackendMountRequest {
                skills: Some(SessionMountSelection {
                    version: 1,
                    skills: vec![SessionSkillMount {
                        name: "overlay_probe".into(),
                        path: source.path().to_str().unwrap().into(),
                    }],
                }),
                mcp_servers: vec![],
            },
        )
        .await
        .expect("native session skill mount");
    assert_eq!(mounted.mounts.skills, ["overlay_probe"]);
}

fn source() -> tempfile::TempDir {
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("SKILL.md"), "---\nname: overlay_probe\ndescription: session overlay metadata\n---\nOVERLAY_BODY_ON_DEMAND\nRead references/probe.txt.\n").unwrap();
    std::fs::create_dir(source.path().join("references")).unwrap();
    std::fs::write(source.path().join("references/probe.txt"), "OVERLAY_ATTACHMENT").unwrap();
    source
}
fn request(source: &std::path::Path) -> BackendMountRequest {
    BackendMountRequest {
        skills: Some(SessionMountSelection {
            version: 1,
            skills: vec![SessionSkillMount {
                name: "overlay_probe".into(),
                path: source.to_str().unwrap().into(),
            }],
        }),
        mcp_servers: vec![],
    }
}

#[tokio::test]
async fn acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access() {
    use crate::tools::{
        Tool, ToolUseContext, file_read::FileReadTool, scope_tool_use_context, skill::SkillTool,
    };
    use serde_json::json;
    use std::sync::Arc;
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::default();
    let session = backend
        .create_session_with_mounts(project.path().into(), request(source.path()))
        .await
        .unwrap();
    let (sources, _) = backend.mounted_runtime(&session.session.id);
    let config = Arc::new(crate::config::Config::default());
    let security = Arc::new(crate::security::SecurityPolicy {
        workspace_dir: project.path().into(),
        workspace_only: true,
        ..Default::default()
    });
    let context = Arc::new(
        ToolUseContext::new(&session.session.id, Some(project.path().to_str().unwrap().into()))
            .with_session_skills(sources.clone())
            .with_explicitly_requested_skills(["overlay_probe"]),
    );
    assert!(!context.full_access_enabled());
    for _ in 0..2 {
        let catalog =
            crate::skills::load_session_skills(project.path(), &config, &sources, false).unwrap();
        let mounted = catalog.iter().find(|s| s.name == "overlay_probe").unwrap();
        assert!(!format!("{mounted:?}").contains("OVERLAY_BODY_ON_DEMAND"));
        let tool = SkillTool::new_with_runtime_config(
            security.clone(),
            session.session.id.clone(),
            false,
            project.path().into(),
            config.clone(),
        )
        .with_skill_directory_context(&context);
        assert!(tool.description().contains("overlay_probe"));
        assert!(!tool.description().contains("OVERLAY_BODY_ON_DEMAND"));
        let result =
            scope_tool_use_context(context.clone(), tool.call(json!({"skill":"overlay_probe"})))
                .await
                .unwrap();
        assert!(result.is_success(), "{:?}", result.error_text());
        assert!(
            result
                .extra_messages
                .iter()
                .any(|m| m.content.to_string().contains("OVERLAY_BODY_ON_DEMAND"))
        );
        let reader = FileReadTool::new(security.clone());
        let result = scope_tool_use_context(
            context.clone(),
            reader.execute(json!({"path":source.path().join("references/probe.txt")})),
        )
        .await
        .unwrap();
        assert!(result.success, "{:?}", result.error);
        assert!(result.output.contains("OVERLAY_ATTACHMENT"));
    }
    let isolated = backend.create_session(project.path().into()).await.unwrap();
    let (other_sources, _) = backend.mounted_runtime(&isolated.id);
    assert!(
        !crate::skills::load_session_skills(project.path(), &config, &other_sources, false)
            .unwrap()
            .iter()
            .any(|s| s.name == "overlay_probe")
    );
    let other_context =
        Arc::new(ToolUseContext::new(&isolated.id, Some(project.path().to_str().unwrap().into())));
    let result = scope_tool_use_context(
        other_context,
        FileReadTool::new(security)
            .execute(json!({"path":source.path().join("references/probe.txt")})),
    )
    .await
    .unwrap();
    assert!(!result.success, "other session cannot read mount attachments");
    std::fs::write(source.path().join("SKILL.disabled"), "").unwrap();
    assert!(crate::skills::load_session_skills(project.path(), &config, &sources, false).is_err());
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::security::SecurityPolicy::default()),
        session.session.id.clone(),
        false,
        project.path().into(),
        config,
    );
    let result =
        scope_tool_use_context(context, tool.call(json!({"skill":"overlay_probe"}))).await.unwrap();
    assert!(!result.is_success(), "disabled source cannot be invoked");
}

#[tokio::test]
async fn acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing() {
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::default();
    let session = backend
        .create_session_with_mounts(project.path().into(), request(source.path()))
        .await
        .unwrap();
    let mut replacement = request(source.path());
    replacement.mcp_servers.push(agent_client_protocol::McpServer::Stdio(
        agent_client_protocol::McpServerStdio::new("broken", "shiju-nonexistent-mcp-command"),
    ));
    assert!(
        backend
            .load_session_with_mounts(&session.session.id, project.path().into(), replacement)
            .await
            .is_err()
    );
    let (sources, _) = backend.mounted_runtime(&session.session.id);
    assert!(sources.validate().is_ok());
    let local = project.path().join("skills/overlay_probe");
    std::fs::create_dir_all(&local).unwrap();
    std::fs::copy(source.path().join("SKILL.md"), local.join("SKILL.md")).unwrap();
    assert!(
        backend
            .load_session_with_mounts(
                &session.session.id,
                project.path().into(),
                request(source.path())
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("conflicts")
    );
}

#[tokio::test]
async fn acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session() {
    use crate::tools::{Tool, ToolUseContext, scope_tool_use_context, skill::SkillTool};
    use serde_json::json;
    use std::sync::Arc;
    let project = tempfile::tempdir().unwrap();
    let source = source();
    // A real stdio MCP server: tools/call reads the attachment from disk.
    let script = project.path().join("attachment_server.py");
    std::fs::write(&script, r#"
import json, sys
from pathlib import Path
for line in sys.stdin:
    req = json.loads(line)
    method = req.get('method')
    if method == 'initialize':
        out = {'protocolVersion': req['params']['protocolVersion'], 'capabilities': {'tools': {}}, 'serverInfo': {'name': 'attachment', 'version': '1'}}
    elif method == 'tools/list':
        out = {'tools': [{'name': 'read_attachment', 'description': 'Read attachment', 'inputSchema': {'type': 'object'}}]}
    elif method == 'tools/call':
        out = {'content': [{'type': 'text', 'text': Path(sys.argv[1]).read_text()}], 'isError': False}
    else:
        out = {}
    if 'id' in req:
        print(json.dumps({'jsonrpc': '2.0', 'id': req['id'], 'result': out}), flush=True)
"#).unwrap();
    let mut mounts = request(source.path());
    mounts.mcp_servers.push(agent_client_protocol::McpServer::Stdio(
        agent_client_protocol::McpServerStdio::new("attachment", "python3").args(vec![
            script.to_str().unwrap().into(),
            source.path().join("references/probe.txt").to_str().unwrap().into(),
        ]),
    ));
    let backend = VibeWindowBackend::default();
    let mounted = backend.create_session_with_mounts(project.path().into(), mounts).await.unwrap();
    let (sources, tools) = backend.mounted_runtime(&mounted.session.id);
    let context = Arc::new(
        ToolUseContext::new(&mounted.session.id, Some(project.path().to_str().unwrap().into()))
            .with_session_skills(sources)
            .with_explicitly_requested_skills(["overlay_probe"]),
    );
    let skill = SkillTool::new_with_runtime_config(
        Arc::new(crate::security::SecurityPolicy::default()),
        mounted.session.id,
        false,
        project.path().into(),
        Arc::new(crate::config::Config::default()),
    );
    let result =
        scope_tool_use_context(context.clone(), skill.call(json!({"skill":"overlay_probe"})))
            .await
            .unwrap();
    assert!(result.is_success());
    assert_eq!(tools.len(), 1);
    let result = scope_tool_use_context(context, tools.tools()[0].call(json!({}))).await.unwrap();
    assert!(result.is_success(), "{:?}", result.error_text());
    assert!(result.model_text().contains("OVERLAY_ATTACHMENT"));
}

#[cfg(unix)]
#[tokio::test]
async fn acp_server_skill_overlay_permission_change_revokes_read_and_invocation() {
    use std::os::unix::fs::PermissionsExt;
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::default();
    let mounted = backend
        .create_session_with_mounts(project.path().into(), request(source.path()))
        .await
        .unwrap();
    let (sources, _) = backend.mounted_runtime(&mounted.session.id);
    let attachment = source.path().join("references/probe.txt");
    assert!(sources.allows_read(&attachment));
    std::fs::set_permissions(&attachment, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert!(sources.validate().is_err());
    assert!(!sources.allows_read(&attachment));
    assert!(
        crate::skills::load_session_skills(
            project.path(),
            &crate::config::Config::default(),
            &sources,
            true
        )
        .is_err()
    );
    std::fs::set_permissions(attachment, std::fs::Permissions::from_mode(0o600)).unwrap();
}

#[tokio::test]
async fn acp_server_skill_overlay_retains_native_skill_permission_denial() {
    use crate::tools::{Tool, ToolUseContext, scope_tool_use_context, skill::SkillTool};
    use std::sync::Arc;
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::default();
    let mounted = backend
        .create_session_with_mounts(project.path().into(), request(source.path()))
        .await
        .unwrap();
    let (sources, _) = backend.mounted_runtime(&mounted.session.id);
    let context = Arc::new(
        ToolUseContext::new(&mounted.session.id, Some(project.path().to_str().unwrap().into()))
            .with_session_skills(sources)
            .with_delegated_skill_rules(vec![vw_shared::permission::Rule {
                permission: "skill".into(),
                pattern: "overlay_probe".into(),
                action: vw_shared::permission::Action::Deny,
            }]),
    );
    let skill = SkillTool::new_with_runtime_config(
        Arc::new(crate::security::SecurityPolicy::default()),
        mounted.session.id,
        false,
        project.path().into(),
        Arc::new(crate::config::Config::default()),
    );
    let result =
        scope_tool_use_context(context, skill.call(serde_json::json!({"skill":"overlay_probe"})))
            .await
            .unwrap();
    assert!(!result.is_success());
    assert!(result.error_text().unwrap().contains("Denied"));
}

#[tokio::test]
async fn acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears() {
    let project = tempfile::tempdir().unwrap();
    let first_source = source();
    let second_source = source();
    std::fs::write(
        second_source.path().join("SKILL.md"),
        "---\nname: overlay_probe\ndescription: second session\n---\nSECOND_SESSION_BODY\n",
    )
    .unwrap();
    let backend = VibeWindowBackend::default();
    let first = backend
        .create_session_with_mounts(project.path().into(), request(first_source.path()))
        .await
        .unwrap();
    let second = backend
        .create_session_with_mounts(project.path().into(), request(second_source.path()))
        .await
        .unwrap();
    let config = crate::config::Config::default();
    for (id, expected, absent) in [
        (&first.session.id, "OVERLAY_BODY_ON_DEMAND", "SECOND_SESSION_BODY"),
        (&second.session.id, "SECOND_SESSION_BODY", "OVERLAY_BODY_ON_DEMAND"),
    ] {
        let (sources, _) = backend.mounted_runtime(id);
        let skills =
            crate::skills::load_session_skills(project.path(), &config, &sources, true).unwrap();
        let skill = skills.iter().find(|skill| skill.name == "overlay_probe").unwrap();
        let body = skill.prompts.join("\n");
        assert!(body.contains(expected));
        assert!(!body.contains(absent));
    }
    let restored = backend
        .load_session_with_mounts(
            &first.session.id,
            project.path().into(),
            BackendMountRequest { skills: None, mcp_servers: vec![] },
        )
        .await
        .unwrap();
    assert_eq!(restored.mounts.skills, ["overlay_probe"]);
    let cleared = backend
        .load_session_with_mounts(
            &first.session.id,
            project.path().into(),
            BackendMountRequest {
                skills: Some(SessionMountSelection { version: 1, skills: vec![] }),
                mcp_servers: vec![],
            },
        )
        .await
        .unwrap();
    assert!(cleared.mounts.skills.is_empty());
    let (sources, _) = backend.mounted_runtime(&first.session.id);
    assert!(!sources.allows_read(&first_source.path().join("references/probe.txt")));
}
