use super::*;
use std::collections::BTreeSet;

pub(super) fn fingerprint(project: &Value) -> Result<String> {
    let mut timeline = project["timeline"].clone();
    let object = timeline.as_object_mut().context("missing timeline")?;
    for key in ["selectedId", "selectedIds", "selected"] {
        object.remove(key);
    }
    if let Some(items) = timeline["items"].as_array_mut() {
        for item in items {
            if let Some(i) = item.as_object_mut() {
                i.remove("selected");
            }
        }
    }
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&timeline)?)))
}
impl ShijuVideoProjectCloud<'_> {
    pub(super) async fn transcript_matches(&self, project: &str) -> Result<bool> {
        let p = self.project(project).await?;
        let items = p["timeline"]["items"].as_array().context("missing source items")?;
        let mut source = items.iter().filter(|i| {
            i["kind"] == "audio"
                && i["hasTranscript"] == true
                && i["resolvedSourceAssetId"] == self.plan.asset_id
        });
        let Some(source_item) = source.next() else { return Ok(false) };
        ensure!(source.next().is_none(), "ambiguous source transcript");
        let mut offset = 0u64;
        let mut received = Vec::new();
        loop {
            let v=self.provider.delivery_call("read_transcript",json!({"editorProjectId":project,"itemId":text_field(source_item,"id")?,"maxWordsPerPhrase":1,"limit":200,"offset":offset})).await?;
            ensure!(
                v["ok"] == true && v["available"] == true && v["timeUnit"] == "milliseconds",
                "source transcript is unavailable"
            );
            let phrases = v["phrases"].as_array().context("incomplete transcript page")?;
            for phrase in phrases {
                ensure!(phrase["wordCount"] == 1, "word transcript was compacted");
                received.push(phrase.clone());
            }
            if v["hasMore"] == false {
                break;
            }
            let next = v["nextOffset"].as_u64().context("missing next transcript page")?;
            ensure!(
                next > offset && received.len() <= self.transcript.words.len(),
                "transcript pagination did not advance"
            );
            offset = next;
        }
        Ok(received.len() == self.transcript.words.len()
            && received.iter().zip(&self.transcript.words).all(|(p, w)| {
                p["text"].as_str().is_some_and(|s| caption_text(s) == caption_text(&w.text))
                    && p["start"].as_f64().is_some_and(|v| (v / 1000.0 - w.start).abs() <= 0.002)
                    && p["end"].as_f64().is_some_and(|v| (v / 1000.0 - w.end).abs() <= 0.002)
            }))
    }
    pub(super) async fn timeline_evidence(&self, project: &str) -> Result<VideoTimelineEvidence> {
        let p = self.project(project).await?;
        self.evidence(project, &p).await
    }
    pub(super) async fn evidence(&self, project: &str, p: &Value) -> Result<VideoTimelineEvidence> {
        let t = &p["timeline"];
        let fps = number(t, "fps")?;
        ensure!(fps > 0.0, "invalid timeline frame rate");
        let items = t["items"].as_array().context("missing items")?;
        let tracks = t["tracks"].as_array().context("missing tracks")?;
        let visible = |item: &Value| {
            tracks.iter().any(|track| {
                track["id"] == item["trackId"] && track["hidden"] == false && track["muted"] != true
            })
        };
        let mut windows = Vec::new();
        let mut duration: f64 = 0.0;
        let mut original_audio = false;
        for item in items {
            let start = number(item, "startFrame")? / fps;
            let length = number(item, "durationInFrames")? / fps;
            duration = duration.max(start + length);
            if item["resolvedSourceAssetId"] == self.plan.asset_id && visible(item) {
                if item["kind"] == "video" {
                    ensure!(
                        item["linkStatus"] == "linked" && item["offline"] == false,
                        "source video is offline"
                    );
                    let source_start = number(item, "sourceStartFrame")? / fps;
                    let source_duration = number(item, "sourceDurationInFrames")? / fps;
                    windows.push(VideoSourceWindow {
                        source_start,
                        source_end: source_start + source_duration,
                        timeline_start: start,
                        timeline_end: start + length,
                    });
                    original_audio |= item["volume"].as_f64().is_some_and(|v| v > 0.0);
                }
                if item["kind"] == "audio" {
                    original_audio |= item["volume"].as_f64().is_some_and(|v| v > 0.0)
                        && item["hasTranscript"] == true;
                }
            }
        }
        windows.sort_by(|a, b| a.timeline_start.total_cmp(&b.timeline_start));
        let mut readable = false;
        if !windows.is_empty() {
            let probe = self
                .provider
                .delivery_call(
                    "media_probe",
                    json!({"projectId":project,"assetId":self.plan.asset_id}),
                )
                .await?;
            let streams = probe["streams"].as_array().context("source probe missing streams")?;
            readable = streams.iter().any(|s| s["type"] == "video" || s["codec_type"] == "video")
                && number(&probe, "durationSec")? > 0.0;
            original_audio &=
                streams.iter().any(|s| s["type"] == "audio" || s["codec_type"] == "audio");
        }
        let captions_enabled = t["captionTracks"].as_array().is_some_and(|tracks| {
            tracks.iter().any(|c| c["enabled"] == true && c["hidden"] != true)
        }) || (t["captions"].is_object()
            && t["captions"]["enabled"] == true);
        let captions = if captions_enabled {
            let c = &t["_delivery_caption_evidence"];
            if c["enabled"] != true || c["complete"] != true || c["archived"] != false {
                false
            } else {
                let cards = c["cards"].as_array().context("incomplete caption cards")?;
                let actual = cards
                    .iter()
                    .filter(|c| c["hidden"] == false)
                    .filter_map(|c| c["text"].as_str())
                    .map(caption_text)
                    .collect::<String>();
                let expected = self
                    .transcript
                    .words
                    .iter()
                    .filter(|word| {
                        windows.iter().any(|w| {
                            word.start >= w.source_start - 1.0 / fps
                                && word.end <= w.source_end + 1.0 / fps
                        })
                    })
                    .map(|w| caption_text(&w.text))
                    .collect::<String>();
                !expected.is_empty()
                    && actual == expected
                    && cards.iter().filter(|c| c["hidden"] == false).all(|c| {
                        c["startMs"].as_f64().zip(c["endMs"].as_f64()).is_some_and(|(a, b)| {
                            a.is_finite()
                                && b.is_finite()
                                && a >= 0.0
                                && b > a
                                && b / 1000.0 <= duration + 1.0 / fps
                        })
                    })
            }
        } else {
            false
        };
        let mut effect_ids = BTreeSet::new();
        for effect in &self.plan.effects {
            let Some(item) = self.effect_item(p, effect)? else { continue };
            let Ok((start, end)) = video_timeline_range(&windows, effect.start, effect.end) else {
                continue;
            };
            let actual_start = number(item, "startFrame")? / fps;
            let actual_end = actual_start + number(item, "durationInFrames")? / fps;
            if (start - actual_start).abs() > 1.0 / fps
                || (end - actual_end).abs() > 1.0 / fps
                || item["offline"] != false
            {
                continue;
            }
            let valid = match self.materials.get(&effect.id) {
                Some(VideoEffectMaterial::Keyword) => {
                    item["kind"] == "text" && item["props"]["text"] == effect.anchor
                }
                Some(VideoEffectMaterial::GeneratedVideo { digest }) => {
                    let reference = self
                        .registry
                        .references
                        .get(digest)
                        .context("missing generated reference")?;
                    item["kind"] == "video"
                        && item["resolvedSourceAssetId"] == self.pool_asset(p, reference)?
                        && item["linkStatus"] == "linked"
                }
                Some(VideoEffectMaterial::MotionGraphic { template_name }) => {
                    item["kind"] == "motion-graphic" && item["name"] == *template_name
                }
                None => false,
            };
            if valid {
                effect_ids.insert(effect.id.clone());
            }
        }
        Ok(VideoTimelineEvidence {
            project_id: project.into(),
            revision: fingerprint(p)?,
            source_asset_id: self.plan.asset_id.clone(),
            windows,
            captions_present: captions,
            effect_ids,
            video_readable: readable,
            original_audio_present: original_audio,
            duration_sec: duration,
        })
    }
    pub(super) async fn preview_evidence(
        &self,
        project: &str,
        revision: &str,
    ) -> Result<VideoPreviewEvidence> {
        ensure!(
            fingerprint(&self.project(project).await?)? == revision,
            "timeline changed before preview"
        );
        let link = self
            .provider
            .delivery_call("get_share_link", json!({"projectId":project,"draft":true}))
            .await?;
        let job = text_field(&link["draft"], "jobId")?;
        let progress = self.provider.delivery_call("track_progress", json!({"jobId":job})).await?;
        ensure!(
            progress["jobId"] == job && progress["kind"] == "draft",
            "wrong preview job identity or kind"
        );
        let status = text_field(&progress, "status")?;
        ensure!(
            ["queued", "running", "succeeded", "failed", "cancelled"].contains(&status),
            "unknown preview job status"
        );
        let mut duration = 0.0;
        let mut playable = false;
        if status == "succeeded" {
            ensure!(progress["result"]["kind"] == "video", "preview job did not produce video");
            let timeline_duration = number(&progress["result"], "durationSeconds")?;
            ensure!(timeline_duration > 0.0, "empty rendered preview");
            let watch=self.provider.delivery_call("watch_timeline",json!({"projectId":project,"range":{"fromSec":0,"toSec":timeline_duration},"times":[0,timeline_duration/2.0,(timeline_duration-0.1).max(0.0)],"proxy":true})).await?;
            ensure!(watch["projectId"] == project, "wrong preview media project");
            if watch["proxy"]["status"] == "succeeded" {
                ensure!(
                    watch["proxy"]["revision"] == link["draft"]["revision"]
                        && watch["proxy"]["notAnExport"] == true,
                    "stale/non-preview media"
                );
                let media = text_field(&watch, "proxyUrl")?;
                let url = url::Url::parse(media)?;
                ensure!(
                    url.scheme() == "https"
                        && url.host_str().is_some()
                        && url.username().is_empty()
                        && url.password().is_none(),
                    "invalid preview media URL"
                );
                let probe = self
                    .provider
                    .delivery_call("media_probe", json!({"projectId":project,"url":media}))
                    .await?;
                duration = number(&probe, "durationSec")?;
                playable = duration > 0.0
                    && probe["streams"].as_array().is_some_and(|s| {
                        s.iter().any(|s| s["type"] == "video" || s["codec_type"] == "video")
                    });
            }
        }
        ensure!(
            fingerprint(&self.project(project).await?)? == revision,
            "timeline changed during preview"
        );
        Ok(VideoPreviewEvidence {
            project_id: project.into(),
            revision: revision.into(),
            job_id: job.into(),
            job_status: status.into(),
            media_playable: playable,
            duration_sec: duration,
        })
    }
}

fn caption_text(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect()
}
