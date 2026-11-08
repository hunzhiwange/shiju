use super::keychain::{EntryError, KeychainEntry};
use std::process::{Command, Stdio};

struct TestNamespace {
    _unique: tempfile::TempDir,
    service: String,
}

impl TestNamespace {
    fn new() -> Self {
        let unique = tempfile::tempdir().unwrap();
        let suffix = unique.path().file_name().unwrap().to_str().unwrap();
        let service = format!("org.vibewindow.auth7.test.{suffix}");
        Self { _unique: unique, service }
    }

    fn entry<'a>(&'a self, account: &'a str) -> KeychainEntry<'a> {
        KeychainEntry::new(&self.service, account).unwrap()
    }
}

impl Drop for TestNamespace {
    fn drop(&mut self) {
        for account in ["primary", "neighbor"] {
            let _ = Command::new("/usr/bin/security")
                .args(["delete-generic-password", "-s", &self.service, "-a", account])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

#[test]
fn auth6_keychain_commits_expected_scoped_result() {
    let namespace = TestNamespace::new();
    let primary = namespace.entry("primary");
    let neighbor = namespace.entry("neighbor");
    assert!(primary.read().expect("fresh item lookup").is_none());
    assert!(primary.write(r#"{"synthetic":"initial"}"#).is_ok());
    assert!(neighbor.write(r#"{"synthetic":"neighbor"}"#).is_ok());
    let replacement = r#"{"synthetic":"quoted\" and escaped\\ text"}"#;
    assert!(primary.write(replacement).is_ok());
    assert!(primary.read().unwrap().is_some_and(|v| v == replacement.as_bytes()));
    let unicode = r#"{"synthetic":"钥匙串"}"#;
    assert!(primary.write(unicode).is_ok());
    assert!(
        primary
            .read()
            .unwrap()
            .is_some_and(|v| v == b"7b2273796e746865746963223a22e992a5e58c99e4b8b2227d")
    );
    assert!(primary.delete().is_ok());
    assert!(primary.delete().is_ok());
    assert!(primary.read().unwrap().is_none());
    assert!(neighbor.read().unwrap().is_some_and(|v| v == br#"{"synthetic":"neighbor"}"#));
    assert!(neighbor.delete().is_ok());
    assert!(neighbor.read().unwrap().is_none());
}

#[test]
fn auth6_keychain_rejects_invalid_or_stale_input() {
    let namespace = TestNamespace::new();
    assert_eq!(
        KeychainEntry::new(&namespace.service, "primary\"\ninvalid").err(),
        Some(EntryError::InvalidIdentity),
        "a name that could alter the security stdin command must fail before access",
    );
    let entry = namespace.entry("primary");
    let original = r#"{"synthetic":"kept"}"#;
    assert!(entry.write(original).is_ok());
    assert_eq!(entry.write(&"x".repeat(4096)), Err(EntryError::InvalidPayload));
    assert!(entry.read().unwrap().is_some_and(|v| v == original.as_bytes()));
    assert!(entry.delete().is_ok());
}
