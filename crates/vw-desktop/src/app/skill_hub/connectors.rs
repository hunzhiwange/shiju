//! 连接器列表、个人绑定草稿与授权流程的账号范围状态。
use super::LoadState;
use std::{collections::BTreeMap, time::Instant};
use vw_gateway_client::{ShijuConnector, ShijuConnectorChallenge, ShijuConnectorSaveRequest};

#[derive(Clone)]
pub(crate) struct ConnectorForm {
    pub connector: ShijuConnector,
    pub account_id: String,
    pub account_name: String,
    pub values: BTreeMap<String, String>,
}
impl std::fmt::Debug for ConnectorForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConnectorForm(<redacted>)")
    }
}
impl ConnectorForm {
    pub fn new(connector: ShijuConnector) -> Self {
        let binding = connector.binding.as_ref();
        let values = connector
            .account_binding_variables
            .iter()
            .map(|variable| {
                let value = if variable.kind == "password" {
                    String::new()
                } else {
                    binding
                        .and_then(|binding| binding.binding_config.get(&variable.name))
                        .filter(|value| !value.is_null())
                        .map(|value| {
                            value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string())
                        })
                        .unwrap_or_default()
                };
                (variable.name.clone(), value)
            })
            .collect();
        Self {
            account_id: binding.map(|b| b.third_party_account_id.clone()).unwrap_or_default(),
            account_name: binding.map(|b| b.third_party_account_name.clone()).unwrap_or_default(),
            connector,
            values,
        }
    }
    pub fn request(&self) -> Result<ShijuConnectorSaveRequest, String> {
        let mut binding_config = serde_json::Map::new();
        for variable in &self.connector.account_binding_variables {
            if !matches!(variable.kind.as_str(), "password" | "string") {
                return Err(format!("暂不支持变量类型：{}", variable.kind));
            }
            let value = self.values.get(&variable.name).cloned().unwrap_or_default();
            if variable.required && value.trim().is_empty() {
                let label = if variable.description.trim().is_empty() {
                    &variable.name
                } else {
                    &variable.description
                };
                return Err(format!("请填写{label}"));
            }
            if variable.kind != "password" || !value.is_empty() {
                binding_config.insert(variable.name.clone(), value.into());
            }
        }
        Ok(ShijuConnectorSaveRequest {
            company_platform_id: self.connector.company_platform_id,
            third_party_account_id: self.account_id.trim().to_string(),
            third_party_account_name: self.account_name.trim().to_string(),
            binding_config,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ConnectorWork {
    Saving,
    Starting,
    Authorizing { challenge: ShijuConnectorChallenge, deadline: Instant },
}
#[derive(Debug, Clone)]
pub(crate) struct ConnectorOperation {
    pub generation: u64,
    pub work: ConnectorWork,
}

#[derive(Debug, Clone)]
pub(crate) struct ConnectorsState {
    pub list: LoadState<Vec<ShijuConnector>>,
    pub form: Option<ConnectorForm>,
    pub work: BTreeMap<u64, ConnectorOperation>,
    pub list_generation: u64,
    pub list_loading: bool,
    pub generation: u64,
    pub account_id: Option<String>,
}
impl Default for ConnectorsState {
    fn default() -> Self {
        Self {
            list: LoadState::Idle,
            form: None,
            work: BTreeMap::new(),
            list_generation: 0,
            list_loading: false,
            generation: 0,
            account_id: None,
        }
    }
}
impl ConnectorsState {
    pub fn clear(&mut self, account_id: Option<String>) {
        self.generation = self.generation.wrapping_add(1);
        self.account_id = account_id;
        self.list = if self.account_id.is_some() { LoadState::Idle } else { LoadState::NeedsLogin };
        self.form = None;
        self.work.clear();
        self.list_generation = self.generation;
        self.list_loading = false;
    }
}

impl ConnectorsState {
    pub fn begin(&mut self, id: u64, work: ConnectorWork) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.work.insert(id, ConnectorOperation { generation: self.generation, work });
        self.generation
    }
    pub fn form_is_saving(&self) -> bool {
        self.form.as_ref().is_some_and(|form| {
            self.work
                .get(&form.connector.company_platform_id)
                .is_some_and(|operation| matches!(operation.work, ConnectorWork::Saving))
        })
    }
}
