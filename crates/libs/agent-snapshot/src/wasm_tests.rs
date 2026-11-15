//! 验证 WebAssembly 快照后端明确返回不支持的契约。
//! WebAssembly 不伪造文件快照能力，所有占位入口都返回文档化的空结果。

use super::*;

#[test]
fn wasm_snapshot_stubs_are_explicit_noops() {
    init("/tmp/worktree");
    assert!(cleanup("/tmp/worktree").is_ok());
    assert_eq!(track("/tmp/worktree").unwrap(), None);
    assert!(diff("/tmp/worktree", "hash").unwrap().is_empty());
    assert!(diff_full("/tmp/worktree", "a", "b").unwrap().is_empty());
    assert_eq!(patch("/tmp/worktree", "hash").unwrap().hash, "hash");
}
