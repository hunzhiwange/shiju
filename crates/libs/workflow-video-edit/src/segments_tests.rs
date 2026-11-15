use super::{segments::*, transcript::*};
fn synthetic(duration: f64, step: f64, sentence: bool) -> VideoTranscript {
    VideoTranscript {
        asset_id: "synthetic".into(),
        duration_sec: duration,
        fps: 30000.0 / 1001.0,
        text: "合成转写".into(),
        source: "synthetic-test".into(),
        diagnostics: vec![],
        words: (0..(duration / step) as u64)
            .map(|i| VideoTranscriptWord {
                word_index: i * 2,
                text: if sentence && i % 10 == 9 { "重复。" } else { "重复" }.into(),
                start: i as f64 * step,
                end: (i as f64 * step + step / 2.0).min(duration),
            })
            .collect(),
    }
}
fn verify(t: &VideoTranscript) -> Vec<VideoSegment> {
    let s = segment_video_transcript(t, VideoSegmentOptions::default()).unwrap();
    assert!(!s.is_empty());
    assert_eq!(s[0].core_start, 0.0);
    assert_eq!(s.last().unwrap().core_end, t.duration_sec);
    let mut owned = vec![];
    for (i, seg) in s.iter().enumerate() {
        assert_eq!(seg.index, i);
        assert!(seg.core_end > seg.core_start && seg.core_end - seg.core_start <= 90.0);
        if i > 0 {
            assert_eq!(s[i - 1].core_end, seg.core_start);
        }
        assert!(seg.context_start >= 0.0 && seg.context_start >= seg.core_start - 5.0);
        assert!(seg.context_end <= t.duration_sec && seg.context_end <= seg.core_end + 5.0);
        let core: Vec<_> = seg.words.iter().filter(|w| w.is_core).collect();
        assert_eq!(
            seg.core_word_range,
            core.first().zip(core.last()).map(|(a, b)| (a.word_index, b.word_index))
        );
        for w in &seg.words {
            let original = t.words.iter().find(|x| x.word_index == w.word_index).unwrap();
            assert_eq!((w.start, w.end, &w.text), (original.start, original.end, &original.text));
            assert_eq!(w.is_core, w.start >= seg.core_start && w.end <= seg.core_end);
            if w.is_core {
                owned.push(w.word_index);
            }
        }
    }
    assert_eq!(owned, t.words.iter().map(|w| w.word_index).collect::<Vec<_>>());
    assert_eq!(s, segment_video_transcript(t, VideoSegmentOptions::default()).unwrap());
    s
}
#[test]
fn video_edit_segment_five_and_ten_minutes_cover_source_once() {
    for duration in [300.0, 600.0] {
        assert!(verify(&synthetic(duration, 1.0, true)).len() >= 5);
    }
}
#[test]
fn video_edit_segment_short_and_tail_merge() {
    assert_eq!(verify(&synthetic(8.0, 1.0, true)).len(), 1);
    let s = verify(&synthetic(73.0, 1.0, false));
    assert_eq!(s.len(), 1);
}
#[test]
fn video_edit_segment_long_sentence_and_boundary_words() {
    let t = synthetic(240.0, 0.7, false);
    let s = verify(&t);
    assert!(s.len() > 2);
    assert!(s.iter().flat_map(|s| &s.words).any(|w| !w.is_core));
}
#[test]
fn video_edit_segment_silent_intervals_preserve_coverage() {
    let mut t = synthetic(600.0, 1.0, true);
    t.words.retain(|w| w.start < 10.0 || w.start > 500.0);
    assert!(verify(&t).iter().any(|s| s.core_word_range.is_none()));
    t.words.clear();
    verify(&t);
}
#[test]
fn video_edit_segment_invalid_contract_is_rejected() {
    let mut t = synthetic(100.0, 1.0, true);
    t.words[1].word_index = 0;
    assert!(segment_video_transcript(&t, VideoSegmentOptions::default()).is_err());
}
#[test]
fn video_edit_segment_rejects_invalid_options_and_uncuttable_word() {
    let t = synthetic(100.0, 1.0, false);
    for options in [
        VideoSegmentOptions { target_sec: 0.0, ..Default::default() },
        VideoSegmentOptions { max_sec: 91.0, ..Default::default() },
        VideoSegmentOptions { context_sec: 6.0, ..Default::default() },
        VideoSegmentOptions { target_sec: f64::NAN, ..Default::default() },
    ] {
        assert!(segment_video_transcript(&t, options).is_err());
    }
    let mut t = t;
    t.words.truncate(1);
    t.words[0].end = 95.0;
    assert!(segment_video_transcript(&t, VideoSegmentOptions::default()).is_err());
}
#[test]
fn video_edit_segment_prefers_sentence_and_preserves_boundary_ownership() {
    let mut t = synthetic(200.0, 1.0, false);
    t.words[64].text = "句末。".into();
    let s = verify(&t);
    assert_eq!(s[0].core_end, 64.5);
    assert!(s[0].words.iter().any(|w| w.word_index == 128 && w.is_core));
    assert!(s[1].words.iter().any(|w| w.word_index == 128 && !w.is_core));
}
