#[cfg(target_os = "macos")]
#[test]
fn cp12_job_cancel_cleans_staging_before_publication() {
    use std::{io, sync::atomic::{AtomicUsize, Ordering}};
    let root = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("SKILL.md"), "# skill").unwrap();
    std::fs::write(source.path().join("data"), vec![0; 65536]).unwrap();
    let checks = AtomicUsize::new(0);
    let check = || {
        if checks.fetch_add(1, Ordering::Relaxed) >= 4 {
            Err(io::Error::other("cancelled"))
        } else { Ok(()) }
    };
    assert!(super::local_install::LocalInstall::prepare_checked(
        root.path(), source.path(), "skill", &check).is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
