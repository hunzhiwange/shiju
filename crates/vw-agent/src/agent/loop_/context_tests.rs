//! 测试代理上下文对记忆相关度、作用域和召回失败的处理。
//! 可控记忆夹具区分低分、无评分、自动保存和召回失败等边界。

use super::*;
use crate::app::agent::memory::{MemoryCategory, MemoryEntry};
use std::sync::Mutex;

#[derive(Default)]
// 召回结果、失败开关和调用记录分离，便于独立验证过滤与后端错误。
struct FakeMemory {
    entries: Vec<MemoryEntry>,
    fail_recall: bool,
    recalls: Mutex<Vec<(String, usize, Option<String>)>>,
}

impl FakeMemory {
    fn with_entries(entries: Vec<MemoryEntry>) -> Self {
        Self { entries, ..Self::default() }
    }

    fn failing() -> Self {
        Self { fail_recall: true, ..Self::default() }
    }

    fn recalls(&self) -> Vec<(String, usize, Option<String>)> {
        self.recalls.lock().unwrap().clone()
    }
}

fn memory_entry(key: &str, content: &str, score: Option<f64>) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Core,
        timestamp: "now".to_string(),
        session_id: None,
        score,
    }
}

#[async_trait::async_trait]
impl Memory for FakeMemory {
    fn name(&self) -> &str {
        "fake"
    }

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
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.recalls.lock().unwrap().push((
            query.to_string(),
            limit,
            session_id.map(str::to_string),
        ));

        if self.fail_recall {
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
        Ok(0)
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn build_context_filters_low_relevance_entries() {
    let memory = FakeMemory::with_entries(vec![
        memory_entry("keep", "use concise answers", Some(0.9)),
        memory_entry("drop", "too weak", Some(0.1)),
    ]);
    let context = build_context(&memory, "style", 0.5).await;

    assert_eq!(context, "[Memory context]\n- keep: use concise answers\n\n");
    assert!(!context.contains("too weak"));
}

#[tokio::test]
async fn build_context_keeps_unscored_entries_and_uses_fixed_recall_scope() {
    let memory =
        FakeMemory::with_entries(vec![memory_entry("preference", "prefers direct answers", None)]);

    let context = build_context(&memory, "answer style", 0.8).await;

    assert_eq!(context, "[Memory context]\n- preference: prefers direct answers\n\n");
    assert_eq!(memory.recalls(), vec![("answer style".to_string(), 5, None)]);
}

#[tokio::test]
async fn build_context_keeps_entries_at_min_relevance_boundary() {
    let memory =
        FakeMemory::with_entries(vec![memory_entry("boundary", "exact threshold", Some(0.5))]);

    let context = build_context(&memory, "threshold", 0.5).await;

    assert_eq!(context, "[Memory context]\n- boundary: exact threshold\n\n");
}

#[tokio::test]
async fn build_context_returns_empty_when_no_entries_are_relevant() {
    let memory = FakeMemory::with_entries(vec![memory_entry("weak", "ignore me", Some(0.49))]);

    let context = build_context(&memory, "topic", 0.5).await;

    assert!(context.is_empty());
}

#[tokio::test]
async fn build_context_skips_assistant_autosave_entries() {
    let memory = FakeMemory::with_entries(vec![
        memory_entry("assistant_resp_123", "internal answer", Some(0.95)),
        memory_entry("user_fact", "likes small patches", Some(0.95)),
    ]);

    let context = build_context(&memory, "patch style", 0.5).await;

    assert_eq!(context, "[Memory context]\n- user_fact: likes small patches\n\n");
}

#[tokio::test]
async fn build_context_clears_header_when_all_entries_are_autosaves() {
    let memory = FakeMemory::with_entries(vec![memory_entry(
        " ASSISTANT_RESP ",
        "internal answer",
        Some(1.0),
    )]);

    let context = build_context(&memory, "hidden", 0.5).await;

    assert!(context.is_empty());
}

#[tokio::test]
async fn build_context_returns_empty_when_recall_fails() {
    let memory = FakeMemory::failing();

    let context = build_context(&memory, "style", 0.5).await;

    assert!(context.is_empty());
}
