use crate::third_party::style;
use desktop_ui_controls::system_settings_common::*;
// Execution-host backups and explicit restore destination confirmation.
use iced::{
    Element, Length,
    widget::{Space, button, checkbox, column, container, row, text},
};

pub fn view<'a, M: Clone + 'static>(
    s: ViewState<'a>,
    base: Element<'a, M>,
    message: fn(Event) -> M,
    app_name: impl Fn(vw_api_app_identity::AppId) -> String,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let control = |label: String, event| {
        button(text(label))
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(event)))
    };
    let has_backups = !s.backups.is_empty();
    let mut body = column![
        text("选择要恢复的技能备份。").size(13).style(settings_muted_text_style)
    ]
    .spacing(12)
    .width(Length::Fill);
    for backup in s.backups {
        let selected = s.selected.as_ref() == Some(&backup.backup_id);
        let entry = row![
            text(backup.name.clone()).size(14).width(Length::Fill),
            text((s.format_date)(backup.created_at)).size(12).style(settings_muted_text_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        body = body.push(
            button(entry)
                .width(Length::Fill)
                .padding([8, 10])
                .style(move |theme, status| settings_tab_style(theme, status, selected))
                .on_press_maybe(
                    (!s.busy).then(|| message(Event::Select(backup.backup_id.clone()))),
                ),
        );
    }
    if !has_backups {
        body = body.push(
            container(
                text(if s.busy {
                    "正在读取备份…"
                } else if s.error.is_some() {
                    "备份读取失败，请刷新"
                } else if s.version.is_some() {
                    "暂无可恢复的技能备份"
                } else {
                    "尚未读取备份"
                })
                .size(14)
                .style(settings_muted_text_style),
            )
            .center_x(Length::Fill)
            .center_y(118),
        );
    }
    if let Some(id) = s.delete_pending {
        if let Some(backup) = s.backups.iter().find(|b| &b.backup_id == id) {
            body = body
                .push(text(format!("确认永久删除备份「{}」？此操作不可撤销。", backup.name)))
                .push(
                    row![
                        control("确认删除".into(), Event::ConfirmDelete),
                        control("取消删除".into(), Event::CancelDelete)
                    ]
                    .spacing(8),
                );
        }
    }
    if s.selected.is_some() {
        body = body.push(settings_field_label("恢复后启用的应用"));
        if s.available.is_empty() {
            body = body.push(text("只恢复技能主副本。").size(12).style(settings_muted_text_style));
        }
        for target in s.available {
            let target = target.clone();
            body =
                body.push(
                    checkbox(s.targets.contains(&target))
                        .label(app_name(target.app_id))
                        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(style::checkbox)
                        .on_toggle_maybe((!s.busy).then_some(move |enabled| {
                            message(Event::Target(target.clone(), enabled))
                        })),
                );
        }
    }
    if let Some(error) = s.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    if s.busy {
        body = body.push(text("正在处理…"));
    }
    let close = message(Event::Close);
    let header =
        settings_modal_header(text("从备份中恢复").size(20), settings_close_button(close.clone()));
    let mut footer = row![Space::new().width(Length::Fill)].spacing(8);
    if s.error.is_some() {
        footer = footer.push(control("刷新".into(), Event::Refresh));
    }
    if has_backups {
        footer = footer.push(
            button("删除所选").padding([8, 10]).style(rounded_action_btn_style).on_press_maybe(
                (!s.busy && s.selected.is_some()).then_some(message(Event::Delete)),
            ),
        );
    }
    footer = footer.push(control("关闭".into(), Event::Close));
    if has_backups {
        footer = footer.push(
            button("恢复").padding([8, 10]).style(style::primary).on_press_maybe(
                (!s.busy && s.selected.is_some() && s.version.is_some())
                    .then_some(message(Event::Restore)),
            ),
        );
    }
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(style::dialog)
        .width(560)
        .height(if has_backups { 480 } else { 300 });
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, card)
}
#[derive(Clone, Copy)]
pub struct ViewState<'a> {
    pub delete_pending: Option<&'a String>,
    pub format_date: fn(u64) -> String,
    pub open: bool,
    pub busy: bool,
    pub backups: &'a Vec<vw_api_app_skills::backups_api::SkillBackup>,
    pub selected: &'a Option<String>,
    pub available: &'a Vec<vw_api_app_skills::bindings_api::SkillBindingTarget>,
    pub targets: &'a Vec<vw_api_app_skills::bindings_api::SkillBindingTarget>,
    pub version: Option<u64>,
    pub error: &'a Option<String>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Close,
    Refresh,
    Select(String),
    Target(vw_api_app_skills::bindings_api::SkillBindingTarget, bool),
    Delete,
    ConfirmDelete,
    CancelDelete,
    Restore,
}
