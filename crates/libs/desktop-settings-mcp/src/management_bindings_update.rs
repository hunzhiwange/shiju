use crate::distribution::{project, summary, supported};
use crate::management_bindings::*;
use crate::management_update::{self as scope, Update};
use iced::Task;
use vw_api_mcp_management::*;
pub fn update(app: &mut Update<'_>, value: BindingsMessage) -> Task<BindingsMessage> {
    let Some(context) = scope::context(app) else {
        return app.error("网关不可用");
    };
    if app.state.bindings.context.as_ref() != Some(&context) {
        let generation = app.state.bindings.generation.wrapping_add(1);
        app.state.bindings =
            BindingsState { context: Some(context.clone()), generation, ..Default::default() };
    }
    match value {
        BindingsMessage::Select(record, selected) => {
            if app.state.bindings.running {
                return Task::none();
            }
            let Some(Ok(current)) = scope::visible(app) else {
                return Task::none();
            };
            if !current.records.contains(&record) {
                return Task::none();
            }
            let current_records = current.records.clone();
            let records = &mut app.state.bindings.selected;
            records.retain(|r| current_records.contains(r) && r != &record);
            if selected {
                records.push(record);
            }
            Task::none()
        }
        BindingsMessage::RetryFailed => {
            if app.state.bindings.running || scope::loading(app) {
                return Task::none();
            }
            let Some(Ok(result)) = app.state.bindings.result.as_ref() else {
                return Task::none();
            };
            let failed: Vec<_> = result
                .items
                .iter()
                .filter(|item| matches!(item.native_outcome, McpDistributionOutcome::Failed { .. }))
                .collect();
            let Some(first) = failed.first() else {
                return Task::none();
            };
            let target = first.target.clone();
            let enabled = first.enabled;
            let Some(Ok(current)) = scope::visible(app) else {
                return app.error("请先刷新目录再重试失败项");
            };
            let mut records = Vec::new();
            for item in failed {
                if item.target != target || item.enabled != enabled {
                    return app.error("失败项目标不一致，请重新选择");
                }
                let Some(record) =
                    current.records.iter().find(|record| record.record == item.record)
                else {
                    return app.error("失败项目录修订已变更，请重新选择");
                };
                records.push(record.clone());
            }
            // Apply performs the same capability, scope and installation checks as a new request.
            app.state.bindings.selected = records;
            update(app, BindingsMessage::Apply(target, enabled))
        }
        BindingsMessage::Apply(target, enabled) => {
            if app.state.bindings.running {
                return Task::none();
            }
            let records = app.state.bindings.selected.clone();
            let valid_catalog = matches!(scope::visible(app), Some(Ok(c)) if records.iter().all(|r| c.records.contains(r)) && supported(c, &records, target.app_id));
            let valid_installation = app.targets.contains(&target);
            if !valid_catalog || !valid_installation || project(&records).is_err() {
                return app.error("选择无效、修订已变更或目标不支持；请刷新并选择同一作用域目录项");
            }
            let client = match app.client.clone() {
                Ok(client) => client,
                Err(_) => return app.error("网关不可用"),
            };
            let state = &mut app.state.bindings;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            state.running = true;
            state.result = None;
            let token = app.token.clone();
            Task::perform(
                async move {
                    let result = apply(client, records, target, enabled, token).await;
                    BindingsMessage::Finished { context, generation, result }
                },
                std::convert::identity,
            )
        }
        BindingsMessage::Finished { context: request, generation, result } => {
            if !app.state.bindings.finish(&context, &request, generation, result) {
                return Task::none();
            }
            let feedback = match app.state.bindings.result.as_ref().expect("finished result") {
                Ok(result) if summary(result) == "全部目标已应用" => {
                    app.success("全部目标已应用")
                }
                Ok(_) => app.error("存在失败或未验证目标，请查看逐项结果"),
                Err(error) => app.error(error.clone()),
            };
            Task::batch([feedback, app.refresh()])
        }
    }
}
