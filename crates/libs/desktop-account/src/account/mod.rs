//! 统一导出账号模型与各平台的账号仓储。

#[cfg(target_arch = "wasm32")]
pub mod browser;
mod models;
mod store;

// 对外只暴露经过校验且会脱敏敏感字段的账号领域类型。
pub use models::{
    AccountIdentity, AccountModelError, AccountPasswordDraft, AccountPasswordField,
    AccountPasswordInput, AccountProfile, AccountProfileDraft, LoginChallenge, LoginPoll,
    ManualSkeyInput, SecretSkey, VerifiedAccount,
};
// 持久化细节留在仓储模块，调用方通过仓储不变量操作账号集合。
pub use store::{AccountOrigin, AccountRepository, AccountStoreError, StoredAccount};

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
