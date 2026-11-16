//! 确认新版技能模块的公开装配与兼容入口保持稳定。
//! 模块级测试覆盖旧目录兼容与命令转发。

use super::*;

#[test]
fn discover_default_sources_includes_legacy_workspace_directory() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let legacy = workspace.path().join("skills");
    std::fs::create_dir_all(&legacy).unwrap();
    let legacy = legacy.canonicalize().expect("legacy skills dir should canonicalize");

    let sources = discover_local_skill_source_dirs(Some(workspace.path()));

    assert!(
        sources.iter().any(|source| {
            source.kind == LocalSkillSourceKind::Workspace && source.path == legacy
        })
    );
}
