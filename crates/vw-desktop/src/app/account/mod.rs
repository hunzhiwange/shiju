//! 桌面账号装配：领域实现来自 desktop-account，配置目录由桌面决定。

#[cfg(target_arch = "wasm32")]
pub(crate) use desktop_account::account::browser;

#[cfg(test)]
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

#[cfg(test)]
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
#[cfg(test)]
fn store_path() -> Result<std::path::PathBuf, String> {
    crate::app::project_dirs()
        .map(|directories| directories.config_dir().join("shiju-accounts.json"))
        .ok_or_else(|| "无法确定账户配置目录".to_string())
}

#[cfg(test)]
pub(crate) use desktop_account::AccountIdentity;
#[cfg(test)]
pub(crate) use desktop_account::AccountOrigin;
#[cfg(test)]
pub(crate) use desktop_account::AccountPasswordDraft;
#[cfg(test)]
pub(crate) use desktop_account::AccountPasswordField;
#[cfg(test)]
pub(crate) use desktop_account::AccountPasswordInput;
#[cfg(test)]
pub(crate) use desktop_account::AccountProfile;
#[cfg(test)]
pub(crate) use desktop_account::AccountProfileDraft;
#[cfg(test)]
pub(crate) use desktop_account::AccountRepository;
#[cfg(test)]
pub(crate) use desktop_account::LoginChallenge;
#[cfg(test)]
pub(crate) use desktop_account::LoginPoll;
#[cfg(test)]
pub(crate) use desktop_account::ManualSkeyInput;
#[cfg(test)]
pub(crate) use desktop_account::SecretSkey;
#[cfg(test)]
pub(crate) use desktop_account::StoredAccount;
#[cfg(test)]
pub(crate) use desktop_account::VerifiedAccount;
