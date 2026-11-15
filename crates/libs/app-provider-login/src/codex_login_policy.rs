//! Login policy adapted from frozen upstream provider/codex_login.rs.
use std::collections::{BTreeMap, BTreeSet};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::codex_login_material::{
    codex_auth_has_credential_login_material, codex_auth_has_login_material,
    codex_auth_has_openai_account_material, codex_live_auth_is_stale_third_party_residue,
    extract_codex_auth_api_key, extract_codex_auth_user_identity,
};

pub const STASH_FILENAME: &str = "shiju-login-stash.json";

/// 暂存文件的格式版本。1：按用户加 workspace 认人（4.0.7）；没有这个字段的是更早写的。
const STASH_VERSION: u32 = 1;

/// 这台设备上暂存的原生登录，按身份存。
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LoginStash {
    #[serde(default)]
    pub logins: BTreeMap<String, Value>,
    /// 最近一次暂存的是谁：切回没有自己登录的官方卡（行里 `auth` 为空）时还它。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    #[serde(default)]
    pub version: u32,
    /// 暂存文件已经存在。为 `false` 时内容是刚从官方行里的快照初始化出来的，要写一次。
    #[serde(skip)]
    pub initialized: bool,
}

impl LoginStash {
    /// 暂存文件还不存在：用官方卡行里存着的 OAuth 登录初始化（旧版回填进去的）。
    pub fn seeded_from_rows<'a>(row_auths: impl IntoIterator<Item = &'a Value>) -> Self {
        let mut stash = Self { version: STASH_VERSION, ..Self::default() };
        for auth in row_auths {
            if let Some(id) = oauth_identity(auth) {
                stash.logins.entry(id).or_insert_with(|| auth.clone());
            }
        }
        stash
    }

    /// 读到的暂存文件：更早的版本先升级（见 [`Self::upgraded`]），升级过的要写一次。
    pub fn loaded<'a>(
        self,
        row_auths: impl IntoIterator<Item = &'a Value>,
        live: Option<&Value>,
    ) -> Self {
        if self.version >= STASH_VERSION {
            return Self { initialized: true, ..self };
        }
        self.upgraded(row_auths, live)
    }

    /// 4.0.7 以前只按用户认人：key 没有 workspace，同一个人的几个 workspace 只留下一份，
    /// 其余的只剩官方卡行里的快照。按现在的 [`identity`] 重算 key，再从行里补回这些
    /// workspace——只补暂存里或 live 里还有这个人的（他别的 workspace 还在用）。两处都没有
    /// 说明是在 Codex 里登出了，不补：登出不复活。
    fn upgraded<'a>(
        self,
        row_auths: impl IntoIterator<Item = &'a Value>,
        live: Option<&Value>,
    ) -> Self {
        let last = self.last.as_ref().and_then(|id| self.logins.get(id)).map(identity);
        let mut logins: BTreeMap<String, Value> =
            self.logins.into_values().map(|auth| (identity(&auth), auth)).collect();
        let live = live.filter(|auth| codex_auth_has_credential_login_material(auth));
        let users: BTreeSet<String> =
            logins.values().chain(live).filter_map(extract_codex_auth_user_identity).collect();
        let live_id = live.map(identity);
        for auth in row_auths {
            let Some(id) = oauth_identity(auth) else {
                continue;
            };
            let same_user =
                extract_codex_auth_user_identity(auth).is_some_and(|user| users.contains(&user));
            if same_user && live_id.as_ref() != Some(&id) {
                logins.entry(id).or_insert_with(|| auth.clone());
            }
        }
        Self { logins, last, version: STASH_VERSION, initialized: false }
    }

    fn put(&mut self, auth: &Value) {
        let id = identity(auth);
        self.logins.insert(id.clone(), auth.clone());
        self.last = Some(id);
    }

    fn take(&mut self, id: &str) -> Option<Value> {
        let auth = self.logins.remove(id)?;
        if self.last.as_deref() == Some(id) {
            self.last = None;
        }
        Some(auth)
    }
}

