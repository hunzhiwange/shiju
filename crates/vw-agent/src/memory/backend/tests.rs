//! 内存后端分类与配置测试模块
//!
//! 本模块提供针对内存后端识别和配置分析功能的单元测试。
//! 测试覆盖已知后端类型的正确分类、未知后端的可扩展性处理，
//! 以及各种后端配置属性的验证。

use super::*;

/// 测试已知内存后端类型的分类功能
///
/// 验证 `classify_memory_backend` 函数能够正确识别以下已知后端类型：
/// - `sqlite`: SQLite 数据库后端
/// - `lucid`: Lucid 内存系统（基于 SQLite）
/// - `postgres`: PostgreSQL 数据库后端
/// - `mariadb` / `mysql`: MariaDB/MySQL 数据库后端（统一映射为 Mariadb）
/// - `markdown`: Markdown 文件存储后端
/// - `none`: 无持久化后端
#[test]
fn classify_known_backends() {
    assert_eq!(classify_memory_backend("sqlite"), MemoryBackendKind::Sqlite);
    assert_eq!(classify_memory_backend("lucid"), MemoryBackendKind::Lucid);
    assert_eq!(classify_memory_backend("postgres"), MemoryBackendKind::Postgres);
    assert_eq!(classify_memory_backend("mariadb"), MemoryBackendKind::Mariadb);
    assert_eq!(classify_memory_backend("mysql"), MemoryBackendKind::Mariadb);
    assert_eq!(classify_memory_backend("qdrant"), MemoryBackendKind::Qdrant);
    assert_eq!(classify_memory_backend("markdown"), MemoryBackendKind::Markdown);
    assert_eq!(classify_memory_backend("none"), MemoryBackendKind::None);
}

/// 测试未知内存后端类型的分类处理
///
/// 验证当传入未注册的后端标识符（如 "redis"）时，
/// `classify_memory_backend` 函数返回 `MemoryBackendKind::Unknown`，
/// 确保系统具备良好的可扩展性。
#[test]
fn classify_unknown_backend() {
    assert_eq!(classify_memory_backend("redis"), MemoryBackendKind::Unknown);
    assert_eq!(classify_memory_backend(" SQLITE "), MemoryBackendKind::Unknown);
    assert_eq!(classify_memory_backend("MYSQL"), MemoryBackendKind::Unknown);
}

#[test]
fn default_backend_key_is_sqlite() {
    assert_eq!(default_memory_backend_key(), "sqlite");
}

#[test]
fn default_backend_matches_first_selectable_profile() {
    let default_profile = memory_backend_profile(default_memory_backend_key());
    let first_selectable = selectable_memory_backends()
        .first()
        .expect("at least one memory backend should be selectable");

    assert_eq!(&default_profile, first_selectable);
}

#[test]
fn selectable_backends_are_stable_user_facing_choices() {
    let keys: Vec<_> = selectable_memory_backends().iter().map(|profile| profile.key).collect();

    assert_eq!(keys, vec!["sqlite", "lucid", "markdown", "none"]);
}

#[test]
fn mysql_alias_returns_mariadb_profile() {
    assert_eq!(memory_backend_profile("mysql"), memory_backend_profile("mariadb"));
}

/// 测试 Lucid 后端的配置属性
///
/// 验证 `memory_backend_profile` 函数针对 "lucid" 后端返回的配置包含：
/// - `sqlite_based`: true（基于 SQLite 实现）
/// - `optional_dependency`: true（为可选依赖）
/// - `uses_sqlite_hygiene`: true（使用 SQLite 卫生规范）
#[test]
fn lucid_profile_is_sqlite_based_optional_backend() {
    let profile = memory_backend_profile("lucid");
    assert!(profile.sqlite_based);
    assert!(profile.optional_dependency);
    assert!(profile.uses_sqlite_hygiene);
}

/// 测试未知后端配置的默认值与可扩展性
///
/// 验证 `memory_backend_profile` 函数针对自定义后端（如 "custom-memory"）返回的配置：
/// - `key`: 提取自后端名称前缀（"custom"）
/// - `auto_save_default`: true（默认启用自动保存）
/// - `uses_sqlite_hygiene`: false（不使用 SQLite 卫生规范）
///
/// 这确保了系统对未知后端提供合理的默认行为，同时保持可扩展性。
#[test]
fn unknown_profile_preserves_extensibility_defaults() {
    let profile = memory_backend_profile("custom-memory");
    assert_eq!(profile.key, "custom");
    assert!(profile.auto_save_default);
    assert!(!profile.uses_sqlite_hygiene);
}

#[test]
fn invalid_backend_spellings_use_custom_profile() {
    let custom_profile = memory_backend_profile("custom-memory");

    for backend in ["", " ", "SQLite", "sqlite ", "MYSQL"] {
        assert_eq!(memory_backend_profile(backend), custom_profile, "backend={backend:?}");
    }
}

#[test]
fn profiles_cover_all_classified_backends() {
    let cases = [
        ("sqlite", "sqlite", true, true, true, false),
        ("lucid", "lucid", true, true, true, true),
        ("postgres", "postgres", true, false, false, true),
        ("mariadb", "mariadb", true, false, false, true),
        ("mysql", "mariadb", true, false, false, true),
        ("qdrant", "qdrant", true, false, false, false),
        ("markdown", "markdown", true, false, false, false),
        ("none", "none", false, false, false, false),
        ("custom-backend", "custom", true, false, false, false),
    ];

    for (
        input,
        expected_key,
        auto_save_default,
        uses_sqlite_hygiene,
        sqlite_based,
        optional_dependency,
    ) in cases
    {
        let profile = memory_backend_profile(input);

        assert_eq!(profile.key, expected_key, "input={input}");
        assert_eq!(profile.auto_save_default, auto_save_default, "input={input}");
        assert_eq!(profile.uses_sqlite_hygiene, uses_sqlite_hygiene, "input={input}");
        assert_eq!(profile.sqlite_based, sqlite_based, "input={input}");
        assert_eq!(profile.optional_dependency, optional_dependency, "input={input}");
        assert!(!profile.label.trim().is_empty(), "input={input}");
    }
}
