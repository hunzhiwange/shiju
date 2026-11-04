//! Typed preparation operations; never exposed as arbitrary workflow tool names.
use super::*;
use crate::workflow::video_edit::cloud_failure::{CloudFailurePhase, CloudOperationFailure};
use crate::workflow::video_edit::{manifest::*, prepare::*};
use anyhow::{Context, Result, bail, ensure};

impl ShijuCloudVideoToolProvider {
    pub(super) async fn prepare_call(&self, operation: &str, args: Value) -> Result<Value> {
        let preflight = async {
            // Reload the official schema before each operation, fail closed on drift.
            ensure!(
                [
                    "get_my_permissions",
                    "create_project",
                    "list_projects",
                    "import_media",
                    "media_probe",
                    "media_audio_extract",
                    "asr_transcribe"
                ]
                .contains(&operation),
                "unsupported preparation operation"
            );
            let text = self
                .occ_capture(&["schema".into(), operation.into()])
                .await
                .map_err(anyhow::Error::msg)?;
            let schema: Value = serde_json::from_str(&text)?;
            ensure!(
                schema["name"] == operation && schema["inputSchema"]["type"] == "object",
                "preparation schema identity changed"
            );
            let properties = schema["inputSchema"]["properties"]
                .as_object()
                .context("missing preparation schema properties")?;
            for (key, value) in args.as_object().context("invalid typed arguments")? {
                let property = properties.get(key).context("preparation schema field changed")?;
                let valid = match property["type"].as_str() {
                    Some("string") => value.is_string(),
                    Some("number") => value.is_number(),
                    Some("integer") => value.is_i64(),
                    Some("boolean") => value.is_boolean(),
                    Some("array") => value.is_array(),
                    _ => false,
                };
                ensure!(valid, "preparation schema type changed");
                if let Some(choices) = property["enum"].as_array() {
                    ensure!(choices.contains(value), "preparation schema enum changed");
                }
            }
            if let Some(required) = schema["inputSchema"]["required"].as_array() {
                for key in required {
                    ensure!(
                        args.get(key.as_str().context("invalid required field")?).is_some(),
                        "preparation schema requires unsupported field"
                    );
                }
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        preflight.map_err(|_| CloudOperationFailure::not_sent(CloudFailurePhase::Preflight))?;
        let value =
            self.memory_exchange(json!({"kind":"call","operation":operation,"args":args})).await?;
        if !value.is_object() || value.get("error").is_some() {
            return Err(CloudOperationFailure::unknown_response(&value).into());
        }
        Ok(value)
    }
}
fn id(value: &Value, key: &str) -> Result<String> {
    let s = value[key]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 200)
        .context("missing cloud identity")?;
    Ok(s.into())
}
#[async_trait::async_trait]
impl VideoPrepareCloud for ShijuCloudVideoToolProvider {
    fn context(&self) -> &ToolRuntimeContext {
        &self.context
    }
    async fn choose_scope(&self, scopes: &[CloudVideoScope]) -> Result<CloudVideoScope> {
        if let Some(allowed) = &self.allowed_tools {
            ensure!(
                allowed.as_array().is_some_and(|a| a
                    .iter()
                    .any(|v| v.as_str() == Some(crate::tools::ASK_USER_QUESTION_TOOL_ID))),
                "cloud scope choice requires authorized native question capability"
            );
        }
        select_scope(scopes, None, &self.context).await
    }
    async fn authorized_scopes(&self) -> Result<Vec<CloudVideoScope>> {
        // Native tenant IDs partition local storage; cloud identity belongs to the
        // current visible client's login, which may use a different company ID.
        let company = self
            .cli(&self.hub_root.join("scripts/shiju"), &["company:info".into()])
            .await
            .map_err(anyhow::Error::msg)?;
        let company_id = id(&company, "company_id")?;
        ensure!(
            company_id.bytes().all(|byte| byte.is_ascii_digit()),
            "invalid authenticated cloud company"
        );
        let response = self.prepare_call("get_my_permissions", json!({})).await?;
        let member = &response["member"];
        ensure!(
            member["companyId"].as_str() == Some(company_id.as_str())
                && member["status"] == "active",
            "cloud tenant or membership mismatch"
        );
        let permissions = member["effectivePermissions"]
            .as_array()
            .context("missing effective cloud permissions")?;
        for permission in ["project.read", "project.create", "material.read", "material.create"] {
            ensure!(
                permissions.iter().any(|p| p.as_str() == Some(permission)),
                "missing cloud preparation permission"
            );
        }
        let mut scopes = Vec::new();
        for field in ["brandIds", "storeIds"] {
            for value in member[field].as_array().context("missing authorized cloud scopes")? {
                let value = value
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .context("invalid cloud scope")?
                    .to_owned();
                let scope = if field == "brandIds" {
                    CloudVideoScope { brand_id: Some(value), store_id: None }
                } else {
                    CloudVideoScope { brand_id: None, store_id: Some(value) }
                };
                if !scopes.contains(&scope) {
                    scopes.push(scope);
                }
            }
        }
        Ok(scopes)
    }
    async fn create_project(&self, run_id: &str, scope: &CloudVideoScope) -> Result<String> {
        // Canvas sizing is finalized by later editing stages; never infer source dimensions.
        let mut args = json!({"name":format!("video-edit-{run_id}")});
        if let Some(brand) = &scope.brand_id {
            args["brandId"] = json!(brand);
        }
        if let Some(store) = &scope.store_id {
            args["storeId"] = json!(store);
        }
        let response = self.prepare_call("create_project", args).await?;
        id(&response, "projectId")
            .map_err(|_| CloudOperationFailure::unknown_response(&response).into())
    }
    async fn find_project(&self, run_id: &str, scope: &CloudVideoScope) -> Result<Option<String>> {
        let name = format!("video-edit-{run_id}");
        let mut args = json!({"q":name,"mine":true,"limit":50});
        if let Some(brand) = &scope.brand_id {
            args["brandId"] = json!(brand);
        }
        if let Some(store) = &scope.store_id {
            args["storeId"] = json!(store);
        }
        let mut found = None;
        loop {
            let response = self.prepare_call("list_projects", args.clone()).await?;
            for project in response["items"].as_array().context("invalid project query")? {
                if project["name"] == name
                    && scope.brand_id.as_deref().is_none_or(|s| project["brandId"] == s)
                    && scope.store_id.as_deref().is_none_or(|s| project["storeId"] == s)
                {
                    ensure!(
                        found.is_none(),
                        "ambiguous cloud project; manual reconciliation required"
                    );
                    found = Some(id(project, "id")?);
                }
            }
            match response["nextCursor"].as_str().filter(|s| !s.is_empty()) {
                None => break,
                Some(cursor) => {
                    ensure!(args["cursor"] != cursor, "cloud pagination did not advance");
                    args["cursor"] = json!(cursor);
                }
            }
        }
        Ok(found)
    }
    async fn create_upload(
        &self,
        project: &str,
        source: &VideoSourceIdentity,
    ) -> Result<VideoUploadTicket> {
        ensure!(
            source.bytes <= 5 * 1024 * 1024 * 1024,
            "single upload exceeds official 5 GiB limit"
        );
        self.authorize_path(&source.path).map_err(anyhow::Error::msg)?;
        let name =
            source.path.file_name().and_then(|s| s.to_str()).context("source filename encoding")?;
        let value = self.prepare_call("import_media",json!({"projectId":project,"action":"create_session","files":[{"name":name,"size":source.bytes}]})).await?;
        let files = value["uploads"]
            .as_array()
            .context("unsupported upload session receipt; reconcile, do not create again")?;
        ensure!(files.len() == 1, "unexpected upload session count");
        let file = &files[0];
        let upload_id = id(file, "uploadId")?;
        let url = file["uploadUrl"].as_str().context("missing memory-only PUT ticket")?.into();
        let headers = match file.get("headers") {
            None => Default::default(),
            Some(value) => serde_json::from_value(value.clone()).context("invalid PUT headers")?,
        };
        Ok(VideoUploadTicket { upload_id, url, headers })
    }
    async fn upload(&self, ticket: &VideoUploadTicket, source: &VideoSourceIdentity) -> Result<()> {
        self.authorize_path(&source.path).map_err(anyhow::Error::msg)?;
        cancelled(&self.context)?;
        let path = source.path.clone();
        let context = self.context.clone();
        ensure!(
            crate::global_database::run_isolated(move || identify_video_source(&path, &context))
                .await??
                == *source,
            "source replaced before native upload"
        );
        self.memory_exchange(json!({"kind":"put","url":ticket.url,"headers":ticket.headers,"path":source.path,"bytes":source.bytes,"sha256":source.sha256})).await?;
        Ok(())
    }
    async fn finalize(
        &self,
        project: &str,
        upload: &str,
        job: Option<&str>,
    ) -> Result<VideoImportResult> {
        let mut args = json!({"projectId":project,"action":"finalize","uploadIds":[upload]});
        if let Some(job) = job {
            args["jobId"] = json!(job);
        }
        let value = self.prepare_call("import_media", args).await?;
        if value["status"] == "running" {
            return Ok(VideoImportResult::Pending(id(&value, "jobId")?));
        }
        if let Some(asset_ids) = value["assetIds"].as_array() {
            ensure!(
                value["ok"] == true && asset_ids.len() == 1 && value.get("items").is_none(),
                "invalid/ambiguous finalized asset receipt"
            );
            let results = value["results"].as_array().context("missing import results")?;
            ensure!(
                results.len() == 1 && results[0]["ok"] == true && results[0].get("error").is_none(),
                "cloud import partially failed"
            );
            let assets = results[0]["assets"].as_array().context("missing imported assets")?;
            ensure!(
                assets.len() == 1 && assets[0]["assetId"] == asset_ids[0],
                "inconsistent imported asset identity"
            );
            return Ok(VideoImportResult::Ready(id(&assets[0], "assetId")?));
        }
        let items = value["items"]
            .as_array()
            .context("invalid finalized media receipt; reconcile original upload")?;
        ensure!(items.len() == 1, "unexpected finalized asset count");
        let item = &items[0];
        ensure!(
            item.get("error").is_none() && item["status"] != "failed",
            "cloud media import failed"
        );
        Ok(VideoImportResult::Ready(id(item, "assetId")?))
    }
    async fn probe(&self, project: &str, asset: &str) -> Result<VideoMediaInfo> {
        let value = super::probe::normalize_media_probe(
            self.prepare_call("media_probe", json!({"projectId":project,"assetId":asset})).await?,
        )?;
        let streams = value["streams"].as_array().context("cloud probe must include streams")?;
        let video = streams.iter().find(|s| s["codec_type"] == "video");
        let Some(video) = video else {
            bail!("cloud probe rejected non-video");
        };
        let width = video["width"]
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .context("invalid probed video width")?;
        let height = video["height"]
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .context("invalid probed video height")?;
        let duration_sec = value["durationSec"].as_f64().context("missing probed duration")?;
        Ok(VideoMediaInfo {
            has_video: true,
            width,
            height,
            duration_sec,
            has_audio: streams.iter().any(|s| s["codec_type"] == "audio"),
        })
    }
}
