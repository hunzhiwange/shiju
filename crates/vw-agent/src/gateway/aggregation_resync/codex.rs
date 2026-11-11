//! Codex files and catalog in the original provider-record publication.
use super::*;
impl RecordPublicationContext {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn codex(
        &self,
        tx: &ProviderTransactions,
        store: &AppProviderStore,
        state: &AppState,
        candidate: &AppState,
        rows: &[(String, Value)],
        change: &ProviderRecordChange,
        mut target: PendingTarget,
        catalog_policy: codex_publish::CatalogPolicy,
    ) -> Result<bool, StatusCode> {
        if !std::fs::symlink_metadata(self.home.join(".codex"))
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
            .file_type()
            .is_dir()
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let mut files = CodexFiles::open(&self.home).map_err(failed)?;
        let default = candidate
            .mode
            .as_ref()
            .and_then(|m| m.proxy_route.as_deref())
            .ok_or(StatusCode::CONFLICT)?;
        let mut configs = Vec::new();
        let mut catalogs = Vec::new();
        for (id, row) in rows {
            let profile = vw_agent_apps::providers::codex_catalog::form::editable_profile(id, row)
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let deployment =
                vw_agent_apps::providers::codex_catalog::identity(&self.host, id, &profile)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            configs.push(
                profile
                    .config
                    .parse::<toml_edit::DocumentMut>()
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
            );
            catalogs.push(
                crate::model_catalog::ModelCatalogStore::read_existing(
                    &self.database,
                    self.project.tenant_scope(),
                    &deployment,
                    row["modelCatalogRevision"].as_str(),
                )
                .map_err(failed)?,
            );
        }
        let default_index =
            rows.iter().position(|(id, _)| id == default).ok_or(StatusCode::CONFLICT)?;
        let row = |i: usize| CodexCatalogRow { config: &configs[i], catalog: catalogs[i].as_ref() };
        let members = rows
            .iter()
            .enumerate()
            .filter(|(_, (id, _))| id != default)
            .map(|(i, (id, v))| {
                Ok(CodexCatalogMember {
                    key: candidate
                        .stack
                        .keys
                        .iter()
                        .find_map(|(key, p)| (p == id).then_some(key.as_str()))
                        .ok_or(StatusCode::CONFLICT)?,
                    provider_name: v["name"].as_str().unwrap_or(id),
                    row: row(i),
                })
            })
            .collect::<Result<Vec<_>, StatusCode>>()?;
        let before = files.guard.read(OsStr::new("config.toml")).map_err(failed)?;
        let before_config: toml::Value = toml::from_str(
            std::str::from_utf8(before.as_deref().unwrap_or_default()).map_err(failed)?,
        )
        .map_err(failed)?;
        let old_pointer = before_config.get("model_catalog_json").and_then(toml::Value::as_str);
        let accepted_external_catalog = match old_pointer {
            Some(pointer)
                if pointer != CATALOG_FILENAME
                    && catalog_policy == codex_publish::CatalogPolicy::UseManaged =>
            {
                Some(pointer.to_owned())
            }
            Some(CATALOG_FILENAME) if state.stack.enabled => state
                .aggregation_recovery
                .as_ref()
                .and_then(|proof| proof.accepted_external_catalog.clone()),
            _ => None,
        };
        let (config, catalog) = codex_publish::plan(
            before.as_deref(),
            catalog_policy,
            CodexCatalogDefault::ThirdParty(row(default_index)),
            &members,
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let Some(catalog) = catalog else { return Err(StatusCode::UNPROCESSABLE_ENTITY) };
        let mut doc: toml_edit::DocumentMut = std::str::from_utf8(&config)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
            .parse()
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let provider = doc
            .get("model_provider")
            .and_then(|v| v.as_str())
            .filter(|id| !id.is_empty())
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
            .to_owned();
        if doc
            .get("model_providers")
            .and_then(|providers| providers.get(&provider))
            .and_then(|route| route.get("base_url"))
            .and_then(|url| url.as_str())
            .is_none()
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }

        let value: Value = serde_json::from_slice(&catalog).map_err(failed)?;
        if state.mode.as_ref().and_then(|mode| mode.proxy_route.as_deref()) != Some(default) {
            // The listener owns provider routing; select the new default's
            // model without changing the catalog owner's pointer protections.
            let model = configs[default_index]
                .get("model")
                .and_then(|model| model.as_str())
                .filter(|model| !model.trim().is_empty())
                .or_else(|| value["models"][0]["slug"].as_str())
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            doc.insert("model", toml_edit::value(model));
        }
        let mut models = Vec::new();
        for model in value["models"].as_array().ok_or(StatusCode::UNPROCESSABLE_ENTITY)? {
            let id = model["slug"].as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            if let vw_agent_apps::aggregation::model_ids::Decoded::Stack { key, model, .. } =
                vw_agent_apps::aggregation::model_ids::decode(AppId::Codex, id)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
            {
                let provider = candidate.stack.keys.get(key).ok_or(StatusCode::CONFLICT)?;
                models.push(AggregationModel {
                    id: id.into(),
                    provider_id: provider.clone(),
                    upstream_model: model.into(),
                });
            }
        }
        let revision =
            hex::encode(Sha256::digest(serde_json::to_vec(&(&candidate, &rows)).map_err(failed)?));
        let publication = self.publication(candidate, rows, models, &revision, default)?;
        let routing = self.routing.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let previous = routing.aggregation_revision(&self.project, &self.host, self.app);
        let old_catalog = files.guard.read(OsStr::new(CATALOG_FILENAME)).map_err(failed)?;
        routing
            .publish_aggregation(publication, previous.as_deref(), |address, token| {
                // Use the native projection owner for local authentication. Never
                // require an ambient environment variable or rewrite auth.json.
                doc["model_providers"][&provider]["base_url"] =
                    toml_edit::value(format!("http://{address}/v1"));
                doc["model_providers"][&provider]["wire_api"] = toml_edit::value("responses");
                let settings =
                    serde_json::json!({"config":doc.to_string(),"auth":{"OPENAI_API_KEY":token}});
                let projection = vw_agent_apps::providers::codex::project(
                    std::str::from_utf8(&config).map_err(|_| RouteCommitError {
                        status: StatusCode::UNPROCESSABLE_ENTITY,
                        recovery_pending: false,
                    })?,
                    &settings,
                    None,
                    false,
                )
                .map_err(|_| RouteCommitError {
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    recovery_pending: false,
                })?;
                let contract = ClientConfiguration::Codex {
                    config: toml::from_str(projection.config()).map_err(|_| RouteCommitError {
                        status: StatusCode::UNPROCESSABLE_ENTITY,
                        recovery_pending: false,
                    })?,
                    auth: serde_json::json!({"OPENAI_API_KEY":token}),
                }
                .contract();
                target.mode = Some(ModeState {
                    mode: Mode::Proxy,
                    attached: true,
                    proxy_route: Some(default.into()),
                    contract: Some(contract),
                });
                target.aggregation_recovery = Some(AggregationRecovery {
                    principal: self.principal.clone(),
                    published: serde_json::json!({"config":projection.config(),"catalog":value}),
                    accepted_external_catalog: accepted_external_catalog.clone(),
                });
                let config_patch = WholeFile::new(
                    before.as_deref(),
                    Some(projection.config().as_bytes().to_vec()),
                );
                let catalog_patch = WholeFile::new(old_catalog.as_deref(), Some(catalog.clone()));
                tx.commit_project_record(
                    &self.project,
                    &mut files.guard,
                    state,
                    &[
                        FileChange {
                            name: OsStr::new(CATALOG_FILENAME),
                            privacy: FilePrivacy::Private,
                            patch: &catalog_patch,
                        },
                        FileChange {
                            name: OsStr::new("config.toml"),
                            privacy: FilePrivacy::Private,
                            patch: &config_patch,
                        },
                    ],
                    target,
                    change,
                    store,
                    || self.validate_inputs(),
                )
                .map_err(|error| RouteCommitError {
                    status: commit_status(error),
                    recovery_pending: tx
                        .pending(self.app)
                        .map_or(true, |p| p.is_some_and(|p| p.published)),
                })
            })
            .map(|_| true)
    }
}
