//! 桌面账号装配：领域实现来自 desktop-account，配置目录由桌面决定。

pub(crate) use desktop_account::{
    AccountIdentity, AccountPasswordDraft, AccountPasswordField, AccountPasswordInput,
    AccountOrigin, AccountProfile, AccountProfileDraft, AccountRepository, LoginChallenge,
    LoginPoll, ManualSkeyInput, SecretSkey, StoredAccount, VerifiedAccount,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use desktop_account::account::browser;

pub(crate) fn load_repository() -> Result<AccountRepository, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        AccountRepository::load_from(&store_path()?).map_err(|error| error.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        AccountRepository::load().map_err(|error| error.to_string())
    }
}

pub(crate) fn save_repository(repository: &AccountRepository) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        repository.save_to(&store_path()?).map_err(|error| error.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        repository.save().map_err(|error| error.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn store_path() -> Result<std::path::PathBuf, String> {
    crate::app::project_dirs()
        .map(|directories| directories.config_dir().join("shiju-accounts.json"))
        .ok_or_else(|| "无法确定账户配置目录".to_string())
}
