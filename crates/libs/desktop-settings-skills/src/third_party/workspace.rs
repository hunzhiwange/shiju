//! Scoped management workspace. Request generations are independent per source.
use super::{Action, DirectoryAction, Message, State as Page, style};
use desktop_assets::assets::Icon;
use desktop_ui_controls::{
    overlays::BelowOverlay,
    single_line_label::SingleLineLabel,
    system_settings_common::{
        help::settings_hover_help_label, icons::icon_btn_svg_style,
        tabs::settings_compact_tab_button, *,
    },
};
use iced::{
    Element, Length,
    widget::{Space, button, checkbox, column, container, row, rule, scrollable, text, tooltip},
};
use vw_api_app_skills::{
    bindings_api::SkillBindingTarget, discovery_api::*, inventory_api::*, repos_api::*,
    search_api::*,
};
use vw_api_types::agent_apps::AppId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Installed,
    Repositories,
    Market,
}
const DISCOVERY_BATCH_SIZE: usize = 100;

#[derive(Clone, Debug)]
pub struct Candidate {
    pub name: String,
    pub description: String,
    pub directory: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub url: Option<String>,
}
impl Candidate {
    pub fn repo_id(&self) -> String {
        format!("{}:{}", self.owner, self.repo)
    }
    pub fn destination(&self) -> &str {
        self.directory.rsplit('/').next().unwrap_or(&self.directory)
    }
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub tab: Tab,
    pub add_menu: bool,
    pub more_menu: bool,
    pub source_menu: bool,
    pub target_menu: bool,
    pub row_menu: Option<String>,
    pub query: String,
    pub installed_source: Option<String>,
    pub market_query: String,
    pub offset: usize,
    pub repositories: Option<SkillReposResponse>,
    pub inventory: Option<SkillInventoryResponse>,
    pub discovery: Option<DiscoverSkillsResponse>,
    additional_discovery_rows: usize,
    pub market: Option<MarketSearchResponse>,
    pub excluded_repos: Vec<String>,
    pub uninstalled: bool,
    pub busy: [bool; 3],
    pub errors: [Option<String>; 3],
    generation: [u64; 3],
    pub detail: Option<String>,
    pub pending: Option<Candidate>,
    pub installing: bool,
    pub notice: Option<String>,
    pub importable: Option<usize>,
    hide_import_notice: bool,
}
#[derive(Clone, Debug)]
pub enum Event {
    AddMenu,
    MoreMenu,
    SourceMenu,
    TargetMenu,
    RowMenu(String),
    CloseMenus,
    CloseDetail,
    HideImportNotice,
    ReloadInventory,
    Tab(Tab),
    Query(String),
    InstalledSource(Option<String>),
    MarketQuery(String),
    Repository(String, bool),
    Uninstalled(bool),
    LoadMoreDiscovery,
    Refresh,
    Search(usize),
    Loaded(u64, Result<(SkillReposResponse, SkillInventoryResponse), String>),
    Discovered(u64, Result<DiscoverSkillsResponse, String>),
    Searched(u64, Result<MarketSearchResponse, String>),
    Detail(String),
    OpenDirectory(String),
    DirectoryOpened(Result<(), String>),
    Uninstall(String),
    Documentation(String),
    Install(Candidate),
    ConfirmRepository,
    CancelRepository,
    Installed(u64, Result<String, String>),
}
pub enum Effect {
    None,
    Documentation(String),
    OpenDirectory(String),
    Uninstall(String),
    Error(String),
    Load(u64),
    Discover(u64, bool),
    Search(u64, String, usize),
    Install {
        generation: u64,
        candidate: Candidate,
        register: bool,
        inventory_version: u64,
        repository_version: u64,
    },
}
impl State {
    pub(super) fn close_menus(&mut self) {
        self.add_menu = false;
        self.more_menu = false;
        self.source_menu = false;
        self.target_menu = false;
        self.row_menu = None;
    }

