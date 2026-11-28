//! 新增与编辑共用的固定标题、滚动正文和保存页脚。
use super::*;
use crate::message::SettingsMessage as S;
use crate::state::CronEditorTab;
use desktop_ui_controls::system_settings_common::{
    settings_modal_layout_with_scroll_id, settings_modal_overlay_with_escape,
};
use iced::widget::{column, row};

pub fn with_edit_modal<'a, M: Clone + 'static>(
    app: &ViewData<'a>,
    dialog: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    let s = app.cron_settings;
    if s.show_cron_templates {
        return templates::overlay(I18n::new(app.desktop_locale), dialog, map);
    }
    let editing = s.editing_job_id.is_some();
    if !editing && !s.show_add_modal {
        return dialog;
    }
    let draft = if editing { &s.edit_draft } else { &s.add_draft };
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let close = if draft.submitting {
        Message::None
    } else {
        Message::Settings(message::SettingsMessage::CronJobEditCanceled)
    };
    let title = settings_modal_header(
        text(if editing { i18n.__("编辑定时任务") } else { i18n.__("新建定时任务") })
            .size(20)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::DEFAULT }),
        settings_close_button(close.clone()),
    );
    let select_tab = |tab| Message::Settings(S::CronEditorTabSelected { editing, tab });
    let tabs = settings_tab_row(
        [
            (CronEditorTab::Basic, "基本"),
            (CronEditorTab::Advanced, "高级设置"),
            (CronEditorTab::Delivery, "结果投递"),
            (CronEditorTab::Preview, "预览未来"),
        ]
        .into_iter()
        .map(|(tab, label)| {
            settings_tab_button(
                i18n.__(label),
                draft.editor_tab == tab,
                (!draft.submitting && draft.editor_tab != tab).then(|| select_tab(tab)),
            )
            .into()
        }),
    )
    .wrap();
    let header = column![title, tabs, settings_divider()].spacing(10);
    let actions = row![
        secondary_button(i18n.__("取消"), close.clone()),
        schedule::save_button(i18n, draft, editing),
    ]
    .spacing(10);
    let mut footer = column![settings_divider()].spacing(10).width(Length::Fill);
    if let Some(error) = save_error_banner(app) {
        footer = footer.push(error);
    }
    let footer = footer.push(container(actions.wrap()).align_right(Length::Fill));
    let card = settings_modal_layout_with_scroll_id(
        header,
        if editing { edit_form(app) } else { add_tab(app) },
        Some(footer.into()),
        Some(EDITOR_SCROLL_ID),
    )
    .style(cron_modal_style)
    .width(800)
    .height(590);
    settings_modal_overlay_with_escape(
        dialog,
        map(close),
        Element::<Message>::from(container(card).padding([10, 12])).map(map),
    )
}
