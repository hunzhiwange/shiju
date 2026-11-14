//! 验证旧入口的类型身份及 Markdown 文件的真实读写。

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn memory_markdown_production_boundary() {
    use crate::memory::{MarkdownMemory, Memory, MemoryCategory};

    let workspace = tempfile::TempDir::new().unwrap();
    let storage = crate::memory::paths::project_data_dir_best_effort(workspace.path());
    assert!(!storage.exists(), "boundary fixture must own a fresh data directory");
    struct StorageCleanup(std::path::PathBuf);
    impl Drop for StorageCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = StorageCleanup(storage.clone());
    let memory: memory_markdown::memory::markdown::MarkdownMemory =
        MarkdownMemory::new(workspace.path());
    let _: &dyn memory_types::memory::Memory = &memory;

    memory.store("boundary", "first", MemoryCategory::Core, None).await.unwrap();
    memory.store("boundary", "second", MemoryCategory::Core, None).await.unwrap();
    memory.store("daily", "local log", MemoryCategory::Daily, None).await.unwrap();
    let core = std::fs::read_to_string(storage.join("MEMORY.md")).unwrap();
    assert_eq!(core, "# Long-Term Memory\n\n- **boundary**: first\n\n- **boundary**: second\n");
    assert_eq!(memory.list(Some(&MemoryCategory::Daily), None).await.unwrap().len(), 1);
    assert_eq!(memory.recall("second", 1, None).await.unwrap()[0].content, "**boundary**: second");
    assert!(!memory.forget("boundary").await.unwrap());
    assert_eq!(memory.count().await.unwrap(), 3);
}
