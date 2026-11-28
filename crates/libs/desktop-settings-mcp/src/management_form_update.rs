use crate::management_form::*;
use crate::management_update::{self as scope, Update};
use iced::Task;
fn authorized(app: &Update<'_>, context: &crate::management_context::Context) -> bool {
    app.context.as_ref() == Some(context) && app.authorized && !context.host.is_empty()
}
pub fn update(app: &mut Update<'_>, event: FormMessage) -> Task<FormMessage> {
    let sync_only = matches!(&event, FormMessage::Sync(_) | FormMessage::OpenGlobal(_, true));
    let open_global = match &event {
        FormMessage::OpenGlobal(index, _) => Some(*index),
        _ => None,
    };
    let event = match event {
        FormMessage::OpenGlobal(..) => FormMessage::Open(None),
        FormMessage::Sync(record) => FormMessage::Open(Some(record)),
        event => event,
    };
    match event {
        FormMessage::Open(record) => {
            if app.state.modal_open() || app.state.resync.open || app.state.mutating() {
                return Task::none();
            }
            let Some(context) = scope::context(app).filter(|context| authorized(app, context))
            else {
                return app.error("网关不可用，请刷新后重试");
            };
            if let Some(record) = &record {
                let current = scope::visible(app).and_then(|r| r.as_ref().ok());
                if !current.is_some_and(|c| c.records.contains(record)) {
                    return app.error("目录已变化，请刷新");
                }
            }
            let mut draft = Draft::new(context.clone(), record);
            draft.sync_only = sync_only;
            if let Some(index) = open_global {
                if app.state.page.loading || app.state.page.transfer_pending {
                    return Task::none();
                }
                if let Err(error) = global::open(&mut draft, &app.state.page, index) {
                    return app.error(error);
                }
            }
            if let Some(Ok(catalog)) = scope::visible(app) {
                draft.set_catalog_targets(catalog, &app.targets);
            }
            app.state.connection_form = Some(draft);
            if !sync_only && let Ok(client) = app.client.clone() {
                let token = app.token.clone();
                let generation = app.state.connection_form.as_ref().unwrap().generation;
                return Task::perform(
                    async move {
                        let result =
                            gateway_mcp_read::GatewayMcpReadClient::new(client.transport().clone())
                                .mcp_management_presets(&context.host, &token)
                                .await
                                .map_err(|_| "预设读取失败，可手动编辑".into());
                        FormMessage::Presets(context, generation, result)
                    },
                    std::convert::identity,
                );
            }
        }
        FormMessage::Saved(context, result) => return finish(app, context, result),
        FormMessage::GlobalSaved { context, generation, written, result } => {
            let current = authorized(app, &context);
            let Some(draft) = app
                .state
                .connection_form
                .as_mut()
                .filter(|d| d.context == context && d.generation == generation && d.saving)
            else {
                return Task::none();
            };
            if !current {
                app.state.connection_form = None;
                return app.refresh();
            }
            let wrote_global = written.is_some();
            if let Some(written) = written
                && let Some(global) = &mut draft.global
            {
                global.original = Some(written);
            }
            match result {
                Ok(Some(record)) => return finish(app, context, Ok(record)),
                Ok(None) => {
                    advance(&mut app.state.connection_form);
                    return app.success("定义已保存");
                }
                Err(error) => {
                    if wrote_global {
                        app.effects.push(crate::management_effect::Effect::Info(
                            "识矩全局配置已保存".into(),
                        ));
                    }
                    return finish(app, context, Err(error));
                }
            }
        }
        FormMessage::Distributed(context, generation, results) => {
            let Some(draft) = app.state.connection_form.as_mut().filter(|draft| {
                draft.context == context && draft.generation == generation && draft.saving
            }) else {
                return Task::none();
            };
            draft.saving = false;
            for (target, enabled, result) in &results {
                if let (Some(record), Ok(result)) = (&mut draft.record, result) {
                    for item in &result.items {
                        if item.record.id == record.record.id {
                            record.record.revision =
                                record.record.revision.max(item.record.revision);
                        }
                    }
                }
                draft.deliveries.retain(|(old, _, _)| old != target);
                let _ = enabled;
            }
            draft.deliveries.extend(results);
            if draft.deliveries.iter().all(|(_, _, result)| delivery_ok(result)) {
                let label = if draft.sync_only {
                    "同步完成"
                } else {
                    advance(&mut app.state.connection_form);
                    "定义已保存，全部目标已应用"
                };
                return Task::batch([app.success(label), app.refresh()]);
            }
            return Task::batch([
                app.error("定义已保存；部分目标发布失败或未验证，可重试失败目标"),
                app.refresh(),
            ]);
        }
        FormMessage::Retry => {
            let Some(draft) = app.state.connection_form.as_mut() else {
                return Task::none();
            };
            if draft.saving || app.context.as_ref() != Some(&draft.context) || !app.authorized {
                return Task::none();
            }
            let Some(record) = draft.record.clone() else {
                return Task::none();
            };
            let jobs: Vec<_> = draft
                .deliveries
                .iter()
                .filter(|(_, _, result)| !delivery_ok(result))
                .map(|(target, enabled, _)| (target.clone(), *enabled))
                .collect();
            if jobs.is_empty() {
                return Task::none();
            }
            if jobs.iter().any(|(target, _)| !app.targets.contains(target)) {
                return app.error("目标安装状态已变化，请刷新后重新编辑");
            }
            let Ok(client) = app.client.clone() else {
                return app.error("网关不可用");
            };
            draft.saving = true;
            return deliver(
                client,
                app.token.clone(),
                draft.context.clone(),
                draft.generation,
                record,
                jobs,
            );
        }

        FormMessage::Save => {
            let Some(draft) = app.state.connection_form.as_ref() else {
                return Task::none();
            };
            if draft.saving || !draft.deliveries.is_empty() {
                return Task::none();
            }
            if !authorized(app, &draft.context) {
                return app.error("连接上下文已变化，请关闭后重新编辑");
            }
            if let Err(error) =
                if draft.replace_connection { draft.connection().map(|_| ()) } else { Ok(()) }
            {
                app.state.connection_form.as_mut().unwrap().error = Some(error);
                return Task::none();
            }
            if draft.targets.iter().any(|(target, selected, _, _)| {
                *selected && (!app.targets.contains(target) || !draft.target_supported(target))
            }) {
                return app.error("目标不再可写或不支持当前传输，请取消勾选后保存");
            }
            let Ok(client) = app.client.clone() else {
                return app.error("网关不可用");
            };
            let token = app.token.clone();
            let mut snapshot = draft.clone();
            if draft.sync_only && !draft.targets.iter().any(|(_, selected, _, _)| *selected) {
                return Task::none();
            }
            if let Some(global) = &draft.global {
                let write = if global.include_global && !draft.sync_only {
                    match global::server(draft, global) {
                        Ok(server) => Some(server),
                        Err(error) => {
                            app.state.connection_form.as_mut().unwrap().error = Some(error);
                            return Task::none();
                        }
                    }
                } else {
                    None
                };
                let publish = !global.include_global
                    || draft.targets.iter().any(|(_, selected, original, _)| {
                        *selected || (!draft.sync_only && *original)
                    });
                if publish && global.include_global {
                    snapshot.record = draft
                        .catalog
                        .iter()
                        .find(|record| {
                            record.record.id.0 == draft.id.trim()
                                && matches!(
                                    &record.scope,
                                    vw_api_mcp_management::McpCatalogScope::Tenant { .. }
                                )
                        })
                        .cloned();
                }
                snapshot.id = snapshot.id.trim().to_owned();
                let original = global.original.clone();
                let context = draft.context.clone();
                let generation = draft.generation;
                let draft = app.state.connection_form.as_mut().unwrap();
                draft.saving = true;
                draft.error = None;
                return Task::perform(
                    async move {
                        let (written, result) =
                            match global::persist(&client, original.as_ref(), write.as_ref()).await
                            {
                                Ok(()) => {
                                    let result = if publish {
                                        save(&client, &token, snapshot).await.map(Some)
                                    } else {
                                        Ok(None)
                                    };
                                    (write, result)
                                }
                                Err(error) => (None, Err(error)),
                            };
                        FormMessage::GlobalSaved { context, generation, written, result }
                    },
                    std::convert::identity,
                );
            }
            if draft.sync_only {
                if let Some(record) = snapshot.record.take() {
                    if !scope::visible(app)
                        .and_then(|result| result.as_ref().ok())
                        .is_some_and(|catalog| catalog.records.contains(&record))
                    {
                        return app.error("目录已变化，请刷新");
                    }
                    app.state.connection_form.as_mut().unwrap().saving = true;
                    return finish(app, snapshot.context, Ok(record));
                }
                return Task::none();
            }
            let draft = app.state.connection_form.as_mut().unwrap();
            draft.saving = true;
            draft.error = None;
            return Task::perform(
                async move {
                    let context = snapshot.context.clone();
                    let result = save(&client, &token, snapshot).await;
                    FormMessage::Saved(context, result)
                },
                std::convert::identity,
            );
        }
        event => {
            let Some(draft) = app.state.connection_form.as_mut() else {
                return Task::none();
            };
            if draft.saving {
                return Task::none();
            }
            let previous_id = draft.id.clone();
            match event {
                FormMessage::Close => {
                    app.state.connection_form = None;
                    return Task::none();
                }
                _ if !draft.deliveries.is_empty() => {}
                FormMessage::Target(index, selected) => {
                    draft.edit(FormMessage::Target(index, selected))
                }
                _ if draft.sync_only => {}
                event @ (FormMessage::Replace(_)
                | FormMessage::IncludeGlobal(_)
                | FormMessage::GlobalEnabled(_)
                | FormMessage::GlobalRequired(_)
                | FormMessage::ArgAdd
                | FormMessage::ArgRemove(_)
                | FormMessage::ArgSet(..)
                | FormMessage::PasteEditor(_)
                | FormMessage::ParsePaste
                | FormMessage::ChoosePaste(_)
                | FormMessage::SkipPaste(_)
                | FormMessage::Details(_)
                | FormMessage::Field(..)
                | FormMessage::ValueAdd
                | FormMessage::ValueRemove(_)
                | FormMessage::ValueName(..)
                | FormMessage::ValueKind(..)
                | FormMessage::ValueText(..)
                | FormMessage::ValueService(..)
                | FormMessage::Description(_)
                | FormMessage::Editor(_)
                | FormMessage::Json(_)
                | FormMessage::Preset(_)) => draft.edit(event),
                FormMessage::Presets(context, generation, result)
                    if draft.context == context && draft.generation == generation =>
                {
                    match result {
                        Ok(response) => draft.presets = response.presets,
                        Err(error) => draft.error = Some(error),
                    }
                }
                _ => {}
            }
            if draft.id != previous_id {
                draft.refresh_targets(&app.targets);
            }
        }
    }
    Task::none()
}

