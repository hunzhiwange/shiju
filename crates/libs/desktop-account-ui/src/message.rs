use desktop_account::*;
#[cfg(not(target_arch = "wasm32"))]
use desktop_skill_hub::NativeShijuSkillPrepared;
use gateway_shiju_account::ShijuAccountMeDto;
#[derive(Debug, Clone, Copy)]
/// 账号操作成功后用于生成反馈文案的动作类别。
pub enum AccountNotice {
    ManualLogin,
    BrowserLogin,
    Switch,
    SignOut,
    Delete,
}

#[derive(Debug, Clone)]
/// 账号登录、资料编辑和密码修改流程中的输入及异步结果消息。
pub enum AccountMessage {
    #[cfg(not(target_arch = "wasm32"))]
    ResetModels {
        generation: u64,
    },
    #[cfg(not(target_arch = "wasm32"))]
    ModelTokenLoaded {
        generation: u64,
        account_id: String,
        result: Result<gateway_shiju_account::ShijuModelTokenDto, String>,
    },
    #[cfg(not(target_arch = "wasm32"))]
    ModelsConfigured {
        generation: u64,
        result: Result<(), String>,
    },
    RepositoryLoaded(Result<AccountRepository, String>),
    ToggleMenu,
    RefreshMonthlyQuota,
    MonthlyQuotaLoaded {
        generation: u64,
        quota: Option<gateway_shiju_account::ShijuMonthlyQuotaDto>,
    },
    CloseMenu,
    OpenDetails,
    CloseDetails,
    OpenProfileEditor,
    CloseProfileEditor,
    ProfileLoaded {
        generation: u64,
        result: Result<ShijuAccountMeDto, String>,
    },
    ProfileNameChanged(String),
    ProfileUsernameChanged(String),
    SubmitProfileEditor,
    ProfileUpdated {
        generation: u64,
        result: Result<ShijuAccountMeDto, String>,
    },
    ProfilePersisted {
        generation: u64,
        result: Result<AccountRepository, String>,
    },
    OpenPasswordEditor,
    ClosePasswordEditor,
    PasswordFieldChanged {
        field: AccountPasswordField,
        value: AccountPasswordInput,
    },
    DismissPasswordError,
    SubmitPasswordEditor,
    PasswordUpdated {
        generation: u64,
        result: Result<(), String>,
    },
    ManualInputChanged(ManualSkeyInput),
    SubmitManual,
    StartBrowserLogin,
    CancelBrowserLogin,
    LoginStarted {
        generation: u64,
        result: Result<LoginChallenge, String>,
    },
    PollLogin {
        generation: u64,
    },
    LoginTimedOut {
        generation: u64,
    },
    LoginPolled {
        generation: u64,
        result: Result<LoginPoll, String>,
    },
    AccountValidated {
        generation: u64,
        origin: AccountOrigin,
        result: Result<VerifiedAccount, String>,
    },
    SelectSaved(String),
    DeleteSaved(String),
    SignOut,
    RepositoryPersisted {
        generation: u64,
        notice: AccountNotice,
        result: Result<AccountRepository, String>,
    },
    #[cfg(not(target_arch = "wasm32"))]
    PrepareNativeShijuSkill,
    #[cfg(not(target_arch = "wasm32"))]
    NativeShijuSkillPrepared {
        generation: u64,
        revision: u64,
        result: Result<NativeShijuSkillPrepared, String>,
    },
}
