use super::hermes_memory_limits::{prepare_memory_enabled, read_memory_limits};
use vw_api_types::hermes_memory::HermesMemoryKind;

#[test]
fn hermes_toggle_preserves_yaml() {
    let pre = b"# native\nmodel: 'keep'\nmemory:\n  memory_char_limit: 3210\n  user_char_limit: 987\n  external_provider:\n    token: 'fixture'\n  unknown: [one, two]\n  memory_enabled: true\n  user_profile_enabled: true\n# tail\nother: {keep: true}\n";
    let next = prepare_memory_enabled(Some(pre), HermesMemoryKind::User, false).unwrap();
    assert_eq!(
        String::from_utf8(next.clone()).unwrap(),
        String::from_utf8(pre.to_vec())
            .unwrap()
            .replace("user_profile_enabled: true", "user_profile_enabled: false")
    );
    let limits = read_memory_limits(Some(&next)).unwrap();
    assert_eq!((limits.memory, limits.user), (3210, 987));
    assert!(limits.memory_enabled);
    assert!(!limits.user_enabled);
    let next = prepare_memory_enabled(Some(&next), HermesMemoryKind::Memory, false).unwrap();
    let limits = read_memory_limits(Some(&next)).unwrap();
    assert!(!limits.memory_enabled && !limits.user_enabled);
}

#[test]
fn hermes_toggle_preserves_yaml_defaults_and_missing_fields() {
    let defaults = read_memory_limits(None).unwrap();
    assert_eq!((defaults.memory, defaults.user), (2200, 1375));
    assert!(defaults.memory_enabled && defaults.user_enabled);
    let invalid = b"memory:\n  memory_char_limit: -1\n  user_char_limit: bad\n  memory_enabled: 'false'\n  user_profile_enabled: []\n";
    assert_eq!(read_memory_limits(Some(invalid)).unwrap(), defaults);
    let next = prepare_memory_enabled(None, HermesMemoryKind::User, false).unwrap();
    assert!(!read_memory_limits(Some(&next)).unwrap().user_enabled);
    assert!(read_memory_limits(Some(b"memory: [broken")).is_err());
    assert!(read_memory_limits(Some(b"- invalid-root")).is_err());
}

#[test]
fn hermes_toggle_preserves_yaml_rejects_ambiguous_edits() {
    for pre in [
        "memory: {memory_enabled: true}\n",
        "memory: broken\n",
        "memory:\n  memory_enabled: true\n  memory_enabled: false\n",
    ] {
        assert!(
            prepare_memory_enabled(Some(pre.as_bytes()), HermesMemoryKind::Memory, false).is_err()
        );
    }
}
