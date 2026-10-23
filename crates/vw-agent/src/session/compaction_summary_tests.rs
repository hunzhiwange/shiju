use super::*;

#[test]
fn compaction_chunks_cover_multibyte_source_without_gaps_and_carry_summary() {
    let transcript = format!("START{}FINAL_DECISION", "中文😀".repeat(10_000));
    let limit = ModelLimit { context: 8192, input: Some(4096), output: 1024 };
    let mut chunks = SummaryChunks::new(&transcript, Some(&limit)).unwrap();
    let mut recovered = String::new();
    let mut calls = 0;
    while let Some(prompt) = chunks.next_prompt() {
        assert!(prompt.len() <= 4096);
        if calls > 0 {
            assert!(prompt.contains("remembered decision"));
        }
        let fragment = prompt.split_once("Next conversation fragment:\n").unwrap().1;
        recovered.push_str(fragment.strip_suffix("\n---").unwrap());
        chunks.accept("remembered decision".to_string()).unwrap();
        calls += 1;
    }
    assert!(calls > 2);
    assert_eq!(recovered, transcript);
    assert_eq!(chunks.finish().unwrap(), "remembered decision");
}

#[test]
fn compaction_chunks_reject_empty_oversized_and_incomplete_summaries() {
    let transcript = "x".repeat(40_000);
    let mut chunks = SummaryChunks::new(&transcript, None).unwrap();
    assert!(chunks.next_prompt().is_some());
    assert!(chunks.accept("  ".to_string()).unwrap_err().contains("empty"));
    assert!(chunks.accept("x".repeat(32_000)).unwrap_err().contains("budget"));
    chunks.accept("valid".to_string()).unwrap();
    assert!(chunks.finish().unwrap_err().contains("all history"));
}

#[test]
fn compaction_chunks_reject_unusable_model_window() {
    let limit = ModelLimit { context: 4096, input: None, output: 4096 };
    assert!(SummaryChunks::new("history", Some(&limit)).is_err());
}
