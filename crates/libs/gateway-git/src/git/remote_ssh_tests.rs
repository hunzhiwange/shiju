use super::*;

#[test]
fn ssh_config_changes_endpoint_without_changing_scp_path() {
    let config = SshConfig::default()
        .parse(
            &mut b"Host github.com\n  HostName ssh.github.com\n  Port 443\n".as_slice(),
            ParseRule::STRICT,
        )
        .unwrap();
    let params = config.query("github.com");
    let scp = Endpoint::parse("git@github.com:owner/repo.git").unwrap();
    assert_eq!(
        scp.resolve(&params).unwrap().as_deref(),
        Some("[git@ssh.github.com:443]:owner/repo.git")
    );
    let uri = Endpoint::parse("ssh://git@github.com:22/owner/repo.git").unwrap();
    assert_eq!(
        uri.resolve(&params).unwrap().as_deref(),
        Some("ssh://git@ssh.github.com:22/owner/repo.git")
    );
}

#[test]
fn ssh_endpoint_override_preserves_remote_and_persisted_config() {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    let original_url = "git@github.com:owner/repo.git";
    let endpoint = "[git@ssh.github.com:443]:owner/repo.git";
    repo.remote("origin", original_url).unwrap();

    let _overlay = override_endpoint(&repo, "origin", true, endpoint).unwrap();
    let remote = repo.find_remote("origin").unwrap();
    assert_eq!(remote.url().unwrap(), original_url);
    assert_eq!(remote.pushurl().unwrap(), Some(endpoint));
    assert_eq!(
        remote.fetch_refspecs().unwrap().get(0),
        Some("+refs/heads/*:refs/remotes/origin/*")
    );

    let persisted = Repository::open(temp.path()).unwrap();
    assert_eq!(persisted.find_remote("origin").unwrap().pushurl().unwrap(), None);
}
