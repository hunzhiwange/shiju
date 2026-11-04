use super::{merge::*, planning::*, segments::*, transcript::*};
pub(super) fn source() -> (VideoTranscript, Vec<VideoSegment>, VideoPlanRequirements) {
    let t = VideoTranscript {
        asset_id: "asset".into(),
        duration_sec: 360.0,
        fps: 25.0,
        text: "same ".repeat(6),
        words: (0..6)
            .map(|i| VideoTranscriptWord {
                word_index: i,
                text: "same".into(),
                start: i as f64 * 60.0,
                end: i as f64 * 60.0 + 1.0,
            })
            .collect(),
        source: "fixture".into(),
        diagnostics: vec![],
    };
    let segments = segment_video_transcript(
        &t,
        VideoSegmentOptions { target_sec: 60.0, max_sec: 60.0, context_sec: 5.0 },
    )
    .unwrap();
    (
        t,
        segments,
        VideoPlanRequirements {
            instructions: "test".into(),
            required_types: vec![VisualType::Keyword, VisualType::BRoll, VisualType::MotionGraphic],
            max_effects_per_word: 1,
        },
    )
}
pub(super) fn effect(i: u64, visual_type: VisualType) -> VideoEditEffect {
    let (_, s, _) = source();
    VideoEditEffect {
        id: format!("effect-{i}"),
        chunk_id: s[i as usize].chunk_id.clone(),
        word_index: i,
        end_word_index: i,
        anchor: "same".into(),
        start: i as f64 * 60.0,
        end: i as f64 * 60.0 + 1.0,
        visual_type,
        assets: vec!["make a visual".into()],
    }
}
fn plans() -> Vec<VideoChunkPlan> {
    (0..6)
        .map(|i| {
            let e = effect(
                i,
                [VisualType::Keyword, VisualType::BRoll, VisualType::MotionGraphic][i as usize % 3],
            );
            VideoChunkPlan { chunk_id: e.chunk_id.clone(), effects: vec![e] }
        })
        .collect()
}
#[test]
fn video_edit_plan_six_reverse_results_use_word_indices_and_source_order() {
    let (t, s, r) = source();
    let mut p = plans();
    p.reverse();
    let merged = merge_video_edit_plans(&t, &s, p, &r).unwrap();
    assert_eq!(
        merged.effects.iter().map(|e| e.word_index).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5]
    );
}
#[test]
fn video_edit_plan_rejects_invalid_times_anchors_assets_and_density() {
    let (t, s, r) = source();
    for case in 0..7 {
        let mut p = plans();
        match case {
            0 => p[0].effects[0].start = f64::NAN,
            1 => p[0].effects[0].end = 61.0,
            2 => p[0].effects[0].assets.clear(),
            3 => p[0].effects[0].anchor = "rewritten".into(),
            4 => p[0].effects[0].word_index = 1,
            5 => {
                let mut e = p[0].effects[0].clone();
                e.id = "extra".into();
                p[0].effects.push(e);
            }
            _ => p[0].effects[0].end = -1.0,
        }
        assert!(merge_video_edit_plans(&t, &s, p, &r).is_err(), "case {case}");
    }
}
#[test]
fn video_edit_plan_context_duplicate_is_deduplicated_but_conflict_fails() {
    let (t, s, r) = source();
    let mut p = plans();
    let e = p[1].effects[0].clone();
    p[0].effects.push(e.clone());
    assert_eq!(merge_video_edit_plans(&t, &s, p.clone(), &r).unwrap().effects.len(), 6);
    p[0].effects.last_mut().unwrap().assets = vec!["conflict".into()];
    assert!(merge_video_edit_plans(&t, &s, p, &r).is_err());
}
#[test]
fn video_edit_plan_missing_chunk_or_global_type_fails() {
    let (t, s, r) = source();
    let mut p = plans();
    p.pop();
    assert!(merge_video_edit_plans(&t, &s, p, &r).is_err());
    let mut p = plans();
    for c in &mut p {
        c.effects[0].visual_type = VisualType::Keyword;
    }
    assert!(merge_video_edit_plans(&t, &s, p, &r).is_err());
}
#[test]
fn video_edit_plan_accepts_source_boundaries_without_repeating_all_types_per_chunk() {
    let (mut t, _, r) = source();
    t.words.last_mut().unwrap().end = t.duration_sec;
    let s = segment_video_transcript(
        &t,
        VideoSegmentOptions { target_sec: 60.0, max_sec: 60.0, context_sec: 5.0 },
    )
    .unwrap();
    let mut p = plans();
    // Reassign deterministic chunk identities after changing the final source word span.
    for (chunk, segment) in p.iter_mut().zip(&s) {
        chunk.chunk_id = segment.chunk_id.clone();
        chunk.effects[0].chunk_id = segment.chunk_id.clone();
    }
    p.last_mut().unwrap().effects[0].end = t.duration_sec;
    let plan = merge_video_edit_plans(&t, &s, p, &r).unwrap();
    assert_eq!(plan.effects.first().unwrap().start, 0.0);
    assert_eq!(plan.effects.last().unwrap().end, 360.0);
}
#[test]
fn video_edit_plan_rejects_orphan_context_duplicate_type_and_tampered_segment() {
    let (t, s, mut r) = source();
    let mut p = plans();
    let mut e = p[1].effects.pop().unwrap();
    e.id = "orphan".into();
    p[0].effects.push(e);
    assert!(merge_video_edit_plans(&t, &s, p, &r).is_err());
    let mut p = plans();
    let mut e = p[0].effects[0].clone();
    e.id = "different-id".into();
    p[0].effects.push(e);
    r.max_effects_per_word = 3;
    assert!(merge_video_edit_plans(&t, &s, p, &r).is_err());
    let mut s = s;
    s[0].words[0].text = "forged".into();
    assert!(merge_video_edit_plans(&t, &s, plans(), &r).is_err());
}
