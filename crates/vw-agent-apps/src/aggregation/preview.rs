//! Read-only aggregation planning from one authorized, settled executor snapshot.
use super::codex_catalog::CodexCatalogRow;
use crate::providers::live::commit::AppState;
use serde_json::{Map, Value};
use toml_edit::DocumentMut;
use vw_api_types::{
    agent_apps::AppId, agent_apps_auth::AuthService,
    app_providers::desktop_mapping::DesktopMapping, id::TenantScope,
};

pub enum Catalog<'a> {
    Claude(&'a Value),
    Codex(CodexCatalogRow<'a>),
    CodexOfficial { native: &'a [Map<String, Value>], config: &'a DocumentMut },
    Desktop(&'a DesktopMapping),
}

pub struct Provider<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub official_service: Option<AuthService>,
    pub catalog: Catalog<'a>,
}

#[derive(Default, Clone, Copy)]
pub enum CatalogPolicy {
    #[default]
    Preserve,
    UseManaged,
}

/// All inputs come from the executor, after host/project/read authorization.
/// Official Codex defaults require the account owner's login and restore proof.
pub struct PreviewInput<'a> {
    pub candidate_default: Option<&'a str>,
    pub providers: &'a [Provider<'a>],
    pub pending_tenant: Option<&'a TenantScope>,
    pub failover_enabled: bool,
    pub direct_restorable: bool,
    pub catalog_policy: CatalogPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewError {
    UnsupportedTarget,
    Forbidden,
    RecoveryPending,
    InvalidConfiguration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCode {
    ModelCollision,
    InvalidMember,
    MissingDefault,
    InvalidDefault,
    FailoverEnabled,
    UnrestorableDirect,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PreviewIssue {
    pub code: IssueCode,
    pub blocking: bool,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PreviewModel {
    pub id: String,
    pub label: String,
    pub supports_1m: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PreviewMember {
    pub provider_id: String,
    pub name: String,
    pub models: Vec<PreviewModel>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Notice {
    ModelReselect,
    NewSession,
    RestartDesktop,
    RestartEditor,
    CodexRouteOwnsCatalog,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Preview {
    pub default_provider_id: Option<String>,
    pub members: Vec<PreviewMember>,
    pub issues: Vec<PreviewIssue>,
    pub notices: Vec<Notice>,
}

/// No token is minted here: the gateway must bind a signed token to its subject,
/// scope and revision after checking every blocking issue and revalidating inputs.
pub fn preview(
    app: AppId,
    tenant: &TenantScope,
    state: &AppState,
    input: PreviewInput<'_>,
) -> Result<Preview, PreviewError> {
    use super::member_validation::{ProviderRole, validate_provider};
    use crate::providers::mode_state::observe;
    use std::collections::HashSet;
    validate_provider(app, None, ProviderRole::Member)
        .map_err(|_| PreviewError::UnsupportedTarget)?;
    observe(state, tenant).map_err(|_| PreviewError::Forbidden)?;
    if let Some(owner) = input.pending_tenant {
        return Err(if owner == tenant {
            PreviewError::RecoveryPending
        } else {
            PreviewError::Forbidden
        });
    }
    let default = input
        .candidate_default
        .or_else(|| state.mode.as_ref().and_then(|mode| mode.proxy_route.as_deref()));
    let mut output = Preview {
        default_provider_id: default.map(str::to_owned),
        members: vec![],
        issues: vec![],
        notices: match app {
            AppId::Claude => vec![Notice::ModelReselect],
            AppId::Codex => vec![Notice::NewSession, Notice::RestartDesktop, Notice::RestartEditor],
            AppId::ClaudeDesktop => vec![Notice::RestartDesktop],
            _ => unreachable!(),
        },
    };
    if app == AppId::Codex && matches!(input.catalog_policy, CatalogPolicy::Preserve) {
        output.notices.push(Notice::CodexRouteOwnsCatalog);
    }
    if input.failover_enabled {
        output.issue(IssueCode::FailoverEnabled, true, None, None);
    }
    if !input.direct_restorable {
        output.issue(IssueCode::UnrestorableDirect, true, None, None);
    }
    if default.is_none() {
        output.issue(IssueCode::MissingDefault, true, None, None);
    }
    let mut order = Vec::new();
    if let Some(id) = default {
        order.push(id);
    }
    order.extend(state.stack.members.iter().map(String::as_str).filter(|id| Some(*id) != default));
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();
    let mut published = HashSet::new();
    for id in order {
        let code =
            if Some(id) == default { IssueCode::InvalidDefault } else { IssueCode::InvalidMember };
        let candidates: Vec<_> = input.providers.iter().filter(|p| p.id == id).collect();
        let Some(provider) = candidates.first().filter(|_| candidates.len() == 1) else {
            output.issue(code, true, Some(id), None);
            continue;
        };
        let role = if Some(id) == default { ProviderRole::Default } else { ProviderRole::Member };
        let key =
            state.stack.keys.iter().find_map(|(key, owner)| (owner == id).then_some(key.as_str()));
        // No key allocation during a preview, including for a proposed default.
        if !ids.insert(id)
            || validate_provider(app, provider.official_service, role).is_err()
            || (provider.official_service.is_none() && key.is_none())
            || key.is_some_and(|key| !keys.insert(key))
        {
            output.issue(code, true, Some(id), None);
            continue;
        }
        let models = match project(app, provider, key.unwrap_or(""), Some(id) == default) {
            Ok(models) => models,
            Err(PreviewError::InvalidConfiguration) => {
                output.issue(code, true, Some(id), None);
                continue;
            }
            Err(error) => return Err(error),
        };
        if Some(id) == default && models.is_empty() {
            output.issue(IssueCode::InvalidDefault, true, Some(id), None);
        }
        let mut kept = Vec::new();
        for model in models {
            if published.insert(model.id.clone()) {
                kept.push(model);
            } else {
                output.issue(
                    IssueCode::ModelCollision,
                    app != AppId::ClaudeDesktop,
                    Some(id),
                    Some(&model.id),
                );
            }
        }
        output.members.push(PreviewMember {
            provider_id: id.into(),
            name: provider.name.into(),
            models: kept,
        });
    }
    Ok(output)
}

impl Preview {
    fn issue(
        &mut self,
        code: IssueCode,
        blocking: bool,
        provider: Option<&str>,
        model: Option<&str>,
    ) {
        self.issues.push(PreviewIssue {
            code,
            blocking,
            provider_id: provider.map(str::to_owned),
            model_id: model.map(str::to_owned),
        });
    }
}

fn project(
    app: AppId,
    provider: &Provider<'_>,
    key: &str,
    default: bool,
) -> Result<Vec<PreviewModel>, PreviewError> {
    use super::{
        claude_catalog::{ClaudeCatalogMember, ClaudeModelCatalog},
        codex_catalog::{CodexCatalogDefault, CodexCatalogError, plan_codex_stack_catalog},
    };
    use crate::providers::desktop_mapping::catalog::{
        DesktopMappingMember, DesktopModelCatalog, member_models,
    };
    let invalid = |_| PreviewError::InvalidConfiguration;
    match (&provider.catalog, app) {
        (Catalog::Claude(record), AppId::Claude) if provider.official_service.is_none() => {
            let catalog = ClaudeModelCatalog::new(
                &[ClaudeCatalogMember { provider_id: provider.id, key, record }],
                Some(provider.id),
            )
            .map_err(invalid)?;
            Ok(catalog
                .models()
                .iter()
                .map(|m| PreviewModel {
                    id: m.id.clone(),
                    label: m.label.clone(),
                    supports_1m: m.supports_1m,
                })
                .collect())
        }
        (Catalog::Codex(row), AppId::Codex) if provider.official_service.is_none() => {
            let catalog = if default {
                plan_codex_stack_catalog(CodexCatalogDefault::ThirdParty(*row), &[])
                    .map_err(|_| PreviewError::InvalidConfiguration)?
            } else {
                let member = super::codex_catalog::CodexCatalogMember {
                    key,
                    provider_name: provider.name,
                    row: *row,
                };
                serde_json::json!({"models": super::codex_catalog::member_rows(&member).map_err(|_| PreviewError::InvalidConfiguration)?})
            };
            Ok(codex_models(
                catalog["models"].as_array().ok_or(PreviewError::InvalidConfiguration)?,
            ))
        }
        (Catalog::CodexOfficial { native, config }, AppId::Codex)
            if default && provider.official_service == Some(AuthService::CodexOauth) =>
        {
            let catalog =
                plan_codex_stack_catalog(CodexCatalogDefault::Official { native, config }, &[])
                    .map_err(|error| match error {
                        CodexCatalogError::NativeCatalogUnavailable => {
                            PreviewError::UnsupportedTarget
                        }
                        _ => PreviewError::InvalidConfiguration,
                    })?;
            Ok(codex_models(
                catalog["models"].as_array().ok_or(PreviewError::InvalidConfiguration)?,
            ))
        }
        (Catalog::Desktop(mapping), AppId::ClaudeDesktop)
            if provider.official_service.is_none() =>
        {
            let member = || DesktopMappingMember {
                provider_id: provider.id,
                key,
                name: provider.name,
                mapping,
            };
            let models = if default {
                DesktopModelCatalog::new(member(), &[])
                    .map_err(|_| PreviewError::InvalidConfiguration)?
                    .models()
                    .iter()
                    .map(|m| PreviewModel {
                        id: m.id.clone(),
                        label: m.label.clone(),
                        supports_1m: m.supports_1m,
                    })
                    .collect()
            } else {
                member_models(&member())
                    .map_err(|_| PreviewError::InvalidConfiguration)?
                    .into_iter()
                    .map(|m| PreviewModel { id: m.id, label: m.label, supports_1m: m.supports_1m })
                    .collect()
            };
            Ok(models)
        }
        _ => Err(PreviewError::InvalidConfiguration),
    }
}

fn codex_models(rows: &[Value]) -> Vec<PreviewModel> {
    rows.iter()
        .map(|row| PreviewModel {
            id: row["slug"].as_str().unwrap_or_default().into(),
            label: row["display_name"]
                .as_str()
                .or_else(|| row["slug"].as_str())
                .unwrap_or_default()
                .into(),
            supports_1m: row["context_window"].as_u64().is_some_and(|w| w >= 1_000_000),
        })
        .collect()
}