    fn begin(&mut self, slot: usize) -> u64 {
        self.generation[slot] = self.generation[slot].wrapping_add(1);
        self.busy[slot] = true;
        self.errors[slot] = None;
        self.generation[slot]
    }
    pub fn installed(&self, c: &Candidate) -> Option<bool> {
        let inventory = self.inventory.as_ref()?;
        let item = inventory.skills.iter().find(|s| s.id.eq_ignore_ascii_case(c.destination()))?;
        let source = item.source.as_ref()?;
        Some(
            item.present
                && source == &format!("{}/{} · {} · {}", c.owner, c.repo, c.branch, c.directory),
        )
    }
    pub fn conflict(&self, c: &Candidate) -> bool {
        self.inventory.as_ref().is_some_and(|data| {
            data.skills.iter().any(|s| s.id.eq_ignore_ascii_case(c.destination()))
        }) && self.installed(c) != Some(true)
    }
    pub fn registered(&self, c: &Candidate) -> bool {
        self.repositories.as_ref().is_some_and(|data| {
            data.repos.iter().any(|r| {
                r.owner == c.owner && r.name == c.repo && r.branch == c.branch && r.enabled
            })
        })
    }
}
pub fn update(s: &mut State, event: Event, minimum_version: u64) -> Effect {
    if s.installing
        && matches!(
            event,
            Event::Refresh | Event::Tab(_) | Event::Install(_) | Event::ConfirmRepository
        )
    {
        return Effect::None;
    }
    let open_directory = matches!(event, Event::OpenDirectory(_));
    match event {
        Event::AddMenu => {
            let open = !s.add_menu;
            s.close_menus();
            s.add_menu = open;
        }
        Event::MoreMenu => {
            let open = !s.more_menu;
            s.close_menus();
            s.more_menu = open;
        }
        Event::SourceMenu => {
            let open = !s.source_menu;
            s.close_menus();
            s.source_menu = open;
        }
        Event::TargetMenu => {
            let open = !s.target_menu;
            s.close_menus();
            s.target_menu = open;
        }
        Event::RowMenu(id) => {
            let open = s.row_menu.as_ref() != Some(&id);
            s.close_menus();
            s.row_menu = open.then_some(id);
        }
        Event::CloseMenus => s.close_menus(),
        Event::CloseDetail => s.detail = None,
        Event::HideImportNotice => s.hide_import_notice = true,
        Event::ReloadInventory if !s.installing => return Effect::Load(s.begin(0)),
        Event::Tab(tab) => {
            s.close_menus();
            s.detail = None;
            if s.tab != tab {
                s.additional_discovery_rows = 0;
            }
            s.tab = tab;
            if s.inventory.is_none() && !s.busy[0] {
                return Effect::Load(s.begin(0));
            }
            if tab == Tab::Repositories && s.discovery.is_none() && !s.busy[1] {
                return Effect::Discover(s.begin(1), false);
            }
        }
        Event::Detail(id) => {
            s.close_menus();
            s.detail = Some(id);
        }
        Event::OpenDirectory(id) | Event::Uninstall(id) => {
            s.close_menus();
            if s.inventory.as_ref().is_some_and(|inventory| {
                inventory.skills.iter().any(|skill| skill.present && skill.id == id)
            }) {
                return if open_directory {
                    Effect::OpenDirectory(id)
                } else {
                    Effect::Uninstall(id)
                };
            }
        }
        Event::DirectoryOpened(Err(error)) => return Effect::Error(error),
        Event::DirectoryOpened(Ok(())) => {}
        Event::Documentation(url) => {
            let known = s
                .discovery
                .as_ref()
                .is_some_and(|d| d.skills.iter().any(|c| c.readme_url.as_ref() == Some(&url)))
                || s.market
                    .as_ref()
                    .is_some_and(|d| d.skills.iter().any(|c| c.readme_url.as_ref() == Some(&url)));
            if known {
                return Effect::Documentation(url);
            }
        }
        Event::Query(q) => {
            s.query = q;
            s.additional_discovery_rows = 0;
        }
        Event::InstalledSource(source) => {
            s.installed_source = source;
            s.close_menus();
        }
        Event::MarketQuery(q) => {
            s.market_query = q;
            s.generation[2] = s.generation[2].wrapping_add(1);
            s.busy[2] = false;
            s.market = None;
            s.offset = 0;
        }
        Event::Repository(id, enabled) => {
            s.additional_discovery_rows = 0;
            s.excluded_repos.retain(|r| r != &id);
            if !enabled {
                s.excluded_repos.push(id);
            }
        }
        Event::Uninstalled(v) => {
            s.uninstalled = v;
            s.additional_discovery_rows = 0;
        }
        Event::LoadMoreDiscovery if s.tab == Tab::Repositories => {
            s.additional_discovery_rows =
                s.additional_discovery_rows.saturating_add(DISCOVERY_BATCH_SIZE);
        }
        Event::Refresh => {
            return match s.tab {
                Tab::Installed => Effect::Load(s.begin(0)),
                Tab::Repositories => {
                    s.additional_discovery_rows = 0;
                    Effect::Discover(s.begin(1), true)
                }
                Tab::Market => return update(s, Event::Search(s.offset), minimum_version),
            };
        }
        Event::Search(offset) => {
            if offset > 10_000
                || s.market_query.trim().is_empty()
                || s.market_query.len() > 512
                || s.market_query.chars().any(char::is_control)
            {
                return Effect::None;
            }
            s.offset = offset;
            return Effect::Search(s.begin(2), s.market_query.trim().into(), offset);
        }
        Event::Loaded(g, result) if g == s.generation[0] && s.busy[0] => {
            s.busy[0] = false;
            match result {
                Ok((repos, inventory)) => {
                    s.repositories = Some(repos);
                    s.inventory = Some(inventory);
                    if s.tab == Tab::Repositories {
                        return Effect::Discover(s.begin(1), false);
                    }
                }
                Err(e) => s.errors[0] = Some(e),
            }
        }
        Event::Discovered(g, result) if g == s.generation[1] && s.busy[1] => {
            s.busy[1] = false;
            match result {
                Ok(data)
                    if data.version
                        >= minimum_version
                            .max(s.repositories.as_ref().map_or(0, |r| r.version)) =>
                {
                    s.discovery = Some(data)
                }
                Ok(_) => s.errors[1] = Some("仓库已变更，请刷新".into()),
                Err(e) => s.errors[1] = Some(e),
            }
        }
        Event::Searched(g, result) if g == s.generation[2] && s.busy[2] => {
            s.busy[2] = false;
            match result {
                Ok(data) if data.query == s.market_query.trim() => s.market = Some(data),
                Ok(_) => s.errors[2] = Some("查询已变化，请重试".into()),
                Err(e) => s.errors[2] = Some(e),
            }
        }
        Event::Install(c)
            if !s.installing && !s.busy[0] && s.inventory.is_some() && s.repositories.is_some() =>
        {
            if s.conflict(&c) || s.installed(&c) == Some(true) {
                return Effect::None;
            }
            if !s.registered(&c) {
                s.pending = Some(c);
                return Effect::None;
            }
            return install(s, c, false);
        }
        Event::ConfirmRepository if !s.installing => {
            if let Some(c) = s.pending.take() {
                return install(s, c, true);
            }
        }
        Event::CancelRepository => s.pending = None,
        Event::Installed(g, result) if s.installing && g == s.generation[0] => {
            s.installing = false;
            match result {
                Ok(notice) => s.notice = Some(notice),
                Err(e) => s.notice = Some(e),
            }
            return Effect::Load(s.begin(0));
        }
        _ => {}
    }
    Effect::None
}
fn install(s: &mut State, candidate: Candidate, register: bool) -> Effect {
    let (Some(inventory), Some(repos)) = (&s.inventory, &s.repositories) else {
        return Effect::None;
    };
    let (inventory_version, repository_version) = (inventory.version, repos.version);
    s.installing = true;
    s.notice = Some("正在安装；完成后才分发到所选应用".into());
    s.generation[0] = s.generation[0].wrapping_add(1);
    Effect::Install {
        generation: s.generation[0],
        candidate,
        register,
        inventory_version,
        repository_version,
    }
}

