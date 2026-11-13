//! 新增与编辑共用的固定标题、滚动正文和保存页脚。
use super::*;
use crate::app::components::system_settings_common::settings_modal_overlay_with_escape;
use iced::widget::{column, row};

pub(super) fn with_edit_modal<'a>(
    app: &'a App,
    dialog: Element<'a, Message>,
) -> Element<'a, Message> {
    let s = &app.cron_settings;
    if s.show_cron_templates {
        return templates::overlay(app, dialog);
    }
    let editing = s.editing_job_id.is_some();
    if !editing && !s.show_add_modal {
        return dialog;
    }
    let draft = if editing { &s.edit_draft } else { &s.add_draft };
    let tr = crate::app::i18n::I18n::new(app.desktop_locale);
    let close = if draft.submitting {
        Message::None
    } else {
        Message::Settings(message::SettingsMessage::CronJobEditCanceled)
    };
    let header = settings_modal_header(
        column![
            text(if editing { tr.__("编辑定时任务") } else { tr.__("新建定时任务") }).size(20),
            text("配置执行内容、时间规则与有效期").size(13).style(settings_muted_text_style)
        ]
        .spacing(4),
        settings_close_button(close.clone()),
    );
    let actions = row![
        secondary_button(tr.__("取消"), close.clone()),
        schedule::save_button(tr, draft, editing),
    ]
    .spacing(10);
    let mut footer = column![schedule::preview(draft, editing)].spacing(8).width(Length::Fill);
    if let Some(error) = save_error_banner(app) {
        footer = footer.push(error);
    }
    let footer = footer.push(container(actions).align_right(Length::Fill));
    let card = settings_modal_layout(
        header,
        if editing { edit_form(app) } else { add_tab(app) },
        Some(footer.into()),
    )
    .style(cron_modal_style)
    .width(740)
    .height(620);
    settings_modal_overlay_with_escape(dialog, close, container(card).padding([10, 12]))
}
