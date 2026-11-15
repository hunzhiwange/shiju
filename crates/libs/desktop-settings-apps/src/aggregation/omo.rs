use desktop_apps_ui::aggregation::omo_state::{Form, Snapshot, Variant};
use iced::Task;
use gateway_aggregation_omo::AggregationOmoError;
use gateway_aggregation_omo::GatewayAggregationOmoClient;
use vw_api_app_aggregation::OmoApplyRequest;
use vw_api_app_aggregation::OmoDisableRequest;

pub use desktop_apps_ui::aggregation::omo::{Context, Event};
fn message(event: Event) -> Event { event }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Read,
    Apply(String),
    Disable,
}
pub async fn request(
    client: GatewayAggregationOmoClient,
    context: Context,
    variant: Variant,
    action: Action,
    revision: String,
) -> Result<Snapshot, AggregationOmoError> {
    let host = context.scope.host;
    let project = context.project;
    match action {
        Action::Read => match variant {
            Variant::Standard => {
                client.aggregation_omo_state(&host, &project).await.map(Snapshot::Standard)
            }
            Variant::Slim => {
                client.aggregation_omo_slim_state(&host, &project).await.map(Snapshot::Slim)
            }
        },
        Action::Apply(provider_id) => {
            let request = OmoApplyRequest {
                target_host_id: host,
                project_id: project,
                expected_revision: revision,
                provider_id,
            };
            match variant {
                Variant::Standard => {
                    client.aggregation_omo_apply(&request).await.map(Snapshot::Standard)
                }
                Variant::Slim => {
                    client.aggregation_omo_slim_apply(&request).await.map(Snapshot::Slim)
                }
            }
        }
        Action::Disable => {
            let request = OmoDisableRequest {
                target_host_id: host,
                project_id: project,
                expected_revision: revision,
            };
            match variant {
                Variant::Standard => {
                    client.aggregation_omo_disable(&request).await.map(Snapshot::Standard)
                }
                Variant::Slim => {
                    client.aggregation_omo_slim_disable(&request).await.map(Snapshot::Slim)
                }
            }
        }
    }
}

pub fn update(
    omo: &mut Option<Form>,
    counter: &mut u64,
    loading: bool,
    saved: bool,
    current: Result<(GatewayAggregationOmoClient, Context), String>,
    notice: &mut Option<super::Notice>,
    event: Event,
) -> Task<Event> {
    if matches!(event, Event::Close) {
        // Closing detaches the view; it does not claim to cancel a target transaction.
        (*omo) = None;
        *counter = counter.wrapping_add(1);
        return Task::none();
    }
    let (client, context) = match current {
        Ok(v) => v,
        Err(_) if matches!(event, Event::Finished { .. }) => return Task::none(),
        Err(e) => return super::error(notice, e),
    };
    if let Event::Finished { context: original, generation, result } = event {
        if original != context || generation != *counter {
            return Task::none();
        }
        let Some(form) = (*omo).as_mut().filter(|f| f.busy) else {
            return Task::none();
        };
        form.finish(result);
        return match form.error.clone() {
            Some(e) => super::error(notice, e),
            None => super::info(notice, "已读取目标实际状态"),
        };
    }
    if let Event::Open(variant) = event {
        if (*omo).is_some() || loading {
            return Task::none();
        }
        let mut form = Form::new(variant);
        form.binding = Some((context.project.clone(), context.access));
        (*omo) = Some(form);
    } else if (*omo).as_ref().is_none_or(|f| f.busy) {
        return Task::none();
    }
    if (*omo)
        .as_ref()
        .is_some_and(|f| f.binding.as_ref() != Some(&(context.project.clone(), context.access)))
    {
        (*omo) = None;
        return super::error(notice, "项目或授权已切换，请重新打开目录");
    }
    let Some(form) = (*omo).as_mut() else { return Task::none() };
    if let Event::Provider(v) = event {
        form.provider = v;
        return Task::none();
    }
    let revision = form.snapshot.as_ref().map(|s| s.revision().to_owned()).unwrap_or_default();
    let action = match event {
        Event::Apply => {
            if revision.is_empty() || !saved {
                return super::error(notice, "请选择已保存供应商，并刷新配置状态");
            }
            Action::Apply(form.provider.clone())
        }
        Event::Disable if !revision.is_empty() => Action::Disable,
        Event::Open(_) | Event::Refresh => Action::Read,
        _ => return Task::none(),
    };
    form.busy = true;
    let variant = form.variant;
    *counter = counter.wrapping_add(1);
    let generation = *counter;
    Task::perform(request(client, context.clone(), variant, action, revision), move |result| {
        message(Event::Finished { context: context.clone(), generation, result })
    })
}

#[cfg(test)]
#[path = "omo_tests.rs"]
mod tests;