pub fn view<'a>(
    page: &'a Page,
    target_label: String,
    targets: Element<'a, Message>,
    bindings: Option<Element<'a, Message>>,
    apps: &[(SkillBindingTarget, String)],
    icon: fn(AppId, f32) -> Element<'static, Message>,
) -> Element<'a, Message> {
    let Some(scope) = page.scope() else {
        return container(text("请刷新应用目录"))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };
    let s = &page.workspace;
    let installed = s.tab == Tab::Installed;
    let send = |event| Message {
        scope: scope.clone(),
        catalog: page.catalog,
        action: Action::Workspace(event),
    };
    let directory = |action| Message {
        scope: scope.clone(),
        catalog: page.catalog,
        action: Action::Directory(action),
    };
    let action = |label: &'static str, event| control(label, send(event));
    let close = send(Event::CloseMenus);
    let count = s.inventory.as_ref().map(|i| i.skills.iter().filter(|s| s.present).count());
    let add = menu(
        button(
            row![
                primary_icon(Icon::Plus, 16.0),
                text("添加 Skill").size(14).align_y(iced::alignment::Vertical::Center),
                primary_icon(Icon::ChevronDown, 11.0),
            ]
            .height(Length::Fill)
            .spacing(7)
            .align_y(iced::Alignment::Center),
        )
        .height(HEADER_CONTROL_HEIGHT)
        .padding([0, 12])
        .style(style::primary)
        .on_press(send(Event::AddMenu)),
        column![
            menu_item("在发现里浏览", send(Event::Tab(Tab::Repositories))),
            menu_item("从 ZIP 安装", directory(DirectoryAction::Install)),
            menu_item("导入已有技能", directory(DirectoryAction::Import)),
        ],
        s.add_menu,
        close.clone(),
        176,
    );
    let mut more_items = column![];
    for (label, entry) in [
        ("管理技能仓库", DirectoryAction::Repositories),
        ("存储与同步", DirectoryAction::Storage),
        ("从备份中恢复", DirectoryAction::Backup),
        ("更新与卸载", DirectoryAction::Updates),
        ("任务进度", DirectoryAction::Jobs),
        ("Claude 配置", DirectoryAction::Claude),
    ] {
        more_items = more_items.push(menu_item(label, directory(entry)));
    }
    let more = menu(
        more_button(send(Event::MoreMenu)).height(HEADER_CONTROL_HEIGHT),
        more_items,
        s.more_menu,
        close.clone(),
        166,
    );
    let header = row![
        icon_svg(Icon::Journals, 20.0).style(icon_btn_svg_style),
        settings_hover_help_label(
            text("Skills").size(23),
            text("安装技能后，可以选择启用它的应用实例。").size(13).into(),
        ),
        Space::new().width(Length::Fill),
        add,
        more,
    ]
    .spacing(9)
    .align_y(iced::Alignment::Center);
    let mut tabs = row![
        tab(
            count.map_or_else(|| "已安装".to_owned(), |count| format!("已安装 {count}")),
            installed,
            send(Event::Tab(Tab::Installed)),
        ),
        tab("发现".into(), !installed, send(Event::Tab(Tab::Repositories))),
        Space::new().width(Length::Fill),
    ]
    .spacing(9)
    .align_y(iced::Alignment::Center);
    if !installed || count != Some(0) {
        let search = match s.tab {
            Tab::Installed => settings_text_input("搜索名称、目录、描述或仓库", &s.query)
                .on_input(move |query| send(Event::Query(query))),
            Tab::Repositories => settings_text_input("搜索名称、描述或仓库", &s.query)
                .on_input(move |query| send(Event::Query(query))),
            Tab::Market => settings_text_input("搜索 skills.sh", &s.market_query)
                .on_input(move |query| send(Event::MarketQuery(query)))
                .on_submit(send(Event::Search(0))),
        };
        tabs = tabs
            .push(container(style::search(search)).width(Length::FillPortion(2)).max_width(280));
        tabs = tabs.push(match s.tab {
            Tab::Installed => toolbar_action("检查更新", directory(DirectoryAction::Updates)),
            Tab::Repositories => toolbar_action("刷新仓库", send(Event::Refresh)),
            Tab::Market => action("搜索", Event::Search(0)),
        });
    }

    let mut page_body = column![header, rule::horizontal(1), tabs, rule::horizontal(1)]
        .spacing(8)
        .height(Length::Fill);
    for error in [
        s.errors[0].as_ref(),
        if installed {
            None
        } else if s.tab == Tab::Market {
            s.errors[2].as_ref()
        } else {
            s.errors[1].as_ref()
        },
    ]
    .into_iter()
    .flatten()
    {
        page_body = page_body.push(
            row![
                settings_error_banner(error.clone()),
                action(
                    "重试",
                    if s.errors[0].as_ref() == Some(error) {
                        Event::ReloadInventory
                    } else {
                        Event::Refresh
                    }
                ),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        );
    }
    if let Some(notice) = &s.notice {
        page_body = page_body.push(text(notice).size(13).style(settings_muted_text_style));
    }
    if s.installing {
        page_body = page_body.push(text("正在安装技能…").size(13));
    }
    if let Some(c) = &s.pending {
        page_body = page_body.push(
            container(
                row![
                    text(format!("登记或启用 {}/{} 后安装？", c.owner, c.repo)).width(Length::Fill),
                    action("确认并安装", Event::ConfirmRepository),
                    action("取消", Event::CancelRepository),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding(10)
            .style(settings_section_card_style),
        );
    }

    if installed && count.is_some_and(|count| count > 0) && !s.hide_import_notice {
        if let Some(importable) = s.importable.filter(|count| *count > 0) {
            page_body = page_body.push(
                container(
                    row![
                        text(format!("发现 {importable} 个可以导入的 Skill"))
                            .size(13)
                            .width(Length::Fill),
                        control("查看并导入", directory(DirectoryAction::Import)),
                        action("忽略", Event::HideImportNotice),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                )
                .padding([8, 12])
                .style(settings_section_card_style),
            );
        }
    }
    let mut entries = Vec::new();
    if installed {
        let mut installed_sources = std::collections::BTreeSet::new();
        if let Some(data) = &s.inventory {
            for skill in data.skills.iter().filter(|skill| skill.present) {
                let provenance = skill.source.as_deref().unwrap_or("来源未知");
                let source = provenance.split(" · ").next().unwrap_or(provenance);
                installed_sources.insert(source.to_owned());
                if s.installed_source.as_deref().is_some_and(|selected| selected != source) {
                    continue;
                }
                let description = skill.description.as_deref().unwrap_or("");
                if !matches_query(&s.query, &[&skill.name, &skill.id, description, provenance]) {
                    continue;
                }
                let applications = Message {
                    scope: scope.clone(),
                    catalog: page.catalog,
                    action: Action::Binding(crate::update::bindings_view::Event::Select(
                        skill.id.clone(),
                    )),
                };
                let row_menu = menu(
                    more_button(send(Event::RowMenu(skill.id.clone()))),
                    column![
                        menu_item("查看文档", send(Event::Detail(skill.id.clone()))),
                        menu_item("打开目录", send(Event::OpenDirectory(skill.id.clone()))),
                        menu_item("管理应用绑定", applications),
                        rule::horizontal(1),
                        button(text("卸载").size(13))
                            .width(Length::Fill)
                            .padding([7, 9])
                            .style(danger_action_btn_style)
                            .on_press(send(Event::Uninstall(skill.id.clone()))),
                    ],
                    s.row_menu.as_ref() == Some(&skill.id),
                    close.clone(),
                    158,
                );
                let mut controls = row![].align_y(iced::Alignment::Center);
                for (target, name) in apps {
                    let state = page.binding_status(target, &skill.id);
                    let cell: Element<'_, Message> = if let Some(state) = state {
                        let id = skill.id.clone();
                        let target = target.clone();
                        checkbox(state)
                            .size(15)
                            .style(style::checkbox)
                            .on_toggle_maybe((!page.bindings.busy).then_some(move |enabled| {
                                Message {
                                    scope: scope.clone(),
                                    catalog: page.catalog,
                                    action: Action::Binding(
                                        crate::update::bindings_view::Event::BindTarget(
                                            id.clone(),
                                            target.clone(),
                                            enabled,
                                        ),
                                    ),
                                }
                            }))
                            .into()
                    } else {
                        text("–").size(13).style(settings_muted_text_style).into()
                    };
                    controls = controls.push(
                        container(
                            tooltip(cell, text(name.clone()).size(12), tooltip::Position::Top)
                                .style(style::popover)
                                .padding(5),
                        )
                        .center_x(APP_COLUMN_WIDTH),
                    );
                }
                entries.push(skill_row(&skill.name, source, description, controls.push(row_menu)));
            }
        }
        if let Some(bindings) = bindings {
            page_body = page_body.push(bindings);
        }
        if let Some(detail) = s
            .detail
            .as_ref()
            .and_then(|id| s.inventory.as_ref()?.skills.iter().find(|c| &c.id == id))
        {
            let dismiss = send(Event::CloseDetail);
            let panel = settings_modal_layout(
                settings_modal_header(
                    text(detail.name.clone()).size(19),
                    settings_close_button(dismiss.clone()),
                ),
                text(detail.markdown.clone().unwrap_or_else(|| "暂无详情".into())),
                None,
            )
            .style(style::dialog)
            .width(580)
            .height(430);
            page_body = page_body.push(list(entries, None));
            return settings_modal_overlay_with_escape_and_backdrop_dismiss(
                page_body.into(),
                dismiss,
                panel,
            );
        }
        if entries.is_empty() && (count.is_none() || count == Some(0)) {
            let (title, detail) = match count {
                Some(0) => ("还没有 Skill", "可以去发现安装，或导入已有的技能。"),
                Some(_) => ("没有匹配的技能", "试试其他名称、描述或来源。"),
                None if s.busy[0] => ("正在读取技能…", ""),
                None => ("技能列表暂不可用", ""),
            };
            let mut empty =
                column![text(title).size(18)].spacing(10).align_x(iced::Alignment::Center);
            if !detail.is_empty() {
                empty = empty.push(text(detail).size(13).style(settings_muted_text_style));
            }
            if count == Some(0) {
                empty = empty.push(
                    row![
                        action("去发现", Event::Tab(Tab::Repositories)),
                        control(
                            s.importable.map_or_else(
                                || "导入已有技能".to_owned(),
                                |count| format!("导入已有技能（{count}）")
                            ),
                            directory(DirectoryAction::Import)
                        ),
                    ]
                    .spacing(8),
                );
            }
            return page_body
                .push(container(empty).center_x(Length::Fill).center_y(Length::Fill))
                .into();
        }
        let source_picker = menu(
            dropdown_trigger(
                s.installed_source.as_ref().map_or_else(
                    || "来源 · 全部".to_owned(),
                    |source| format!("来源 · {}", shorten_label(source, 16)),
                ),
                s.source_menu,
                send(Event::SourceMenu),
            ),
            scrollable(column(
                std::iter::once(menu_item("全部来源", send(Event::InstalledSource(None))).into())
                    .chain(installed_sources.into_iter().map(|source| {
                        menu_item(source.clone(), send(Event::InstalledSource(Some(source)))).into()
                    })),
            ))
            .direction(scrollbar())
            .height(Length::Shrink),
            s.source_menu,
            close.clone(),
            260,
        );
        if entries.is_empty() {
            entries.push(
                container(text("没有匹配的技能").size(13).style(settings_muted_text_style))
                    .center_x(Length::Fill)
                    .padding(24)
                    .into(),
            );
        }
        let mut table_header =
            row![source_picker, Space::new().width(Length::Fill),].align_y(iced::Alignment::Center);
        for (target, name) in apps {
            let enabled_count = s.inventory.as_ref().and_then(|inventory| {
                inventory.skills.iter().filter(|skill| skill.present).try_fold(0, |total, skill| {
                    page.binding_status(target, &skill.id)
                        .map(|enabled| total + if enabled { 1 } else { 0 })
                })
            });
            let heading = column![
                icon(target.app_id, 16.0),
                text(enabled_count.map_or_else(|| "–".to_owned(), |count| count.to_string()))
                    .size(11)
                    .style(settings_muted_text_style),
            ]
            .spacing(2)
            .align_x(iced::Alignment::Center);
            table_header = table_header.push(
                container(
                    tooltip(heading, text(name.clone()).size(12), tooltip::Position::Top)
                        .style(style::popover)
                        .padding(5),
                )
                .center_x(APP_COLUMN_WIDTH),
            );
        }
        let table_header = table_header
            .push(Space::new().width(32))
            .padding(iced::Padding::from([6, 19]).right(23));
        return page_body.push(list(entries, Some(table_header.into()))).into();
    }

    if s.tab == Tab::Repositories {
        if let Some(data) = s.discovery.as_ref().filter(|data| !data.failures.is_empty()) {
            let names = data
                .failures
                .iter()
                .map(|failure| format!("{}/{}", failure.owner, failure.name))
                .collect::<Vec<_>>()
                .join("、");
            page_body = page_body.push(
                container(
                    row![
                        column![
                            text(format!("{} 个仓库读取失败", data.failures.len())).size(14),
                            text(names).size(12).style(settings_muted_text_style),
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                        action("重试", Event::Refresh),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                )
                .padding([10, 12])
                .style(|theme| {
                    let mut style = settings_section_card_style(theme);
                    style.background =
                        Some(theme.extended_palette().warning.base.color.scale_alpha(0.16).into());
                    style
                }),
            );
        }
    }
    let mut filters = row![
        settings_compact_tab_button(
            "仓库",
            s.tab == Tab::Repositories,
            send(Event::Tab(Tab::Repositories))
        ),
        settings_compact_tab_button(
            "skills.sh",
            s.tab == Tab::Market,
            send(Event::Tab(Tab::Market))
        ),
    ]
    .spacing(7)
    .align_y(iced::Alignment::Center);
    if s.tab == Tab::Repositories {
        let mut sources = column![];
        if let Some(repos) = &s.repositories {
            for repo in repos.repos.iter().filter(|repo| repo.enabled) {
                let id = format!("{}:{}", repo.owner, repo.name);
                let selected = !s.excluded_repos.contains(&id);
                let status =
                    s.discovery.as_ref().filter(|data| data.version == repos.version).map(|data| {
                        if data.failures.iter().any(|failure| {
                            failure.owner == repo.owner
                                && failure.name == repo.name
                                && failure.branch == repo.branch
                        }) {
                            "读取失败".to_owned()
                        } else {
                            data.skills
                                .iter()
                                .filter(|skill| {
                                    skill.repo_owner == repo.owner
                                        && skill.repo_name == repo.name
                                        && skill.repo_branch == repo.branch
                                })
                                .count()
                                .to_string()
                        }
                    });
                sources = sources.push(
                    row![
                        checkbox(selected)
                            .size(15)
                            .style(style::checkbox)
                            .on_toggle(move |value| send(Event::Repository(id.clone(), value))),
                        Element::from(SingleLineLabel::new(
                            format!("{}/{}", repo.owner, repo.name),
                            13.0
                        )),
                        text(repo.branch.clone()).size(12).style(settings_muted_text_style),
                        text(status.unwrap_or_else(|| "—".to_owned()))
                            .size(12)
                            .style(settings_muted_text_style),
                    ]
                    .padding([5, 6])
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                );
            }
        }
        sources = sources
            .push(rule::horizontal(1))
            .push(Space::new().height(4))
            .push(menu_item("添加或管理仓库", directory(DirectoryAction::Repositories)));
        let source_label = if s.excluded_repos.is_empty() {
            "来源 · 全部".to_owned()
        } else {
            format!(
                "来源 · {} 个",
                s.repositories.as_ref().map_or(0, |repos| {
                    repos
                        .repos
                        .iter()
                        .filter(|repo| {
                            repo.enabled
                                && !s
                                    .excluded_repos
                                    .contains(&format!("{}:{}", repo.owner, repo.name))
                        })
                        .count()
                })
            )
        };
        filters = filters.push(menu(
            dropdown_trigger(source_label, s.source_menu, send(Event::SourceMenu)),
            scrollable(sources).direction(scrollbar()).height(Length::Shrink),
            s.source_menu,
            close.clone(),
            320,
        ));
    }
    filters = filters.push(
        container(
            checkbox(s.uninstalled)
                .label("只看未安装")
                .size(15)
                .spacing(8)
                .text_size(13)
                .style(style::checkbox)
                .on_toggle(move |value| send(Event::Uninstalled(value))),
        )
        .center_y(FILTER_CONTROL_HEIGHT),
    );
    let filters = container(
        row![
            filters.width(Length::Fill).wrap(),
            menu(
                dropdown_trigger(target_label, s.target_menu, send(Event::TargetMenu)),
                targets,
                s.target_menu,
                close,
                328
            ),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 8])
    .width(Length::Fill);
    let repository_tab = s.tab == Tab::Repositories;
    let discovery_limit = DISCOVERY_BATCH_SIZE.saturating_add(s.additional_discovery_rows);
    let repository_candidates = s
        .discovery
        .as_ref()
        .filter(|_| repository_tab)
        .into_iter()
        .flat_map(|data| &data.skills)
        .filter(|skill| {
            let repo = format!("{}:{}", skill.repo_owner, skill.repo_name);
            !s.excluded_repos.contains(&repo)
                && matches_query(&s.query, &[&skill.name, &skill.description, &repo])
        })
        .map(|skill| Candidate {
            name: skill.name.clone(),
            description: skill.description.clone(),
            directory: skill.directory.clone(),
            owner: skill.repo_owner.clone(),
            repo: skill.repo_name.clone(),
            branch: skill.repo_branch.clone(),
            url: skill.readme_url.clone(),
        });
    let market_candidates = s
        .market
        .as_ref()
        .filter(|_| !repository_tab)
        .into_iter()
        .flat_map(|data| &data.skills)
        .map(|skill| Candidate {
            name: skill.name.clone(),
            description: format!("{} 次安装", skill.installs),
            directory: skill.directory.clone(),
            owner: skill.repo_owner.clone(),
            repo: skill.repo_name.clone(),
            branch: skill.repo_branch.clone(),
            url: skill.readme_url.clone(),
        });
    let mut has_more = false;
    for candidate in repository_candidates.chain(market_candidates) {
        let installed = s.installed(&candidate) == Some(true);
        if s.uninstalled && installed {
            continue;
        }
        if repository_tab && entries.len() == discovery_limit {
            has_more = true;
            break;
        }
        let conflict = s.conflict(&candidate);
        let mut controls = row![
            button(
                text(if installed {
                    "已安装"
                } else if conflict {
                    "来源冲突"
                } else {
                    "安装"
                })
                .size(13)
            )
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (!s.installing && !s.busy[0] && !conflict && !installed)
                    .then(|| send(Event::Install(candidate.clone())))
            )
        ]
        .spacing(6);
        if let Some(url) = &candidate.url {
            controls = controls.push(action("文档", Event::Documentation(url.clone())));
        }
        entries.push(skill_row(
            &candidate.name,
            &format!("{}/{}", candidate.owner, candidate.repo),
            &candidate.description,
            controls,
        ));
    }
    if entries.is_empty() {
        let status = if s.busy.iter().any(|busy| *busy) {
            "正在读取技能…"
        } else if s.tab == Tab::Market && s.market_query.is_empty() {
            "输入关键词搜索 skills.sh"
        } else {
            "没有找到匹配的技能"
        };
        return page_body
            .push(filters.style(settings_section_card_style))
            .push(
                container(text(status).size(14).style(settings_muted_text_style))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .into();
    }
    if has_more {
        entries.push(
            container(action("加载更多", Event::LoadMoreDiscovery))
                .center_x(Length::Fill)
                .padding([10, 0])
                .into(),
        );
    }
    if s.tab == Tab::Market {
        entries.push(
            row![
                button("上一页").padding([6, 10]).style(rounded_action_btn_style).on_press_maybe(
                    (s.offset >= 50 && !s.busy[2])
                        .then(|| send(Event::Search(s.offset.saturating_sub(50))))
                ),
                button("下一页").padding([6, 10]).style(rounded_action_btn_style).on_press_maybe(
                    (!s.busy[2]
                        && s.offset < 10_000
                        && s.market
                            .as_ref()
                            .is_some_and(|market| s.offset + 50 < market.total_count))
                    .then(|| send(Event::Search(s.offset + 50)))
                ),
            ]
            .spacing(8)
            .padding(8)
            .into(),
        );
    }
    page_body.push(list(entries, Some(filters.into()))).into()
}

const APP_COLUMN_WIDTH: u32 = 28;
const HEADER_CONTROL_HEIGHT: u32 = 36;
const FILTER_CONTROL_HEIGHT: u32 = 30;

fn primary_icon(icon: Icon, size: f32) -> iced::widget::Svg<'static> {
    icon_svg(icon, size).style(|theme, _| iced::widget::svg::Style {
        color: Some(style::primary(theme, button::Status::Active).text_color),
    })
}

fn dropdown_trigger<'a>(
    label: impl text::IntoFragment<'a>,
    active: bool,
    event: Message,
) -> button::Button<'a, Message> {
    button(
        row![
            text(label).size(13).align_y(iced::alignment::Vertical::Center),
            icon_svg(Icon::ChevronDown, 10.0).style(icon_btn_svg_style),
        ]
        .height(Length::Fill)
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .height(FILTER_CONTROL_HEIGHT)
    .padding([0, 9])
    .style(move |theme, status| settings_tab_style(theme, status, active))
    .on_press(event)
}

fn toolbar_action<'a>(label: &'static str, event: Message) -> button::Button<'a, Message> {
    button(
        row![icon_svg(Icon::ArrowClockwise, 16.0).style(icon_btn_svg_style), text(label).size(13),]
            .spacing(5)
            .align_y(iced::Alignment::Center),
    )
    .padding([7, 8])
    .style(|theme, status| settings_tab_style(theme, status, false))
    .on_press(event)
}

