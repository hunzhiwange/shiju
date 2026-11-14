use super::hermes_memory::HermesMemory;
use crate::workspace_paths::PathError;
use std::{fs, os::unix::fs::symlink};
use vw_api_types::hermes_memory::HermesMemoryKind::{Memory, User};

#[test]
fn hermes_memory_roundtrips_kinds() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("hermes");
    let reader = HermesMemory::open("host", "host", home.path(), &root, false).unwrap();
    for kind in [Memory, User] {
        let missing = reader.read_memory(kind).unwrap();
        assert_eq!(missing.kind, kind);
        assert_eq!(missing.content, "");
        assert!(!missing.exists);
    }
    assert!(!root.exists());
    let memory = HermesMemory::open("host", "host", home.path(), &root, true).unwrap();
    fs::write(root.join("config.yaml"), "memory:\n  memory_enabled: false\n  user_profile_enabled: false\n").unwrap();
    let content = "中文记忆🧠\n".repeat(4096);
    memory.write_memory(Memory, &content).unwrap();
    memory.write_memory(User, "用户资料").unwrap();
    assert_eq!(memory.read_memory(Memory).unwrap().content, content);
    assert_eq!(memory.read_memory(User).unwrap().content, "用户资料");
    memory.write_memory(User, "").unwrap();
    let empty = memory.read_memory(User).unwrap();
    assert!(empty.exists);
    assert_eq!(empty.content, "");
    assert_eq!(memory.read_memory(Memory).unwrap().content, content);
    assert_eq!(fs::read_to_string(root.join("memories/MEMORY.md")).unwrap(), content);
    assert!(fs::read_to_string(root.join("config.yaml")).unwrap().contains("memory_enabled: false"));
}

#[test]
fn hermes_memory_roundtrips_kinds_rejects_unsafe_entries_and_hosts() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("hermes");
    assert!(matches!(
        HermesMemory::open("other", "host", home.path(), &root, true),
        Err(PathError::UnauthorizedHost)
    ));
    assert!(!root.exists());
    let memory = HermesMemory::open("host", "host", home.path(), &root, true).unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "unchanged").unwrap();
    symlink(outside.path(), root.join("memories/MEMORY.md")).unwrap();
    assert_eq!(memory.read_memory(Memory), Err(PathError::UnsafeEntry));
    assert_eq!(memory.write_memory(Memory, "overwrite"), Err(PathError::UnsafeEntry));
    memory.write_memory(User, "safe").unwrap();
    assert_eq!(fs::read_to_string(outside.path()).unwrap(), "unchanged");
    fs::remove_file(root.join("memories/MEMORY.md")).unwrap();
    fs::write(root.join("memories/MEMORY.md"), [0xff]).unwrap();
    assert_eq!(memory.read_memory(Memory), Err(PathError::Io));
}
