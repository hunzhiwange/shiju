use super::*;
use crate::acp::acp_option_error;
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct View {
    root: PathBuf,
    valid: bool,
}

impl SkillLease for View {
    fn fingerprint(&self) -> &str {
        "test-binding"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn validate(&self) -> Result<(), Error> {
        if self.valid { Ok(()) } else { Err(acp_option_error("changed skill view")) }
    }
}

#[test]
fn acp_skill_coexistence_dimcode_projects_only_a_valid_view() {
    let directory = tempfile::tempdir().unwrap();
    let mut view = View { root: directory.path().into(), valid: true };
    let selected = selection(Some(&view)).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected.get("SHIJU_ACP_DIMCODE_SKILL_ROOT").map(Path::new),
        Some(directory.path().canonicalize().unwrap().as_path())
    );
    assert!(selection(None::<&View>).unwrap().is_empty());
    view.valid = false;
    assert!(selection(Some(&view)).is_err());
}
