use anyhow::Result;
use vw_agent_apps::skills::repos::SkillRepo;
use vw_api_types::id::{ProjectScope, TenantScope};

use super::{
    agent_apps_skills_store::{
        NAMESPACE, SkillReposState, SkillReposStore, SkillReposVersionConflict, config_key,
    },
    settings_store::GatewaySettingsStore,
};
use crate::global_database::GlobalDatabase;

#[cfg(target_os = "macos")]
#[test]
fn cp12_local_install_persists_in_scope_and_rolls_back_failed_commit() -> Result<()> {
    use super::agent_apps_skills_store::SkillInstallsStore;

    let fixture = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(fixture.path())?;
    let project = scope("platform", "company", "project")?;
    let source = fixture.path().join("source");
    let ssot = fixture.path().join("ssot");
    std::fs::create_dir(&source)?;
    std::fs::create_dir(&ssot)?;
    std::fs::write(source.join("SKILL.md"), "---\nname: Fixture\n---\n# Instructions")?;
    let store = SkillInstallsStore::new(database.clone())?;
    let installed = store.install_local(&project, "host", 0, &ssot, &source, "fixture")?;
    assert_eq!(installed.version, 1);
    assert_eq!(installed.skills[0].directory, "fixture");
    assert_eq!(installed.skills[0].name, "Fixture");
    assert_eq!(SkillInstallsStore::new(database.clone())?.get(&project, "host", &ssot)?, installed);
    assert!(store.get(&scope("platform", "company", "other")?, "host", &ssot)?.skills.is_empty());
    assert!(store.get(&project, "other-host", &ssot)?.skills.is_empty());
    assert!(store.install_local(&project, "host", 0, &ssot, &source, "stale").is_err());
    assert!(!ssot.join("stale").exists());
    // Publication precedes settings commit; a real SQLite write failure must
    // restore the filesystem and preserve the previous persisted metadata.
    database.connect_sqlite()?.execute_batch(
        "CREATE TRIGGER reject_skill_install AFTER UPDATE ON gateway_settings
         WHEN NEW.namespace = 'agent_apps.skills.installed'
         BEGIN SELECT RAISE(FAIL, 'fixture write rejected'); END;",
    )?;
    assert!(store.install_local(&project, "host", 1, &ssot, &source, "rejected").is_err());
    assert!(!ssot.join("rejected").exists());
    assert_eq!(store.get(&project, "host", &ssot)?, installed);
    assert_eq!(std::fs::read_dir(&ssot)?.count(), 1);
    database.connect_sqlite()?.execute_batch("DROP TRIGGER reject_skill_install")?;
    let other_source = fixture.path().join("other-source");
    std::fs::create_dir(&other_source)?;
    std::fs::write(other_source.join("SKILL.md"), "# Other")?;
    assert!(store.install_local(&project, "host", 1, &ssot, &other_source, "fixture").is_err());
    assert_eq!(store.get(&project, "host", &ssot)?, installed);
    let other_root = fixture.path().join("other-root");
    std::fs::create_dir(&other_root)?;
    assert!(store.install_local(&project, "host", 1, &other_root, &source, "fixture").is_err());
    assert_eq!(std::fs::read_dir(other_root)?.count(), 0);
    Ok(())
}

fn scope(platform: &str, company: &str, project: &str) -> Result<ProjectScope> {
    Ok(ProjectScope::new(TenantScope::new(platform, company)?, project.into())?)
}

fn repository(branch: &str) -> SkillRepo {
    SkillRepo { owner: "owner".into(), name: "repo".into(), branch: branch.into(), enabled: true }
}

#[test]
fn cp12_repos_success() -> Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    let project = scope("platform", "company", "project")?;
    let store = SkillReposStore::new(database.clone())?;
    assert_eq!(store.get(&project)?, SkillReposState::default());
    let added = store.save(&project, 0, repository("feature/native"))?;
    assert_eq!(added.version, 1);
    assert_eq!(SkillReposStore::new(database.clone())?.get(&project)?, added);
    let disabled = store.set_enabled(&project, 1, "OWNER", "repo", false)?;
    assert!(!disabled.repos[0].enabled);
    assert_eq!(disabled.repos[0].branch, "feature/native");
    assert_eq!(store.set_enabled(&project, 2, "owner", "repo", false)?, disabled);
    assert!(
        store.set_enabled(&project, 1, "owner", "repo", true)
            .unwrap_err().is::<SkillReposVersionConflict>()
    );
    let enabled = store.set_enabled(&project, 2, "owner", "repo", true)?;
    assert!(enabled.repos[0].enabled);
    for other in [
        scope("other", "company", "project")?,
        scope("platform", "other", "project")?,
        scope("platform", "company", "other")?,
    ] {
        assert_eq!(store.get(&other)?, SkillReposState::default());
    }
    // Skill data in another key is not owned by repository removal or disabling.
    let settings = GatewaySettingsStore::new(database.clone())?;
    settings.upsert(project.tenant_scope(), "agent_apps.skills.entities", "fixture", "{}")?;
    let removed = store.remove(&project, 3, "owner", "repo")?;
    assert!(removed.repos.is_empty());
    assert_eq!(removed.version, 4);
    assert_eq!(SkillReposStore::new(database)?.get(&project)?, removed);
    assert_eq!(
        settings.get(project.tenant_scope(), "agent_apps.skills.entities", "fixture")?.as_deref(),
        Some("{}")
    );
    Ok(())
}

