//! Stable column keys match the reference. Only deviations from defaults are saved.
use std::{collections::BTreeMap, path::PathBuf};

use desktop_assets::assets::{Icon, get_icon};
use desktop_ui_controls::system_settings_common::{
    rounded_action_btn_style, settings_modal_card_style, settings_pick_list_menu_style,
};
use iced::{
    Element, Length, Theme,
    widget::{Space, button, column, container, row, scrollable, svg, text, tooltip},
};

pub(super) type Column = (&'static str, &'static str, bool, f32);

pub const COLUMNS: [(&str, &str, bool, f32); 14] = [
    ("time", "时间", true, 104.0),
    ("app", "应用", true, 70.0),
    ("provider", "供应商", true, 82.0),
    ("model", "模型", true, 132.0),
    ("cost", "成本", true, 80.0),
    ("freshInput", "新增输入", true, 74.0),
    ("output", "输出", true, 60.0),
    ("cacheRead", "缓存命中", true, 74.0),
    ("cacheWrite", "缓存写入", false, 74.0),
    ("hitRate", "命中率", false, 72.0),
    ("speed", "速度", true, 82.0),
    ("firstToken", "首字", true, 70.0),
    ("duration", "总耗时", false, 74.0),
    ("statusCode", "状态码", false, 68.0),
];

#[derive(Default)]
pub struct Preferences {
    overrides: BTreeMap<String, bool>,
}
impl Preferences {
    pub fn visible(&self, index: usize) -> bool {
        self.visible_in(&COLUMNS, index)
    }
    pub(super) fn visible_in(&self, columns: &[Column], index: usize) -> bool {
        columns.get(index).is_some_and(|(key, _, default, _)| {
            index == 0 || self.overrides.get(*key).copied().unwrap_or(*default)
        })
    }
    pub fn toggle(&mut self, index: usize) {
        self.toggle_in(&COLUMNS, index);
    }
    pub(super) fn toggle_in(&mut self, columns: &[Column], index: usize) {
        if index == 0 {
            return;
        }
        let Some((key, _, default, _)) = columns.get(index) else {
            return;
        };
        let next = !self.visible_in(columns, index);
        if next == *default {
            self.overrides.remove(*key);
        } else {
            self.overrides.insert((*key).into(), next);
        }
    }
    fn path(file: &str) -> Result<PathBuf, String> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or("无法获取用户目录")?;
        Ok(config_runtime_paths::home_config_dir(PathBuf::from(home)).join(file))
    }
    pub fn load() -> Result<Self, String> {
        Self::load_from(&COLUMNS, "usage-log-columns.json")
    }
    pub(super) fn load_from(columns: &[Column], file: &str) -> Result<Self, String> {
        let path = Self::path(file)?;
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
        };
        let mut overrides: BTreeMap<String, bool> =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        overrides.retain(|key, value| {
            columns.iter().skip(1).any(|(id, _, default, _)| key == id && *value != *default)
        });
        Ok(Self { overrides })
    }
    pub fn save(&self) -> Result<(), String> {
        self.save_to("usage-log-columns.json")
    }
    pub(super) fn save_to(&self, file: &str) -> Result<(), String> {
        let path = Self::path(file)?;
        let parent = path.parent().ok_or("列设置路径无效")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec(&self.overrides).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }
}

pub(super) fn column_menu<'a, M: Clone + 'a>(i18n: desktop_i18n::I18n,
    options_iter: impl IntoIterator<Item = (usize, &'static str)>,
    visible: impl Fn(usize) -> bool,
    open: bool,
    toggle_menu: M,
    close_menu: M,
    toggle: impl Fn(usize) -> M,
) -> Element<'a, M> {
    let icon = |icon| {
        svg(get_icon(icon))
            .width(16)
            .height(16)
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
    };
    let mut options = column![].spacing(2);
    for (index, label) in options_iter {
        let check: Element<'_, M> =
            if visible(index) { icon(Icon::Check).into() } else { Space::new().width(16).into() };
        options = options.push(
            button(row![check, text(i18n.__(label)).size(13)].spacing(9).align_y(iced::Alignment::Center))
                .width(Length::Fill)
                .padding([6, 9])
                .style(|theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    style.border = iced::border::rounded(7);
                    if status == button::Status::Active {
                        style.background = None;
                    }
                    style
                })
                .on_press(toggle(index)),
        );
    }
    let menu =
        container(scrollable(container(options).padding([0, 7]).width(Length::Fill)).direction(
            scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ),
        ))
        .padding([5, 0])
        .width(192)
        .max_height(458)
        .style(|theme| {
            let menu_style = settings_pick_list_menu_style(theme);
            let mut style = settings_modal_card_style(theme);
            style.background = Some(menu_style.background);
            style.border = menu_style.border.rounded(12);
            style.shadow.offset.y = 4.0;
            style.shadow.blur_radius = 18.0;
            style
        });
    let trigger = tooltip(
        button(container(icon(Icon::Columns)).center_x(Length::Fill).center_y(Length::Fill))
            .width(32)
            .height(30)
            .padding(0)
            .style(move |theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border = iced::border::rounded(7);
                let ink = theme.palette().text;
                style.background = match status {
                    _ if open || status == button::Status::Pressed => {
                        Some(ink.scale_alpha(0.10).into())
                    }
                    button::Status::Hovered => Some(ink.scale_alpha(0.06).into()),
                    _ => None,
                };
                style
            })
            .on_press(toggle_menu),
        text(i18n.__("显示列")).size(12),
        tooltip::Position::Top,
    )
    .snap_within_viewport(true);
    desktop_ui_controls::overlays::BelowOverlay::new(trigger, menu)
        .show(open)
        .gap(5.0)
        .on_close(close_menu)
        .capture_outside_click(true)
        .into()
}
