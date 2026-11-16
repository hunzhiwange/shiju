//! 生产实现不得通过系统 Git 子进程读取退出状态。

#[test]
fn exit_worktree_production_status_check_does_not_spawn_system_git() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/exit_worktree/mod.rs"
    ))
    .expect("exit worktree source should read");

    assert!(!source.contains("git_tokio_command"));
    assert!(!source.contains("Command::new(\"git\")"));
}
