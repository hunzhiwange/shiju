//! Preview and commit missing metadata for the selected saved model.

use super::catalog_api;
use desktop_provider_apps::state::model_catalog::{CatalogForm, FillState};
use iced::Task;
use gateway_model_catalog::{AgentAppsReadError, GatewayModelCatalogClient};
use vw_api_model_catalog::{
        ModelCatalogFillCommitRequest, ModelCatalogFillPreviewRequest,
        ModelCatalogFillPreviewResponse, ModelCatalogReadResponse, ModelMetadataDto,
        ScopedModelCatalogIdentity,
};

#[derive(Clone)]
pub enum Event {
    Preview,
    Previewed {
        generation: u64,
        result: Result<Box<ModelCatalogFillPreviewResponse>, AgentAppsReadError>,
    },
    Silent(bool),
    Commit,
    Committed {
        generation: u64,
        result: Result<CommitOutcome, AgentAppsReadError>,
    },
}

#[derive(Clone)]
pub enum CommitOutcome {
    Unchanged,
    Changed(Result<ModelCatalogReadResponse, String>),
}

impl Event {
    pub(crate) fn is_response(&self) -> bool {
        matches!(self, Self::Previewed { .. } | Self::Committed { .. })
    }
}

pub fn message(event: Event) -> catalog_api::Event {
    catalog_api::message(catalog_api::Event::Fill(event))
}

fn target(form: &CatalogForm) -> Option<ScopedModelCatalogIdentity> {
    let id = form.edit_id.as_ref()?;
    let deployment = form.deployment();
    form.catalog
        .as_ref()?
        .items
        .iter()
        .find(|row| {
            &row.identity.model_id == id
                && row.identity.deployment == deployment
                && row.identity.scope == form.context.scope.context.tenant
        })
        .map(|row| row.identity.clone())
}

fn failure(error: AgentAppsReadError, writing: bool) -> &'static str {
    match error {
        AgentAppsReadError::Transport(_) if writing => {
            "网关暂不可达，补全结果未确认。请刷新已保存模型。"
        }
        AgentAppsReadError::Transport(_) => "网关离线，暂时无法预览。已保存参数和当前草稿均保留。",
        AgentAppsReadError::Http { status: 409, .. } => {
            "目录已被修改，请刷新已保存模型后重新预览。"
        }
        _ if writing => "未能应用补全，请刷新已保存模型后重试。",
        _ => "暂时无法预览补全，请重试。",
    }
}

pub fn update(catalog: &mut Option<CatalogForm>, generation: &mut u64, token: &str, client: GatewayModelCatalogClient, event: Event) -> Task<catalog_api::Event> {
    let Some(form) = &mut (*catalog) else { return Task::none() };
    match event {
        Event::Previewed { generation: request, result } => {
            if !matches!(form.fill, FillState::Loading(active) if active == request) {
                return Task::none();
            }
            form.fill = match result {
                Ok(preview) if target(form).as_ref() == Some(&preview.identity) => {
                    if form
                        .catalog
                        .as_ref()
                        .is_some_and(|catalog| catalog.revision == preview.revision)
                    {
                        FillState::Preview(preview)
                    } else {
                        FillState::Notice("目录已被修改，请刷新已保存模型后重新预览。")
                    }
                }
                Ok(_) => FillState::Notice("补全预览与当前模型不匹配，请重试。"),
                Err(error) => FillState::Notice(failure(error, false)),
            };
        }
        Event::Committed { generation: request, result } => {
            if !matches!(form.fill, FillState::Committing(active) if active == request) {
                return Task::none();
            }
            form.saving = false;
            let mut notify = false;
            form.fill = FillState::Notice(match result {
                Ok(CommitOutcome::Unchanged) => "没有新增参数。",
                Ok(CommitOutcome::Changed(catalog)) => {
                    notify = !form.fill_silent;
                    if let Ok(catalog) = catalog {
                        form.apply_catalog(catalog);
                        "已补全空白参数。"
                    } else {
                        "已补全空白参数，请刷新查看保存结果。"
                    }
                }
                Err(error) => failure(error, true),
            });
            if notify {
                return Task::done(catalog_api::Event::Feedback { success: true, text: "已补全模型参数，请核对来源和取值".into() });
            }
        }
        _ if form.busy() => {}
        Event::Silent(value) => form.fill_silent = value,
        Event::Preview => {
            let Some(identity) = target(form) else { return Task::none() };
            *generation = generation.wrapping_add(1);
            let request = *generation;
            form.loading = false;
            form.fill = FillState::Loading(request);
            let project = form.context.project.clone();
            let token = token.to_owned();
            return Task::perform(
                async move {
                    client
                        .preview_model_catalog_fill(
                            &project,
                            &ModelCatalogFillPreviewRequest { identity },
                            &token,
                        )
                        .await
                        .map(Box::new)
                },
                move |result| message(Event::Previewed { generation: request, result }),
            );
        }
        Event::Commit => {
            let FillState::Preview(preview) = &form.fill else { return Task::none() };
            if preview.proposed == ModelMetadataDto::default()
                || target(form).as_ref() != Some(&preview.identity)
                || !form
                    .catalog
                    .as_ref()
                    .is_some_and(|catalog| catalog.revision == preview.revision)
            {
                return Task::none();
            }
            let input = ModelCatalogFillCommitRequest {
                identity: preview.identity.clone(),
                expected_revision: preview.revision.clone(),
            };
            *generation = generation.wrapping_add(1);
            let request = *generation;
            form.loading = false;
            form.saving = true;
            form.fill = FillState::Committing(request);
            let context = form.context.clone();
            let token = token.to_owned();
            return Task::perform(
                async move {
                    let write =
                        client.commit_model_catalog_fill(&context.project, &input, &token).await?;
                    if write.changed {
                        let catalog = catalog_api::read_catalog(
                            &client,
                            &context,
                            input.identity.deployment,
                            &token,
                        )
                        .await;
                        Ok(CommitOutcome::Changed(catalog))
                    } else {
                        Ok(CommitOutcome::Unchanged)
                    }
                },
                move |result| message(Event::Committed { generation: request, result }),
            );
        }
    }
    Task::none()
}
