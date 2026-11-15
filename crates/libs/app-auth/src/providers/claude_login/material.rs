use super::*;

impl LoginMaterial {
    /// The same normalized OAuth generation used by the native rotation check.
    pub fn credential_generation(&self) -> Result<Option<vw_api_transport::TimestampMs>, WriteError> {
        crate::auth::claude_rotation::expiry(self.oauth().and_then(|oauth| oauth.get("expiresAt")))
    }

    pub fn oauth(&self) -> Option<&Value> {
        self.secret.as_ref()?.get("claudeAiOauth")
    }

    pub fn usage_oauth(&self) -> Option<&Value> {
        self.oauth().or_else(|| self.secret.as_ref()?.get("claude.ai_oauth"))
    }

    /// An MCP-only keychain item does not represent the Claude login in this slot.
    pub fn with_usage_fallback(mut self) -> Self {
        if self.keychain.is_some() && self.usage_oauth().is_none() {
            // read already validated the file; select the token and plan together.
            self.secret =
                self.fallback.as_deref().and_then(|bytes| serde_json::from_slice(bytes).ok());
        }
        self
    }

    /// Snapshot the parked login under the same CLI locks as account checkout.
    pub fn locked_account(
        path: &Path,
        keychain: &dyn ClaudeKeychain,
    ) -> Result<Self, WriteError> {
        let (dir, _locks) = lock_account(path)?;
        Self::read(&dir, &dir, Some(path), keychain)
    }

    pub fn identity(&self) -> Result<Option<String>, WriteError> {
        if !self
            .oauth()
            .and_then(|oauth| oauth.get("refreshToken"))
            .and_then(Value::as_str)
            .is_some_and(|token| !token.is_empty())
        {
            return Ok(None);
        }
        let (id, org) = self.profile_identity()?.ok_or(WriteError::InvalidResource)?;
        if self
            .secret
            .as_ref()
            .and_then(|v| v.get("organizationUuid"))
            .and_then(Value::as_str)
            .is_some_and(|s| s != org)
        {
            return Err(WriteError::InvalidResource);
        }
        Ok(Some(format!("{id}:{org}")))
    }
    pub(super) fn profile_identity(&self) -> Result<Option<(String, String)>, WriteError> {
        let Some(profile) = self.profile.as_deref() else { return Ok(None) };
        let profile: Value =
            serde_json::from_slice(profile).map_err(|_| WriteError::InvalidResource)?;
        let Some(account) =
            profile.as_object().ok_or(WriteError::InvalidResource)?.get("oauthAccount")
        else {
            return Ok(None);
        };
        let valid = |id: &str| !id.contains(':') && !id.chars().any(char::is_control);
        let id = account
            .get("accountUuid")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && valid(id))
            .ok_or(WriteError::InvalidResource)?;
        let org = account.get("organizationUuid").and_then(Value::as_str).unwrap_or("");
        if !valid(org) {
            return Err(WriteError::InvalidResource);
        }
        Ok(Some((id.to_owned(), org.to_owned())))
    }
    pub fn from_account(path: &Path, keychain: &dyn ClaudeKeychain) -> Result<Self, WriteError> {
        let dir = Directory::open(path)?;
        Self::read(&dir, &dir, Some(path), keychain)
    }
    pub(super) fn read(
        credentials: &Directory,
        profile: &Directory,
        keychain_path: Option<&Path>,
        keychain: &dyn ClaudeKeychain,
    ) -> Result<Self, WriteError> {
        let fallback = credentials.read(".credentials.json")?;
        let native = keychain.read(keychain_path).map_err(|_| WriteError::Io)?;
        let file_secret = fallback
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()
            .map_err(|_| WriteError::InvalidResource)?;
        let secret = native.clone().or(file_secret);
        if secret.as_ref().is_some_and(|value| !value.is_object()) {
            return Err(WriteError::InvalidResource);
        }
        Ok(Self { secret, keychain: native, profile: profile.read(".claude.json")?, fallback })
    }
    /// The caller holds the scoped account lease. Recheck the destination after
    /// taking Claude's locks because its CLI can change the directory independently.
    pub fn archive(&self, path: &Path, keychain: &dyn ClaudeKeychain) -> Result<(), WriteError> {
        let owner = self.identity()?.ok_or(WriteError::InvalidResource)?;
        let (directory, _locks) = lock_account(path)?;
        let before = Self::read(&directory, &directory, Some(path), keychain)?;
        let existing = match before.identity()? {
            Some(identity) => Some(identity),
            None => before.profile_identity()?.map(|(id, org)| format!("{id}:{org}")),
        };
        if existing.as_ref().is_some_and(|identity| identity != &owner) {
            return Err(WriteError::Conflict);
        }
        check_generation(before.oauth(), self.oauth())?;
        let after = merge(&before, self)?;
        if Self::read(&directory, &directory, Some(path), keychain)? != before {
            return Err(WriteError::Conflict);
        }
        if existing.is_none() {
            // Establish ownership before publishing credentials so an interrupted
            // first archive can be retried against a recognizable destination.
            directory.write(".claude.json", after.profile.as_deref())?;
        }
        if before.keychain != after.secret {
            keychain.write(Some(path), after.secret.as_ref()).map_err(|_| WriteError::Io)?;
            if keychain.read(Some(path)).map_err(|_| WriteError::Io)? != after.secret {
                return Err(WriteError::Io);
            }
        }
        if before.fallback.is_some() {
            directory.write(".credentials.json", None)?;
        }
        Ok(())
    }
}
pub(crate) fn lock_account(path: &Path) -> Result<(Directory, Vec<CliLock>), WriteError> {
    let directory = Directory::open(path)?;
    let parent = Directory::open(path.parent().ok_or(WriteError::InvalidResource)?)?;
    let legacy = format!(
        "{}.lock",
        path.file_name().and_then(|name| name.to_str()).ok_or(WriteError::InvalidResource)?
    );
    let locks = vec![
        directory.cli_lock(".oauth_refresh.lock")?,
        directory.cli_lock(".storage-write.lock")?,
        directory.cli_lock(".claude.json.lock")?,
        parent.cli_lock(&legacy)?,
    ];
    Ok((directory, locks))
}
pub(super) fn merge(base: &LoginMaterial, account: &LoginMaterial) -> Result<LoginMaterial, WriteError> {
    let mut secret = base.secret.clone().unwrap_or(json!({}));
    let map = secret.as_object_mut().ok_or(WriteError::InvalidResource)?;
    for key in ACCOUNT_KEYS {
        map.remove(*key);
        if let Some(value) = account.secret.as_ref().and_then(|v| v.get(*key)) {
            map.insert((*key).into(), value.clone());
        }
    }
    let mut profile: Value = serde_json::from_slice(base.profile.as_deref().unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    let target: Value = serde_json::from_slice(account.profile.as_deref().unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    let object = profile.as_object_mut().ok_or(WriteError::InvalidResource)?;
    object.remove("oauthAccount");
    if let Some(value) = target.get("oauthAccount") {
        object.insert("oauthAccount".into(), value.clone());
    }
    for key in ACCOUNT_CACHE_KEYS {
        object.remove(*key);
    }
    Ok(LoginMaterial {
        secret: Some(secret.clone()),
        keychain: Some(secret),
        profile: Some(serde_json::to_vec(&profile).map_err(|_| WriteError::InvalidResource)?),
        fallback: None,
    })
}
