use super::{AccountIdentity, AccountProfile, SecretSkey, VerifiedAccount};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use vw_gateway_client::ShijuAccountMeDto;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const STORE_FILE_NAME: &str = "shiju-accounts.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccountOrigin {
    ManualSkey,
    BrowserLogin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredAccount {
    id: String,
    identity: AccountIdentity,
    profile: Option<AccountProfile>,
    origin: AccountOrigin,
    skey: SecretSkey,
}

impl StoredAccount {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn identity(&self) -> &AccountIdentity {
        &self.identity
    }

    pub(crate) fn profile(&self) -> Option<&AccountProfile> {
        self.profile.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn origin(&self) -> AccountOrigin {
        self.origin
    }

    #[cfg(test)]
    pub(crate) fn skey(&self) -> &SecretSkey {
        &self.skey
    }

    #[allow(dead_code)]
    pub(crate) fn skill_hub_credential(&self) -> SecretSkey {
        self.skey.clone()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AccountRepository {
    accounts: Vec<StoredAccount>,
    active_account_id: Option<String>,
}

impl AccountRepository {
    pub(crate) fn accounts(&self) -> &[StoredAccount] {
        &self.accounts
    }

    pub(crate) fn active_account_id(&self) -> Option<&str> {
        self.active_account_id.as_deref()
    }

    pub(crate) fn active_account(&self) -> Option<&StoredAccount> {
        let active_id = self.active_account_id.as_deref()?;
        self.accounts.iter().find(|account| account.id == active_id)
    }

    pub(crate) fn upsert_verified(
        &mut self,
        verified: VerifiedAccount,
        origin: AccountOrigin,
    ) -> String {
        let (identity, profile, skey) = verified.into_parts();
        let id = account_id(&skey);
        let updated = StoredAccount {
            id: id.clone(),
            identity,
            profile,
            origin,
            skey,
        };
        if let Some(account) = self.accounts.iter_mut().find(|account| account.id == id) {
            *account = updated;
        } else {
            self.accounts.push(updated);
        }
        self.active_account_id = Some(id.clone());
        id
    }

    pub(crate) fn activate(&mut self, id: &str) -> Result<(), AccountStoreError> {
        if !self.accounts.iter().any(|account| account.id == id) {
            return Err(AccountStoreError::new("无法激活不存在的账户"));
        }
        self.active_account_id = Some(id.to_string());
        Ok(())
    }

    pub(crate) fn sign_out(&mut self) {
        self.active_account_id = None;
    }

    pub(crate) fn remove(&mut self, id: &str) -> Result<(), AccountStoreError> {
        let Some(index) = self.accounts.iter().position(|account| account.id == id) else {
            return Err(AccountStoreError::new("无法删除不存在的账户"));
        };
        self.accounts.remove(index);
        if self.active_account_id.as_deref() == Some(id) {
            self.active_account_id = None;
        }
        Ok(())
    }

    pub(crate) fn refresh_active_profile(
        &mut self,
        dto: ShijuAccountMeDto,
    ) -> Result<(), AccountStoreError> {
        let active_id = self
            .active_account_id
            .as_deref()
            .ok_or_else(|| AccountStoreError::new("当前没有活动账户"))?;
        let account = self
            .accounts
            .iter_mut()
            .find(|account| account.id == active_id)
            .ok_or_else(|| AccountStoreError::new("当前账户不存在"))?;
        let company_status = account
            .profile
            .as_ref()
            .map(AccountProfile::company_status)
            .unwrap_or_else(|| account.identity.status())
            .to_string();
        let departments = dto.departments.into_iter().map(|item| item.department_name).collect();
        let roles = dto.roles.into_iter().map(|item| item.role_name).collect();
        account.profile = Some(
            AccountProfile::new(
                company_status,
                dto.name,
                dto.account_name,
                dto.status,
                departments,
                roles,
            )
            .map_err(|_| AccountStoreError::new("账户资料字段无效"))?,
        );
        Ok(())
    }

    pub(crate) fn load() -> Result<Self, AccountStoreError> {
        let path = default_path()?;
        Self::load_from(&path)
    }

    pub(crate) fn save(&self) -> Result<(), AccountStoreError> {
        let path = default_path()?;
        self.save_to(&path)
    }

    pub(crate) fn load_from(path: &Path) -> Result<Self, AccountStoreError> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(_) => return Err(AccountStoreError::at(path, "无法读取账户文件")),
        };
        let file: StoreFile = serde_json::from_slice(&bytes)
            .map_err(|_| AccountStoreError::at(path, "账户文件格式无效"))?;
        let mut seen = HashSet::new();
        let mut accounts = Vec::with_capacity(file.accounts.len());
        for item in file.accounts {
            let identity = AccountIdentity::new(
                item.company_id,
                item.company_name,
                item.founder_user_id,
                item.status,
            )
            .map_err(|_| AccountStoreError::at(path, "账户身份字段无效"))?;
            let profile = item
                .profile
                .map(|profile| {
                    AccountProfile::new(
                        profile.company_status,
                        profile.name,
                        profile.account_name,
                        profile.user_status,
                        profile.departments,
                        profile.roles,
                    )
                })
                .transpose()
                .map_err(|_| AccountStoreError::at(path, "账户资料字段无效"))?;
            let skey = SecretSkey::new(item.skey)
                .map_err(|_| AccountStoreError::at(path, "账户凭据为空"))?;
            if item.id != account_id(&skey) || !seen.insert(item.id.clone()) {
                return Err(AccountStoreError::at(path, "账户 ID 重复或无效"));
            }
            accounts.push(StoredAccount {
                id: item.id,
                identity,
                profile,
                origin: item.origin,
                skey,
            });
        }
        if file
            .active_account_id
            .as_ref()
            .is_some_and(|id| !seen.contains(id))
        {
            return Err(AccountStoreError::at(path, "当前账户 ID 不存在"));
        }
        Ok(Self {
            accounts,
            active_account_id: file.active_account_id,
        })
    }

    pub(crate) fn save_to(&self, path: &Path) -> Result<(), AccountStoreError> {
        self.validate()
            .map_err(|error| AccountStoreError::at(path, error.message))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| AccountStoreError::at(path, "无法创建账户目录"))?;
        }
        let file = StoreFile {
            accounts: self
                .accounts
                .iter()
                .map(|account| StoreAccount {
                    id: account.id.clone(),
                    company_id: account.identity.company_id().to_string(),
                    company_name: account.identity.company_name().to_string(),
                    founder_user_id: account.identity.founder_user_id().to_string(),
                    status: account.identity.status().to_string(),
                    profile: account.profile.as_ref().map(StoreProfile::from),
                    origin: account.origin,
                    skey: account.skey.expose().to_string(),
                })
                .collect(),
            active_account_id: self.active_account_id.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|_| AccountStoreError::at(path, "无法编码账户文件"))?;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut output = options
            .open(path)
            .map_err(|_| AccountStoreError::at(path, "无法写入账户文件"))?;
        output
            .write_all(&bytes)
            .and_then(|()| output.sync_all())
            .map_err(|_| AccountStoreError::at(path, "无法保存账户文件"))?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| AccountStoreError::at(path, "无法限制账户文件权限"))?;
        Ok(())
    }

    fn validate(&self) -> Result<(), AccountStoreError> {
        let mut seen = HashSet::new();
        for account in &self.accounts {
            if account.id != account_id(&account.skey) || !seen.insert(account.id.as_str()) {
                return Err(AccountStoreError::new("账户 ID 重复或无效"));
            }
        }
        if self
            .active_account_id
            .as_ref()
            .is_some_and(|id| !seen.contains(id.as_str()))
        {
            return Err(AccountStoreError::new("当前账户 ID 不存在"));
        }
        Ok(())
    }
}

