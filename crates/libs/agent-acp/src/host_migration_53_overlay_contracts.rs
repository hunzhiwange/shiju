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