fn more_button(event: Message) -> button::Button<'static, Message> {
    button(text("⋯").size(17).width(Length::Fill).height(Length::Fill).center())
        .width(32)
        .height(30)
        .padding(0)
        .style(|theme, status| settings_tab_style(theme, status, false))
        .on_press(event)
}

fn control<'a>(label: impl text::IntoFragment<'a>, event: Message) -> button::Button<'a, Message> {
    button(text(label).size(13)).padding([6, 10]).style(rounded_action_btn_style).on_press(event)
}

fn menu_item<'a>(
    label: impl text::IntoFragment<'a>,
    event: Message,
) -> button::Button<'a, Message> {
    button(text(label).size(13).height(Length::Fill).align_y(iced::alignment::Vertical::Center))
        .width(Length::Fill)
        .height(FILTER_CONTROL_HEIGHT)
        .padding([0, 9])
        .style(|theme, status| {
            let mut style = settings_tab_style(theme, status, false);
            style.text_color = theme.palette().text;
            style
        })
        .on_press(event)
}

fn menu<'a>(
    trigger: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    open: bool,
    close: Message,
    width: u32,
) -> Element<'a, Message> {
    BelowOverlay::new(
        trigger,
        container(content).padding(6).width(width).max_height(320).style(style::popover),
    )
    .gap(4.0)
    .show(open)
    .on_close(close)
    .dismiss_on_escape()
    .into()
}

