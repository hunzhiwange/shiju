//! Narrow attached skills to the current committed host session revision.
use super::MountedSkills;
use crate::session::runtime::SessionScope;
use acp_types::SessionSkillMount;
use std::path::Path;

pub(in crate::session::llm::acp) async fn prepare_reviewed_revision(
    defaults: &mut Vec<SessionSkillMount>,
    cwd: &Path,
    session: &str,
    scoped_session: &str,
    security: &config_skills::SkillSecurityConfig,
) -> anyhow::Result<Option<MountedSkills>> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let Some(project) = crate::project::instance::project() else {
        anyhow::bail!("Reviewed ACP session skills require a project");
    };
    let scope = SessionScope::new(tenant, cwd)?;
    let owner = agent_database::run_sync_database(|| {
        session_store::ui_store::load_agent_session_strict(scope.tenant(), &project.id, session)
    })?;
    let read = agent_acp::mount_store::reader(&scope, session).await;
    let committed = read().map_err(|error| anyhow::anyhow!("{error}"))?;
    if let Some(owner) = owner {
        anyhow::ensure!(
            owner.id == session
                && owner.project_id == project.id
                && Path::new(&owner.directory).canonicalize()? == scope.directory(),
            "ACP skill mount belongs to another session or directory"
        );
    } else {
        anyhow::ensure!(committed.is_none(), "Cannot establish ACP skill mount ownership");
    }
    let mut committed_view = None;
    if let Some(record) = committed.as_ref().filter(|record| record.revision.0 > 0) {
        if let Some(expected) = record.fingerprint.as_deref() {
            let view = agent_acp::mounts::prepare_skill_view(
                &scope,
                session,
                &record.skills,
                record.revision,
                security,
            )?;
            anyhow::ensure!(view.fingerprint() == expected, "Committed ACP skill view changed");
            committed_view = Some(view);
        }
        // The session revision can remove bound profile choices. New grants still
        // require the original profile's skill permissions and admission checks.
        defaults.retain(|selected| {
            record.skills.iter().any(|mount| {
                mount.name == selected.name && same_directory(&mount.path, &selected.path)
            })
        });
    }

    let view = MountedSkills::prepare(defaults, cwd, scoped_session, security, scope.tenant())
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    if let Some(view) = committed_view {
        view.validate()?;
    }
    let expected = committed.map(|record| (record.revision, record.fingerprint, record.skills));
    let current = move || {
        read().is_ok_and(|record| {
            record.map(|record| (record.revision, record.fingerprint, record.skills)) == expected
        })
    };
    anyhow::ensure!(current(), "ACP skill mount revision changed during preparation");
    Ok(view.map(|view| view.with_current(current)))
}

fn same_directory(left: &str, right: &str) -> bool {
    Path::new(left)
        .canonicalize()
        .is_ok_and(|left| Path::new(right).canonicalize().is_ok_and(|right| left == right))
}
