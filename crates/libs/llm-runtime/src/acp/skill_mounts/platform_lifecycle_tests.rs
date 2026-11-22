use super::MountedSkills;
use std::{fs, os::unix::fs::symlink, path::Path};

fn skill(root: &Path, name: &str, marker: &str) -> acp_types::SessionSkillMount {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: fixture\n---\n{marker}\n"),
    )
    .unwrap();
    acp_types::SessionSkillMount { name: name.into(), path: root.to_str().unwrap().into() }
}

#[test]
fn acp_platform_mount_lifecycle_owns_each_projection() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("项目");
    let native = skill(&workspace.join("skills/native"), "native", "native marker");
    let external = skill(&root.path().join("外挂"), "external", "external marker");
    let prepare = |mounts: &[acp_types::SessionSkillMount], session| {
        MountedSkills::prepare(
            mounts,
            &workspace,
            session,
            &Default::default(),
            &Default::default(),
        )
    };
    let mounts = [external.clone()];
    let original = prepare(&mounts, "owner").unwrap().unwrap();
    let restored = prepare(&mounts, "owner").unwrap().unwrap();
    let fork = prepare(&mounts, "fork").unwrap().unwrap();
    assert_eq!(original.0.fingerprint(), restored.0.fingerprint());
    assert_ne!(restored.0.fingerprint(), fork.0.fingerprint());

    for view in [&original, &restored, &fork] {
        assert!(
            fs::read_to_string(Path::new(&native.path).join("SKILL.md"))
                .unwrap()
                .contains("native marker")
        );
        assert!(
            fs::read_to_string(view.0.root().join("external/SKILL.md"))
                .unwrap()
                .contains("external marker")
        );
        assert!(!view.0.root().join("native").exists());
    }

    let retired_root = original.0.root().to_owned();
    fs::rename(&retired_root, retired_root.with_file_name("retired")).unwrap();
    fs::create_dir(&retired_root).unwrap();
    symlink(fs::canonicalize(&external.path).unwrap(), retired_root.join("external")).unwrap();
    assert!(original.0.validate().is_err(), "a replacement directory cannot inherit a lease");

    assert!(prepare(&[], "owner").unwrap().is_none());
    drop(original);
    assert!(!retired_root.exists());
    restored.0.validate().unwrap();
    fork.0.validate().unwrap();
    assert!(Path::new(&native.path).join("SKILL.md").exists());
    assert!(Path::new(&external.path).join("SKILL.md").exists());
}

#[test]
fn acp_platform_mount_lifecycle_rejects_collisions_and_illegal_links() {
    let root = tempfile::tempdir().unwrap();
    let first = skill(&root.path().join("native/fixture"), "fixture", "first");
    let second = skill(&root.path().join("second"), "Fixture", "second");
    let prepare = |mounts: &[acp_types::SessionSkillMount]| {
        MountedSkills::prepare(
            mounts,
            root.path(),
            "owner",
            &Default::default(),
            &Default::default(),
        )
    };
    assert!(prepare(&[first.clone(), second.clone()]).is_err());
    let native = agent_skills::loader::load_skill_directory(
        Path::new(&first.path),
        agent_skills::types::SkillLoadMode::Full,
        &Default::default(),
        true,
    )
    .unwrap()
    .unwrap();
    let single = [second.clone()];
    let view = prepare(&single).unwrap().unwrap();
    let view = std::sync::Arc::try_unwrap(view.0).ok().unwrap();
    let sources = agent_skills::session_sources::SessionSkillSources::new(view, single.into());
    assert!(sources.extend(vec![native], &Default::default(), true).is_err());
    assert!(
        std::fs::read_to_string(Path::new(&first.path).join("SKILL.md")).unwrap().contains("first")
    );

    symlink(Path::new(&second.path).join("SKILL.md"), Path::new(&first.path).join("link")).unwrap();
    assert!(prepare(&[first]).is_err());
}
