use super::compaction_replay::{encode_summary, replay_text};

#[test]
fn aggregation_t25_replay_handles_legacy_and_opaque_payloads() {
    let replay = replay_text("ccswitch-compaction-v1:5bey5L-d5a2Y").unwrap();
    assert_eq!(encode_summary("已保存"), "ccswitch-compaction-v1:5bey5L-d5a2Y");
    assert!(replay.starts_with("Another language model started"));
    assert!(replay.ends_with("\n已保存"));
    assert_eq!(replay_text(""), None);

    for payload in ["opaque-foreign", "ccswitch-compaction-v1:***", "ccswitch-compaction-v1:_w"] {
        assert_eq!(
            replay_text(payload).as_deref(),
            Some(
                "[earlier conversation was compacted; the summary is stored in a format this model cannot read]"
            )
        );
    }
}
