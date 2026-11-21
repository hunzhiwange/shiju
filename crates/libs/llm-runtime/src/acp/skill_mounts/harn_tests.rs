use super::*;

struct Fixture(std::path::PathBuf);

impl SkillLease for Fixture {
    fn fingerprint(&self) -> &str {
        "fixture"
    }
    fn root(&self) -> &Path {
        &self.0
    }
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }

    fn snapshot(&self, destination: &Path) -> Result<(), Error> {
        std::fs::create_dir_all(destination.join("selected"))
            .map_err(super::super::super::to_api_error)?;
        std::fs::write(destination.join("selected/SKILL.md"), "selected")
            .map_err(super::super::super::to_api_error)
    }
}

#[test]
fn acp_skill_coexistence_harn_uses_only_request_selection() {
    let root = tempfile::tempdir().unwrap();
    let native_only = root.path().join("baseline");
    let mounted = root.path().join("mounted");
    std::fs::create_dir(&native_only).unwrap();
    std::fs::create_dir(&mounted).unwrap();
    let selected = Fixture(root.path().join("view"));

    assert!(selection(&native_only, false, Some(&selected)).is_err());
    assert!(selection::<Fixture>(&native_only, false, None).unwrap().is_empty());
    let baseline = selection::<Fixture>(&native_only, true, None).unwrap();
    assert!(!baseline.contains_key("SHIJU_HARN_SELECTED_VIEW"));
    assert!(!Path::new(&baseline["SHIJU_HARN_SKILL_ROOT"]).join("selected").exists());

    let env = selection(&mounted, true, Some(&selected)).unwrap();
    assert_eq!(Path::new(&env["SHIJU_HARN_SELECTED_VIEW"]), selected.root());
    let snapshot = Path::new(&env["SHIJU_HARN_SKILL_ROOT"]);
    assert_eq!(
        std::fs::read_to_string(snapshot.join("selected/selected/SKILL.md")).unwrap(),
        "selected"
    );
    assert_ne!(&env["SHIJU_HARN_SKILL_ROOT"], &baseline["SHIJU_HARN_SKILL_ROOT"]);
}
