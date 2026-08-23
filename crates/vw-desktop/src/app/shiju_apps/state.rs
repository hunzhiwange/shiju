//! 定义识矩应用目录、已安装列表、密钥绑定与启动过程的状态模型。

use vw_gateway_client::{ShijuAppCatalogDto, ShijuAppListDto, ShijuIamApiKeyDto};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩应用页面的商店与已安装页签。
pub(crate) enum ShijuAppsTab {
    Store,
    #[default]
    Mine,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 账号范围列表请求的加载状态；代次用于拒绝迟到响应。
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
/// 应用详情在商店或已安装列表中的稳定索引目标。
pub(crate) enum ShijuAppDetailTarget {
    Store(usize),
    Mine(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 从密钥绑定检查到 WebView 启动完成的状态机。
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
/// 识矩应用页面的账号范围状态与独立请求代次。
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
    /// 切换账号时使全部在途结果失效并清空目录、详情和启动状态。
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

    /// 仅当代次与账号同时匹配时接受列表响应。
    pub(crate) fn response_is_current(&self, generation: u64, account_id: &str) -> bool {
        self.generation == generation && self.account_id.as_deref() == Some(account_id)
    }

    /// 仅当代次、账号与安装标识都匹配当前启动流程时接受响应。
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
