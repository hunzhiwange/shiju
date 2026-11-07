use super::commit::{ExecutionHostId, ProviderTransactions};
use crate::providers::claude::ClaudeFile;
use std::os::unix::fs::PermissionsExt;
use vw_api_types::id::TenantScope;

#[test]
fn cc04_ui_paths_reopens_saved_native_directory_and_refuses_replacement() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::set_permissions(private.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let target = home.path().join("自定义配置");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("settings.json"), b"{}").unwrap();
    let host = ExecutionHostId::new("host").unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let other_tenant = TenantScope::new("platform", "other").unwrap();

    let transactions = ProviderTransactions::new(private.path(), host.clone()).unwrap();
    transactions.set_claude_directory(home.path(), &tenant, Some("~/自定义配置")).unwrap();
    drop(transactions);

    let reopened = ProviderTransactions::open_existing(private.path(), host).unwrap().unwrap();
    assert!(reopened.claude_directory(&other_tenant).is_err());
    let selected = reopened.claude_directory(&tenant).unwrap().unwrap();
    assert_eq!(
        ClaudeFile::open_selected(home.path(), Some(&selected), false).unwrap().bytes.as_deref(),
        Some(b"{}".as_slice()),
    );
    std::fs::rename(&target, home.path().join("旧目录")).unwrap();
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("settings.json"), b"{}").unwrap();
    assert!(ClaudeFile::open_selected(home.path(), Some(&selected), true).is_err());
    assert!(!home.path().join(".claude").exists());
    assert!(reopened.set_claude_directory(home.path(), &tenant, None).unwrap().is_none());
    assert!(reopened.claude_directory(&tenant).unwrap().is_none());
}
