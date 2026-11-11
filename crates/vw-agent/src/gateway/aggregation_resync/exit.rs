//! Explicit exit through the original native restoration and route owners.
use super::*;
use vw_agent_apps::aggregation::exit_restore::{self, Restore, RestoreError};
use vw_api_types::agent_apps::aggregation::{AggregationMode, AggregationState, RecoveryState};
impl RecordPublicationContext {
    pub(crate) fn exit_aggregation(&self, authorized: impl Fn() -> bool) -> Result<(), StatusCode> {
        let routing = self.routing.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let tx = ProviderTransactions::open_existing(
            &self.root,
            ExecutionHostId::new(&self.host).map_err(failed)?,
        )
        .map_err(failed)?
        .ok_or(StatusCode::CONFLICT)?;
        let scope = vw_api_types::agent_apps_high_availability::FailoverQueueScope {
            project: self.project.clone(),
            target_host_id: self.host.clone(),
            app_id: self.app,
        };
        routing.restore_provider(&scope, || {
            let result = self.restore_exit(&tx, &authorized);
            result.map_err(|status| RouteCommitError {
                status,
                recovery_pending: tx.pending(self.app).map_or(true, |p| p.is_some()),
            })
        })
    }
    fn restore_exit(
        &self,
        tx: &ProviderTransactions,
        authorized: &impl Fn() -> bool,
    ) -> Result<(), StatusCode> {
        let unchanged = || authorized() && self.validate_inputs().is_ok();
        if !unchanged() {
            return Err(StatusCode::CONFLICT);
        }
        let attached = self.expected.mode.as_ref().is_some_and(|m| m.attached);
        if !self.expected.stack.enabled {
            return if attached
                || self.expected.mode.as_ref().is_some_and(|mode| mode.mode != Mode::Direct)
            {
                Err(StatusCode::CONFLICT)
            } else {
                Ok(())
            };
        }
        if !attached {
            let mut guard =
                vw_agent_apps::providers::live::write_guard::lock_app(self.app, &self.home)
                    .map_err(failed)?;
            let mut stack = self.expected.stack.clone();
            stack.enabled = false;
            let mut mode = self.expected.mode.clone().ok_or(StatusCode::CONFLICT)?;
            mode.mode = Mode::Direct;
            tx.commit_project_checked(
                &self.project,
                &mut guard,
                &self.expected,
                false,
                &[],
                PendingTarget { stack: Some(stack), mode: Some(mode), ..Default::default() },
                || {
                    if unchanged() {
                        Ok(())
                    } else {
                        Err(vw_agent_apps::providers::live::write_guard::WriteError::Conflict)
                    }
                },
            )
            .map_err(commit_status)?;
            return Ok(());
        }
        let direct_id = self.expected.pointer.as_ref().map(|p| p.id.as_str());
        let direct = direct_id
            .map(|id| {
                AppProviderStore::preview_record(
                    &self.database,
                    self.project.tenant_scope(),
                    self.app,
                    id,
                )
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
            })
            .transpose()?;
        if self.app == AppId::ClaudeDesktop {
            use vw_agent_apps::providers::claude_desktop::publication::*;
            let profile = direct
                .as_ref()
                .map(|row| {
                    row.get("claude_desktop")
                        .cloned()
                        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)
                        .and_then(|v| {
                            serde_json::from_value(v).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
                        })
                })
                .transpose()?;
            return run(
                &self.home,
                DesktopInstallation::Installed,
                tx,
                self.project.tenant_scope(),
                &self.expected,
                DesktopTarget::Restore(profile.as_ref()),
                unchanged,
            )
            .map_err(|e| match e {
                DesktopPublicationError::Forbidden => StatusCode::FORBIDDEN,
                DesktopPublicationError::RevisionConflict
                | DesktopPublicationError::RecoveryPending => StatusCode::CONFLICT,
                DesktopPublicationError::TargetFailed => StatusCode::BAD_GATEWAY,
                _ => StatusCode::UNPROCESSABLE_ENTITY,
            });
        }
        let direct = direct.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        if direct["category"] == "official"
            || direct.get("managedCodexLogin").is_some()
            || direct.get("managedClaudeLogin").is_some()
            || direct.pointer("/meta/authBinding").is_some()
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let proof = self.expected.aggregation_recovery.as_ref().ok_or(StatusCode::CONFLICT)?;
        let restore = Restore {
            project: &self.project,
            expected: &self.expected,
            published: &proof.published,
            direct: direct.get("settingsConfig").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
        };
        let result = match self.app {
            AppId::Claude => {
                let _directory = tx.lock_claude_directory().map_err(failed)?;
                let selected = tx.claude_directory(self.project.tenant_scope()).map_err(failed)?;
                let mut file = ClaudeFile::open_selected(&self.home, selected.as_ref(), false)
                    .map_err(failed)?;
                exit_restore::restore_claude(&mut file, tx, restore, unchanged)
            }
            AppId::Codex => {
                use vw_agent_apps::providers::codex_login::{AuthTarget, LoginInput};
                let id = direct_id.ok_or(StatusCode::CONFLICT)?;
                let profile =
                    vw_agent_apps::providers::codex_catalog::form::editable_profile(id, &direct)
                        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                let deployment =
                    vw_agent_apps::providers::codex_catalog::identity(&self.host, id, &profile)
                        .map_err(failed)?;
                let catalog = crate::model_catalog::ModelCatalogStore::read_existing(
                    &self.database,
                    self.project.tenant_scope(),
                    &deployment,
                    direct["modelCatalogRevision"].as_str(),
                )
                .map_err(failed)?;
                let mut files = CodexFiles::open(&self.home).map_err(failed)?;
                exit_restore::restore_codex(
                    &mut files,
                    tx,
                    restore,
                    LoginInput {
                        target: AuthTarget::ThirdParty { preserve: profile.preserve_login },
                        leaving_official: None,
                        official_logins: &[],
                        third_party_keys: &[],
                        live_is_managed: false,
                        now: chrono::Utc::now().timestamp(),
                    },
                    catalog.as_ref(),
                    unchanged,
                )
            }
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        result.map_err(|error| match error {
            RestoreError::Forbidden => StatusCode::FORBIDDEN,
            RestoreError::RevisionConflict | RestoreError::RecoveryPending => StatusCode::CONFLICT,
            RestoreError::TargetFailed => StatusCode::BAD_GATEWAY,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        })
    }
    pub(crate) fn exited_state(&self) -> Result<AggregationState, StatusCode> {
        use crate::gateway::api::handlers::aggregation_read_snapshot as snapshot;
        let _guard = vw_agent_apps::providers::live::write_guard::lock_app(self.app, &self.home)
            .map_err(failed)?;
        let tx = ProviderTransactions::open_existing(
            &self.root,
            ExecutionHostId::new(&self.host).map_err(failed)?,
        )
        .map_err(failed)?
        .ok_or(StatusCode::CONFLICT)?;
        let state = tx.state(self.app).map_err(failed)?;
        state.require_project(&self.project).map_err(|_| StatusCode::FORBIDDEN)?;
        if state.stack.enabled
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
            || tx.pending(self.app).map_err(failed)?.is_some()
        {
            return Err(StatusCode::CONFLICT);
        }
        let input = snapshot::load(
            &self.database,
            &self.project,
            &self.host,
            self.app,
            &self.home,
            &state,
            None,
        )?;
        let revision = snapshot::revision(
            &self.project,
            &self.host,
            self.app,
            &self.principal,
            &state,
            &input,
        )?;
        let projected =
            snapshot::project(self.app, self.project.tenant_scope(), &state, &input, None)?;
        let mut members = projected.members;
        for member in &mut members {
            member.models.clear();
        }
        Ok(AggregationState {
            revision,
            mode: AggregationMode::Direct,
            attached: false,
            default_provider_id: state.mode.as_ref().and_then(|m| m.proxy_route.clone()),
            direct_provider_id: state.pointer.map(|p| p.id),
            members,
            notices: projected.notices,
            recovery: if self.expected.stack.enabled {
                RecoveryState::Restored
            } else {
                RecoveryState::None
            },
        })
    }
}
