use super::Event;
use desktop_ui_controls::system_settings_common::icons::{
    icon_btn_style, icon_btn_svg_style, icon_btn_tooltip_style,
};
use desktop_ui_controls::system_settings_common::*;
use gateway_app_prompts::{AgentAppPromptCatalog, AgentAppPromptsScope};
use iced::{
    Element,
    widget::{button, column, text},
};
use vw_api_app_operations::PromptTargetPreview;
use vw_api_types::agent_apps::AppId;

#[derive(Clone, Debug)]
pub struct State<C> {
    pub query: String,
    pub menu: Option<Menu>,
    pub sync: super::sync::State<C>,
    pub templates: super::templates::State,
    pub shared_import: Option<super::shared_import::Draft>,
    pub transfer: super::transfer::State,
    pub pi_files: super::pi_files::State,
    pub scope: Option<AgentAppPromptsScope>,
    pub generation: u64,
    pub loading: bool,
    pub catalog: Option<AgentAppPromptCatalog>,
    pub preview: Option<PromptTargetPreview>,
    pub error: Option<String>,
    pub preview_error: Option<String>,
    pub mutating: bool,
    pub undo: Option<super::activation::Undo>,
    pub editor: Option<super::editor::Draft>,
}
impl<C> State<C> {
    pub fn invalidate(&mut self) {
        let mut sync = std::mem::take(&mut self.sync);
        sync.open = false;
        *self = Self { sync, generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
/// Only one action surface can be open in the current scope.
#[derive(Clone, Debug, PartialEq)]
pub enum Menu {
    Apps,
    Page,
    Entry(String),
    Help,
}

pub fn view<'a, M: Clone + 'static, C: Clone + 'a>(
    state: &'a State<C>,
    id: AppId,
    application: &str,
    selector: Element<'a, M>,
    local: bool,
    created_date: fn(i64) -> Option<String>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    use super::transfer::TransferEvent as Transfer;
    use desktop_assets::assets::Icon;
    use iced::{
        Alignment, Font, Length,
        widget::{Space, container, row, scrollable},
    };

    let ready = !state.loading && !state.mutating;
    let catalog =
        state.catalog.as_ref().filter(|_| state.scope.as_ref().is_some_and(|s| s.app_id == id));
    let loaded = catalog.is_some();
    let add = (ready && loaded).then(|| message(Event::Edit(None)));
    let more = Menu::Page;
    let path = state.preview.as_ref().and_then(|p| p.display_path.as_deref());
    let importable = ready
        && loaded
        && state
            .preview
            .as_ref()
            .is_some_and(|p| p.snapshot.content.as_deref().is_some_and(|c| !c.trim().is_empty()));
    let filename = path.and_then(|path| path.rsplit(['/', '\\']).find(|part| !part.is_empty()));
    let import_label = filename
        .map(|filename| format!("导入当前的 {filename}"))
        .unwrap_or_else(|| "导入当前文件".into());
    let mut import = column![text(import_label).size(14)].spacing(3);
    if !importable {
        let reason = match state.preview.as_ref().map(|p| p.snapshot.content.as_deref()) {
            Some(None) => format!("{}不存在，没有可导入的内容", filename.unwrap_or("目标文件")),
            Some(Some(content)) if content.trim().is_empty() => {
                "目标文件为空，没有可导入的内容".into()
            }
            _ if state.preview_error.is_some() => "目标文件读取失败，请刷新后重试".into(),
            _ if !loaded && state.error.is_some() => "提示词库不可用，请刷新后重试".into(),
            _ => "读取目标文件后可导入".into(),
        };
        import = import.push(text(reason).size(12).style(settings_muted_text_style));
    }
    let mut page_actions = column![
        button(import)
            .width(Length::Fill)
            .padding([8, 10])
            .style(menu_action_style)
            .on_press_maybe(importable.then(|| message(Event::Transfer(Transfer::ImportHost)))),
        menu_action(
            if local { "打开所在文件夹" } else { "复制远端路径" },
            (ready && path.is_some()).then(|| message(Event::Transfer(Transfer::Reveal))),
        ),
    ]
    .spacing(2);
    if local {
        page_actions = page_actions.push(menu_action(
            "复制文件路径",
            (ready && path.is_some()).then(|| message(Event::Transfer(Transfer::Path))),
        ));
    }
    page_actions = page_actions
        .push(settings_divider())
        .push(menu_action(
            "从本机选择文件…",
            (ready && loaded).then(|| message(Event::Transfer(Transfer::ImportClient))),
        ))
        .push(menu_action("刷新提示词", ready.then(|| message(Event::Open(id)))))
        .push(menu_action(
            "同步提示词",
            ready.then(|| message(Event::Sync(super::sync::Event::Open))),
        ))
        .push(menu_action("识矩系统提示词", ready.then(|| message(Event::Agents))))
        .push(menu_action("项目规则", ready.then(|| message(Event::Projects))));
    if id == AppId::Pi {
        page_actions = page_actions
            .push(menu_action(
                "Pi 原生模板",
                ready.then(|| message(Event::Template(super::templates::Event::Open))),
            ))
            .push(menu_action(
                "Pi 系统文件",
                ready.then(|| message(Event::PiFile(super::pi_files::FileEvent::Open))),
            ));
    }

    let close = message(Event::Menu(None));
    let toggle =
        |menu: Menu| message(Event::Menu((state.menu.as_ref() != Some(&menu)).then_some(menu)));
    let header = row![
        icon_svg(Icon::FileText, 19.0).style(icon_btn_svg_style),
        text("提示词")
            .size(21)
            .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }),
        prompt_menu(
            icon_btn(Icon::QuestionCircle, "提示词帮助", Some(toggle(Menu::Help))),
            container(
                text("启用提示词会写入当前应用的目标文件。复制到其他应用后，提示词保持停用。")
                    .size(13)
            )
            .padding(10)
            .into(),
            &Menu::Help,
            state.menu == Some(Menu::Help),
            close.clone(),
        ),
        Space::new().width(Length::Fill),
        button(
            container(
                row![text("+").size(17), text("添加提示词").size(14)]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
            .center_y(Length::Fill)
        )
        .height(34)
        .padding([0, 13])
        .style(primary_action_btn_style)
        .on_press_maybe(add.clone()),
        prompt_menu(
            more_button("更多操作", ready.then(|| toggle(more.clone()))),
            page_actions.into(),
            &more,
            state.menu.as_ref() == Some(&more),
            close.clone(),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let status = if state.loading {
        "正在读取…"
    } else if state.preview_error.is_some() {
        "读取失败"
    } else {
        match state.preview.as_ref().map(|p| p.snapshot.content.as_deref()) {
            Some(None) => "文件不存在",
            Some(Some("")) => "空文件",
            Some(Some(_)) => "已有内容",
            None => "尚未读取",
        }
    };
    let mut target = row![text("目标文件").size(13).style(settings_muted_text_style)]
        .spacing(5)
        .align_y(Alignment::Center);
    if let Some(path) = path {
        target = target.push(text(path).size(13).font(if path.is_ascii() {
            Font::MONOSPACE
        } else {
            Font::DEFAULT
        }));
    }
    target = target.push(
        text(if path.is_some() { format!("· {status}") } else { status.into() })
            .size(13)
            .style(settings_muted_text_style),
    );
    let mut tools = row![
        selector,
        container(target.width(Length::Fill).wrap().vertical_spacing(3))
            .center_y(iced::Length::Shrink)
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if catalog.is_some_and(|catalog| !catalog.entries.is_empty()) {
        let input = settings_text_input("搜索名称、说明或正文", &state.query)
            .width(224)
            .padding(iced::Padding::from(SETTINGS_CONTROL_PADDING).left(33.0).right(32.0))
            .on_input(move |value| message(Event::Query(value)));
        let lens = container(icon_svg(Icon::Search, 15.0).style(|theme: &iced::Theme, _| {
            iced::widget::svg::Style { color: Some(theme.palette().text.scale_alpha(0.62)) }
        }))
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .padding([0, 11]);
        let mut search = iced::widget::stack![input, lens];
        if !state.query.is_empty() {
            search = search.push(
                container(icon_btn(
                    Icon::X,
                    "清除搜索",
                    Some(message(Event::Query(String::new()))),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Center)
                .padding([0, 4]),
            );
        }
        tools = tools.push(search);
    }
    let mut workspace = column![container(tools).padding([0, 7])].spacing(12).height(Length::Fill);
    let library_read_error = if loaded { None } else { state.error.as_deref() };
    let preview_read_error = state.preview_error.as_deref();
    if library_read_error.is_some() || preview_read_error.is_some() {
        let mut read_errors = column![].spacing(3).width(Length::Fill);
        for error in [library_read_error, preview_read_error].into_iter().flatten() {
            read_errors = read_errors.push(text(error).size(13).style(settings_muted_text_style));
        }
        let retry = button(text("重试").size(13))
            .padding([5, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(ready.then(|| message(Event::Open(id))));
        workspace = workspace.push(
            container(row![read_errors, retry].spacing(10).align_y(Alignment::Center))
                .padding([0, 7]),
        );
    }
    if loaded && let Some(error) = &state.error {
        workspace = workspace.push(container(settings_error_banner(error.clone())).padding([0, 7]));
    }
    if let Some(hint) = match id {
        AppId::Pi => Some("更改后请在 Pi 执行 /reload。"),
        AppId::Hermes => Some("Hermes 提示词在新会话中生效。"),
        _ => None,
    } {
        workspace = workspace.push(text(hint).size(12).style(settings_muted_text_style));
    }
    if state.undo.is_some() {
        workspace = workspace.push(
            button(text("撤销上次操作").size(13))
                .padding([5, 10])
                .style(rounded_action_btn_style)
                .on_press_maybe(ready.then(|| message(Event::Undo))),
        );
    }

    if let Some(catalog) = catalog {
        let query = state.query.trim().to_lowercase();
        let mut rows = column![].spacing(7);
        let mut visible = false;
        for entry in &catalog.entries {
            if ![
                Some(entry.name.as_str()),
                entry.description.as_deref(),
                Some(entry.content.as_str()),
            ]
            .into_iter()
            .flatten()
            .any(|v| v.to_lowercase().contains(&query))
            {
                continue;
            }
            visible = true;
            let active = catalog.active_id.as_ref() == Some(&entry.id);
            let summary: String = entry
                .description
                .as_deref()
                .and_then(|description| description.lines().find(|line| !line.trim().is_empty()))
                .or_else(|| entry.content.lines().find(|line| !line.trim().is_empty()))
                .unwrap_or_default()
                .trim()
                .chars()
                .take(100)
                .collect();
            let bytes = entry.content.len();
            let mut meta = if summary.is_empty() {
                format!("{bytes} B")
            } else {
                format!("{summary} · {bytes} B")
            };
            if let Some(created) = entry.created_at.and_then(created_date) {
                meta.push_str(&format!(" · 创建于 {created}"));
            }
            let details = column![
                text(&entry.name).size(15),
                text(meta).size(12).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill);
            let mutation = if active {
                super::activation::Action::Deactivate(entry.id.clone())
            } else {
                super::activation::Action::Activate(entry.id.clone())
            };
            let menu = Menu::Entry(entry.id.clone());
            let entries = column![
                menu_action(
                    "复制到其他应用…",
                    ready.then(|| message(Event::Transfer(Transfer::Open(entry.id.clone()))))
                ),
                menu_action(
                    "复制正文",
                    ready.then(|| message(Event::Transfer(Transfer::Body(entry.id.clone()))))
                ),
                settings_divider(),
                button(text("删除").size(13))
                    .width(Length::Fill)
                    .padding([7, 10])
                    .style(danger_action_btn_style)
                    .on_press_maybe((ready && state.preview.is_some()).then(|| message(
                        Event::Mutate(super::activation::Action::Delete(entry.id.clone()))
                    ))),
            ];
            let menu_open = state.menu.as_ref() == Some(&menu);
            let actions = row![
                text(if state.mutating {
                    "处理中…"
                } else if active {
                    "已启用"
                } else {
                    ""
                })
                .size(12)
                .style(settings_muted_text_style),
                button(text(if active { "停用" } else { "启用" }).size(13))
                    .padding([6, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(
                        (ready && state.preview.is_some())
                            .then(|| message(Event::Mutate(mutation)))
                    ),
                icon_btn(
                    Icon::Pencil,
                    "编辑提示词",
                    ready.then(|| message(Event::Edit(Some(entry.id.clone()))))
                ),
                prompt_menu(
                    more_button("条目操作", ready.then(|| toggle(menu.clone()))),
                    entries.into(),
                    &menu,
                    menu_open,
                    close.clone()
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            rows = rows.push(
                container(row![details, actions].spacing(10).align_y(Alignment::Center))
                    .padding([10, 12])
                    .width(Length::Fill)
                    .style(move |theme| {
                        let mut style = settings_panel_style(theme);
                        style.border.radius = 10.0.into();
                        style.shadow = iced::Shadow::default();
                        if active || menu_open {
                            style.background =
                                Some(theme.extended_palette().background.weak.color.into());
                        }
                        style
                    }),
            );
        }
        if visible {
            workspace = workspace.push(
                scrollable(container(rows).padding([0, 7]))
                    .direction(scrollable::Direction::Vertical(
                        scrollable::Scrollbar::new().width(4).scroller_width(4),
                    ))
                    .height(Length::Fill),
            );
        } else {
            let empty = catalog.entries.is_empty();
            let title = if empty {
                format!("{application} 还没有提示词")
            } else {
                "没有匹配的提示词".to_owned()
            };
            let detail =
                if empty { path.map(|path| format!("启用后将写入 {path}")) } else { None };
            let action = if empty { add } else { Some(message(Event::Query(String::new()))) };
            let mut empty_state = column![
                text(title)
                    .size(17)
                    .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
            ]
            .spacing(12)
            .align_x(Alignment::Center);
            if let Some(detail) = detail {
                empty_state = empty_state.push(
                    text(detail)
                        .size(13)
                        .align_x(Alignment::Center)
                        .style(settings_muted_text_style),
                );
            }
            empty_state = empty_state.push(
                button(text(if empty { "+ 添加提示词" } else { "清除搜索" }).size(13))
                    .padding([7, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(action),
            );
            workspace = workspace.push(container(empty_state).center(Length::Fill).padding(12));
        }
    }
    let base = column![
        container(header).padding([8, 18]).width(Length::Fill),
        settings_divider(),
        container(workspace).padding([12, 11]).width(Length::Fill).height(Length::Fill),
    ]
    .height(Length::Fill)
    .into();
    let base = super::sync::overlay(&state.sync, base, message);
    super::shared_import::overlay(state, base, message)
}

fn more_button<'a, M: Clone + 'static>(label: &'static str, event: Option<M>) -> Element<'a, M> {
    use desktop_assets::assets::Icon;
    use iced::widget::{container, tooltip};

    let dots = icon_svg(Icon::DotsThreeVertical, 15.0)
        .rotation(std::f32::consts::FRAC_PI_2)
        .style(icon_btn_svg_style);
    tooltip(
        button(dots).padding([6, 8]).style(icon_btn_style).on_press_maybe(event),
        container(text(label).size(12)).padding([6, 10]).style(icon_btn_tooltip_style),
        tooltip::Position::Bottom,
    )
    .gap(5)
    .into()
}

pub fn menu_action<'a, M: Clone + 'a>(
    label: &'a str,
    event: Option<M>,
) -> iced::widget::Button<'a, M> {
    button(text(label).size(14))
        .width(iced::Length::Fill)
        .padding([7, 10])
        .style(menu_action_style)
        .on_press_maybe(event)
}

pub fn menu_action_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.width = 0.0;
    style.border.radius = 6.0.into();
    if matches!(status, button::Status::Active | button::Status::Disabled) {
        style.background = None;
    }
    style
}

pub fn prompt_menu<'a, M: Clone + 'static>(
    trigger: impl Into<Element<'a, M>>,
    items: Element<'a, M>,
    kind: &Menu,
    open: bool,
    close: M,
) -> Element<'a, M> {
    use desktop_ui_controls::overlays::BelowOverlay;
    use iced::{
        Length,
        widget::{container, scrollable},
    };
    let width = match kind {
        Menu::Apps => 300,
        Menu::Page => 290,
        Menu::Entry(_) => 234,
        Menu::Help => 260,
    };
    let menu = container(
        scrollable(container(items).padding([0, 7]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Shrink),
    )
    .width(width)
    .max_height(408)
    .padding(4)
    .style(|theme| {
        let mut style = settings_modal_card_style(theme);
        style.background = Some(theme.extended_palette().background.base.color.into());
        style.border.radius = 10.0.into();
        style.shadow.offset = iced::Vector::new(0.0, 4.0);
        style.shadow.blur_radius = 14.0;
        style
    });
    BelowOverlay::new(trigger, menu)
        .align_right(matches!(kind, Menu::Page | Menu::Entry(_)))
        .gap(4.0)
        .show(open)
        .on_close(close)
        .dismiss_on_escape()
        .into()
}

impl<C> Default for State<C> {
    fn default() -> Self {
        Self {
            query: String::new(),
            menu: None,
            sync: Default::default(),
            templates: Default::default(),
            shared_import: Default::default(),
            transfer: Default::default(),
            pi_files: Default::default(),
            scope: Default::default(),
            generation: Default::default(),
            loading: Default::default(),
            catalog: Default::default(),
            preview: Default::default(),
            error: Default::default(),
            preview_error: None,
            mutating: Default::default(),
            undo: Default::default(),
            editor: Default::default(),
        }
    }
}
