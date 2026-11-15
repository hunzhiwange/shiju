//! Deterministic source-time planning units; no media is cut or retimed.
use super::transcript::VideoTranscript;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VideoSegmentOptions {
    pub target_sec: f64,
    pub max_sec: f64,
    pub context_sec: f64,
}
impl Default for VideoSegmentOptions {
    fn default() -> Self {
        Self { target_sec: 60.0, max_sec: 90.0, context_sec: 5.0 }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSegment {
    pub chunk_id: String,
    pub index: usize,
    pub core_start: f64,
    pub core_end: f64,
    /// Inclusive stable source indices, absent for a silent core.
    pub core_word_range: Option<(u64, u64)>,
    pub context_start: f64,
    pub context_end: f64,
    pub words: Vec<VideoSegmentWord>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSegmentWord {
    pub word_index: u64,
    pub start: f64,
    pub end: f64,
    pub text: String,
    /// Only core words may publish effects. Context words are read-only.
    pub is_core: bool,
}
pub fn segment_video_transcript(
    transcript: &VideoTranscript,
    options: VideoSegmentOptions,
) -> Result<Vec<VideoSegment>> {
    use anyhow::ensure;
    let VideoSegmentOptions { target_sec, max_sec, context_sec } = options;
    ensure!(
        target_sec.is_finite()
            && target_sec > 0.0
            && max_sec.is_finite()
            && max_sec >= target_sec
            && max_sec <= 90.0
            && context_sec.is_finite()
            && (0.0..=5.0).contains(&context_sec),
        "invalid video segment options"
    );
    let duration = transcript.duration_sec;
    ensure!(
        duration.is_finite()
            && duration > 0.0
            && transcript.fps.is_finite()
            && transcript.fps > 0.0
            && !transcript.asset_id.is_empty(),
        "invalid video segment media"
    );
    let mut previous_end = 0.0;
    let mut previous_index = None;
    for word in &transcript.words {
        ensure!(
            word.start.is_finite()
                && word.end.is_finite()
                && word.start >= previous_end
                && word.end > word.start
                && word.end <= duration
                && previous_index.is_none_or(|index| word.word_index > index)
                && !word.text.trim().is_empty(),
            "invalid source word coordinates"
        );
        ensure!(word.end - word.start <= max_sec, "source word exceeds segment limit");
        previous_end = word.end;
        previous_index = Some(word.word_index);
    }
    let mut boundaries = vec![0.0];
    let mut start = 0.0;
    while duration - start > target_sec {
        let target = start + target_sec;
        let limit = start + max_sec;
        let candidates = || {
            transcript.words.iter().filter(|w| w.end >= start + target_sec / 2.0 && w.end <= limit)
        };
        let nearest = |a: &&super::transcript::VideoTranscriptWord,
                       b: &&super::transcript::VideoTranscriptWord| {
            (a.end - target)
                .abs()
                .total_cmp(&(b.end - target).abs())
                .then_with(|| a.end.total_cmp(&b.end))
        };
        let sentence = candidates()
            .filter(|w| w.text.ends_with(['。', '！', '？', '.', '!', '?', ';', '；']))
            .min_by(nearest);
        let mut end = sentence.or_else(|| candidates().min_by(nearest)).map_or(target, |w| w.end);
        // Silence can be divided at the target; speech can only be divided at real boundaries.
        if let Some(word) = transcript.words.iter().find(|w| w.start < end && end < w.end) {
            end = if word.end <= limit { word.end } else { word.start };
        }
        ensure!(end > start && end <= limit, "no safe source-time segment boundary");
        boundaries.push(end);
        start = end;
    }
    if boundaries.len() > 1
        && duration - start < target_sec / 2.0
        && duration - boundaries[boundaries.len() - 2] <= max_sec
    {
        boundaries.pop();
    }
    boundaries.push(duration);
    Ok(boundaries
        .windows(2)
        .enumerate()
        .map(|(index, pair)| {
            let (core_start, core_end) = (pair[0], pair[1]);
            let context_start = (core_start - context_sec).max(0.0);
            let context_end = (core_end + context_sec).min(duration);
            let words: Vec<_> = transcript
                .words
                .iter()
                .filter(|w| w.start >= context_start && w.end <= context_end)
                .map(|w| VideoSegmentWord {
                    word_index: w.word_index,
                    start: w.start,
                    end: w.end,
                    text: w.text.clone(),
                    is_core: w.start >= core_start && w.end <= core_end,
                })
                .collect();
            let mut core = words.iter().filter(|w| w.is_core);
            let core_word_range = core
                .next()
                .map(|first| (first.word_index, core.next_back().unwrap_or(first).word_index));
            VideoSegment {
                chunk_id: format!(
                    "{}:{index}:{}:{}",
                    transcript.asset_id,
                    core_start.to_bits(),
                    core_end.to_bits()
                ),
                index,
                core_start,
                core_end,
                core_word_range,
                context_start,
                context_end,
                words,
            }
        })
        .collect())
}
