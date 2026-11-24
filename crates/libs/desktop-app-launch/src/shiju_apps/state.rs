//! 定义识矩应用目录、已安装列表、密钥绑定与启动过程的状态模型。

use gateway_shiju_app::{
    ShijuAppApiKeyBindingDto, ShijuAppCatalogDto, ShijuAppCategory, ShijuAppCopyRequest,
    ShijuAppListDto, ShijuCreatedApp, ShijuFeaturedApp, ShijuIamApiKeyDto,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩应用页面的商店与已安装页签。
pub enum ShijuAppsTab {
    Store,
    #[default]
    Mine,
    Favorites,
    Created,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 账号范围列表请求的加载状态；代次用于拒绝迟到响应。
pub enum ShijuAppsLoadState<T> {
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
pub enum ShijuAppDetailTarget {
    Store(usize),
    Mine(usize),
    Featured(usize),
    Created(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 从密钥绑定检查到 WebView 启动完成的状态机。
pub enum ShijuAppLaunchState {
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
pub struct ShijuAppsBrowse {
    pub query: String,
    pub category: String,
    pub source: String,
    pub recent: bool,
    pub lifecycle: String,
    pub page: usize,
    pub page_size: usize,
}

impl Default for ShijuAppsBrowse {
    fn default() -> Self {
        Self {
            query: String::new(),
            category: String::new(),
            source: String::new(),
            recent: false,
            lifecycle: String::new(),
            page: 0,
            page_size: 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShijuDetailMetadata {
    Development(gateway_shiju_app::ShijuDevelopmentDetail),
    Installation(gateway_shiju_app::ShijuInstalledAppDto),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShijuMarketplaceAction {
    Favorite(String, bool),
    Install(String),
    Copy(String, ShijuAppCopyRequest),
    Download(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 识矩应用页面的账号范围状态与独立请求代次。
pub struct ShijuAppsState {
    pub tab: ShijuAppsTab,
    pub store: ShijuAppsLoadState<ShijuAppCatalogDto>,
    pub mine: ShijuAppsLoadState<ShijuAppListDto>,
    pub studio_lookup_generation: Option<u64>,
    pub created: ShijuAppsLoadState<Vec<ShijuCreatedApp>>,
    pub featured: ShijuAppsLoadState<Vec<ShijuFeaturedApp>>,
    pub featured_generation: u64,
    pub artwork: std::collections::HashMap<u64, Option<iced_core::image::Handle>>,
    pub categories: Vec<ShijuAppCategory>,
    pub browse: ShijuAppsBrowse,
    pub action: Option<(u64, ShijuMarketplaceAction)>,
    pub copy_form: Option<ShijuAppCopyRequest>,
    pub binding: ShijuAppsLoadState<ShijuAppApiKeyBindingDto>,
    pub changing_binding: bool,
    pub account_id: Option<String>,
    pub generation: u64,
    pub detail: Option<ShijuAppDetailTarget>,
    pub detail_metadata: ShijuAppsLoadState<ShijuDetailMetadata>,
    pub launch: ShijuAppLaunchState,
    pub launch_feedback: Option<(u64, String, desktop_skill_hub::SkillHubError)>,
}

impl ShijuAppsState {
    pub fn artwork(&self, url: Option<&str>) -> Option<iced_core::image::Handle> {
        let key = Self::artwork_key(url?);
        self.artwork.get(&key)?.clone()
    }

    pub fn artwork_key(url: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut key = std::collections::hash_map::DefaultHasher::new();
        url.hash(&mut key);
        key.finish()
    }

    pub fn installed_detail(&self) -> Option<&gateway_shiju_app::ShijuInstalledAppDto> {
        let id = self.detail_installation()?;
        let ShijuAppsLoadState::Loaded(mine) = &self.mine else { return None };
        mine.apps.iter().find(|item| item.installation_id == id)
    }

    pub fn detail_installation(&self) -> Option<&str> {
        let id = match self.detail? {
            ShijuAppDetailTarget::Store(index) => {
                let ShijuAppsLoadState::Loaded(store) = &self.store else { return None };
                store.apps.get(index)?.installation_id.as_deref()?
            }
            ShijuAppDetailTarget::Mine(index) => {
                let ShijuAppsLoadState::Loaded(mine) = &self.mine else { return None };
                &mine.apps.get(index)?.installation_id
            }
            ShijuAppDetailTarget::Featured(index) => {
                let ShijuAppsLoadState::Loaded(featured) = &self.featured else { return None };
                &featured.get(index)?.app.installation_id
            }
            ShijuAppDetailTarget::Created(index) => {
                let ShijuAppsLoadState::Loaded(created) = &self.created else { return None };
                created.get(index)?.installation_id.as_deref()?
            }
        };
        (!id.is_empty()).then_some(id)
    }

    pub fn render_launch_error(&self, source: &str, i18n: desktop_i18n::I18n) -> String {
        if let Some((generation, original, error)) = &self.launch_feedback
            && original == source
                && self.launch.request_identity().is_some_and(|(active, _)| active == *generation)
            {
                return error.render(i18n);
            }
        source.to_string()
    }

    /// 切换账号时使全部在途结果失效并清空目录、详情和启动状态。
    pub fn clear_for_account(&mut self, account_id: Option<String>) {
        self.generation = self.generation.wrapping_add(1);
        self.account_id = account_id;
        self.store = ShijuAppsLoadState::Idle;
        self.mine = ShijuAppsLoadState::Idle;
        self.studio_lookup_generation = None;
        self.created = ShijuAppsLoadState::Idle;
        self.featured = ShijuAppsLoadState::Idle;
        self.featured_generation = self.generation;
        self.artwork.clear();
        self.categories.clear();
        self.browse = Default::default();
        self.action = None;
        self.copy_form = None;
        self.binding = ShijuAppsLoadState::Idle;
        self.changing_binding = false;
        self.detail = None;
        self.detail_metadata = ShijuAppsLoadState::Idle;
        self.launch = ShijuAppLaunchState::Idle;
        self.launch_feedback = None;
    }

    pub fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    /// 仅当代次与账号同时匹配时接受列表响应。
    pub fn response_is_current(&self, generation: u64, account_id: &str) -> bool {
        self.generation == generation && self.account_id.as_deref() == Some(account_id)
    }

    /// 仅当代次、账号与安装标识都匹配当前启动流程时接受响应。
    pub fn launch_response_is_current(
        &self,
        generation: u64,
        account_id: &str,
        installation_id: &str,
    ) -> bool {
        self.account_id.as_deref() == Some(account_id)
            && self.launch.request_identity() == Some((generation, installation_id))
    }
}

impl Default for ShijuAppsState {
    fn default() -> Self {
        Self {
            tab: ShijuAppsTab::Mine,
            store: ShijuAppsLoadState::Idle,
            mine: ShijuAppsLoadState::Idle,
            studio_lookup_generation: None,
            created: ShijuAppsLoadState::Idle,
            featured: ShijuAppsLoadState::Idle,
            featured_generation: 0,
            artwork: Default::default(),
            categories: Vec::new(),
            browse: ShijuAppsBrowse::default(),
            action: None,
            binding: ShijuAppsLoadState::Idle,
            copy_form: None,
            changing_binding: false,
            account_id: None,
            generation: 0,
            detail: None,
            detail_metadata: ShijuAppsLoadState::Idle,
            launch: ShijuAppLaunchState::Idle,
            launch_feedback: None,
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
