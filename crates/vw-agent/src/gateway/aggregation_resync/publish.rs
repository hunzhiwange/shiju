//! Initial aggregation attachment through the existing publication owner.
use super::*;
impl RecordPublicationContext {
    /// First attachment uses the same record/file journal and listener as
    /// subsequent edits. The preview caller owns authorization and intent checks.
    pub(crate) fn publish_aggregation(
        &self,
        default: &str,
        policy: codex_publish::CatalogPolicy,
        preview_record: &Value,
    ) -> Result<bool, StatusCode> {
        if self.routing.as_ref().and_then(|routing| routing.bound_address()).is_none() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let row = AppProviderStore::preview_record(
            &self.database,
            self.project.tenant_scope(),
            self.app,
            default,
        )
        .map_err(|_| StatusCode::NOT_FOUND)?;
        if &row != preview_record {
            return Err(StatusCode::CONFLICT);
        }
        let plan = vw_agent_apps::aggregation::membership::plan_member(
            self.app,
            &self.expected,
            vw_agent_apps::aggregation::membership::MembershipChange::SetDefault {
                provider_id: default,
                saved: Some(vw_agent_apps::aggregation::membership::SavedMember {
                    name: row["name"].as_str().unwrap_or(default),
                    icon: row["icon"].as_str(),
                    official_service: None,
                }),
            },
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let mut candidate = self.expected.clone();
        if let Some(stack) = plan.stack {
            candidate.stack = stack;
        }
        candidate.stack.enabled = true;
        candidate.stack_tenant = Some(self.project.tenant_scope().clone());
        candidate.mode_tenant = Some(self.project.tenant_scope().clone());
        candidate.mode = Some(ModeState {
            mode: Mode::Proxy,
            attached: true,
            proxy_route: Some(default.into()),
            contract: self.expected.mode.as_ref().and_then(|mode| mode.contract.clone()),
        });
        let store = AppProviderStore::new(self.database.clone()).map_err(failed)?;
        let raw = store
            .publication_record(self.project.tenant_scope(), self.app, default)
            .map_err(failed)?;
        if serde_json::from_str::<Value>(&raw).map_err(failed)? != *preview_record {
            return Err(StatusCode::CONFLICT);
        }
        let key = format!("{}:{default}", serde_json::to_string(&self.app).map_err(failed)?);
        self.replace_candidate(&key, &raw, &raw, Some(&candidate), policy)
    }
    /// Return the same scoped public state used by the read API, after the
    /// journal and listener have both settled. No private record enters results.
    pub(crate) fn published_state(
        &self,
    ) -> Result<vw_api_types::agent_apps::aggregation::AggregationState, StatusCode> {
        use crate::gateway::api::handlers::aggregation_read_snapshot as snapshot;
        use vw_api_types::agent_apps::aggregation::{
            AggregationMode, AggregationState, RecoveryState,
        };
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
        if tx.pending(self.app).map_err(failed)?.is_some() {
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
        let observed =
            vw_agent_apps::providers::mode_state::observe(&state, self.project.tenant_scope())
                .map_err(|_| StatusCode::FORBIDDEN)?;
        if !observed.attached || !observed.stack.active {
            return Err(StatusCode::CONFLICT);
        }
        Ok(AggregationState {
            revision,
            mode: AggregationMode::Aggregation,
            attached: true,
            default_provider_id: observed.proxy_route,
            direct_provider_id: observed.direct_provider,
            members: projected.members,
            notices: projected.notices,
            recovery: RecoveryState::None,
        })
    }
}
