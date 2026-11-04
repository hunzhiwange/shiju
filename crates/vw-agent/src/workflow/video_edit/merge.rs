//! Validate every chunk before publishing one source-time plan.
use super::{planning::*, segments::VideoSegment, transcript::VideoTranscript};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub fn merge_video_edit_plans(
    transcript: &VideoTranscript,
    segments: &[VideoSegment],
    plans: Vec<VideoChunkPlan>,
    requirements: &VideoPlanRequirements,
) -> Result<VideoEditPlan> {
    validate_segments(transcript, segments)?;
    ensure!(requirements.max_effects_per_word > 0, "invalid effect density limit");
    let chunks: BTreeMap<_, _> = segments.iter().map(|s| (s.chunk_id.as_str(), s)).collect();
    let words: BTreeMap<_, _> = transcript.words.iter().map(|w| (w.word_index, w)).collect();
    let mut received = BTreeSet::new();
    let mut effects = BTreeMap::<String, VideoEditEffect>::new();
    let mut published = BTreeSet::new();
    for plan in plans {
        let chunk = chunks.get(plan.chunk_id.as_str()).context("unknown plan chunk")?;
        ensure!(received.insert(plan.chunk_id.clone()), "duplicate chunk {}", plan.chunk_id);
        let mut local_ids = BTreeSet::new();
        for effect in plan.effects {
            let id = &effect.id;
            ensure!(
                !id.trim().is_empty() && local_ids.insert(id.clone()),
                "chunk {}: duplicate/empty effect ID",
                chunk.chunk_id
            );
            let owner =
                chunks.get(effect.chunk_id.as_str()).context("effect has unknown owning chunk")?;
            let first = words.get(&effect.word_index).context("unknown anchor word index")?;
            let last = words.get(&effect.end_word_index).context("unknown end word index")?;
            ensure!(
                effect.word_index <= effect.end_word_index
                    && owner
                        .core_word_range
                        .is_some_and(|(a, b)| a <= effect.word_index && effect.end_word_index <= b)
                    && chunk.words.iter().any(|w| w.word_index == effect.word_index)
                    && chunk.words.iter().any(|w| w.word_index == effect.end_word_index),
                "chunk {}: effect {id} outside core ownership or visible context",
                chunk.chunk_id
            );
            ensure!(
                effect.anchor == first.text,
                "chunk {}: effect {id} anchor mismatch",
                chunk.chunk_id
            );
            ensure!(
                effect.start.is_finite()
                    && effect.end.is_finite()
                    && effect.start == first.start
                    && effect.end == last.end
                    && effect.start >= owner.core_start
                    && effect.end <= owner.core_end
                    && effect.start >= 0.0
                    && effect.start < effect.end
                    && effect.end <= transcript.duration_sec,
                "chunk {}: effect {id} invalid absolute time/word coordinates",
                chunk.chunk_id
            );
            ensure!(
                !effect.assets.is_empty() && effect.assets.iter().all(|s| !s.trim().is_empty()),
                "chunk {}: effect {id} missing asset description",
                chunk.chunk_id
            );
            if chunk.chunk_id == effect.chunk_id {
                published.insert(id.clone());
            }
            if let Some(existing) = effects.get(id) {
                ensure!(existing == &effect, "chunk {}: conflicting effect {id}", chunk.chunk_id);
            } else {
                effects.insert(id.clone(), effect);
            }
        }
    }
    ensure!(received.len() == chunks.len(), "missing chunk plan; refusing partial video plan");
    ensure!(
        effects.keys().all(|id| published.contains(id)),
        "context effect has no core owner publication"
    );
    let mut effects: Vec<_> = effects.into_values().collect();
    effects.sort_by(|a, b| {
        a.start.total_cmp(&b.start).then(a.word_index.cmp(&b.word_index)).then(a.id.cmp(&b.id))
    });
    let mut types = BTreeSet::new();
    let mut density = BTreeMap::<u64, usize>::new();
    let mut placements = BTreeSet::new();
    for effect in &effects {
        types.insert(effect.visual_type);
        ensure!(
            placements.insert((effect.word_index, effect.visual_type)),
            "duplicate visual at source word {}",
            effect.word_index
        );
        for word in transcript
            .words
            .iter()
            .filter(|w| effect.word_index <= w.word_index && w.word_index <= effect.end_word_index)
        {
            let count = density.entry(word.word_index).or_default();
            *count += 1;
            ensure!(
                *count <= requirements.max_effects_per_word,
                "visual density exceeded at word {}",
                word.word_index
            );
        }
    }
    ensure!(
        requirements.required_types.iter().all(|t| types.contains(t)),
        "missing globally required visual type"
    );
    Ok(VideoEditPlan {
        asset_id: transcript.asset_id.clone(),
        duration_sec: transcript.duration_sec,
        effects,
    })
}

/// Validate trusted source coordinates as well as deserialized segment data.
pub(super) fn validate_segments(
    transcript: &VideoTranscript,
    segments: &[VideoSegment],
) -> Result<()> {
    ensure!(!segments.is_empty(), "missing video segments");
    // Reuse source validation without changing caller-selected segment boundaries.
    super::segments::segment_video_transcript(transcript, Default::default())?;
    let mut end = 0.0;
    let mut ids = BTreeSet::new();
    let mut owned = BTreeSet::new();
    for (index, segment) in segments.iter().enumerate() {
        ensure!(
            segment.index == index
                && ids.insert(&segment.chunk_id)
                && !segment.chunk_id.is_empty()
                && segment.core_start == end
                && segment.core_end.is_finite()
                && segment.core_end > end
                && segment.core_end <= transcript.duration_sec
                && segment.context_start.is_finite()
                && segment.context_end.is_finite()
                && segment.context_start >= 0.0
                && segment.context_start <= segment.core_start
                && segment.context_end >= segment.core_end
                && segment.context_end <= transcript.duration_sec,
            "invalid segment coordinates or identity"
        );
        let expected: Vec<_> = transcript
            .words
            .iter()
            .filter(|w| w.start >= segment.context_start && w.end <= segment.context_end)
            .collect();
        ensure!(expected.len() == segment.words.len(), "segment context words missing");
        let mut core = Vec::new();
        for (source, word) in expected.iter().zip(&segment.words) {
            let is_core = source.start >= segment.core_start && source.end <= segment.core_end;
            ensure!(
                source.word_index == word.word_index
                    && source.text == word.text
                    && source.start == word.start
                    && source.end == word.end
                    && is_core == word.is_core,
                "segment source word mismatch"
            );
            if is_core {
                ensure!(owned.insert(word.word_index), "duplicate core word ownership");
                core.push(word.word_index);
            }
        }
        let range = core.first().zip(core.last()).map(|(a, b)| (*a, *b));
        ensure!(range == segment.core_word_range, "segment core word range mismatch");
        end = segment.core_end;
    }
    ensure!(
        end == transcript.duration_sec && owned.len() == transcript.words.len(),
        "incomplete source word ownership"
    );
    Ok(())
}