fn tab(label: String, active: bool, event: Message) -> Element<'static, Message> {
    column![
        button(text(label).size(14))
            .padding([7, 10])
            .style(move |theme, status| {
                let mut style = settings_tab_style(theme, status, false);
                if active {
                    style.text_color = theme.palette().text;
                }
                style
            })
            .on_press(event),
        container(Space::new().width(Length::Fill).height(2)).style(move |theme: &iced::Theme| {
            container::Style {
                background: active.then(|| theme.palette().text.into()),
                border: iced::Border { radius: 1.0.into(), ..Default::default() },
                ..Default::default()
            }
        }),
    ]
    .width(Length::Shrink)
    .into()
}

fn skill_row<'a>(
    name: &str,
    source: &str,
    description: &str,
    controls: iced::widget::Row<'a, Message>,
) -> Element<'a, Message> {
    let muted = |label: &str| {
        text(label.to_owned())
            .size(12)
            .style(settings_muted_text_style)
            .wrapping(text::Wrapping::None)
    };
    let labels = container(
        column![
            row![
                container(SingleLineLabel::new(name.to_owned(), 14.0)).max_width(215),
                muted(source),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            muted(description),
        ]
        .spacing(3),
    )
    .width(Length::Fill)
    .clip(true);
    column![
        row![labels, controls].spacing(10).align_y(iced::Alignment::Center).padding([9, 12]),
        rule::horizontal(1),
    ]
    .into()
}

fn scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4))
}

fn list<'a>(
    entries: Vec<Element<'a, Message>>,
    header: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut table = column![];
    if let Some(header) = header {
        table = table.push(container(header).style(style::toolbar)).push(rule::horizontal(1));
    }
    container(
        table.push(
            scrollable(container(column(entries)).padding([0, 7]))
                .direction(scrollbar())
                .height(Length::Shrink),
        ),
    )
    .width(Length::Fill)
    .clip(true)
    .style(style::table)
    .into()
}

fn shorten_label(value: &str, length: usize) -> String {
    let Some((end, _)) = value.char_indices().nth(length) else {
        return value.to_owned();
    };
    format!("{}…", &value[..end])
}

fn matches_query(query: &str, values: &[&str]) -> bool {
    let q = query.trim().to_lowercase();
    values.iter().any(|v| v.to_lowercase().contains(&q))
}