fn default_path() -> Result<PathBuf, AccountStoreError> {
    crate::app::project_dirs()
        .map(|directories| directories.config_dir().join(STORE_FILE_NAME))
        .ok_or_else(|| AccountStoreError::new("无法确定账户配置目录"))
}

fn account_id(skey: &SecretSkey) -> String {
    hex::encode(Sha256::digest(skey.expose().as_bytes()))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreFile {
    accounts: Vec<StoreAccount>,
    active_account_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreAccount {
    id: String,
    company_id: String,
    company_name: String,
    founder_user_id: String,
    status: String,
    #[serde(default)]
    profile: Option<StoreProfile>,
    origin: AccountOrigin,
    skey: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreProfile {
    company_status: String,
    name: String,
    account_name: String,
    user_status: String,
    departments: Vec<String>,
    roles: Vec<String>,
}

impl From<&AccountProfile> for StoreProfile {
    fn from(profile: &AccountProfile) -> Self {
        Self {
            company_status: profile.company_status().to_string(),
            name: profile.name().to_string(),
            account_name: profile.account_name().to_string(),
            user_status: profile.user_status().to_string(),
            departments: profile.departments().to_vec(),
            roles: profile.roles().to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountStoreError {
    message: String,
}

impl AccountStoreError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    fn at(path: &Path, message: impl AsRef<str>) -> Self {
        Self::new(format!("{}：{}", path.display(), message.as_ref()))
    }
}

impl fmt::Display for AccountStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AccountStoreError {}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
