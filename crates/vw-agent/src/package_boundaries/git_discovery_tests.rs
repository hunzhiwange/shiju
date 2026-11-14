use crate::git_repository;

#[test]
fn git_discovery_production_boundary() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    repo.set_head("refs/heads/boundary").expect("set unborn branch");
    let nested = temp.path().join("nested");
    std::fs::create_dir(&nested).expect("nested directory");

    let discovered: git_repository::DiscoveredRepository =
        git_repository::discover(&nested).expect("discover through agent entry");
    let shared: &git_discovery::git_repository::DiscoveredRepository = &discovered;
    let root = std::fs::canonicalize(temp.path()).expect("canonical root");
    assert_eq!(shared.workdir, root);
    assert_eq!(shared.common_root, root);
    assert_eq!(shared.common_dir, root.join(".git"));
    assert!(shared.root_commit_ids.is_empty());
    assert_eq!(git_repository::current_branch(&nested).as_deref(), Some("boundary"));
}
