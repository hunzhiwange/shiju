use super::*;
use crate::acp::{host::SkillLease, skill_mounts::MountedSkills};

#[test]
fn acp_skill_coexistence_auggie_keeps_empty_selection_empty() {
    assert_eq!(selection(None::<&MountedSkills>).unwrap(), (String::new(), Vec::new()));
    assert_eq!(
        super::super::validate("auggie", true).is_ok(),
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    );
}

#[test]
fn acp_skill_coexistence_auggie_projects_only_selected_roots() {
    let workspace = tempfile::tempdir().unwrap();
    for name in ["native", "selected", "unselected"] {
        let path = workspace.path().join(name);
        fs::create_dir(&path).unwrap();
        fs::write(
            path.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: fixture\n---\nFixture body"),
        )
        .unwrap();
    }
    let selected = workspace.path().join("selected");
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: selected.to_string_lossy().into_owned(),
        }],
        workspace.path(),
        "auggie-test",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let (root, paths) = selection(Some(&view)).unwrap();
    assert_eq!(PathBuf::from(root), view.root().canonicalize().unwrap());
    assert_eq!(paths, [view.root().canonicalize().unwrap(), selected.canonicalize().unwrap()]);
    assert!(workspace.path().join("native/SKILL.md").is_file());
}
