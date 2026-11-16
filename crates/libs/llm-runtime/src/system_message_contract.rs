//! Assertions over real host-rendered system messages, without a replacement renderer.

pub fn merged_sources(system: &[String]) {
    assert_eq!(system.len(), 1);
    let joined = &system[0];
    let agent_idx = joined.find("agent prompt").expect("agent prompt should be present");
    let session_idx = joined.find("session system").expect("session system should be present");
    let user_idx = joined.find("user system").expect("user system should be present");
    assert!(agent_idx < session_idx);
    assert!(session_idx < user_idx);
    assert!(!joined.contains("\n \n"));
}

pub fn unified_once(without_codex: &str, with_codex: &str, marker: &str) {
    let without_count = without_codex.matches(marker).count();
    let with_count = with_codex.matches(marker).count();
    eprintln!("unified marker count without_codex={without_count} with_codex={with_count}");
    eprintln!(
        "joined lengths without_codex={} with_codex={}",
        without_codex.len(),
        with_codex.len()
    );

    assert_eq!(without_count, 1, "non-codex path should include unified prompt once");
    assert_eq!(with_count, 1, "codex path must not duplicate unified prompt");
    assert!(with_codex.contains("agent prompt"));
    assert!(with_codex.contains("session system"));
}

pub fn rendered_runtime_once(joined: &str, marker: &str) {
    assert_eq!(
        joined.matches(marker).count(),
        1
    );
    assert!(joined.contains("deepseek/deepseek-v4-flash"));
    assert!(joined.contains("/workspace/project"));
    assert!(joined.contains("## Task Tracking"));
    assert!(joined.contains("## Subagents"));
}

pub fn provider_only(system: Vec<String>, provider_prompt: String) {
    assert_eq!(system, vec![provider_prompt]);
}
