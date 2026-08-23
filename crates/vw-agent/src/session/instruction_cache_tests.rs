//! 验证指令缓存的命中、失效、并发访问与文件变化处理。

use std::sync::Mutex;

use super::instruction_cache::{
    InstructionCache, InstructionsLoaded, InstructionsLoadedHook, LoadReason,
};
use super::{InstructionSource, Resolved};

fn resolved(path: &std::path::Path, content: &str) -> Resolved {
    let filepath = path.to_string_lossy().to_string();
    Resolved {
        filepath: filepath.clone(),
        content: content.to_string(),
        sources: vec![InstructionSource { filepath, parent: None, conditions: Vec::new() }],
        diagnostics: Vec::new(),
    }
}

#[test]
fn instructions_loaded_cache_reloads_after_change_and_compaction() {
    let directory = tempfile::tempdir().expect("temporary workspace");
    let agents = directory.path().join("AGENTS.md");
    std::fs::write(&agents, "first instructions").expect("write initial instructions");
    let mut cache = InstructionCache::default();

    let (initial, initial_reason) = cache.load(directory.path(), "session-1", &agents, || {
        Some(resolved(&agents, "first instructions"))
    });
    assert_eq!(initial_reason, LoadReason::Initial);
    assert_eq!(initial.expect("initial instructions").content, "first instructions");

    let (cached, cached_reason) = cache.load(directory.path(), "session-1", &agents, || {
        panic!("stable instruction should be served from cache")
    });
    assert_eq!(cached_reason, LoadReason::CacheHit);
    assert_eq!(cached.expect("cached instructions").content, "first instructions");

    let (other_session, other_session_reason) =
        cache.load(directory.path(), "session-2", &agents, || {
            Some(resolved(&agents, "isolated session instructions"))
        });
    assert_eq!(other_session_reason, LoadReason::Initial);
    assert_eq!(
        other_session.expect("isolated session instructions").content,
        "isolated session instructions"
    );

    cache.clear_session(directory.path(), "session-1");
    let (cleared, cleared_reason) = cache.load(directory.path(), "session-1", &agents, || {
        Some(resolved(&agents, "reloaded after clear"))
    });
    assert_eq!(cleared_reason, LoadReason::Cleared);
    assert_eq!(cleared.expect("cleared instructions").content, "reloaded after clear");

    std::fs::write(&agents, "changed instructions with a distinct length")
        .expect("rewrite instructions");
    let (changed, changed_reason) = cache.load(directory.path(), "session-1", &agents, || {
        Some(resolved(&agents, "changed instructions with a distinct length"))
    });
    assert_eq!(changed_reason, LoadReason::FileChanged);
    assert_eq!(
        changed.expect("changed instructions").content,
        "changed instructions with a distinct length"
    );

    cache.invalidate_for_compaction(directory.path(), "session-1");
    let (compacted, compacted_reason) = cache.load(directory.path(), "session-1", &agents, || {
        Some(resolved(&agents, "reloaded after compaction"))
    });
    assert_eq!(compacted_reason, LoadReason::Compaction);
    assert_eq!(compacted.expect("compacted instructions").content, "reloaded after compaction");
}

#[derive(Default)]
struct RecordingHook {
    events: Mutex<Vec<InstructionsLoaded>>,
    fail: bool,
}

impl InstructionsLoadedHook for RecordingHook {
    fn instructions_loaded(&self, event: &InstructionsLoaded) -> Result<(), String> {
        self.events.lock().expect("record event").push(event.clone());
        if self.fail { Err("observer unavailable".to_string()) } else { Ok(()) }
    }
}

#[test]
fn instructions_loaded_hook_receives_resolved_sources() {
    let directory = tempfile::tempdir().expect("temporary workspace");
    let agents = directory.path().join("AGENTS.md");
    std::fs::write(&agents, "instructions").expect("write instructions");
    let mut cache = InstructionCache::default();
    let hook = RecordingHook { fail: true, ..Default::default() };

    let (loaded, reason) = cache.load_with_hook(
        directory.path(),
        "session-1",
        &agents,
        || Some(resolved(&agents, "instructions")),
        Some(&hook),
    );

    assert_eq!(reason, LoadReason::Initial);
    assert_eq!(loaded.expect("instructions remain available").content, "instructions");
    let events = hook.events.lock().expect("read events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].reason, LoadReason::Initial);
    assert_eq!(events[0].sources[0].filepath, agents.to_string_lossy().to_string());
    assert_eq!(events[0].files, vec![agents.to_string_lossy().to_string()]);
}