fn finish(
    app: &mut Update<'_>,
    context: crate::management_context::Context,
    result: Result<vw_api_mcp_management::McpCatalogRecord, String>,
) -> Task<FormMessage> {
    let current = authorized(app, &context);
    let targets = &app.targets;
    let Some(draft) =
        app.state.connection_form.as_mut().filter(|draft| draft.context == context && draft.saving)
    else {
        return Task::none();
    };
    if !current {
        app.state.connection_form = None;
        return app.refresh();
    }
    draft.finish(&result);
    let record = match result {
        Ok(record) => record,
        Err(error) => return Task::batch([app.error(error), app.refresh()]),
    };
    draft.record = Some(record.clone());
    let jobs = draft
        .targets
        .iter()
        .filter(|(_, selected, original, _)| {
            if draft.sync_only {
                *selected
            } else {
                selected != original || (draft.replace_connection && *selected)
            }
        })
        .map(|(target, selected, _, _)| (target.clone(), *selected))
        .collect::<Vec<_>>();
    if jobs.is_empty() {
        advance(&mut app.state.connection_form);
        return Task::batch([app.success("定义已保存"), app.refresh()]);
    }
    if jobs.iter().any(|(target, enabled)| {
        !targets.contains(target) || (*enabled && !draft.target_supported(target))
    }) {
        return app.error("目标安装状态已变化，请刷新后重新编辑");
    }
    let Ok(client) = app.client.clone() else {
        return app.error("定义已保存，网关不可用，请重试发布");
    };
    draft.saving = true;
    deliver(client, app.token.clone(), context, draft.generation, record, jobs)
}

