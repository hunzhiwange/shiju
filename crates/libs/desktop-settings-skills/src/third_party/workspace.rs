//! Scoped management workspace. Request generations are independent per source.
use super::{Action, DirectoryAction, Message, State as Page};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};
use vw_api_app_skills::{discovery_api::*, inventory_api::*, repos_api::*, search_api::*};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Installed,
    Repositories,
    Market,
}
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
    pub query: String,
    pub market_query: String,
    pub offset: usize,
    pub repositories: Option<SkillReposResponse>,
    pub inventory: Option<SkillInventoryResponse>,
    pub discovery: Option<DiscoverSkillsResponse>,
    pub market: Option<MarketSearchResponse>,
    pub selected_repos: Vec<String>,
    pub uninstalled: bool,
    pub busy: [bool; 3],
    pub errors: [Option<String>; 3],
    generation: [u64; 3],
    pub detail: Option<String>,
    pub pending: Option<Candidate>,
    pub installing: bool,
    pub notice: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Event {
    AddMenu,
    MoreMenu,
    ReloadInventory,
    Tab(Tab),
    Query(String),
    MarketQuery(String),
    Repository(String, bool),
    Uninstalled(bool),
    Refresh,
    Search(usize),
    Loaded(u64, Result<(SkillReposResponse, SkillInventoryResponse), String>),
    Discovered(u64, Result<DiscoverSkillsResponse, String>),
    Searched(u64, Result<MarketSearchResponse, String>),
    Detail(String),
    Documentation(String),
    Install(Candidate),
    ConfirmRepository,
    CancelRepository,
    Installed(u64, Result<String, String>),
}
pub enum Effect {
    None,
    Documentation(String),
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
    match event {
        Event::AddMenu => { s.add_menu = !s.add_menu; s.more_menu = false; }
        Event::MoreMenu => { s.more_menu = !s.more_menu; s.add_menu = false; }
        Event::ReloadInventory if !s.installing => return Effect::Load(s.begin(0)),
        Event::Tab(tab) => {
            s.add_menu = false;
            s.more_menu = false;
            s.tab = tab;
            if s.inventory.is_none() && !s.busy[0] {
                return Effect::Load(s.begin(0));
            }
            if tab == Tab::Repositories && s.discovery.is_none() && !s.busy[1] {
                return Effect::Discover(s.begin(1), false);
            }
        }
        Event::Detail(id) => s.detail = Some(id),
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
        Event::Query(q) => s.query = q,
        Event::MarketQuery(q) => {
            s.market_query = q;
            s.generation[2] = s.generation[2].wrapping_add(1);
            s.busy[2] = false;
            s.market = None;
            s.offset = 0;
        }
        Event::Repository(id, enabled) => {
            s.selected_repos.retain(|r| r != &id);
            if enabled {
                s.selected_repos.push(id);
            }
        }
        Event::Uninstalled(v) => s.uninstalled = v,
        Event::Refresh => {
            return match s.tab {
                Tab::Installed => Effect::Load(s.begin(0)),
                Tab::Repositories => Effect::Discover(s.begin(1), true),
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

pub fn view<'a>(page: &'a Page, targets: Element<'a, Message>) -> Element<'a, Message> {
    let Some(scope) = page.scope() else {
        return text("请连接执行主机").into();
    };
    let s = &page.workspace;
    let send = move |event| Message {
        scope: scope.clone(),
        catalog: page.catalog,
        action: Action::Workspace(event),
    };
    let directory = |action| Message {
        scope: scope.clone(),
        catalog: page.catalog,
        action: Action::Directory(action),
    };
    let action = |label: &'static str, event| {
        button(label).padding([8, 10]).style(rounded_action_btn_style).on_press(send(event))
    };
    let count = s.inventory.as_ref().map_or(0, |i| i.skills.iter().filter(|s| s.present).count());
    let header = row![
        text(format!("Skills · 已安装 {count}")).size(24),
        button("添加 Skill")
            .padding([8, 10])
            .style(primary_action_btn_style)
            .on_press(send(Event::AddMenu)),
        button("更多")
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press(send(Event::MoreMenu)),
        action("刷新 / 重试", Event::Refresh)
    ]
    .spacing(8);
    let mut body = column![text("技能主副本由当前主机共享。勾选应用实例后安装会分发；未勾选时只安装主副本。应用偏好不代表逐技能启用状态。")].spacing(10);
    for error in s.errors.iter().flatten() {
        body = body.push(settings_error_banner(error.clone()));
    }
    if let Some(notice) = &s.notice {
        body = body.push(text(notice));
    }
    if s.busy.iter().any(|v| *v) {
        body = body.push(text("正在读取…"));
    }
    if s.installing {
        body = body.push(text("安装进行中，请等待任务终态；断线后可从任务进度核对。"));
    }
    if let Some(c) = &s.pending {
        body = body.push(
            container(column![
                text(format!("登记或启用仓库 {}/{}（{}）后安装？", c.owner, c.repo, c.branch)),
                row![
                    action("确认登记并安装", Event::ConfirmRepository),
                    action("取消", Event::CancelRepository)
                ]
                .spacing(8)
            ])
            .padding(12),
        );
    }
    if s.tab == Tab::Installed {
        body = body.push(
            text_input("筛选名称、描述、来源", &s.query)
                .on_input(move |v| send(Event::Query(v)))
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style),
        );
        let mut matched = 0;
        if let Some(data) = &s.inventory {
            for skill in data.skills.iter().filter(|skill| skill.present) {
                if !matches_query(
                    &s.query,
                    &[
                        &skill.name,
                        skill.description.as_deref().unwrap_or(""),
                        skill.source.as_deref().unwrap_or(""),
                    ],
                ) {
                    continue;
                }
                matched += 1;
                body = body.push(
                    column![
                        text(skill.name.clone()).size(18),
                        text(skill.description.clone().unwrap_or_default()),
                        text(skill.source.clone().unwrap_or_else(|| "来源未知".into())),
                        row![
                            action("详情", Event::Detail(skill.id.clone())),
                            button("选择应用动作")
                                .padding([8, 10])
                                .style(rounded_action_btn_style)
                                .on_press(Message {
                                    scope: scope.clone(),
                                    catalog: page.catalog,
                                    action: Action::Binding(
                                        crate::update::bindings_view::Event::Select(
                                            skill.id.clone()
                                        )
                                    )
                                }),
                            button("更新 / 卸载")
                                .padding([8, 10])
                                .style(rounded_action_btn_style)
                                .on_press(directory(DirectoryAction::Updates))
                        ]
                        .spacing(8)
                    ]
                    .spacing(6),
                );
            }
        }
        if let Some(detail) = s
            .detail
            .as_ref()
            .and_then(|id| s.inventory.as_ref()?.skills.iter().find(|c| &c.id == id))
        {
            body = body.push(
                container(column![
                    text(detail.name.clone()).size(20),
                    text(detail.markdown.clone().unwrap_or_else(|| "暂无详情".into()))
                ])
                .padding(12),
            );
        }
        if matched == 0 && !s.busy[0] {
            body = body.push(
                container(column![
                    text(if count == 0 { "尚未安装技能" } else { "无匹配技能" }),
                    row![
                        action("发现技能", Event::Tab(Tab::Repositories)),
                        button("导入技能")
                            .padding([8, 10])
                            .style(rounded_action_btn_style)
                            .on_press(directory(DirectoryAction::Import))
                    ]
                    .spacing(8)
                ])
                .center_x(Length::Fill)
                .padding(24),
            );
        }
    } else {
        if s.tab == Tab::Repositories {
            body = body.push(
                text_input("本地筛选名称、描述或仓库", &s.query)
                    .on_input(move |v| send(Event::Query(v)))
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style),
            );
            if let Some(repos) = &s.repositories {
                for repo in &repos.repos {
                    let id = format!("{}:{}", repo.owner, repo.name);
                    let selected = s.selected_repos.contains(&id);
                    body = body.push(
                        checkbox(selected)
                            .label(format!("{}/{}", repo.owner, repo.name))
                            .style(settings_checkbox_style)
                            .on_toggle(move |v| send(Event::Repository(id.clone(), v))),
                    );
                }
            }
            if let Some(data) = &s.discovery {
                for failure in &data.failures {
                    body = body.push(settings_error_banner(format!(
                        "{}/{}：{:?}；可刷新重试",
                        failure.owner, failure.name, failure.error
                    )));
                }
            }
        } else {
            body = body.push(
                row![
                    text_input("搜索 skills.sh", &s.market_query)
                        .on_input(move |v| send(Event::MarketQuery(v)))
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style),
                    action("搜索", Event::Search(0))
                ]
                .spacing(8),
            );
        }
        body = body.push(
            checkbox(s.uninstalled)
                .label("仅看未安装")
                .style(settings_checkbox_style)
                .on_toggle(move |v| send(Event::Uninstalled(v))),
        );
        let candidates: Vec<Candidate> = if s.tab == Tab::Repositories {
            s.discovery
                .as_ref()
                .map(|data| {
                    data.skills
                        .iter()
                        .map(|c| Candidate {
                            name: c.name.clone(),
                            description: c.description.clone(),
                            directory: c.directory.clone(),
                            owner: c.repo_owner.clone(),
                            repo: c.repo_name.clone(),
                            branch: c.repo_branch.clone(),
                            url: c.readme_url.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            s.market
                .as_ref()
                .map(|data| {
                    data.skills
                        .iter()
                        .map(|c| Candidate {
                            name: c.name.clone(),
                            description: format!("{} 次安装", c.installs),
                            directory: c.directory.clone(),
                            owner: c.repo_owner.clone(),
                            repo: c.repo_name.clone(),
                            branch: c.repo_branch.clone(),
                            url: c.readme_url.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut matched = 0;
        for c in candidates {
            if s.uninstalled && s.installed(&c) == Some(true) {
                continue;
            }
            if s.tab == Tab::Repositories
                && ((!s.selected_repos.is_empty() && !s.selected_repos.contains(&c.repo_id()))
                    || !matches_query(&s.query, &[&c.name, &c.description, &c.repo_id()]))
            {
                continue;
            }
            matched += 1;
            let label = if s.installed(&c) == Some(true) {
                "已安装"
            } else if s.conflict(&c) {
                "目录冲突 / 来源待核对"
            } else {
                "安装"
            };
            if let Some(url) = &c.url {
                body = body.push(
                    button("文档")
                        .padding([8, 10])
                        .style(rounded_action_btn_style)
                        .on_press(send(Event::Documentation(url.clone()))),
                );
            }
            body = body.push(
                column![
                    text(c.name.clone()).size(18),
                    text(format!("{}/{} · {}", c.owner, c.repo, c.directory)),
                    text(c.description.clone()),
                    button(label).padding([8, 10]).style(rounded_action_btn_style).on_press_maybe(
                        (!s.installing
                            && !s.busy[0]
                            && !s.conflict(&c)
                            && s.installed(&c) != Some(true))
                        .then(|| send(Event::Install(c)))
                    )
                ]
                .spacing(6),
            );
        }
        if matched == 0 {
            body = body.push(text("暂无匹配结果；请选择来源并刷新或搜索"));
        }
        if s.tab == Tab::Market {
            body = body.push(
                row![
                    button("上一页")
                        .padding([8, 10])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            (s.offset >= 50 && !s.busy[2])
                                .then(|| send(Event::Search(s.offset.saturating_sub(50))))
                        ),
                    button("下一页")
                        .padding([8, 10])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            (!s.busy[2]
                                && s.offset < 10_000
                                && s.market
                                    .as_ref()
                                    .is_some_and(|m| s.offset + 50 < m.total_count))
                            .then(|| send(Event::Search(s.offset + 50)))
                        )
                ]
                .spacing(8),
            );
        }
    }
    let mut menus = column![].spacing(6);
    if s.add_menu {
        menus = menus.push(row![
            action("从仓库发现", Event::Tab(Tab::Repositories)),
            button("从 ZIP 安装").padding([8, 10]).style(rounded_action_btn_style)
                .on_press(directory(DirectoryAction::Install)),
            button("导入已有技能").padding([8, 10]).style(rounded_action_btn_style)
                .on_press(directory(DirectoryAction::Import)),
        ].spacing(8));
    }
    if s.more_menu {
        for (label, entry) in [
            ("仓库管理", DirectoryAction::Repositories),
            ("存储与同步", DirectoryAction::Storage),
            ("备份与恢复", DirectoryAction::Backup),
            ("更新与卸载", DirectoryAction::Updates),
            ("任务进度", DirectoryAction::Jobs),
            ("Claude 配置", DirectoryAction::Claude),
        ] {
            menus = menus.push(button(label).padding([8, 10]).style(rounded_action_btn_style)
                .on_press(directory(entry)));
        }
    }
    column![
        header,
        menus,
        row![
            action("已安装", Event::Tab(Tab::Installed)),
            action("仓库目录", Event::Tab(Tab::Repositories)),
            action("skills.sh", Event::Tab(Tab::Market))
        ]
        .spacing(8),
        targets,
        scrollable(container(body).padding([0, 7]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4)
            ))
            .height(Length::Fill)
    ]
    .spacing(12)
    .into()
}
fn matches_query(query: &str, values: &[&str]) -> bool {
    let q = query.trim().to_lowercase();
    values.iter().any(|v| v.to_lowercase().contains(&q))
}
