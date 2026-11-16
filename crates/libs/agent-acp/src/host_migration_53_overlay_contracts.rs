//! Native-backed mount selection and lease contract assertions.
use crate::{protocol::{VibeWindowBackend, Host}, mcp::SessionToolset};
use agent_skills::{session_sources::SessionSkillSources, Skill};
use acp_server::server::{BackendMountRequest, SessionBackend};
use acp_types::{SessionMountSelection, SessionSkillMount};

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

pub async fn acp_server_skill_overlay_accepts_native_directory<H: Host<Tools = SessionToolset, Skills = SessionSkillSources> + Default>() {
    let project = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("SKILL.md"), "---\nname: overlay_probe\ndescription: inspect overlay fixture\n---\nRead references/probe.txt on demand.\n").unwrap();
    std::fs::create_dir(source.path().join("references")).unwrap();
    std::fs::write(source.path().join("references/probe.txt"), "overlay attachment").unwrap();
    let backend = VibeWindowBackend::<H>::default();
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

pub async fn acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing<H: Host<Tools = SessionToolset, Skills = SessionSkillSources> + Default>() {
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::<H>::default();
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

#[cfg(unix)]
pub async fn acp_server_skill_overlay_permission_change_revokes_read_and_invocation<H: Host<Tools = SessionToolset, Skills = SessionSkillSources> + Default>(catalog: impl Fn(&std::path::Path, &SessionSkillSources) -> Result<Vec<Skill>, String>) {
    use std::os::unix::fs::PermissionsExt;
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::<H>::default();
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
        catalog(project.path(), &sources)
        .is_err()
    );
    std::fs::set_permissions(attachment, std::fs::Permissions::from_mode(0o600)).unwrap();
}

pub async fn acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears<H: Host<Tools = SessionToolset, Skills = SessionSkillSources> + Default>(catalog: impl Fn(&std::path::Path, &SessionSkillSources) -> Result<Vec<Skill>, String>) {
    let project = tempfile::tempdir().unwrap();
    let first_source = source();
    let second_source = source();
    std::fs::write(
        second_source.path().join("SKILL.md"),
        "---\nname: overlay_probe\ndescription: second session\n---\nSECOND_SESSION_BODY\n",
    )
    .unwrap();
    let backend = VibeWindowBackend::<H>::default();
    let first = backend
        .create_session_with_mounts(project.path().into(), request(first_source.path()))
        .await
        .unwrap();
    let second = backend
        .create_session_with_mounts(project.path().into(), request(second_source.path()))
        .await
        .unwrap();
    for (id, expected, absent) in [
        (&first.session.id, "OVERLAY_BODY_ON_DEMAND", "SECOND_SESSION_BODY"),
        (&second.session.id, "SECOND_SESSION_BODY", "OVERLAY_BODY_ON_DEMAND"),
    ] {
        let (sources, _) = backend.mounted_runtime(id);
        let skills =
            catalog(project.path(), &sources).unwrap();
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

use std::sync::Arc;
use tool_types::Tool;
use tool_context::{context::{ToolUseContext, scope_tool_use_context}, host::ContextHost};
pub async fn acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session<H: Host<Tools = SessionToolset, Skills = SessionSkillSources> + Default, C: ContextHost>(
    context: impl Fn(&str, &std::path::Path, SessionSkillSources, bool) -> Arc<ToolUseContext<C>>,
    skill: impl Fn(String, &std::path::Path) -> Arc<dyn Tool>,
) {
    use serde_json::json;
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
    let backend = VibeWindowBackend::<H>::default();
    let mounted = backend.create_session_with_mounts(project.path().into(), mounts).await.unwrap();
    let (sources, tools) = backend.mounted_runtime(&mounted.session.id);
    let context = context(&mounted.session.id, project.path(), sources, false);
    let skill = skill(mounted.session.id, project.path());
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

pub async fn acp_server_skill_overlay_retains_native_skill_permission_denial<H: Host<Tools = SessionToolset, Skills = SessionSkillSources> + Default, C: ContextHost>(
    context: impl Fn(&str, &std::path::Path, SessionSkillSources, bool) -> Arc<ToolUseContext<C>>,
    skill: impl Fn(String, &std::path::Path) -> Arc<dyn Tool>,
) {
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::<H>::default();
    let mounted = backend
        .create_session_with_mounts(project.path().into(), request(source.path()))
        .await
        .unwrap();
    let (sources, _) = backend.mounted_runtime(&mounted.session.id);
    let context = context(&mounted.session.id, project.path(), sources, true);
    let skill = skill(mounted.session.id, project.path());
    let result =
        scope_tool_use_context(context, skill.call(serde_json::json!({"skill":"overlay_probe"})))
            .await
            .unwrap();
    assert!(!result.is_success());
    assert!(result.error_text().unwrap().contains("Denied"));
}

pub async fn acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access<H: Host<Tools = SessionToolset, Skills = SessionSkillSources> + Default, C: ContextHost>(
    make_context: impl Fn(&str, &std::path::Path, Option<SessionSkillSources>) -> Arc<ToolUseContext<C>>,
    make_skill: impl Fn(String, &std::path::Path, &Arc<ToolUseContext<C>>, bool) -> Arc<dyn Tool>,
    make_reader: impl Fn(&std::path::Path) -> Arc<dyn Tool>,
    load_catalog: impl Fn(&std::path::Path, &SessionSkillSources) -> Result<Vec<Skill>, String>,
) {
    use serde_json::json;
    let project = tempfile::tempdir().unwrap();
    let source = source();
    let backend = VibeWindowBackend::<H>::default();
    let session = backend
        .create_session_with_mounts(project.path().into(), request(source.path()))
        .await
        .unwrap();
    let (sources, _) = backend.mounted_runtime(&session.session.id);
    let context = make_context(&session.session.id, project.path(), Some(sources.clone()));
    assert!(!context.full_access_enabled());
    for _ in 0..2 {
        let catalog =
            load_catalog(project.path(), &sources).unwrap();
        let mounted = catalog.iter().find(|s| s.name == "overlay_probe").unwrap();
        assert!(!format!("{mounted:?}").contains("OVERLAY_BODY_ON_DEMAND"));
        let tool = make_skill(session.session.id.clone(), project.path(), &context, true);
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
        let reader = make_reader(project.path());
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
        !load_catalog(project.path(), &other_sources)
            .unwrap()
            .iter()
            .any(|s| s.name == "overlay_probe")
    );
    let other_context = make_context(&isolated.id, project.path(), None);
    let result = scope_tool_use_context(
        other_context,
        make_reader(project.path())
            .execute(json!({"path":source.path().join("references/probe.txt")})),
    )
    .await
    .unwrap();
    assert!(!result.success, "other session cannot read mount attachments");
    std::fs::write(source.path().join("SKILL.disabled"), "").unwrap();
    assert!(load_catalog(project.path(), &sources).is_err());
    let tool = make_skill(session.session.id.clone(), project.path(), &context, false);
    let result =
        scope_tool_use_context(context, tool.call(json!({"skill":"overlay_probe"}))).await.unwrap();
    assert!(!result.is_success(), "disabled source cannot be invoked");
}