pub(crate) fn delivery_ok(
    result: &Result<vw_api_mcp_management::McpDistributionResult, String>,
) -> bool {
    result.as_ref().is_ok_and(|result| crate::distribution::summary(result) == "全部目标已应用")
}
fn deliver(
    client: vw_gateway_client::GatewayClient,
    token: String,
    context: crate::management_context::Context,
    generation: u64,
    mut record: vw_api_mcp_management::McpCatalogRecord,
    jobs: Vec<(vw_api_mcp_management::McpAppTarget, bool)>,
) -> Task<FormMessage> {
    Task::perform(
        async move {
            let mut results = Vec::new();
            for (target, enabled) in jobs {
                let result = crate::management_bindings::apply(
                    client.clone(),
                    vec![record.clone()],
                    target.clone(),
                    enabled,
                    token.clone(),
                )
                .await;
                // Desired-binding writes may advance the catalog revision. Carry the returned
                // revision into the next target, including partial native failures.
                if let Ok(result) = &result {
                    for item in &result.items {
                        if item.record.id == record.record.id {
                            record.record.revision =
                                record.record.revision.max(item.record.revision);
                        }
                    }
                }
                results.push((target, enabled, result));
            }
            FormMessage::Distributed(context, generation, results)
        },
        std::convert::identity,
    )
}

// Multi-entry paste stays open until the user explicitly fills or skips each item.
fn advance(form: &mut Option<Draft>) {
    let Some(previous) = form.take() else {
        return;
    };
    if previous.candidates.is_empty() {
        return;
    }
    let mut next = Draft::new(previous.context, None);
    next.global = previous.global.map(|mut global| {
        global.original = None;
        global.enabled = true;
        global.required = false;
        global.timeout = "30".into();
        global
    });
    next.candidates = previous.candidates;
    next.catalog = previous.catalog;
    if let Some(record) = previous.record {
        next.catalog.retain(|r| r.record.id != record.record.id);
        next.catalog.push(record);
    }
    next.bindings = previous.bindings;
    next.capabilities = previous.capabilities;
    next.presets = previous.presets;
    next.targets = previous
        .targets
        .into_iter()
        .map(|(target, _, _, _)| (target, false, false, false))
        .collect();
    next.select_default_target();
    *form = Some(next);
}
