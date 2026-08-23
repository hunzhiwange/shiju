#![allow(clippy::expect_used)]

//! 静态审计 TUI 运行时代码，防止重新依赖管理器逃生接口。
//! 该边界保证复制后的展示层只通过显式桥接数据工作。

use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn rust_sources_under(dir: &Path) -> Vec<PathBuf> {
    // 递归收集并排序 Rust 文件，使违规报告顺序在不同文件系统上稳定。
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).expect("source directory should be readable");
    for entry in entries {
        let entry = entry.expect("source directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_sources_under(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

#[test]
// 任一源文件出现认证或线程管理器入口都应作为架构回归明确失败。
fn tui_runtime_source_does_not_depend_on_manager_escape_hatches() {
    let src_file = codex_utils_cargo_bin::find_resource!("src/chatwidget.rs")
        .expect("chatwidget source runfile should resolve");
    let src_dir = src_file
        .parent()
        .expect("chatwidget source file should have a parent");
    let sources = rust_sources_under(src_dir);
    let forbidden = [
        "AuthManager",
        "ThreadManager",
        "auth_manager(",
        "thread_manager(",
    ];

    let violations: Vec<String> = sources
        .iter()
        .flat_map(|path| {
            let contents = fs::read_to_string(path).expect("Rust source file should be readable");
            let path_display = path.display().to_string();
            forbidden
                .iter()
                .filter(move |needle| contents.contains(**needle))
                .map(move |needle| format!("{path_display} contains `{needle}`"))
        })
        .collect();

    assert!(
        violations.is_empty(),
        "unexpected manager dependency regression(s):\n{}",
        violations.join("\n")
    );
}
