use super::repos::{RepoError, SkillRepo, SkillRepos, validate_repo_ref};

fn repository(branch: &str) -> SkillRepo {
    SkillRepo { owner: "owner".into(), name: "repo".into(), branch: branch.into(), enabled: true }
}

#[test]
fn cp12_repos_success() {
    let mut repos = SkillRepos::default();
    repos.save(repository("feature/native")).unwrap();
    repos.set_enabled("OWNER", "REPO", false).unwrap();
    assert!(!repos.entries()[0].enabled);
    assert_eq!(repos.entries()[0].branch, "feature/native");
    let saved = repos.entries().to_vec();
    assert_eq!(SkillRepos::from_saved(saved).unwrap(), repos);
    repos.save(repository("release/v1.0")).unwrap();
    assert_eq!(repos.entries().len(), 1);
    assert_eq!(repos.entries()[0].branch, "release/v1.0");
    assert!(repos.remove("OWNER", "repo").unwrap());
    assert!(!repos.remove("owner", "repo").unwrap());
    assert!(repos.entries().is_empty());
    for branch in ["", "HEAD", "head", "main", "release/v1.0", "feature/技能"] {
        assert!(validate_repo_ref("owner", "repo.name_1", branch).is_ok(), "{branch:?}");
    }
}

#[test]
fn cp12_repos_rejects_without_side_effects() {
    let mut repos = SkillRepos::from_saved(vec![repository("main")]).unwrap();
    let before = repos.clone();
    for branch in [
        "../../../releases/download/v1/evil", "a/./b", "a/.../b", "a..b", "/main", "main/",
        "a//b", "a.lock", "a.lock/b", ".hidden", "main.", "a@{b", "@", "main#fragment",
        "main?query", "%2e%2e", "a\\b", "a b", "a\n", "a\u{7f}", "a~b", "a:b", "a*b",
    ] {
        assert_eq!(repos.save(repository(branch)), Err(RepoError::InvalidBranch), "{branch:?}");
        assert_eq!(repos, before);
    }
    assert!(repos.save(repository(&"a".repeat(256))).is_err());
    for (owner, name) in [("../owner", "repo"), ("owner", ".."), ("owner", "a/b"), ("", "r")] {
        assert!(validate_repo_ref(owner, name, "main").is_err());
        assert!(repos.remove(owner, name).is_err());
        assert!(repos.set_enabled(owner, name, false).is_err());
        assert_eq!(repos, before);
    }
    assert_eq!(repos.set_enabled("other", "repo", false), Err(RepoError::RepositoryNotFound));
    assert_eq!(repos, before);
    let mut alias = repository("main");
    alias.owner = "OWNER".into();
    assert_eq!(
        SkillRepos::from_saved(vec![repository("main"), alias]),
        Err(RepoError::DuplicateRepository)
    );
    assert!(SkillRepos::from_saved(vec![repository("../bad")]).is_err());
}
