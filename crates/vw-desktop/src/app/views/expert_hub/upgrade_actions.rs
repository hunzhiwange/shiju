//! 已安装识矩技能页的升级操作与目标目录说明。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style,
};
use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::{LoadState, ShijuPackageId, SkillInstallScope, SkillInstallTarget};
use crate::app::{App, Message};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let state = &app.expert_hub;
    let busy = state.skill_operations_busy();
    let upgrades = match &state.installed {
        LoadState::Loaded(skills) => skills.iter().filter(|skill| skill.upgrade_available).count(),
        _ => 0,
    };
    let functions_label = if state.function_upgrade.is_some() {
        i18n.__("正在升级功能…")
    } else if !state.function_cache_retry.is_empty() {
        i18n.__("升级所有功能 / 重试缓存同步")
    } else {
        i18n.__("一键升级所有功能")
    };
    let functions = button(text(functions_label).size(12))
        .padding([9, 14])
        .style(primary_action_btn_style)
        .on_press_maybe(
            (!busy && (upgrades > 0 || !state.function_cache_retry.is_empty()))
                .then_some(Message::SkillHub(SkillHubMessage::UpgradeAllFunctions)),
        );
    let main = button(
        text(i18n.__(if state.main_upgrade.is_some() {
            "正在升级主技能…"
        } else {
            "升级主技能"
        }))
        .size(12),
    )
    .padding([9, 14])
    .style(rounded_action_btn_style)
    .on_press_maybe(
        (!busy && ShijuPackageId::current().is_some())
            .then_some(Message::SkillHub(SkillHubMessage::UpgradeMainSkill)),
    );
    let configure = button(text(i18n.__("选择智能体目录")).size(12))
        .padding([9, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe((!busy).then_some(Message::SkillHub(SkillHubMessage::OpenInstallGuide)));
    let mut targets = vec![super::install_guide::target_path_label(
        SkillInstallTarget::VibeWindow,
        SkillInstallScope::Global,
    )];
    targets.extend(
        state
            .install_guide
            .selected_install_selections()
            .into_iter()
            .filter(|selection| {
                selection.target != SkillInstallTarget::VibeWindow
                    || selection.scope != SkillInstallScope::Global
            })
            .map(|selection| {
                super::install_guide::target_path_label(selection.target, selection.scope)
            }),
    );
    let paths = targets.join("、");
    column![
        row![configure, functions, main].spacing(8).align_y(Alignment::Center).wrap(),
        text(
            i18n.format("主技能升级目录：%s", &[paths.into()]).expect("valid upgrade destinations")
        )
        .size(12)
        .style(super::styles::muted_text_style),
    ]
    .spacing(8)
    .into()
}
