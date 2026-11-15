//! 处理账号登录、切换、资料更新及凭据同步的消息状态转换。

#[cfg(test)]
use crate::app::account::{
    AccountOrigin, AccountPasswordDraft, AccountPasswordField, AccountPasswordInput,
    AccountProfileDraft, AccountRepository, LoginPoll, ManualSkeyInput, SecretSkey,
    VerifiedAccount,
};
#[cfg(test)]
use crate::app::state::{
    AccountOperation, AccountPasswordEditorState, AccountProfileEditorState, MonthlyQuotaState,
};
use crate::app::{App, Message};
pub(crate) use desktop_account_ui::message::AccountMessage;
#[cfg(test)]
use desktop_account_ui::message::AccountNotice;
#[cfg(test)]
use gateway_shiju_account::ShijuAccountMeDto;
use iced::Task;
#[cfg(not(target_arch = "wasm32"))]
#[path = "account_shiju_skill.rs"]
mod shiju_skill;
fn services() -> desktop_account_ui::update::Services {
    desktop_account_ui::update::Services {
        gateway: crate::app::config::gateway_client(),
        store_path: crate::app::project_dirs()
            .map(|d| d.config_dir().join("shiju-accounts.json"))
            .ok_or_else(|| "无法确定账户配置目录".to_string()),
    }
}
pub(crate) fn load_repository() -> Task<Message> {
    desktop_account_ui::update::load_repository(services()).map(Message::Account)
}
pub(crate) fn update(app: &mut App, message: AccountMessage) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    match message {
        AccountMessage::ResetModels { generation } => {
            return super::settings::providers::managed_shiju::reset(app, generation);
        }
        AccountMessage::ModelTokenLoaded { generation, account_id, result } => {
            return super::settings::providers::managed_shiju::token_loaded(
                app, generation, account_id, result,
            );
        }
        AccountMessage::ModelsConfigured { generation, result } => {
            return super::settings::providers::managed_shiju::configured(app, generation, result);
        }
        AccountMessage::PrepareNativeShijuSkill => return shiju_skill::start(app),
        AccountMessage::NativeShijuSkillPrepared { generation, revision, result } => {
            return shiju_skill::finished(app, generation, revision, result);
        }
        _ => {}
    }
    let mut effects = Vec::new();
    let task =
        desktop_account_ui::update::update(&mut app.account, message, &services(), &mut effects)
            .map(Message::Account);
    let mut tasks = vec![task];
    for effect in effects {
        use desktop_account_ui::update::Effect;
        tasks.push(match effect {
            Effect::Toast(kind, source, args) => app.show_localized_toast(kind, source, args),
            #[cfg(not(target_arch = "wasm32"))]
            Effect::Host(message) => update(app, message),
            Effect::Error(message) => app.show_error_toast(message),
            Effect::Validation(feedback) => model_validation_feedback(app, feedback),
            #[cfg(not(target_arch = "wasm32"))]
            Effect::RefreshDomains => crate::app::message::skill_hub::account_changed(app),
            #[cfg(not(target_arch = "wasm32"))]
            Effect::SyncModels => super::settings::providers::managed_shiju::sync(app),
            #[cfg(not(target_arch = "wasm32"))]
            Effect::PrepareSkill => shiju_skill::start(app),
        });
    }
    Task::batch(tasks)
}
fn model_validation_feedback(
    app: &mut App,
    feedback: crate::app::i18n::workflow::Feedback,
) -> Task<Message> {
    let task = app.show_warning_toast(feedback.as_str());
    app.active_toast.as_mut().expect("inserted validation toast").localized_message =
        Some(feedback.message());
    task
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod account_tests;
