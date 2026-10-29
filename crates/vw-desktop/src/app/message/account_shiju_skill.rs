//! 将账户恢复及登录后的原生技能准备接入桌面异步消息。

use super::AccountMessage;
use crate::app::skill_hub::{
    NativeShijuSkillPrepared, ShijuInstallationsReport, ShijuTargetInstallation, SkillInstallScope,
    SkillInstallSelection, SkillInstallTarget, download_native_shiju_skill,
    prepare_native_shiju_skill, resolve_skill_destinations,
};
use crate::app::{App, Message};
use directories::BaseDirs;
use iced::Task;

pub(super) fn start(app: &mut App) -> Task<Message> {
    let state = app.account.native_shiju_skill.clone();
    let Some((generation, revision)) = state.begin() else {
        return Task::none();
    };
    Task::perform(
        async move {
            let home = BaseDirs::new().ok_or_else(|| "无法确定用户主目录".to_string())?;
            prepare_native_shiju_skill(
                state,
                home.home_dir().to_path_buf(),
                download_native_shiju_skill,
            )
            .await
            .map_err(|_| "识矩原生技能准备失败".to_string())
        },
        move |result| {
            Message::Account(AccountMessage::NativeShijuSkillPrepared {
                generation,
                revision,
                result,
            })
        },
    )
}

pub(super) fn finished(
    app: &mut App,
    generation: u64,
    revision: u64,
    result: Result<NativeShijuSkillPrepared, String>,
) -> Task<Message> {
    let state = &app.account.native_shiju_skill;
    if !state.finish(generation) {
        return Task::none();
    }
    let completed_revision = result.as_ref().map_or(revision, |prepared| prepared.revision);
    if state.revision() != completed_revision {
        return start(app);
    }
    match result {
        Ok(prepared) => {
            app.expert_hub.package_status.set_local_version(
                prepared
                    .installation
                    .as_ref()
                    .and_then(|installation| installation.version.clone()),
            );
            if let Some(home) = BaseDirs::new()
                && let Ok(mut destinations) = resolve_skill_destinations(
                    home.home_dir(),
                    None,
                    &[SkillInstallSelection {
                        target: SkillInstallTarget::VibeWindow,
                        scope: SkillInstallScope::Global,
                    }],
                )
                && let Some(destination) = destinations.pop()
            {
                let guide = &mut app.expert_hub.install_guide;
                let mut targets = guide.installations.clone();
                targets.retain(|target| {
                    target.destination.target != SkillInstallTarget::VibeWindow
                        || target.destination.scope != SkillInstallScope::Global
                });
                targets.push(ShijuTargetInstallation {
                    destination,
                    installation: prepared.installation,
                });
                guide.set_installations(ShijuInstallationsReport { targets });
            }
            Task::none()
        }
        Err(_) => app.show_warning_toast("识矩原生技能自动安装或登录同步失败，可在安装页重试"),
    }
}