#[test]
fn cp12_repos_rejects_without_side_effects() -> Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    let project = scope("platform", "company", "project")?;
    let store = SkillReposStore::new(database.clone())?;
    let settings = GatewaySettingsStore::new(database.clone())?;
    let key = config_key(&project)?;
    assert!(store.save(&project, 0, repository("../bad")).is_err());
    assert_eq!(settings.get(project.tenant_scope(), NAMESPACE, &key)?, None);
    let added = store.save(&project, 0, repository("main"))?;
    let before = settings.get(project.tenant_scope(), NAMESPACE, &key)?;
    assert!(store.save(&project, 1, repository("main#fragment")).is_err());
    assert!(store.set_enabled(&project, 1, "missing", "repo", false).is_err());
    assert_eq!(settings.get(project.tenant_scope(), NAMESPACE, &key)?, before);
    let connection = database.connect_sqlite()?;
    connection.execute_batch(
        "CREATE TRIGGER reject_skill_repos AFTER UPDATE ON gateway_settings
         WHEN NEW.namespace = 'agent_apps.skills.repos'
         BEGIN SELECT RAISE(FAIL, 'fixture write rejected'); END;",
    )?;
    assert!(store.set_enabled(&project, 1, "owner", "repo", false).is_err());
    assert!(store.remove(&project, 1, "owner", "repo").is_err());
    assert!(store.save(&project, 1, repository("release/new")).is_err());
    assert_eq!(settings.get(project.tenant_scope(), NAMESPACE, &key)?, before);
    assert_eq!(store.get(&project)?, added);
    connection.execute_batch("DROP TRIGGER reject_skill_repos")?;
    for corrupt in [
        "not json",
        r#"{"schema_version":2,"version":1,"repos":[]}"#,
        r#"{"schema_version":1,"version":1,"repos":[{"owner":"o","name":"r","branch":"../bad","enabled":true}]}"#,
    ] {
        settings.upsert(project.tenant_scope(), NAMESPACE, &key, corrupt)?;
        assert!(store.get(&project).is_err());
        assert!(store.save(&project, 1, repository("main")).is_err());
        assert_eq!(
            settings.get(project.tenant_scope(), NAMESPACE, &key)?.as_deref(),
            Some(corrupt)
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn cp12_zip_install_reports_partial_results_and_preserves_existing() -> Result<()> {
    use std::io::Write;
    use super::agent_apps_skills_store::SkillInstallsStore;
    let fixture = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(fixture.path())?;
    let project = scope("platform", "company", "zip-project")?;
    let ssot = fixture.path().join("ssot");
    std::fs::create_dir(&ssot)?;
    std::fs::create_dir(ssot.join("taken"))?;
    std::fs::write(ssot.join("taken/keep"), "external")?;
    let path = fixture.path().join("skills.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&path)?);
    for (name, body) in [
        ("alpha/SKILL.md", "---\nname: Alpha\n---\nInstructions"),
        ("broken/SKILL.md", "---\nname: [broken\n---\n"),
        ("taken/SKILL.md", "# Taken"),
        ("zeta/SKILL.md", "# Zeta"),
    ] {
        writer.start_file(name, zip::write::SimpleFileOptions::default())?;
        writer.write_all(body.as_bytes())?;
    }
    writer.finish()?;
    let store = SkillInstallsStore::new(database)?;
    let result = store.install_from_zip(&project, "host", &ssot, &path)?;
    assert_eq!(result.installed.len(), 2);
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(result.skipped[0].existing_id, "taken");
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].directory, "broken");
    assert!(!ssot.join("broken").exists());
    assert_eq!(std::fs::read_to_string(ssot.join("taken/keep"))?, "external");
    let state = store.get(&project, "host", &ssot)?;
    assert_eq!(state.version, 2);
    assert!(state.skills.iter().all(|skill| skill.source == path));
    let repeated = store.install_from_zip(&project, "host", &ssot, &path)?;
    assert!(repeated.installed.is_empty());
    assert_eq!(repeated.skipped.len(), 3);
    assert_eq!(repeated.skipped[0].name, "Alpha");
    assert_eq!(store.get(&project, "host", &ssot)?, state);
    Ok(())
}
