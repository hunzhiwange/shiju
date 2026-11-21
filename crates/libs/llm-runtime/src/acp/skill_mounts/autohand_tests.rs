use super::*;
use crate::acp::skill_mounts::MountedSkills;

fn profile() -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &Default::default(),
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "fixture".into(),
            model: "model".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_skill_coexistence_autohand_no_selection_has_no_skill_root() {
    let mut profile = profile();
    let workspace = tempfile::tempdir().unwrap();
    configure(&mut profile, workspace.path(), None::<&MountedSkills>).unwrap();
    assert_eq!(profile.config().env[MANAGED], "1");
    assert!(profile.config().env[ROOT].is_empty());
    assert!(!profile.directory().join("shiju-session-skills-autohand").exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_autohand_snapshots_only_the_verified_selection() {
    use std::{fs, os::unix::fs::PermissionsExt, path::Path};

    let workspace = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let source = external.path().join("selected");
    fs::create_dir_all(source.join("references")).unwrap();
    fs::create_dir(external.path().join("unselected")).unwrap();
    let body = "---\nname: selected\ndescription: fixture\n---\nRead the fixture.";
    fs::write(source.join("SKILL.md"), body).unwrap();
    fs::write(source.join("references/value.txt"), "attachment").unwrap();
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: source.to_str().unwrap().into(),
        }],
        workspace.path(),
        "autohand-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    let mut launch = profile();
    configure(&mut launch, workspace.path(), Some(&view)).unwrap();
    let snapshot = Path::new(&launch.config().env[ROOT]);
    assert!(snapshot.starts_with(launch.directory().canonicalize().unwrap()));
    assert!(!snapshot.join("unselected").exists());
    let definition = snapshot.join("selected/SKILL.md");
    assert_eq!(fs::read_to_string(&definition).unwrap(), body);
    assert_eq!(fs::symlink_metadata(&definition).unwrap().permissions().mode() & 0o777, 0o400);
    assert_eq!(
        fs::read_to_string(snapshot.join("selected/references/value.txt")).unwrap(),
        "attachment"
    );
    fs::write(source.join("SKILL.md"), "changed").unwrap();
    assert!(configure(&mut profile(), workspace.path(), Some(&view)).is_err());
}
