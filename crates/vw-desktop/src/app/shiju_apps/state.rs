//! State model for the SHIJU application catalog and installed list.

use vw_gateway_client::{ShijuAppCatalogDto, ShijuAppListDto, ShijuIamApiKeyDto};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ShijuAppsTab {
    Store,
    #[default]
    Mine,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ShijuAppsLoadState<T> {
    #[default]
    Idle,
    NeedsLogin,
    Loading {
        generation: u64,
    },
    Loaded(T),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShijuAppDetailTarget {
    Store(usize),
    Mine(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ShijuAppLaunchState {
    #[default]
    Idle,
    CheckingBinding {
        generation: u64,
        installation_id: String,
    },
    LoadingApiKeys {
        generation: u64,
        installation_id: String,
        can_edit: bool,
    },
    SelectingApiKey {
        generation: u64,
        installation_id: String,
        can_edit: bool,
        api_keys: Vec<ShijuIamApiKeyDto>,
        selected_api_key_id: Option<u64>,
        error: Option<String>,
    },
    SavingApiKeyBinding {
        generation: u64,
        installation_id: String,
        can_edit: bool,
        api_keys: Vec<ShijuIamApiKeyDto>,
        selected_api_key_id: u64,
    },
    Preparing {
        generation: u64,
        installation_id: String,
    },
    Error {
        generation: u64,
        installation_id: String,
        message: String,
    },
}

impl ShijuAppLaunchState {
    fn request_identity(&self) -> Option<(u64, &str)> {
        match self {
            Self::CheckingBinding { generation, installation_id }
            | Self::LoadingApiKeys { generation, installation_id, .. }
            | Self::SelectingApiKey { generation, installation_id, .. }
            | Self::SavingApiKeyBinding { generation, installation_id, .. }
            | Self::Preparing { generation, installation_id }
            | Self::Error { generation, installation_id, .. } => {
                Some((*generation, installation_id))
            }
            Self::Idle => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuAppsState {
    pub(crate) tab: ShijuAppsTab,
    pub(crate) store: ShijuAppsLoadState<ShijuAppCatalogDto>,
    pub(crate) mine: ShijuAppsLoadState<ShijuAppListDto>,
    pub(crate) account_id: Option<String>,
    pub(crate) generation: u64,
    pub(crate) detail: Option<ShijuAppDetailTarget>,
    pub(crate) launch: ShijuAppLaunchState,
}

impl ShijuAppsState {
    pub(crate) fn clear_for_account(&mut self, account_id: Option<String>) {
        self.generation = self.generation.wrapping_add(1);
        self.account_id = account_id;
        self.store = ShijuAppsLoadState::Idle;
        self.mine = ShijuAppsLoadState::Idle;
        self.detail = None;
        self.launch = ShijuAppLaunchState::Idle;
    }

    pub(crate) fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    pub(crate) fn response_is_current(&self, generation: u64, account_id: &str) -> bool {
        self.generation == generation && self.account_id.as_deref() == Some(account_id)
    }

    pub(crate) fn launch_response_is_current(
        &self,
        generation: u64,
        account_id: &str,
        installation_id: &str,
    ) -> bool {
        self.response_is_current(generation, account_id)
            && self.launch.request_identity() == Some((generation, installation_id))
    }
}

impl Default for ShijuAppsState {
    fn default() -> Self {
        Self {
            tab: ShijuAppsTab::Mine,
            store: ShijuAppsLoadState::Idle,
            mine: ShijuAppsLoadState::Idle,
            account_id: None,
            generation: 0,
            detail: None,
            launch: ShijuAppLaunchState::Idle,
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
