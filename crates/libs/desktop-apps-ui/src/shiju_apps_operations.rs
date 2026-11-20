//! Detail actions use the current installation and the active account.
use desktop_app_launch::ShijuMarketplaceAction;
use gateway_shiju_app::{ShijuAppCopyRequest, valid_installation_id};
use iced::Task;

use super::{Effect, Input, ShijuAppsMessage, marketplace_action};

#[derive(Debug, Clone)]
pub enum DetailAction {
    Share,
    Copy,
    CopyTitle(String),
    CopyId(String),
    CopySubmit,
    CopyCancel,
    #[cfg(not(target_arch = "wasm32"))]
    Download,
    #[cfg(not(target_arch = "wasm32"))]
    Downloaded(u64, String, Result<SkillArchive, String>),
    #[cfg(not(target_arch = "wasm32"))]
    Destination(u64, String, SkillArchive, Option<std::path::PathBuf>),
    #[cfg(not(target_arch = "wasm32"))]
    Saved(u64, String, Result<(), String>),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct SkillArchive(std::sync::Arc<Vec<u8>>);

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Debug for SkillArchive {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SkillArchive(<redacted>)")
    }
}

pub fn valid_copy_id(value: &str) -> bool {
    let value = value.as_bytes();
    (2..=63).contains(&value.len())
        && value[0].is_ascii_lowercase()
        && value[value.len() - 1].is_ascii_alphanumeric()
        && value
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

pub(super) fn update(app: &mut Input<'_>, event: DetailAction) -> Task<ShijuAppsMessage> {
    match event {
        DetailAction::Share => {
            let Some(id) =
                app.shiju_apps.detail_installation().filter(|id| valid_installation_id(id))
            else {
                return Task::none();
            };
            // Share the canonical runtime URL, which carries no credentials.
            let task = iced::clipboard::write(format!("https://app.shiju.cloud/app/{id}/"));
            app.effects.push(Effect::Success("应用链接已复制".into()));
            task
        }
        DetailAction::Copy => {
            if app.shiju_apps.action.is_some() {
                return Task::none();
            }
            let Some(item) =
                app.shiju_apps.installed_detail().filter(|item| item.presentation.can_copy)
            else {
                return Task::none();
            };
            let id = item.app_id.chars().take(58).collect::<String>();
            app.shiju_apps.copy_form = Some(ShijuAppCopyRequest {
                request_id: uuid::Uuid::new_v4().to_string(),
                app_id: format!("{id}-copy"),
                title: format!("{} 副本", item.name.chars().take(120).collect::<String>()),
                expected_version: item.version.clone(),
            });
            Task::none()
        }
        DetailAction::CopyTitle(_) | DetailAction::CopyId(_) => {
            if let Some(form) =
                app.shiju_apps.copy_form.as_mut().filter(|_| app.shiju_apps.action.is_none())
            {
                match event {
                    DetailAction::CopyTitle(title) => form.title = title,
                    DetailAction::CopyId(id) => form.app_id = id,
                    _ => unreachable!(),
                }
                form.request_id = uuid::Uuid::new_v4().to_string();
            }
            Task::none()
        }
        DetailAction::CopyCancel => {
            if app.shiju_apps.action.is_none() {
                app.shiju_apps.copy_form = None;
            }
            Task::none()
        }
        DetailAction::CopySubmit => {
            let (Some(form), Some(item)) =
                (&app.shiju_apps.copy_form, app.shiju_apps.installed_detail())
            else {
                return Task::none();
            };
            if !item.presentation.can_copy
                || !valid_copy_id(&form.app_id)
                || form.title.trim().is_empty()
                || form.title.chars().count() > 128
                || form.expected_version.is_empty()
            {
                return Task::none();
            }
            marketplace_action(
                app,
                ShijuMarketplaceAction::Copy(item.installation_id.clone(), form.clone()),
            )
        }
        #[cfg(not(target_arch = "wasm32"))]
        DetailAction::Download => download(app),
        #[cfg(not(target_arch = "wasm32"))]
        DetailAction::Downloaded(generation, account, result)
            if current_download(app, generation, &account) =>
        {
            let archive = match result {
                Ok(archive) => archive,
                Err(error) => {
                    app.shiju_apps.action = None;
                    return app.show_localized_toast((), "技能包下载失败：%s", vec![error.into()]);
                }
            };
            Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .add_filter("ZIP", &["zip"])
                        .set_file_name("companion-skill.zip")
                        .save_file()
                        .await
                        .map(|file| file.path().to_owned())
                },
                move |path| {
                    ShijuAppsMessage::DetailAction(DetailAction::Destination(
                        generation, account, archive, path,
                    ))
                },
            )
        }
        #[cfg(not(target_arch = "wasm32"))]
        DetailAction::Destination(generation, account, archive, path)
            if current_download(app, generation, &account) =>
        {
            let Some(path) = path else {
                app.shiju_apps.action = None;
                return Task::none();
            };
            Task::perform(
                async move {
                    tokio::fs::write(path, archive.0.as_slice())
                        .await
                        .map_err(|error| error.to_string())
                },
                move |result| {
                    ShijuAppsMessage::DetailAction(DetailAction::Saved(generation, account, result))
                },
            )
        }
        #[cfg(not(target_arch = "wasm32"))]
        DetailAction::Saved(generation, account, result)
            if current_download(app, generation, &account) =>
        {
            app.shiju_apps.action = None;
            match result {
                Ok(()) => {
                    app.effects.push(Effect::Success("技能包已保存".into()));
                    Task::none()
                }
                Err(error) => {
                    app.show_localized_toast((), "技能包保存失败：%s", vec![error.into()])
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        _ => Task::none(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn current_download(app: &Input<'_>, generation: u64, account: &str) -> bool {
    app.account.is_some_and(|active| active.id() == account)
        && matches!(&app.shiju_apps.action, Some((current, ShijuMarketplaceAction::Download(_))) if *current == generation)
}

#[cfg(not(target_arch = "wasm32"))]
fn download(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    let Some(id) = app.shiju_apps.detail_installation().map(str::to_owned) else {
        return Task::none();
    };
    let Some(account) = app.account else { return Task::none() };
    if app.shiju_apps.action.is_some() || app.shiju_apps.account_id.as_deref() != Some(account.id())
    {
        return Task::none();
    }
    let account_id = account.id().to_owned();
    let credential = account.skill_hub_credential();
    let generation = app.shiju_apps.next_generation();
    app.shiju_apps.action = Some((generation, ShijuMarketplaceAction::Download(id.clone())));
    let client = app.client.clone();
    Task::perform(
        async move {
            let client = gateway_shiju_app::GatewayShijuAppClient::new(client?.transport().clone());
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            client
                .download_companion_skill(&credential, &id)
                .await
                .map(std::sync::Arc::new)
                .map(SkillArchive)
        },
        move |result| {
            ShijuAppsMessage::DetailAction(DetailAction::Downloaded(generation, account_id, result))
        },
    )
}
