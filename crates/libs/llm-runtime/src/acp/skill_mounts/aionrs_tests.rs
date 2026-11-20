use super::*;
use std::{fs, os::unix::fs::symlink, path::PathBuf};

#[test]
fn acp_skill_coexistence_aionrs_projects_only_the_selected_workspace_directory() {
    let workspace = tempfile::tempdir().unwrap();
    let launch = tempfile::tempdir().unwrap();
    let view = tempfile::tempdir().unwrap();
    let selected = workspace.path().canonicalize().unwrap().join("selected");
    fs::create_dir(&selected).unwrap();
    fs::create_dir(workspace.path().join("other")).unwrap();
    symlink(&selected, view.path().join("selected")).unwrap();
    let lease = TestLease(view.path().to_path_buf());

    publish(launch.path(), workspace.path(), None::<&TestLease>).unwrap();
    assert!(!launch.path().join(".aionrs").exists());
    publish(launch.path(), workspace.path(), Some(&lease)).unwrap();
    let skills = launch.path().join(".aionrs/skills");
    assert_eq!(fs::read_dir(&skills).unwrap().count(), 1);
    assert_eq!(fs::read_link(skills.join("selected")).unwrap(), selected);

    symlink(launch.path(), view.path().join("external")).unwrap();
    let another_launch = tempfile::tempdir().unwrap();
    assert!(publish(another_launch.path(), workspace.path(), Some(&lease)).is_err());
    assert!(!another_launch.path().join(".aionrs").exists());
}

#[derive(Clone)]
struct TestLease(PathBuf);

impl SkillLease for TestLease {
    fn fingerprint(&self) -> &str {
        "aionrs-test"
    }

    fn root(&self) -> &Path {
        &self.0
    }

    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
