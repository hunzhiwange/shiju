use crate::app::account::{
    AccountPasswordDraft, AccountProfileDraft, AccountRepository, LoginChallenge, ManualSkeyInput,
    StoredAccount,
};
use std::time::Instant;

#[derive(Debug, Clone)]
pub(crate) enum AccountOperation {
    Idle,
    Validating,
    StartingBrowser,
    WaitingAuthorization { challenge: LoginChallenge, started_at: Instant },
    Saving,
}

impl AccountOperation {
    pub(crate) fn is_busy(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    pub(crate) fn label(&self) -> Option<&'static str> {
        match self {
            Self::Idle => None,
            Self::WaitingAuthorization { .. } => Some("等待授权…"),
            Self::StartingBrowser => Some("正在登录…"),
            Self::Validating | Self::Saving => Some("正在验证…"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum AccountProfileEditorState {
    #[default]
    Closed,
    Loading,
    Editing(AccountProfileDraft),
    Saving(AccountProfileDraft),
    Failed(String),
}

impl AccountProfileEditorState {
    pub(crate) fn draft(&self) -> Option<&AccountProfileDraft> {
        match self {
            Self::Editing(draft) | Self::Saving(draft) => Some(draft),
            Self::Closed | Self::Loading | Self::Failed(_) => None,
        }
    }

    pub(crate) fn draft_mut(&mut self) -> Option<&mut AccountProfileDraft> {
        match self {
            Self::Editing(draft) => Some(draft),
            Self::Closed | Self::Loading | Self::Saving(_) | Self::Failed(_) => None,
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        matches!(self, Self::Loading | Self::Saving(_))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum AccountPasswordEditorState {
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
    pub(crate) fn draft(&self) -> Option<&AccountPasswordDraft> {
        match self {
            Self::Editing(draft) | Self::Saving(draft) | Self::Failed { draft, .. } => Some(draft),
            Self::Closed => None,
        }
    }

    pub(crate) fn error(&self) -> Option<&str> {
        match self {
            Self::Failed { message, .. } => Some(message),
            Self::Closed | Self::Editing(_) | Self::Saving(_) => None,
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        matches!(self, Self::Saving(_))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AccountState {
    repository: AccountRepository,
    pub(crate) menu_open: bool,
    pub(crate) details_open: bool,
    pub(crate) manual_input: ManualSkeyInput,
    pub(crate) operation: AccountOperation,
    generation: u64,
    pub(crate) profile_editor: AccountProfileEditorState,
    profile_generation: u64,
    pub(crate) password_editor: AccountPasswordEditorState,
    password_generation: u64,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            repository: AccountRepository::default(),
            menu_open: false,
            details_open: false,
            manual_input: ManualSkeyInput::default(),
            operation: AccountOperation::Idle,
            generation: 0,
            profile_editor: AccountProfileEditorState::Closed,
            profile_generation: 0,
            password_editor: AccountPasswordEditorState::Closed,
            password_generation: 0,
        }
    }
}

impl AccountState {
    pub(crate) fn repository(&self) -> &AccountRepository {
        &self.repository
    }

    pub(crate) fn accounts(&self) -> &[StoredAccount] {
        self.repository.accounts()
    }

    pub(crate) fn active_account(&self) -> Option<&StoredAccount> {
        self.repository.active_account()
    }

    pub(crate) fn replace_repository(&mut self, repository: AccountRepository) {
        self.repository = repository;
    }

    pub(crate) fn begin(&mut self, operation: AccountOperation) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.operation = operation;
        self.generation
    }

    pub(crate) fn is_current(&self, generation: u64) -> bool {
        self.generation == generation
    }

    pub(crate) fn finish(&mut self) {
        self.operation = AccountOperation::Idle;
    }

    pub(crate) fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.operation = AccountOperation::Idle;
    }

    pub(crate) fn begin_profile_editor(&mut self, state: AccountProfileEditorState) -> u64 {
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.profile_editor = state;
        self.profile_generation
    }

    pub(crate) fn profile_is_current(&self, generation: u64) -> bool {
        self.profile_generation == generation
    }

    pub(crate) fn cancel_profile_editor(&mut self) {
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.profile_editor = AccountProfileEditorState::Closed;
    }

    pub(crate) fn begin_password_editor(&mut self, state: AccountPasswordEditorState) -> u64 {
        self.password_generation = self.password_generation.wrapping_add(1);
        self.password_editor = state;
        self.password_generation
    }

    pub(crate) fn password_is_current(&self, generation: u64) -> bool {
        self.password_generation == generation
    }

    pub(crate) fn cancel_password_editor(&mut self) {
        self.password_generation = self.password_generation.wrapping_add(1);
        self.password_editor = AccountPasswordEditorState::Closed;
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod account_tests;
