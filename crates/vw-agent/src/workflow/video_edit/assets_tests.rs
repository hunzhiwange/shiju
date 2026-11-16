use super::{assets::*, manifest::*, planning::*};
use crate::global_database::GlobalDatabase;
use std::collections::BTreeMap;

pub(super) fn fixture(
    n: usize,
) -> (tempfile::TempDir, VideoEditManifestStore, VideoEditRunManifest, Vec<VideoAssetRequest>) {
    let dir = tempfile::tempdir().unwrap();
    let store =
        VideoEditManifestStore::new(GlobalDatabase::new(dir.path().join("db")).unwrap()).unwrap();
    let mut run = super::manifest_tests::candidate();
    run.project_id = Some("project".into());
    let run = store.claim(run, "asset-test", chrono::Utc::now().timestamp_millis()).unwrap();
    let mut recipes = BTreeMap::new();
    let mut effects = Vec::new();
    for i in 0..n {
        let description = format!("素材{i}");
        recipes.insert(
            description.clone(),
            VideoAssetRecipe {
                prompt: format!("landscape {i}"),
                kind: "environment".into(),
                model: "wan3.0-video".into(),
                duration_sec: 5,
                ratio: "9:16".into(),
                resolution: "720P".into(),
            },
        );
        effects.push(VideoEditEffect {
            id: i.to_string(),
            chunk_id: "chunk".into(),
            word_index: i as u64,
            end_word_index: i as u64,
            anchor: "word".into(),
            start: i as f64,
            end: i as f64 + 1.0,
            visual_type: VisualType::BRoll,
            assets: vec![description.clone(), description],
        });
    }
    let plan = VideoEditPlan { asset_id: "source".into(), duration_sec: n as f64, effects };
    let requests = video_asset_requests(&plan, "project", &recipes).unwrap();
    assert_eq!(requests.len(), n, "duplicate materials must collapse");
    (dir, store, run, requests)
}
