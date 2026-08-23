//! 统一导出桌面端账号模型与本地账号仓储。

mod models;
mod store;

// 对外只暴露经过校验且会脱敏敏感字段的账号领域类型。
pub(crate) use models::{
    AccountIdentity, AccountPasswordDraft, AccountPasswordField, AccountPasswordInput,
    AccountProfile, AccountProfileDraft, LoginChallenge, LoginPoll, ManualSkeyInput, SecretSkey,
    VerifiedAccount,
};
// 持久化细节留在仓储模块，调用方通过仓储不变量操作账号集合。
pub(crate) use store::{AccountOrigin, AccountRepository, StoredAccount};

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
