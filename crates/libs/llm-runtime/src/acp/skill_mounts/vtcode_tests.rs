use super::with_snapshot_policy;
use std::path::Path;

#[test]
fn acp_skill_coexistence_vtcode_adds_only_read_access_for_the_snapshot() {
    let base = "(deny file-write*)\n(deny process-fork process-exec)\n(deny network*)\n";
    let policy = with_snapshot_policy(base, Path::new("/private/session/selected")).unwrap();
    assert_eq!(
        policy,
        format!(
            "{base}\n(allow file-read-data (subpath \"/private/session/selected\"))\n(deny file-write* (subpath \"/private/session/selected\"))\n"
        )
    );
    assert!(with_snapshot_policy(base, Path::new("/private/session/selected\n")).is_err());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_vtcode_snapshots_only_the_selected_skill() {
    use super::super::MountedSkills;
    use super::publish_snapshot;
    use std::{fs, os::unix::fs::PermissionsExt};

    let workspace = tempfile::tempdir().unwrap();
    let selected = workspace.path().join("selected");
    let unselected = workspace.path().join("unselected");
    fs::create_dir(&selected).unwrap();
    fs::create_dir(&unselected).unwrap();
    let body = "---\nname: selected\ndescription: fixture\n---\nSelected.";
    fs::write(selected.join("SKILL.md"), body).unwrap();
    fs::write(unselected.join("SKILL.md"), body.replace("selected", "unselected")).unwrap();
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: selected.to_str().unwrap().into(),
        }],
        workspace.path(),
        "vtcode-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    let launch = tempfile::tempdir().unwrap();
    assert!(publish_snapshot(launch.path(), None::<&MountedSkills>).unwrap().is_none());
    assert_eq!(fs::read_dir(launch.path()).unwrap().count(), 0);
    let snapshot = publish_snapshot(launch.path(), Some(&view)).unwrap().unwrap();
    let entry = snapshot.join("selected/SKILL.md");
    assert_eq!(fs::read_to_string(&entry).unwrap(), body);
    assert_eq!(fs::metadata(entry).unwrap().permissions().mode() & 0o777, 0o400);
    assert_eq!(fs::read_dir(snapshot).unwrap().count(), 1);
}
