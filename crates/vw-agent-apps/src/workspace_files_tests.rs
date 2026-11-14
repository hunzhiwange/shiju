use super::workspace_files::{read_workspace_file, write_workspace_file};
use super::workspace_paths::PathError;
use std::{fs, io::Read, os::unix::fs::symlink};
use vw_api_types::workspace_memory::WorkspaceFilename;

#[test]
fn files_roundtrip_whitelist_all_nine_preserve_unicode_and_empty() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("openclaw/workspace");
    let global = home.path().join("MEMORY.md");
    fs::write(&global, "全局记忆不变").unwrap();
    for filename in [
        WorkspaceFilename::Agents,
        WorkspaceFilename::Soul,
        WorkspaceFilename::User,
        WorkspaceFilename::Identity,
        WorkspaceFilename::Tools,
        WorkspaceFilename::Memory,
        WorkspaceFilename::Heartbeat,
        WorkspaceFilename::Bootstrap,
        WorkspaceFilename::Boot,
    ] {
        assert_eq!(read_workspace_file("host", "host", home.path(), &root, filename), Ok(None));
        for content in ["中文记忆\n🙂\r\n", "", "新内容"] {
            write_workspace_file("host", "host", home.path(), &root, filename, content).unwrap();
            assert_eq!(
                read_workspace_file("host", "host", home.path(), &root, filename)
                    .unwrap()
                    .as_deref(),
                Some(content)
            );
        }
    }
    assert_eq!(fs::read_to_string(global).unwrap(), "全局记忆不变");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 9);
}

#[test]
fn files_roundtrip_whitelist_missing_read_and_host_denial_do_not_create_directories() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("openclaw/workspace");
    let name = WorkspaceFilename::Agents;
    assert_eq!(read_workspace_file("host", "host", home.path(), &root, name), Ok(None));
    assert!(!root.exists());
    assert_eq!(
        read_workspace_file("other", "host", home.path(), &root, name),
        Err(PathError::UnauthorizedHost)
    );
    assert_eq!(
        write_workspace_file("other", "host", home.path(), &root, name, "data"),
        Err(PathError::UnauthorizedHost)
    );
    assert!(!root.exists());
}

#[test]
fn files_roundtrip_whitelist_replacement_keeps_previous_inode_intact() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let name = WorkspaceFilename::Memory;
    write_workspace_file("host", "host", home.path(), &root, name, "旧记忆").unwrap();
    let mut previous = fs::File::open(root.join(name.as_str())).unwrap();
    write_workspace_file("host", "host", home.path(), &root, name, "新记忆").unwrap();
    let mut content = String::new();
    previous.read_to_string(&mut content).unwrap();
    assert_eq!(content, "旧记忆");
    assert_eq!(fs::read_to_string(root.join(name.as_str())).unwrap(), "新记忆");
    assert_eq!(fs::read_dir(root).unwrap().count(), 1);
}

#[test]
fn files_roundtrip_whitelist_unsafe_destination_preserves_original() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let original = home.path().join("original");
    fs::write(&original, "不可覆盖").unwrap();
    let name = WorkspaceFilename::Agents;
    let destination = root.join(name.as_str());
    symlink(&original, &destination).unwrap();
    assert_eq!(
        read_workspace_file("host", "host", home.path(), &root, name),
        Err(PathError::UnsafeEntry)
    );
    assert_eq!(
        write_workspace_file("host", "host", home.path(), &root, name, "changed"),
        Err(PathError::UnsafeEntry)
    );
    fs::remove_file(&destination).unwrap();
    fs::hard_link(&original, &destination).unwrap();
    assert_eq!(
        write_workspace_file("host", "host", home.path(), &root, name, "changed"),
        Err(PathError::UnsafeEntry)
    );
    assert_eq!(fs::read_to_string(&original).unwrap(), "不可覆盖");
    assert_eq!(fs::read_to_string(destination).unwrap(), "不可覆盖");
    assert_eq!(fs::read_dir(root).unwrap().count(), 1);
}

#[test]
fn files_roundtrip_whitelist_rejects_names_outside_contract() {
    for name in ["../MEMORY.md", "/MEMORY.md", "memory/MEMORY.md", "agents.md", "config.json", ""] {
        assert!(name.parse::<WorkspaceFilename>().is_err());
    }
}
