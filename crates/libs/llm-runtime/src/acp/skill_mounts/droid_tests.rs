use super::*;
use std::fs;

#[derive(Clone)]
struct FixtureLease(PathBuf);
impl SkillLease for FixtureLease {
    fn fingerprint(&self) -> &str {
        "fixture"
    }
    fn root(&self) -> &Path {
        &self.0
    }
    fn validate(&self) -> Result<(), crate::Error> {
        Ok(())
    }
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_droid_publishes_only_selected_view() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let workspace = root.join("workspace");
    let native = workspace.join(".factory/skills");
    let home = root.join("private");
    let skill = root.join("selected");
    let view = root.join("view");
    for path in [&native, &home, &skill, &view] {
        fs::create_dir_all(path).unwrap();
    }
    let settings = native.parent().unwrap().join("settings.json");
    fs::write(&settings, r#"{"disabledSkills":["disabled"]}"#).unwrap();
    assert_eq!(publish(&home, &workspace, None::<&FixtureLease>).unwrap(), [native.clone()]);
    assert!(!home.join(".factory").exists());
    std::os::unix::fs::symlink(&skill, view.join("selected")).unwrap();
    let reads = publish(&home, &workspace, Some(&FixtureLease(view.clone()))).unwrap();
    assert_eq!(reads, [native, view.clone(), skill]);
    assert_eq!(fs::read_link(home.join(".factory/skills")).unwrap(), view);
    assert_eq!(fs::read_to_string(settings).unwrap(), r#"{"disabledSkills":["disabled"]}"#);
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_droid_rejects_linked_project_settings() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    let factory = workspace.join(".factory");
    fs::create_dir_all(&factory).unwrap();
    let foreign = tmp.path().join("settings.json");
    fs::write(&foreign, "{}").unwrap();
    std::os::unix::fs::symlink(&foreign, factory.join("settings.json")).unwrap();
    assert!(publish(tmp.path(), &workspace, None::<&FixtureLease>).is_err());
}
