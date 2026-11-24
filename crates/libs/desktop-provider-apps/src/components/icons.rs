use desktop_ui_controls::system_settings_common::*;
use crate::{
    Message,
    message::icons::{Event, MAX_ICON_BYTES, PNG_PREFIX, SCROLL_ID, message},
    state::icons::{Draft, Form},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, image, row, scrollable, text, text_input},
};
use vw_api_app_provider_edit::icons::AppProviderIconChange as Change;

fn action(label: &str, event: Event, enabled: bool, primary: bool) -> Element<'_, Message> {
    button(text(label))
        .padding([8, 14])
        .style(if primary { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}

fn thumbnail(data: &str) -> Option<image::Handle> {
    if data.len() > MAX_ICON_BYTES {
        return None;
    }
    let png = STANDARD.decode(data.strip_prefix(PNG_PREFIX)?).ok()?;
    // Only hand bounded inline PNGs to the renderer, never a path or URL.
    if !png.starts_with(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR") {
        return None;
    }
    let width = u32::from_be_bytes(png.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(png.get(20..24)?.try_into().ok()?);
    if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
        return None;
    }
    // Decode before creating a handle so truncated or corrupt images use the fallback too.
    // Dimensions and encoded size are bounded above; never pass paths or URLs to Iced.
    let pixels = ::image::load_from_memory_with_format(&png, ::image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    Some(image::Handle::from_rgba(pixels.width(), pixels.height(), pixels.into_raw()))
}

fn preview(i18n: desktop_i18n::I18n, data: &str) -> Element<'static, Message> {
    match thumbnail(data) {
        Some(handle) => image(handle).width(32).height(32).into(),
        None => text(i18n.__("预览不可用")).size(12).style(settings_muted_text_style).into(),
    }
}

pub fn card<'a>(i18n: desktop_i18n::I18n, form: &'a Form) -> Element<'a, Message> {
    let ready = !form.busy;
    let selecting = ready && form.draft.is_none();
    let current = match &form.current {
        Some(icon) if icon.data.is_some() => {
            row![text(i18n.__("已保存：")), preview(i18n, icon.data.as_deref().unwrap())]
        }
        Some(_) => row![text(i18n.__("已保存：无自定义图标"))],
        None => row![text(i18n.__("当前图标尚未读取"))],
    }
    .align_y(Alignment::Center)
    .spacing(6);

    let selected = match &form.selection {
        Some(Change::Select { icon_id }) => {
            form.rows.as_ref().and_then(|rows| rows.iter().find(|row| &row.id == icon_id))
        }
        _ => None,
    };
    let pending = match (&form.selection, selected) {
        (Some(Change::Clear), _) => row![text(i18n.__("待保存：清除自定义图标"))],
        (_, Some(icon)) => row![text(i18n.format("待保存：%s", &[(format!("{}", icon.name)).into()]).expect("valid display template")), preview(i18n, &icon.data)],
        _ => row![text(i18n.__("没有待保存的选择"))],
    }
    .align_y(Alignment::Center)
    .spacing(6);

    let input = |name: &'a str| {
        text_input(i18n.__("图标名称"), name)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(ready.then_some(|value| message(Event::DraftName(value))))
    };
    let mut fields = column![
        settings_panel(column![current, pending].spacing(6)),
        text(i18n.__("我的图标")).size(18),
        text(i18n.__("库操作立即保存。删除库条目会保留供应商已保存的图标。"))
            .size(13)
            .style(settings_muted_text_style),
    ]
    .spacing(10);
    match &form.draft {
        Some(Draft::Upload { name, .. }) => {
            fields = fields.push(settings_panel(
                column![
                    text(i18n.__("上传 PNG · 名称")),
                    input(name),
                    action(i18n.__("更换文件"), Event::Pick, ready, false),
                ]
                .spacing(8),
            ));
        }
        Some(Draft::Rename { name, .. }) => {
            fields =
                fields.push(settings_panel(column![text(i18n.__("重命名 · 名称")), input(name)].spacing(8)));
        }
        Some(Draft::Delete { name, .. }) => {
            fields = fields.push(settings_panel(text(i18n.format("从图标库删除「%s」？", &[(format!("{}", name)).into()]).expect("valid display template"))));
        }
        None => {
            fields = fields.push(action(
                i18n.__("选择 PNG 文件"),
                Event::Pick,
                selecting && form.rows.is_some(),
                false,
            ));
        }
    }
    if let Some(rows) = &form.rows {
        if rows.is_empty() {
            fields = fields.push(text(i18n.__("图标库为空。")));
        }
        for icon in rows {
            fields = fields.push(settings_panel(
                row![
                    preview(i18n, &icon.data),
                    text(&icon.name).width(Length::Fill),
                    action(i18n.__("选择"), Event::Select(icon.id.clone()), selecting, false),
                    action(i18n.__("重命名"), Event::Rename(icon.id.clone()), selecting, false),
                    action(i18n.__("删除"), Event::Delete(icon.id.clone()), selecting, false),
                ]
                .align_y(Alignment::Center)
                .spacing(8),
            ));
        }
    }

    let footer = if let Some(draft) = &form.draft {
        let commit = match draft {
            Draft::Upload { .. } => i18n.__("上传到库"),
            Draft::Rename { .. } => i18n.__("保存名称"),
            Draft::Delete { .. } => i18n.__("确认删除"),
        };
        row![
            action(i18n.__("返回"), Event::CancelDraft, ready, false),
            action(commit, Event::CommitDraft, ready, true),
        ]
    } else {
        row![
            action(i18n.__("刷新"), Event::Refresh, ready, false),
            action(
                i18n.__("清除图标"),
                Event::Clear,
                ready && form.current.as_ref().is_some_and(|icon| icon.data.is_some()),
                false
            ),
            action(i18n.__("取消"), Event::Close, ready, false),
            action(
                i18n.__("保存图标"),
                Event::Save,
                ready && (selected.is_some() || matches!(form.selection, Some(Change::Clear))),
                true,
            ),
        ]
    }
    .spacing(8);
    let mut layout = column![
        row![
            text(i18n.format("供应商图标 · %s", &[(format!("{}", form.name)).into()]).expect("valid display template")).size(22).width(Length::Fill),
            settings_close_button(message(Event::Close)),
        ]
        .spacing(8),
    ]
    .spacing(12);
    if let Some(error) = &form.notice {
        layout = layout.push(settings_panel(text(i18n.format("图标操作失败：%s", &[(format!("{}", error)).into()]).expect("valid display template")).size(13)));
    } else if form.busy {
        layout = layout.push(text(i18n.__("正在处理…")).size(13).style(settings_muted_text_style));
    }
    let card = settings_modal_card(
        layout
            .push(
                scrollable(container(fields).padding([0, 7]))
                    .id(SCROLL_ID)
                    .direction(scrollable::Direction::Vertical(
                        scrollable::Scrollbar::new().width(4).scroller_width(4),
                    ))
                    .height(Length::Fill),
            )
            .push(footer),
    )
    .width(Length::Fixed(680.))
    .height(Length::Fixed(620.));
    card.into()
}
