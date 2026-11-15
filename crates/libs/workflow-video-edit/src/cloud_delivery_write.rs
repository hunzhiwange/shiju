use super::*;

impl ShijuVideoProjectCloud<'_> {
    async fn write_effect(
        &self,
        project: &str,
        effect: &VideoEditEffect,
        material: &VideoEffectMaterial,
        start: f64,
        end: f64,
    ) -> Result<VideoWriteStatus> {
        let mut p = self.project(project).await?;
        let name = track_name(effect);
        let tracks = p["timeline"]["tracks"].as_array().context("missing tracks")?;
        let matches: Vec<_> = tracks.iter().filter(|t| t["name"] == name).collect();
        ensure!(matches.len() <= 1, "ambiguous managed effect track");
        if matches.is_empty() {
            self.mutation("edit_track",json!({"editorProjectId":project,"action":"create","json":{"trackType":"video","name":name}})).await?;
            p = self.project(project).await?;
        }
        let track = p["timeline"]["tracks"]
            .as_array()
            .context("missing tracks")?
            .iter()
            .find(|t| t["name"] == name)
            .context("effect track not created")?;
        let track = text_field(track, "id")?;
        ensure!(
            self.effect_item(&p, effect)?.is_none(),
            "effect track is occupied; reconcile before retry"
        );
        let fps = number(&p["timeline"], "fps")?;
        let from = (start * fps).round();
        let duration = ((end - start) * fps).round();
        ensure!(duration >= 1.0, "effect shorter than one frame");
        match material{
            VideoEffectMaterial::Keyword=>self.mutation("edit_item",json!({"editorProjectId":project,"adds":[{"type":"text","text":effect.anchor,"name":effect.id,"track":track,"fromFrame":from,"durationInFrames":duration}]})).await,
            VideoEffectMaterial::GeneratedVideo{digest}=>{
                let reference=self.registry.references.get(digest).context("missing generated reference")?;
                let asset=self.pool_asset(&p,reference)?;
                self.mutation("edit_item",json!({"editorProjectId":project,"adds":[{"type":"video","assetId":asset,"trackId":track,"fromFrame":from,"durationInFrames":duration}]})).await
            }
            VideoEffectMaterial::MotionGraphic{template_name}=>{
                self.mutation("add_motion_graphic",json!({"editorProjectId":project,"templateName":template_name,"track":track,"startFrame":from})).await?;
                let p=self.project(project).await?;
                let item=self.effect_item(&p,effect)?.context("motion graphic was not placed")?;
                ensure!(item["name"]==*template_name && item["kind"]=="motion-graphic","wrong motion graphic template");
                if number(item,"durationInFrames")?==duration{return Ok(VideoWriteStatus::Applied)}
                self.mutation("edit_item",json!({"editorProjectId":project,"updates":[{"type":"motion-graphic","itemId":text_field(item,"id")?,"durationInFrames":duration}]})).await
            }
        }
    }
}
#[async_trait::async_trait]
impl VideoProjectCloud for ShijuVideoProjectCloud<'_> {
    async fn read_timeline(&self, project: &str) -> Result<VideoTimelineEvidence> {
        self.timeline_evidence(project).await
    }
    async fn write(
        &self,
        project: &str,
        _key: &str,
        write: &VideoProjectWrite,
    ) -> Result<VideoWriteStatus> {
        match write{
            VideoProjectWrite::Import{reference}=>{
                let v=self.provider.delivery_call("import_media",json!({"projectId":project,"action":"from_url","urls":[{"url":reference,"name":import_name(reference)}]})).await?;
                if v["status"]=="running"{return Ok(VideoWriteStatus::Unknown)}
                ensure!(v["ok"]==true && v["assetIds"].as_array().is_some_and(|a|a.len()==1),"material import not complete");
                self.pool_asset(&self.project(project).await?,reference)?;Ok(VideoWriteStatus::Applied)
            }
            VideoProjectWrite::Original{asset_id}=>self.mutation("edit_item",json!({"editorProjectId":project,"adds":[{"type":"video","assetId":asset_id,"track":"V1","fromFrame":0}]})).await,
            VideoProjectWrite::Transcript { asset_id, words } => {
                let status=self.mutation("manage_transcript",json!({"projectId":project,"action":"import","assetId":asset_id,"words":words.iter().map(|w|json!({"text":w.text,"start":w.start,"end":w.end})).collect::<Vec<_>>()})).await?;
                if matches!(status,VideoWriteStatus::Applied){ensure!(self.transcript_matches(project).await?,"imported transcript content/timing mismatch");}
                Ok(status)
            }
            VideoProjectWrite::Cleanup=>self.mutation("clean_script",json!({"projectId":project,"mode":"safe"})).await,
            VideoProjectWrite::Captions=>self.mutation("edit_captions",json!({"projectId":project,"action":"enable"})).await,
            VideoProjectWrite::Effect{effect,material,start,end}=>self.write_effect(project,effect,material,*start,*end).await,
        }
    }
    async fn reconcile(&self, project: &str, entry: &VideoWriteEntry) -> Result<VideoWriteStatus> {
        let p = self.project(project).await?;
        let evidence = self.evidence(project, &p).await?;
        let applied = match &entry.write {
            VideoProjectWrite::Import { reference } => self.pool_asset(&p, reference).is_ok(),
            VideoProjectWrite::Original { .. } => {
                !evidence.windows.is_empty() && evidence.video_readable
            }
            VideoProjectWrite::Transcript { .. } => self.transcript_matches(project).await?,
            VideoProjectWrite::Captions => evidence.captions_present,
            VideoProjectWrite::Effect { effect, .. } => evidence.effect_ids.contains(&effect.id),
            // Cleanup acceptance cannot be inferred from the lack of an error, or
            // from a changed duration (which may be somebody else's edit).
            VideoProjectWrite::Cleanup => false,
        };
        Ok(if applied { VideoWriteStatus::Applied } else { VideoWriteStatus::Unknown })
    }
    async fn preview(&self, project: &str, revision: &str) -> Result<VideoPreviewEvidence> {
        self.preview_evidence(project, revision).await
    }
    async fn viewer_url(&self, project: &str) -> Result<String> {
        let v = self.provider.delivery_call("get_share_link", json!({"projectId":project})).await?;
        ensure!(v["kind"] == "live", "viewer is not a live preview");
        Ok(text_field(&v, "url")?.into())
    }
}
