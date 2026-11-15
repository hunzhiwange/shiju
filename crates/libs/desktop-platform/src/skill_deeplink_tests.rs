use super::*;

#[test]
fn cp12_deeplink_success() {
    let request = parse("ccswitch://v1/import?resource=skill&repo=owner/name&branch=feature%2Fone&enabled=false",
        "/project".into(), "host".into()).unwrap();
    assert_eq!(request.repo.branch, "feature/one");
    assert!(!request.repo.enabled);
    assert_eq!(request.directory, "/project");

}

#[test]
fn cp12_deeplink_rejects_without_side_effects() {
    for link in [
        "https://example.org/?resource=skill&repo=a/b",
        "ccswitch://v1/import?resource=provider&repo=a/b",
        "ccswitch://v1/import?resource=skill&repo=a/b&branch=..%2Fevil",
        "ccswitch://v1/import?resource=skill&repo=a/b&repo=c/d",
        "ccswitch://v1/import?resource=skill&repo=a/b&directory=/tmp",
        "ccswitch://v1/import?resource=skill&repo=a/..",
    ] {
        assert!(parse(link, "/project".into(), "host".into()).is_err(), "{link}");
    }

}
