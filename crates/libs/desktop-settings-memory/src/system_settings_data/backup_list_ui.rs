#[derive(Clone)]
pub enum OverlayEvent<E> {
    Local(Event),
    External(E),
}
use vw_api_app_operations::DataBackupSummary;
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Principal(String),
    Name(String),
    Interval(String),
    Retain(String),
    ToggleLocations,
    Refresh,
    Create,
    SavePolicy,
    Cancel,
    Confirm,
    Rename(DataBackupSummary),
    Delete(DataBackupSummary),
}
fn message<E>(event: Event) -> OverlayEvent<E> { OverlayEvent::Local(event) }
pub enum Pending<'a> {
    Cleanup(&'a str),
    Rename(&'a str),
    Delete(&'a str),
}
pub struct ViewState<'a> {
    pub busy: bool,
    pub show_locations: bool,
    pub principal: &'a str,
    pub name: &'a str,
    pub interval: &'a str,
    pub retain: &'a str,
    pub pending: Option<Pending<'a>>,
    pub rows: &'a [DataBackupSummary],
}
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length};
use iced::widget::{button, column, container, row, scrollable, text};

#[derive(Clone, Copy)]
struct BackupInterval(u32, I18n);

impl PartialEq for BackupInterval { fn eq(&self, other: &Self) -> bool { self.0 == other.0 } }
impl Eq for BackupInterval {}
impl std::fmt::Display for BackupInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let i18n = self.1;
        match self.0 {
            0 => f.write_str(i18n.__("禁用")),
            168 => f.write_str(i18n.__("7 天")),
            hours => f.write_str(&desktop_i18n::__!(i18n, "%u 小时", hours).expect("valid interval")),
        }
    }
}

fn action<'a, E: Clone + 'static>(i18n: I18n, label: &'a str, event: Event, enabled: bool) -> Element<'a, OverlayEvent<E>> {
    button(text(i18n.__(label))).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event))).into()
}
pub fn view<'a, E: Clone + 'static>(i18n: I18n,
    state: ViewState<'a>,
    visible: bool,
    available: bool,
    recovery: Element<'a, E>,
    locations: Element<'a, E>,
    restore: impl Fn(String) -> E,
) -> Element<'a, OverlayEvent<E>> {
    if !visible {
        return row![action(i18n, i18n.__("加载备份"), Event::Open, available), recovery.map(OverlayEvent::External)]
            .spacing(8)
            .into();
    }
    let enabled = !state.busy;
    let editable = enabled && state.pending.is_none();
    let interval = state.interval.parse().ok().map(|hours| BackupInterval(hours, i18n));
    let interval_select = settings_select(
        [0, 6, 12, 24, 48, 168].map(|hours| BackupInterval(hours, i18n)),
        interval,
        |value| message(Event::Interval(value.0.to_string())),
    )
    .width(Length::Fill)
    .placeholder(i18n.__("选择备份间隔"))
    .disabled(!editable);
    let mut body = column![
        row![
            column![
                text(i18n.__("自动备份间隔")).size(14),
                interval_select,
            ].spacing(6).width(Length::Fill),
            column![
                text(i18n.__("备份保留数量")).size(14),
                settings_text_input(i18n.__("保留数量"), state.retain)
                    .on_input_maybe(editable.then_some(|v| message(Event::Retain(v)))),
            ].spacing(6).width(Length::Fill),
        ].spacing(12),
        action(i18n, i18n.__("保存策略"), Event::SavePolicy, editable && interval.is_some() && !state.retain.is_empty()),
        settings_divider(),
        row![
            text(i18n.__("偏好设置备份")).size(16).width(Length::Fill),
            action(i18n, if enabled { i18n.__("刷新") } else { i18n.__("处理中…") }, Event::Refresh, editable),
            recovery.map(OverlayEvent::External),
            button(text(i18n.__("立即备份")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 14]).style(primary_action_btn_style)
                .on_press_maybe(editable.then_some(message(Event::Create))),
        ].spacing(8).align_y(iced::Alignment::Center).wrap(),
    ].spacing(12).width(Length::Fill);
    if let Some(pending) = &state.pending {
        let prompt = match pending {
            Pending::Cleanup(id) => desktop_i18n::__!(i18n, "确认永久清理分类 %s？可能因保护规则仅清理部分文件。", (id).to_string()).expect("valid settings message"),
            Pending::Rename(row) => desktop_i18n::__!(i18n, "确认重命名备份“%s”？", (row).to_string()).expect("valid settings message"),
            Pending::Delete(row) => desktop_i18n::__!(i18n, "确认永久删除备份“%s”？受保护或版本冲突将拒绝删除。", (row).to_string()).expect("valid settings message"),
        };
        let mut confirmation = column![text(prompt)].spacing(8);
        if matches!(pending, Pending::Rename(_)) {
            confirmation = confirmation.push(settings_text_input(i18n.__("备份名称"), state.name)
                .on_input_maybe(enabled.then_some(|name| message(Event::Name(name)))));
        }
        body = body.push(confirmation.push(row![
            action(i18n, i18n.__("取消操作"), Event::Cancel, enabled),
            action(i18n, i18n.__("确认操作"), Event::Confirm, enabled),
        ].spacing(10)));
    }
    if state.rows.is_empty() && enabled {
        body = body.push(text(i18n.__("暂无备份")).size(13).style(settings_muted_text_style));
    }
    let mut list = column![].spacing(8);
    for backup in state.rows {
        list = list.push(settings_panel(row![
            column![
                text(backup.name.clone()).wrapping(text::Wrapping::Glyph),
                text(format!("{} · {:.1} MB",
                    chrono::DateTime::from_timestamp_millis(backup.created_at_ms)
                        .map(|time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default(),
                    backup.size_bytes as f64 / 1_000_000.0))
                    .size(13).style(settings_muted_text_style),
            ].spacing(4).width(Length::Fill),
            action(i18n, i18n.__("重命名"), Event::Rename(backup.clone()), editable),
            action(i18n, i18n.__("删除"), Event::Delete(backup.clone()), editable),
            button(text(i18n.__("恢复")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 14]).style(rounded_action_btn_style)
                .on_press_maybe(editable.then(|| OverlayEvent::External(restore(backup.backup_id.0.clone())))),
        ].spacing(8).align_y(iced::Alignment::Center).wrap()));
    }
    if !state.rows.is_empty() {
        body = body.push(container(
            scrollable(list).direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
        ).max_height(300).width(Length::Fill));
    }
    let total_mb = state.rows.iter().map(|backup| backup.size_bytes as f64).sum::<f64>() / 1_000_000.0;
    body = body.push(settings_divider()).push(row![
        action(i18n, if state.show_locations { i18n.__("收起其他备份") } else { i18n.__("其他备份") }, Event::ToggleLocations, enabled),
        text(desktop_i18n::__!(i18n, "本地备份合计 %s MB", format!("{:.1}", total_mb)).expect("valid settings message")).size(13)
            .style(settings_muted_text_style).width(Length::Fill).align_x(iced::Alignment::End),
    ].spacing(8).align_y(iced::Alignment::Center));
    if state.show_locations {
        body = body.push(locations.map(OverlayEvent::External));
    }
    body.into()
}
