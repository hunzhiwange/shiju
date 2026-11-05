#[cfg(unix)]
mod supported {
    use super::super::session_mounts::*;
    use std::{fs, path::Path};
    use vw_acp::SessionSkillMount;
    use vw_config_types::skills::SkillSecurityConfig;

    fn binding(project: &Path, session: &str) -> SessionSkillBinding {
        SessionSkillBinding {
            tenant: "tenant".into(),
            project: project.into(),
            session: session.into(),
            revision: 1,
        }
    }
    fn skill(root: &Path, name: &str) -> SessionSkillMount {
        let path = root.join(name);
        fs::create_dir_all(path.join("references")).unwrap();
        fs::create_dir_all(path.join("scripts")).unwrap();
        fs::write(
            path.join("SKILL.md"),
            "---\nname: demo\ndescription: example\n---\nRead references/info.txt\n",
        )
        .unwrap();
        fs::write(path.join("references/info.txt"), "first").unwrap();
        fs::write(path.join("scripts/run.sh"), "echo example\n").unwrap();
        SessionSkillMount { name: name.into(), path: path.to_str().unwrap().into() }
    }
    fn prepare(
        project: &Path,
        session: &str,
        mounts: &[SessionSkillMount],
    ) -> anyhow::Result<SessionSkillView> {
        SessionSkillView::prepare(
            binding(project, session),
            mounts,
            &SkillSecurityConfig::default(),
        )
    }
    #[test]
    fn session_skill_mount_view_isolates_and_revokes_without_deleting_source() {
        let temp = tempfile::tempdir().unwrap();
        let mount = skill(temp.path(), "demo");
        let first = prepare(temp.path(), "one", std::slice::from_ref(&mount)).unwrap();
        let second = prepare(temp.path(), "two", std::slice::from_ref(&mount)).unwrap();
        assert_ne!(first.root(), second.root());
        assert!(!first.root().starts_with(temp.path()));
        assert_eq!(
            fs::read_to_string(first.root().join("demo/references/info.txt")).unwrap(),
            "first"
        );
        assert!(first.root().join("demo/scripts/run.sh").is_file());
        let root = first.root().to_owned();
        drop(first);
        assert!(!root.exists());
        assert!(Path::new(&mount.path).join("SKILL.md").exists());
        second.validate().unwrap();
    }
    #[test]
    fn session_skill_mount_view_tracks_content_and_exact_replacement_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let a = skill(temp.path(), "alpha");
        let b = skill(temp.path(), "beta");
        let old = prepare(temp.path(), "one", std::slice::from_ref(&a)).unwrap();
        fs::write(Path::new(&a.path).join("references/info.txt"), "other").unwrap();
        assert!(old.validate().is_err());
        let changed = prepare(temp.path(), "one", std::slice::from_ref(&a)).unwrap();
        assert_ne!(old.fingerprint(), changed.fingerprint());
        let both = prepare(temp.path(), "one", &[a.clone(), b.clone()]).unwrap();
        assert!(both.root().join("alpha").exists());
        let reduced = prepare(temp.path(), "one", std::slice::from_ref(&b)).unwrap();
        assert!(!reduced.root().join("alpha").exists());
        fs::remove_dir_all(&a.path).unwrap();
        assert!(prepare(temp.path(), "one", &[b, a]).is_err());
        reduced.validate().unwrap();
        assert!(
            prepare(temp.path(), "one", &[]).unwrap().root().read_dir().unwrap().next().is_none()
        );
    }
    #[test]
    fn session_skill_mount_view_rejects_duplicate_malicious_disabled_and_missing_sources() {
        let temp = tempfile::tempdir().unwrap();
        let mount = skill(temp.path(), "demo");
        assert!(prepare(temp.path(), "one", &[mount.clone(), mount.clone()]).is_err());
        for name in ["../escape", ".", "bad/name", "bad\\name", ""] {
            let mut invalid = mount.clone();
            invalid.name = name.into();
            assert!(prepare(temp.path(), "one", &[invalid]).is_err());
        }
        fs::write(Path::new(&mount.path).join("SKILL.disabled"), "").unwrap();
        assert!(prepare(temp.path(), "one", std::slice::from_ref(&mount)).is_err());
        fs::remove_file(Path::new(&mount.path).join("SKILL.disabled")).unwrap();
        fs::remove_file(Path::new(&mount.path).join("SKILL.md")).unwrap();
        fs::write(Path::new(&mount.path).join("SKILL.toml"), "name = 'demo'").unwrap();
        assert!(prepare(temp.path(), "one", std::slice::from_ref(&mount)).is_err());
        fs::remove_dir_all(&mount.path).unwrap();
        assert!(prepare(temp.path(), "one", &[mount]).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn session_skill_mount_view_rejects_links_and_source_identity_replacement() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let mount = skill(temp.path(), "demo");
        let view = prepare(temp.path(), "one", std::slice::from_ref(&mount)).unwrap();
        fs::rename(&mount.path, temp.path().join("moved")).unwrap();
        symlink(temp.path().join("moved"), &mount.path).unwrap();
        assert!(view.validate().is_err());
        assert!(prepare(temp.path(), "one", std::slice::from_ref(&mount)).is_err());
        fs::remove_file(&mount.path).unwrap();
        fs::rename(temp.path().join("moved"), &mount.path).unwrap();
        symlink(temp.path().join("outside"), Path::new(&mount.path).join("references/escape"))
            .unwrap();
        assert!(prepare(temp.path(), "one", &[mount]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn session_skill_mount_view_link_failure_preserves_previous_lease() {
        let temp = tempfile::tempdir().unwrap();
        let mount = skill(temp.path(), "demo");
        let old = prepare(temp.path(), "one", std::slice::from_ref(&mount)).unwrap();
        let mut oversized = mount.clone();
        oversized.name = "x".repeat(300); // Real filesystem ENAMETOOLONG when creating the link.
        assert!(prepare(temp.path(), "one", &[mount.clone(), oversized]).is_err());
        old.validate().unwrap();
        assert_eq!(old.root().read_dir().unwrap().count(), 1);
        let original = Path::new(&mount.path);
        fs::rename(original, temp.path().join("previous")).unwrap();
        skill(temp.path(), "demo");
        assert!(old.validate().is_err());
    }

    #[test]
    fn session_skill_mount_view_revision_and_tenant_bind_the_view() {
        let temp = tempfile::tempdir().unwrap();
        let mount = skill(temp.path(), "demo");
        let first = prepare(temp.path(), "one", std::slice::from_ref(&mount)).unwrap();
        let mut revised = binding(temp.path(), "one");
        revised.revision = 2;
        let second = SessionSkillView::prepare(
            revised.clone(),
            std::slice::from_ref(&mount),
            &SkillSecurityConfig::default(),
        )
        .unwrap();
        assert_ne!(first.fingerprint(), second.fingerprint());
        revised.tenant = "other".into();
        let third =
            SessionSkillView::prepare(revised, &[mount], &SkillSecurityConfig::default()).unwrap();
        assert_ne!(second.fingerprint(), third.fingerprint());
        assert_eq!(third.binding().tenant, "other");
    }

    #[test]
    fn session_skill_mount_view_reuses_security_audit() {
        let temp = tempfile::tempdir().unwrap();
        let mount = skill(temp.path(), "demo");
        let security = SkillSecurityConfig { reject_script_files: true, ..Default::default() };
        assert!(
            SessionSkillView::prepare(binding(temp.path(), "one"), &[mount], &security).is_err()
        );
    }
}

#[cfg(not(unix))]
#[test]
fn session_skill_mount_view_reports_unsupported_platform() {
    use super::session_mounts::{SessionSkillBinding, SessionSkillView};
    let temp = tempfile::tempdir().unwrap();
    let mount = vw_acp::SessionSkillMount {
        name: "demo".into(),
        path: temp.path().to_str().unwrap().into(),
    };
    let binding = SessionSkillBinding {
        tenant: "tenant".into(),
        project: temp.path().into(),
        session: "session".into(),
        revision: 1,
    };
    let result = SessionSkillView::prepare(binding, &[mount], &Default::default());
    assert!(result.err().unwrap().to_string().contains("unsupported"));
}
