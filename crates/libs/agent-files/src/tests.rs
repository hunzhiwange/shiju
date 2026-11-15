//! 文件模块行为测试。
//!
//! 本测试文件覆盖文件列表接口与默认忽略规则的交互，确保常见构建目录不会进入
//! 返回结果，同时普通源码目录仍可见。

use std::fs;

use super::{NodeType, list};

#[test]
fn list_skips_ignored_target_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();

    fs::create_dir_all(root.join("target/debug")).expect("create target");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("target/debug/output.txt"), "ignored").expect("write ignored");
    fs::write(root.join("src/main.rs"), "fn main() {}").expect("write source");

    let nodes = list(root, None).expect("list root");

    assert!(
        nodes.iter().all(|node| node.name != "target"),
        "ignored target directory should not be returned"
    );
    assert!(
        nodes.iter().any(|node| node.name == "src" && matches!(node.r#type, NodeType::Directory)),
        "non-ignored directory should still be returned"
    );
}
