// 覆盖 Markdown 记忆的文件布局、读写、检索与内容保留。

// Markdown fixture 检查标题只写一次，并让召回评分、限制和过滤保持可重复。
use super::*;
use tempfile::TempDir;

fn temp_memory() -> (TempDir, MarkdownMemory) {
    let tmp = TempDir::new().unwrap();
    let memory = MarkdownMemory::new(tmp.path());
    (tmp, memory)
}

#[tokio::test]
async fn append_to_file_creates_core_header_once_and_appends() {
    let (_tmp, memory) = temp_memory();
    let path = memory.core_path();

    memory.append_to_file(&path, "first").await.unwrap();
    memory.append_to_file(&path, "second").await.unwrap();

    let content = tokio::fs::read_to_string(path).await.unwrap();
    assert!(content.starts_with("# Long-Term Memory\n\nfirst\n"));
    assert!(content.contains("\nsecond\n"));
    assert_eq!(content.matches("# Long-Term Memory").count(), 1);
}

#[tokio::test]
async fn append_to_file_creates_daily_header_for_daily_path() {
    let (_tmp, memory) = temp_memory();
    let path = memory.daily_path();

    memory.append_to_file(&path, "daily entry").await.unwrap();

    let content = tokio::fs::read_to_string(path).await.unwrap();
    assert!(content.starts_with("# Daily Log"));
    assert!(content.contains("daily entry"));
}

#[tokio::test]
async fn append_to_file_reports_error_when_existing_path_is_directory() {
    let (_tmp, memory) = temp_memory();
    let path = memory.workspace_dir.join("blocked.md");
    std::fs::create_dir_all(&path).unwrap();

    let result = memory.append_to_file(&path, "entry").await;

    assert!(result.is_err());
    assert!(path.is_dir());
}

#[test]
fn parse_entries_from_file_skips_headers_and_cleans_markdown_bullets() {
    let path = std::path::Path::new("/tmp/2024-05-01.md");
    let entries = MarkdownMemory::parse_entries_from_file(
        path,
        "# Heading\n\n- first item\nplain item\n  - second item  \n",
        &MemoryCategory::Daily,
    );

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].id, "2024-05-01:0");
    assert_eq!(entries[0].content, "first item");
    assert_eq!(entries[1].content, "plain item");
    assert_eq!(entries[2].content, "second item");
    assert!(entries.iter().all(|entry| entry.category == MemoryCategory::Daily));
    assert!(entries.iter().all(|entry| entry.session_id.is_none()));
    assert!(entries.iter().all(|entry| entry.score.is_none()));
}

#[test]
fn parse_entries_from_file_uses_unknown_when_filename_is_missing() {
    let entries = MarkdownMemory::parse_entries_from_file(
        std::path::Path::new(""),
        "- loose entry\n",
        &MemoryCategory::Core,
    );

    assert_eq!(entries[0].id, "unknown:0");
    assert_eq!(entries[0].timestamp, "unknown");
}

#[tokio::test]
async fn recall_scores_multiple_keywords_and_truncates_to_limit() {
    let (_tmp, memory) = temp_memory();

    memory.store("a", "Rust async sqlite", MemoryCategory::Core, None).await.unwrap();
    memory.store("b", "Rust only", MemoryCategory::Core, None).await.unwrap();
    memory.store("c", "Python async", MemoryCategory::Core, None).await.unwrap();

    let results = memory.recall("rust async", 2, None).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].content.to_lowercase().contains("rust"));
    assert!(results[0].content.to_lowercase().contains("async"));
    assert_eq!(results[0].score, Some(1.0));
    assert!(results.iter().all(|entry| entry.score.unwrap_or_default() > 0.0));
}

#[tokio::test]
async fn recall_empty_query_and_zero_limit_return_no_results() {
    let (_tmp, memory) = temp_memory();
    memory.store("a", "Rust async sqlite", MemoryCategory::Core, None).await.unwrap();

    assert!(memory.recall("", 10, None).await.unwrap().is_empty());
    assert!(memory.recall("rust", 0, None).await.unwrap().is_empty());
}

#[tokio::test]
async fn get_can_match_content_when_generated_key_is_unknown_to_caller() {
    let (_tmp, memory) = temp_memory();
    memory.store("pref", "needle value", MemoryCategory::Core, None).await.unwrap();

    let found = memory.get("needle").await.unwrap().unwrap();

    assert!(found.content.contains("needle value"));
}

#[tokio::test]
async fn list_without_category_returns_all_and_ignores_session_filter() {
    let (_tmp, memory) = temp_memory();
    memory.store("core", "core fact", MemoryCategory::Core, Some("s1")).await.unwrap();
    memory.store("daily", "daily note", MemoryCategory::Daily, Some("s2")).await.unwrap();

    let all = memory.list(None, Some("unused")).await.unwrap();

    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|entry| entry.category == MemoryCategory::Core));
    assert!(all.iter().any(|entry| entry.category == MemoryCategory::Daily));
}

#[tokio::test]
async fn health_check_reflects_storage_directory_existence() {
    let (_tmp, memory) = temp_memory();

    assert!(memory.health_check().await);
    std::fs::remove_dir_all(&memory.workspace_dir).unwrap();
    assert!(!memory.health_check().await);
}
