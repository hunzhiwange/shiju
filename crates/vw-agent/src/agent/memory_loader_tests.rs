//! 测试记忆加载器的召回筛选、格式化与失败降级。

use super::*;
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;
use std::sync::Mutex;

struct StubMemory {
    entries: Vec<MemoryEntry>,
    fail: bool,
    queries: Mutex<Vec<String>>,
    limits: Mutex<Vec<usize>>,
}

impl StubMemory {
    fn with_entries(entries: Vec<MemoryEntry>) -> Self {
        Self {
            entries,
            fail: false,
            queries: Mutex::new(Vec::new()),
            limits: Mutex::new(Vec::new()),
        }
    }

    fn failing() -> Self {
        Self {
            entries: Vec::new(),
            fail: true,
            queries: Mutex::new(Vec::new()),
            limits: Mutex::new(Vec::new()),
        }
    }
}

fn entry(key: &str, content: &str, score: Option<f64>) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Conversation,
        timestamp: "now".to_string(),
        session_id: None,
        score,
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for StubMemory {
    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.queries.lock().unwrap().push(query.to_string());
        self.limits.lock().unwrap().push(limit);
        if self.fail {
            anyhow::bail!("recall failed");
        }
        Ok(self.entries.clone())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.entries.len())
    }

    async fn health_check(&self) -> bool {
        !self.fail
    }

    fn name(&self) -> &str {
        "stub"
    }
}

#[tokio::test]
async fn new_clamps_zero_limit_to_one_and_passes_query_to_recall() {
    let memory = StubMemory::with_entries(vec![entry("preference", "concise", None)]);
    let loader = DefaultMemoryLoader::new(0, 0.5);

    let context = loader.load_context(&memory, "style").await.unwrap();

    assert_eq!(memory.queries.lock().unwrap().as_slice(), ["style"]);
    assert_eq!(memory.limits.lock().unwrap().as_slice(), [1]);
    assert!(context.contains("[Memory context]"));
    assert!(context.contains("- preference: concise"));
    assert!(context.ends_with("\n\n"));
}

#[tokio::test]
async fn load_context_returns_empty_when_recall_has_no_entries() {
    let memory = StubMemory::with_entries(Vec::new());
    let loader = DefaultMemoryLoader::default();

    let context = loader.load_context(&memory, "anything").await.unwrap();

    assert_eq!(context, "");
}

#[tokio::test]
async fn load_context_filters_low_scores_and_assistant_autosaves() {
    let memory = StubMemory::with_entries(vec![
        entry("assistant_resp_123", "old assistant text", Some(0.99)),
        entry("low_score", "too weak", Some(0.39)),
        entry("at_threshold", "kept", Some(0.4)),
        entry("missing_score", "also kept", None),
    ]);
    let loader = DefaultMemoryLoader::new(10, 0.4);

    let context = loader.load_context(&memory, "history").await.unwrap();

    assert!(context.contains("- at_threshold: kept"));
    assert!(context.contains("- missing_score: also kept"));
    assert!(!context.contains("assistant_resp_123"));
    assert!(!context.contains("too weak"));
}

#[tokio::test]
async fn load_context_returns_empty_when_all_entries_are_filtered() {
    let memory = StubMemory::with_entries(vec![
        entry("assistant_resp", "legacy", Some(1.0)),
        entry("low_score", "weak", Some(0.1)),
    ]);
    let loader = DefaultMemoryLoader::new(5, 0.5);

    let context = loader.load_context(&memory, "history").await.unwrap();

    assert_eq!(context, "");
}

#[tokio::test]
async fn load_context_propagates_recall_errors() {
    let memory = StubMemory::failing();
    let loader = DefaultMemoryLoader::default();

    let err = loader.load_context(&memory, "boom").await.unwrap_err();

    assert!(err.to_string().contains("recall failed"));
}
