//! 验证会话持久化模块对索引路径和名称规范化能力的重导出。
//!
//! 这些轻量用例确保模块门面只暴露既有辅助能力，不引入额外转换语义。

use super::*;

// 下列行为用例从重导出入口确认路径与名称处理契约保持一致。
#[test]
fn session_persistence_reexports_index_path_helper() {
    let path = session_index_path("/tmp/vw-acp-sessions");

    assert!(path.ends_with("index.json"));
}

#[test]
fn normalize_name_reexport_trims_empty_names() {
    assert_eq!(normalize_name(Some("  main ")).as_deref(), Some("main"));
    assert_eq!(normalize_name(Some("  ")), None);
    assert_eq!(normalize_name(None), None);
}
