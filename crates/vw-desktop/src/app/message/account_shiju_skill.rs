//! 将账户恢复及登录后的原生技能准备接入桌面异步消息。

use super::AccountMessage;
use crate::app::skill_hub::{
    NativeShijuSkillPrepared, ShijuInstallationsReport, ShijuTargetInstallation, SkillInstallScope,
    SkillInstallSelection, SkillInstallTarget, resolve_skill_destinations,
};
use crate::app::{App, Message};
use directories::BaseDirs;
use iced::Task;

pub(super) fn start(app: &mut App) -> Task<Message> {
    desktop_account_ui::native::start(app.account.native_shiju_skill.clone()).map(Message::Account)
}

pub(super) fn finished(
    app: &mut App,
    generation: u64,
    revision: u64,
    result: Result<NativeShijuSkillPrepared, String>,
) -> Task<Message> {
    let result = match desktop_account_ui::native::finished(
        &app.account.native_shiju_skill,
        generation,
        revision,
        result,
    ) {
        desktop_account_ui::native::Completion::Ignore => return Task::none(),
        desktop_account_ui::native::Completion::Retry => return start(app),
        desktop_account_ui::native::Completion::Ready(result) => result,
    };
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
        Err(_) => app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "识矩原生技能自动安装或登录同步失败，可在安装页重试",
            vec![],
        ),
    }
}
