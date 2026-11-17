use super::*;
use crate::Identity;
use gateway_usage_read::UsageSummary;
#[derive(Clone, Debug)]
pub enum Event {
    Load(bool),
    Refresh,
    Display(display::Event),
    Pricing(pricing::Event),
    Models(crate::models::Event),
    Charts(charts::Event),
    Scan(scan::Event),
    Logs(logs::Event),
    Groups(groups::Event),
    /// Host calls this when page/window visibility or identity changes.
    ActivityChanged,
    ActivitySettled(Identity, bool, Result<(), String>),
    SyncStatus(u64, Identity, Result<gateway_usage_scan::UsageLastScan, String>),
    Tick,
    Filter(overview::Change),
    Close,
    Loaded(u64, Identity, Result<UsageSummary, String>),
}


pub fn load(client: Result<vw_gateway_client::GatewayClient, String>, ticket: u64, scope: Identity,
    query: vw_api_usage::UsageQuery) -> iced::Task<Event> {
    let client = client.map(|client| gateway_usage_read::GatewayUsageReadClient::new(client.transport().clone()));
    iced::Task::perform(async move {
        let result = async {
            let client = client?;
            client.usage_summary(&gateway_usage_read::UsageReadScope {
                project_id: scope.project.clone().into(), target_host: scope.host.clone(),
                time_zone: "UTC".into(),
            }, &query).await.map_err(|e| e.to_string())
        }.await;
        (scope, result)
    }, move |(scope, result)| Event::Loaded(ticket, scope, result))
}
