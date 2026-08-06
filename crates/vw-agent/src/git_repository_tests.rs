use super::*;
use git2::Repository;
use tempfile::TempDir;

fn signature() -> git2::Signature<'static> {
    git2::Signature::now("Vibe Window", "vw@example.com").expect("signature")
}

fn commit_tree(repo: &Repository, message: &str) -> git2::Oid {
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).expect("add all");
        index.write().expect("index write");
        index.write_tree().expect("tree")
    };
    let tree = repo.find_tree(tree_id).expect("tree");
    let sig = signature();
    let parents = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .into_iter()
        .collect::<Vec<_>>();
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs).expect("commit")
}

#[test]
fn discover_returns_none_outside_git_repository() {
    let temp = TempDir::new().expect("tempdir");
    assert!(discover(temp.path()).is_none());
}

#[test]
fn discover_reads_empty_repository_without_root_commits() {
    let temp = TempDir::new().expect("tempdir");
    Repository::init(temp.path()).expect("repo init");

    let discovered = discover(temp.path()).expect("empty repo should discover");
    assert!(discovered.root_commit_ids.is_empty());
    assert_eq!(
        discovered.workdir,
        std::fs::canonicalize(temp.path()).expect("workdir canonicalize")
    );
    assert!(discovered.common_dir.ends_with(".git"));
    assert_eq!(discovered.common_root, discovered.workdir);
}

#[test]
fn discover_reads_committed_repository_root_id() {
    let temp = TempDir::new().expect("tempdir");
    let repo = Repository::init(temp.path()).expect("repo init");
    std::fs::write(temp.path().join("README.md"), "demo\n").expect("readme");
    let oid = commit_tree(&repo, "initial");

    let discovered = discover(temp.path()).expect("committed repo should discover");
    assert_eq!(discovered.root_commit_ids, vec![oid.to_string()]);
}

#[test]
fn discover_resolves_linked_worktree_to_common_root() {
    let temp = TempDir::new().expect("tempdir");
    let primary = temp.path().join("primary");
    let linked = temp.path().join("linked");
    std::fs::create_dir_all(&primary).expect("primary dir");
    let repo = Repository::init(&primary).expect("repo init");
    std::fs::write(primary.join("README.md"), "demo\n").expect("readme");
    let oid = commit_tree(&repo, "initial");
    let mut opts = git2::WorktreeAddOptions::new();
    repo.worktree("linked-discovery", &linked, Some(&mut opts)).expect("linked worktree");

    let discovered = discover(&linked).expect("linked worktree should discover");
    assert_eq!(discovered.root_commit_ids, vec![oid.to_string()]);
    assert_eq!(
        discovered.workdir,
        std::fs::canonicalize(&linked).expect("linked canonicalize")
    );
    assert_eq!(
        discovered.common_root,
        std::fs::canonicalize(&primary).expect("primary canonicalize")
    );
}

#[test]
fn current_branch_reads_checked_out_and_detached_head() {
    let temp = TempDir::new().expect("tempdir");
    let repo = Repository::init(temp.path()).expect("repo init");
    std::fs::write(temp.path().join("README.md"), "demo\n").expect("readme");
    let oid = commit_tree(&repo, "initial");

    let branch = current_branch(temp.path()).expect("branch");
    assert!(!branch.is_empty());
    assert_ne!(branch, "HEAD");

    repo.set_head_detached(oid).expect("detach");
    assert_eq!(current_branch(temp.path()).as_deref(), Some("HEAD"));
}

#[test]
fn current_branch_reads_unborn_head_name() {
    let temp = TempDir::new().expect("tempdir");
    Repository::init(temp.path()).expect("repo init");

    let branch = current_branch(temp.path()).expect("unborn branch");
    assert!(!branch.is_empty());
    assert_ne!(branch, "HEAD");
}
