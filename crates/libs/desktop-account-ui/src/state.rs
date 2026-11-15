//! 保存账号操作、资料编辑与密码编辑的界面状态。

use desktop_account::{
    AccountPasswordDraft, AccountProfileDraft, AccountRepository, LoginChallenge, ManualSkeyInput,
    StoredAccount,
};
use web_time::Instant;

#[derive(Debug, Clone)]
/// 当前账号登录或持久化操作的互斥阶段。
pub enum AccountOperation {
    Idle,
    Validating,
    StartingBrowser,
    WaitingAuthorization { challenge: LoginChallenge, started_at: Instant },
    Saving,
}

impl AccountOperation {
    pub fn is_busy(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    pub fn label(&self) -> Option<&'static str> {
        match self {
            Self::Idle => None,
            Self::WaitingAuthorization { .. } => Some("等待授权…"),
            Self::StartingBrowser => Some("正在登录…"),
            Self::Validating | Self::Saving => Some("正在验证…"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// 账号资料编辑器从关闭到加载、编辑、保存和失败的状态。
pub enum AccountProfileEditorState {
    #[default]
    Closed,
    Loading,
    Editing(AccountProfileDraft),
    Saving(AccountProfileDraft),
    Failed(String),
}

impl AccountProfileEditorState {
    pub fn draft(&self) -> Option<&AccountProfileDraft> {
        match self {
            Self::Editing(draft) | Self::Saving(draft) => Some(draft),
            Self::Closed | Self::Loading | Self::Failed(_) => None,
        }
    }

    pub fn draft_mut(&mut self) -> Option<&mut AccountProfileDraft> {
        match self {
            Self::Editing(draft) => Some(draft),
            Self::Closed | Self::Loading | Self::Saving(_) | Self::Failed(_) => None,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Loading | Self::Saving(_))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// 密码编辑器状态；敏感草稿始终由脱敏类型持有。
pub enum AccountPasswordEditorState {
    #[default]
    Closed,
    Editing(AccountPasswordDraft),
    Saving(AccountPasswordDraft),
    Failed {
        draft: AccountPasswordDraft,
        message: String,
    },
}

impl AccountPasswordEditorState {
    pub fn draft(&self) -> Option<&AccountPasswordDraft> {
        match self {
            Self::Editing(draft) | Self::Saving(draft) | Self::Failed { draft, .. } => Some(draft),
            Self::Closed => None,
        }
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Failed { message, .. } => Some(message),
            Self::Closed | Self::Editing(_) | Self::Saving(_) => None,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Saving(_))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MonthlyQuotaState {
    Idle,
    Loading(Option<gateway_shiju_account::ShijuMonthlyQuotaDto>),
    Ready(gateway_shiju_account::ShijuMonthlyQuotaDto),
    Unavailable,
}

#[derive(Debug, Clone)]
/// 账号仓储、弹窗和独立请求代次的界面状态。
pub struct AccountState {
    repository: AccountRepository,
    pub menu_open: bool,
    pub details_open: bool,
    pub monthly_quota: MonthlyQuotaState,
    pub monthly_quota_generation: u64,
    pub manual_input: ManualSkeyInput,
    pub operation: AccountOperation,
    generation: u64,
    pub profile_editor: AccountProfileEditorState,
    profile_generation: u64,
    pub password_editor: AccountPasswordEditorState,
    password_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    model_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    pub native_shiju_skill: desktop_skill_hub::NativeShijuSkillState,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            repository: AccountRepository::default(),
            menu_open: false,
            details_open: false,
            monthly_quota: MonthlyQuotaState::Idle,
            monthly_quota_generation: 0,
            manual_input: ManualSkeyInput::default(),
            operation: AccountOperation::Idle,
            generation: 0,
            profile_editor: AccountProfileEditorState::Closed,
            profile_generation: 0,
            password_editor: AccountPasswordEditorState::Closed,
            password_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            model_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            native_shiju_skill: desktop_skill_hub::NativeShijuSkillState::default(),
        }
    }
}

impl AccountState {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn begin_model_sync(&mut self) -> u64 {
        self.model_generation = self.model_generation.wrapping_add(1);
        self.model_generation
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn model_sync_is_current(&self, generation: u64) -> bool {
        self.model_generation == generation
    }

    pub fn repository(&self) -> &AccountRepository {
        &self.repository
    }

    pub fn accounts(&self) -> &[StoredAccount] {
        self.repository.accounts()
    }

    pub fn active_account(&self) -> Option<&StoredAccount> {
        self.repository.active_account()
    }

    pub fn replace_repository(&mut self, repository: AccountRepository) {
        #[cfg(not(target_arch = "wasm32"))]
        self.native_shiju_skill.set_credential(
            repository.active_account().map(|account| account.skill_hub_credential()),
        );
        if self.repository.active_account_id() != repository.active_account_id() {
            self.monthly_quota = MonthlyQuotaState::Idle;
            self.monthly_quota_generation = self.monthly_quota_generation.wrapping_add(1);
        }
        self.repository = repository;
    }

    /// 开始账号操作并递增代次，返回值用于校验异步结果是否仍有效。
    pub fn begin(&mut self, operation: AccountOperation) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.operation = operation;
        self.generation
    }

    /// 判断结果是否属于当前账号操作代次。
    pub fn is_current(&self, generation: u64) -> bool {
        self.generation == generation
    }

    pub fn finish(&mut self) {
        self.operation = AccountOperation::Idle;
    }

    pub fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.operation = AccountOperation::Idle;
    }

    /// 更新资料编辑器并使上一轮资料请求失效。
    pub fn begin_profile_editor(&mut self, state: AccountProfileEditorState) -> u64 {
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.profile_editor = state;
        self.profile_generation
    }

    pub fn profile_is_current(&self, generation: u64) -> bool {
        self.profile_generation == generation
    }

    pub fn cancel_profile_editor(&mut self) {
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.profile_editor = AccountProfileEditorState::Closed;
    }

    /// 更新密码编辑器并使上一轮密码请求失效。
    pub fn begin_password_editor(&mut self, state: AccountPasswordEditorState) -> u64 {
        self.password_generation = self.password_generation.wrapping_add(1);
        self.password_editor = state;
        self.password_generation
    }

    pub fn password_is_current(&self, generation: u64) -> bool {
        self.password_generation == generation
    }

    pub fn cancel_password_editor(&mut self) {
        self.password_generation = self.password_generation.wrapping_add(1);
        self.password_editor = AccountPasswordEditorState::Closed;
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod account_tests;