/// 登录的身份：id_token 里的用户加 ChatGPT workspace，缺一个就用另一个；API Key 登录共用
/// 一个位置。同一个人的 Plus 和 Team 是两个登录（codex-rs `same_owner` 也按用户加 workspace 认）。
fn identity(auth: &Value) -> String {
    let user = extract_codex_auth_user_identity(auth);
    let account = auth
        .pointer("/tokens/account_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty());
    match (user, account) {
        (Some(user), Some(account)) => format!("{user}|account:{account}"),
        (Some(user), None) => user,
        (None, Some(account)) => format!("account:{account}"),
        (None, None) if extract_codex_auth_api_key(auth).is_some() => "api-key".to_string(),
        (None, None) => "unknown".to_string(),
    }
}

/// 行里存的是 OAuth 登录（不是 API Key）时，它的身份。
fn oauth_identity(auth: &Value) -> Option<String> {
    codex_auth_has_credential_login_material(auth).then(|| identity(auth))
}

/// 官方卡的行里存的是 API Key（直连 OpenAI API）：静态凭据，不会过期，照写。
fn is_api_key_credential(auth: &Value) -> bool {
    extract_codex_auth_api_key(auth).is_some() && !codex_auth_has_credential_login_material(auth)
}

/// 旧版切到第三方时写进 `auth.json` 的 Key（只有 `OPENAI_API_KEY`，而且就是某个第三方
/// 供应商的 Key）。能证明是 CC Switch 写的才算残留；用户自己 `codex login --api-key`
/// 登录的 Key 不算。
fn is_residue(auth: &Value, third_party_keys: &[String]) -> bool {
    codex_live_auth_is_stale_third_party_residue(auth)
        && extract_codex_auth_api_key(auth).is_some_and(|key| third_party_keys.contains(&key))
}

/// 登录的 access_token 已经过期：按 JWT 里的 `exp` 算，解不出的当没过期。Codex 在用的
/// 登录每隔几天会刷新一次，过期说明这份登录很久没用过了。
fn access_token_expired(auth: &Value, now: i64) -> bool {
    auth.pointer("/tokens/access_token")
        .and_then(Value::as_str)
        .and_then(|token| token.split('.').nth(1))
        .and_then(|payload| URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|claims| claims.get("exp").and_then(Value::as_i64))
        .is_some_and(|exp| exp <= now)
}

/// 目标供应商对 `auth.json` 的要求。
#[derive(Clone, Copy)]
pub enum AuthTarget<'a> {
    /// 直连的第三方：保留登录开关关闭时删掉 `auth.json`。
    ThirdParty { preserve: bool },
    /// 代理的第三方路由：不动用户的原生登录（请求凭据由代理注入）。`restore_last`：进入或
    /// 接上代理时 `auth.json` 里没有登录，就把最近暂存的那份还回去。Codex 桌面版续会话要
    /// ChatGPT 登录，旧版退出 CC Switch 时分离会把它暂存走，接上时又不还（#8431）。
    /// access_token 已过期的不还：那是很久没用的旧登录，还回去 Codex 会拿它刷新、失败后
    /// 卡在登录页，比没有登录更糟（#8573）。它留在暂存里，切回官方卡时照常还。
    ProxyThirdParty { restore_last: bool },
    /// 没绑托管账号的官方卡（直连，或代理的官方路由）。
    Official { row_auth: &'a Value },
    /// 托管账号：整份写它的登录。
    Managed { auth: &'a Value },
}

pub struct AuthInput<'a> {
    /// live 的 `auth.json`（文件不存在为 `None`，解析不了的按非对象处理）。
    pub live: Option<&'a Value>,
    /// live 里是某个托管账号的登录（切走时要清掉，不进暂存）。
    pub live_is_managed: bool,
    /// 第三方供应商行里的 Key，用来认出旧版留下的残留。
    pub third_party_keys: &'a [String],
    /// 切走的是没绑托管账号的官方卡（它的行 `auth`）：live 里没有登录，说明用户在
    /// Codex 里登出了，暂存里对应的登录也作废，免得切回来又登上。
    pub leaving_official: Option<&'a Value>,
    pub target: AuthTarget<'a>,
    pub stash: LoginStash,
    /// 现在的 Unix 时间（秒），判断暂存的登录过没过期。
    pub now: i64,
}

#[derive(Clone, PartialEq)]
pub struct AuthPlan {
    /// `None` 不动；`Some(None)` 删掉；`Some(Some(v))` 整份写 `v`。
    pub auth: Option<Option<Value>>,
    /// 暂存有变化时的新内容。
    pub stash: Option<LoginStash>,
    /// 写完之后 `auth.json` 里有没有登录（第三方路由表的 `requires_openai_auth` 跟着它；
    /// 登录存在系统钥匙串里时另算，见 `codex_direct`）。
    pub login_on_disk: bool,
}

pub fn plan(input: AuthInput<'_>) -> AuthPlan {
    let original = input.stash.clone();
    let mut stash = input.stash;
    let managed = input.live_is_managed;
    let residue =
        input.live.is_some_and(|auth| !managed && is_residue(auth, input.third_party_keys));
    let native =
        input.live.filter(|auth| !managed && !residue && codex_auth_has_login_material(auth));

    if let Some(row_auth) = input.leaving_official
        && native.is_none()
        && !managed
    {
        match oauth_identity(row_auth) {
            Some(id) => {
                stash.take(&id);
            }
            None => {
                if let Some(last) = stash.last.take() {
                    stash.logins.remove(&last);
                }
            }
        }
        stash.last = None;
    }

    let auth = match input.target {
        AuthTarget::ThirdParty { preserve } => {
            if let Some(live) = native.filter(|_| !preserve) {
                stash.put(live);
            }
            // 保留登录关闭时第三方路由旁边不留任何 auth.json（写 `{}` 不等于登出）。
            (managed || residue || (!preserve && input.live.is_some())).then_some(None)
        }
        AuthTarget::ProxyThirdParty { restore_last } => {
            let restored = (restore_last && !managed && native.is_none())
                .then(|| stash.last.clone())
                .flatten()
                .filter(|id| {
                    stash.logins.get(id).is_some_and(|auth| !access_token_expired(auth, input.now))
                })
                .and_then(|id| stash.take(&id));
            match restored {
                Some(saved) => Some(Some(saved)),
                None => managed.then_some(None),
            }
        }
        AuthTarget::Managed { auth } => {
            if let Some(live) = native {
                stash.put(live);
            }
            Some(Some(auth.clone()))
        }
        AuthTarget::Official { row_auth } => {
            official(row_auth, native, managed || residue, &mut stash)
        }
    };

    // 只算 Codex 会拿来登录的东西（OAuth token、API Key 等），Bedrock 凭据和元数据不算。
    let login_on_disk = match &auth {
        None => native.is_some_and(codex_auth_has_openai_account_material),
        Some(None) => false,
        Some(Some(auth)) => codex_auth_has_openai_account_material(auth),
    };
    let changed = stash != original || !original.initialized;
    AuthPlan {
        auth,
        stash: changed.then_some(LoginStash { initialized: true, ..stash }),
        login_on_disk,
    }
}

/// 切到没绑托管账号的官方卡。`clearable` 表示 live 里是要清掉的东西（托管账号的登录、
/// 旧版的第三方 Key 残留）。
fn official(
    row_auth: &Value,
    native: Option<&Value>,
    clearable: bool,
    stash: &mut LoginStash,
) -> Option<Option<Value>> {
    if is_api_key_credential(row_auth) {
        if let Some(live) = native.filter(|live| *live != row_auth) {
            stash.put(live);
        }
        return Some(Some(row_auth.clone()));
    }

    let wanted = oauth_identity(row_auth);
    if let Some(live) = native {
        // 行里没存登录：跟随 Codex 当前的登录。存的就是当前这个人：不动。
        let id = wanted.filter(|id| identity(live) != *id)?;
        // 行里存的是另一个人：暂存里有他才换，否则不动（行里的快照可能早已作废，也可能是
        // 在 Codex 里登出了）。旧版漏掉的 workspace 在读暂存时补回（见 `LoginStash::upgraded`）。
        let saved = stash.take(&id)?;
        stash.put(live);
        return Some(Some(saved));
    }

    let restored = match wanted {
        Some(id) => stash.take(&id),
        None => stash.last.clone().and_then(|id| stash.take(&id)),
    };
    match restored {
        Some(saved) => Some(Some(saved)),
        None => clearable.then_some(None),
    }
}
