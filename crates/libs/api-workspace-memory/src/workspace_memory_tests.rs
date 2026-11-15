use crate::workspace_memory::*;
use serde_json::json;

#[test]
fn wire_rejects_invalid_names_workspace_and_directories() {
    for name in [
        "AGENTS.md",
        "SOUL.md",
        "USER.md",
        "IDENTITY.md",
        "TOOLS.md",
        "MEMORY.md",
        "HEARTBEAT.md",
        "BOOTSTRAP.md",
        "BOOT.md",
    ] {
        let file: WorkspaceFilename = serde_json::from_value(json!(name)).unwrap();
        assert_eq!(file.as_str(), name);
        assert_eq!(name.parse::<WorkspaceFilename>().unwrap(), file);
        assert_eq!(serde_json::to_value(file).unwrap(), json!(name));
    }
    for name in [
        "agents.md",
        "OTHER.md",
        "../AGENTS.md",
        "/AGENTS.md",
        "workspace/AGENTS.md",
        "AGENTS.md\0",
        "AGENTS.md\\",
    ] {
        assert!(serde_json::from_value::<WorkspaceFilename>(json!(name)).is_err());
        assert!(name.parse::<WorkspaceFilename>().is_err());
    }
    for name in ["workspace", "memory"] {
        let dir: WorkspaceDirectory = serde_json::from_value(json!(name)).unwrap();
        assert_eq!(serde_json::to_value(dir).unwrap(), json!(name));
    }
    for name in ["..", "workspace/memory", "/memory", "Memory", "env"] {
        assert!(serde_json::from_value::<WorkspaceDirectory>(json!(name)).is_err());
    }
    assert!(
        serde_json::from_value::<WorkspaceOpenDirectoryRequest>(
            json!({"subdir":"memory", "path":"/tmp"})
        )
        .is_err()
    );
}

#[test]
fn wire_rejects_invalid_names_gregorian_dates_and_paths() {
    for name in ["0001-01-01.md", "2000-02-29.md", "2024-02-29.md", "9999-12-31.md"] {
        let file: DailyMemoryFilename = serde_json::from_value(json!(name)).unwrap();
        assert_eq!(file.as_str(), name);
        assert_eq!(file.date(), &name[..10]);
        assert_eq!(serde_json::to_value(file).unwrap(), json!(name));
    }
    for name in [
        "",
        "a",
        "0000-01-01.md",
        "1900-02-29.md",
        "2100-02-29.md",
        "2025-02-29.md",
        "2026-04-31.md",
        "2026-00-01.md",
        "2026-13-01.md",
        "2026-01-00.md",
        "2026-1-01.md",
        "2026-01-01.MD",
        "2026-01-01.md\n",
        "../2026-01-01.md",
        "memory/2026-01-01.md",
        "2026-01-01.md/",
        "２０２６-01-01.md",
    ] {
        assert!(name.parse::<DailyMemoryFilename>().is_err(), "{name:?}");
        assert!(serde_json::from_value::<DailyMemoryFilename>(json!(name)).is_err(), "{name:?}");
    }
}

#[test]
fn workspace_wire_preserves_missing_empty_metadata_and_search() {
    let missing: WorkspaceFileContent = serde_json::from_value(json!(null)).unwrap();
    let empty: WorkspaceFileContent = serde_json::from_value(json!("")).unwrap();
    assert!(missing.0.is_none());
    assert_eq!(empty.0.as_deref(), Some(""));
    assert_eq!(serde_json::to_value(missing).unwrap(), json!(null));
    assert_eq!(serde_json::to_value(empty).unwrap(), json!(""));
    let list = json!({"filename":"2026-11-14.md", "date":"2026-11-14", "sizeBytes":4294967296_u64, "modifiedAt":1794614400_u64, "preview":"中文记忆"});
    let entry: DailyMemoryFileInfo = serde_json::from_value(list.clone()).unwrap();
    assert_eq!(entry.size_bytes, 4294967296);
    assert_eq!(entry.modified_at, 1794614400);
    assert_eq!(serde_json::to_value(entry).unwrap(), list);
    let search = json!({"filename":"2026-11-14.md", "date":"2026-11-14", "sizeBytes":12, "modifiedAt":1794614400_u64, "snippet":"中文摘要", "matchCount":2});
    let entry: DailyMemorySearchResult = serde_json::from_value(search.clone()).unwrap();
    assert_eq!(serde_json::to_value(entry).unwrap(), search);
    for wire in [json!({"content":"记忆", "path":"/tmp"}), json!({})] {
        assert!(serde_json::from_value::<WorkspaceWriteRequest>(wire).is_err());
    }
    assert!(
        serde_json::from_value::<DailyMemorySearchRequest>(json!({"query":"中文", "command":"ls"}))
            .is_err()
    );
}
