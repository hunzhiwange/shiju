//! Single-target publication using the same scoped provider-record transaction.
use super::*;
use crate::app_routing::provider_publication::PublishedProvider;
use vw_api_types::agent_apps_high_availability::FailoverQueueScope;

impl RecordPublicationContext {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn publish_route(
        &self,
        tx: &ProviderTransactions,
        store: &AppProviderStore,
        state: &AppState,
        candidate: &AppState,
        rows: &[(String, Value)],
        change: &ProviderRecordChange,
        mut target: PendingTarget,
    ) -> Result<bool, StatusCode> {
        let default = candidate
            .mode
            .as_ref()
            .and_then(|mode| mode.proxy_route.as_deref())
            .ok_or(StatusCode::CONFLICT)?;
        let row = rows.iter().find(|(id, _)| id == default).ok_or(StatusCode::CONFLICT)?;
        // Reuse the saved-profile target owner; no aggregate routes are published.
        let publication = self.publication(
            candidate,
            std::slice::from_ref(row),
            vec![],
            &self.revision,
            default,
        )?;
        let selected = &publication.default.target;
        crate::model_relay::upstream::validate_target(
            &selected.url,
            &reqwest::Request::new(reqwest::Method::POST, selected.url.clone()),
        )?;
        vw_agent_apps::providers::validation::validate(self.app, &selected.request_options)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let binding = VerifiedRouteBinding::new(
            self.project.clone(),
            self.host.clone(),
            self.app,
            format!("saved-provider:{default}"),
            1,
        );
        let messages_api_key = publication.default.messages_api_key;
        let routing = self.routing.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let scope = FailoverQueueScope {
            project: self.project.clone(),
            target_host_id: self.host.clone(),
            app_id: self.app,
        };
        let error = |status| RouteCommitError {
            status,
            recovery_pending: tx
                .pending(self.app)
                .map_or(true, |pending| pending.is_some_and(|pending| pending.published))
                || tx.state(self.app).map_or(true, |current| current != *state),
        };
        match self.app {
            AppId::Claude => {
                let _directory = tx.lock_claude_directory().map_err(failed)?;
                let selected = tx.claude_directory(self.project.tenant_scope()).map_err(failed)?;
                let mut file = ClaudeFile::open_selected(&self.home, selected.as_ref(), false)
                    .map_err(failed)?;
                routing.publish_provider(
                    binding,
                    publication.default.target,
                    |address, token| {
                        let mut env =
                            ClientConfiguration::saved(self.app, &row.1["settingsConfig"], false)
                                .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY))?
                                .contract()
                                .exclusive;
                        let saved = row
                            .1
                            .pointer("/settingsConfig/env")
                            .and_then(Value::as_object)
                            .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY))?;
                        env.extend(
                            saved
                                .iter()
                                .filter(|(key, _)| {
                                    key.as_str() == "ANTHROPIC_MODEL"
                                        || key.starts_with("ANTHROPIC_DEFAULT_")
                                })
                                .map(|(key, value)| (key.clone(), value.clone())),
                        );
                        env.insert(
                            "ANTHROPIC_BASE_URL".into(),
                            serde_json::json!(format!("http://{address}")),
                        );
                        env.insert("ANTHROPIC_AUTH_TOKEN".into(), serde_json::json!(token));
                        let settings = serde_json::json!({"env":env});
                        let mut mode =
                            candidate.mode.clone().ok_or_else(|| error(StatusCode::CONFLICT))?;
                        mode.contract = Some(
                            ClientConfiguration::saved(self.app, &settings, false)
                                .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY))?
                                .contract(),
                        );
                        target.mode = Some(mode);
                        let previous = state
                            .mode
                            .as_ref()
                            .and_then(|mode| mode.contract.as_ref())
                            .map(|contract| serde_json::json!({"env":contract.exclusive}));
                        let after = vw_agent_apps::providers::claude::patch_with_residue(
                            file.bytes.as_deref(),
                            previous.as_ref(),
                            &settings,
                            false,
                        )
                        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY))?;
                        let patch = WholeFile::new(file.bytes.as_deref(), Some(after.clone()));
                        tx.commit_project_record(
                            &self.project,
                            &mut file.guard,
                            state,
                            &[FileChange {
                                name: OsStr::new(file.name),
                                privacy: FilePrivacy::Private,
                                patch: &patch,
                            }],
                            target,
                            change,
                            store,
                            || self.validate_inputs(),
                        )
                        .map_err(|failure| error(commit_status(failure)))?;
                        if file
                            .guard
                            .read(OsStr::new(file.name))
                            .map_err(|_| error(StatusCode::BAD_GATEWAY))?
                            .as_deref()
                            != Some(after.as_slice())
                        {
                            return Err(error(StatusCode::CONFLICT));
                        }
                        PublishedProvider::committed_record(
                            self.database.clone(),
                            &self.root,
                            &self.home,
                            scope,
                            default,
                            address,
                            &after,
                            messages_api_key,
                        )
                        .map_err(error)
                    },
                )?;
            }
            AppId::Codex => {
                if !std::fs::symlink_metadata(self.home.join(".codex"))
                    .map_err(failed)?
                    .file_type()
                    .is_dir()
                {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                let mut files = CodexFiles::open(&self.home).map_err(failed)?;
                let before = files.guard.read(OsStr::new("config.toml")).map_err(failed)?;
                let live = std::str::from_utf8(before.as_deref().unwrap_or_default())
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let live_doc: toml_edit::DocumentMut =
                    live.parse().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let mut settings = row.1["settingsConfig"].clone();
                let mut doc: toml_edit::DocumentMut = settings["config"]
                    .as_str()
                    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                    .parse()
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                if let Some(pointer) =
                    live_doc.get("model_catalog_json").and_then(|item| item.as_str())
                {
                    let owned = std::path::Path::new(pointer)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            [CATALOG_FILENAME, "cc-switch-model-catalog.json"].contains(&name)
                        });
                    if !owned
                        && doc.get("model_catalog_json").and_then(|item| item.as_str())
                            != Some(pointer)
                    {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }
                }
                let provider = doc
                    .get("model_provider")
                    .and_then(|item| item.as_str())
                    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                    .to_owned();
                let table = doc
                    .get("model_providers")
                    .and_then(|providers| providers.get(&provider))
                    .and_then(|item| item.as_table_like())
                    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                if table.iter().any(|(key, _)| {
                    !["base_url", "name", "wire_api", "requires_openai_auth"].contains(&key)
                }) {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                routing.publish_provider(
                    binding,
                    publication.default.target,
                    |address, token| {
                        doc["model_providers"][&provider]["base_url"] =
                            toml_edit::value(format!("http://{address}/v1"));
                        doc["model_providers"][&provider]["wire_api"] =
                            toml_edit::value("responses");
                        settings["config"] = Value::String(doc.to_string());
                        settings["auth"] = serde_json::json!({"OPENAI_API_KEY":token});
                        let projection =
                            vw_agent_apps::providers::codex::project(live, &settings, None, false)
                                .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY))?;
                        let after = projection.config().as_bytes();
                        let mut mode =
                            candidate.mode.clone().ok_or_else(|| error(StatusCode::CONFLICT))?;
                        mode.contract = Some(
                            ClientConfiguration::Codex {
                                config: toml::from_str(projection.config())
                                    .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY))?,
                                auth: settings["auth"].clone(),
                            }
                            .contract(),
                        );
                        target.mode = Some(mode);
                        let patch = WholeFile::new(before.as_deref(), Some(after.to_vec()));
                        tx.commit_project_record(
                            &self.project,
                            &mut files.guard,
                            state,
                            &[FileChange {
                                name: OsStr::new("config.toml"),
                                privacy: FilePrivacy::Private,
                                patch: &patch,
                            }],
                            target,
                            change,
                            store,
                            || self.validate_inputs(),
                        )
                        .map_err(|failure| error(commit_status(failure)))?;
                        if files
                            .guard
                            .read(OsStr::new("config.toml"))
                            .map_err(|_| error(StatusCode::BAD_GATEWAY))?
                            .as_deref()
                            != Some(after)
                        {
                            return Err(error(StatusCode::CONFLICT));
                        }
                        PublishedProvider::committed_record(
                            self.database.clone(),
                            &self.root,
                            &self.home,
                            scope,
                            default,
                            address,
                            after,
                            false,
                        )
                        .map_err(error)
                    },
                )?;
            }
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
        Ok(true)
    }
}
