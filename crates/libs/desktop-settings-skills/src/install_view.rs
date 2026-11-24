use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::{
    install_api::InstallSkillResponse, inventory_api::SkillInventoryResponse,
    repos_api::SkillReposQuery,
};
#[derive(Debug, Clone, Default)]
pub struct State {
    pub archive: bool,
    pub file_id: String,
    pub preview: bool,
    pub open: bool,
    pub busy: bool,
    pub inventory: Option<SkillInventoryResponse>,
    pub selected: Option<String>,
    pub repo: String,
    pub selection: String,
    pub directory: String,
    pub error: Option<String>,
    pub generation: u64,
    pub context: Option<AgentAppsContext>,
    pub scope: Option<SkillReposQuery>,
    pub catalog: u64,
}
#[derive(Debug, Clone)]
pub enum Event {
    OpenArchive,
    FileId(String),
    Preview,
    Open,
    Close,
    Refresh,
    Select(String),
    Repo(String),
    Selection(String),
    Directory(String),
    Install,
    Loaded(u64, Result<SkillInventoryResponse, String>),
    Installed(u64, Result<(), String>),
    Submitted(u64, Result<InstallSkillResponse, String>),
}

// Scoped inventory and explicit installation confirmation, alongside built-in skills.

use crate::third_party::style;
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

pub fn view<'a, M: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    s: &'a State,
    _host: &str,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let mut body = column![
        text(i18n.__("从已登记仓库安装。安装后可选择分发到应用。")),
        settings_field_label(i18n.__("仓库标识（owner:name）")),
        settings_text_input("owner:name", &s.repo).on_input(move |v| message(Event::Repo(v))),
        settings_field_label(i18n.__("仓库内来源目录")),
        settings_text_input(i18n.__("来源目录"), &s.selection)
            .on_input(move |v| message(Event::Selection(v))),
        settings_field_label(i18n.__("安装目录名")),
        settings_text_input(i18n.__("单段目录名"), &s.directory)
            .on_input(move |v| message(Event::Directory(v))),
    ]
    .spacing(10)
    .width(Length::Fill);
    if s.archive {
        body = column![
            text(i18n.__("请输入已登记的 ZIP 文件标识。")),
            settings_field_label(i18n.__("预授权文件标识")),
            settings_text_input("file_id", &s.file_id).on_input(move |v| message(Event::FileId(v)))
        ]
        .spacing(10);
        if s.preview {
            body = body.push(text(
                desktop_i18n::__!(i18n, "确认安装 ZIP：%s", (s.file_id.trim()).to_string())
                    .expect("valid extension display template"),
            ));
        }
    }
    if s.busy {
        body = body.push(text(i18n.__("正在读取或安装，请等待任务结束…")));
    }
    if let Some(e) = &s.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
            i18n,
            &e.clone(),
        )));
    }
    if let Some(data) = s.inventory.as_ref().filter(|_| !s.archive) {
        body = body.push(text(
            desktop_i18n::__!(i18n, "库存版本：%s", (data.version).to_string())
                .expect("valid extension display template"),
        ));
        for skill in &data.skills {
            body = body.push(
                button(text(skill.name.clone()))
                    .padding([8, 10])
                    .style(rounded_action_btn_style)
                    .on_press(message(Event::Select(skill.id.clone()))),
            );
        }
        if let Some(detail) = data.skills.iter().find(|item| Some(&item.id) == s.selected.as_ref())
        {
            body = body.push(settings_group_card(
                column![
                    text(detail.name.clone()),
                    text(
                        desktop_i18n::__!(
                            i18n,
                            "来源：%s",
                            (detail.source.as_deref().unwrap_or(i18n.__("本地或归档"))).to_string()
                        )
                        .expect("valid extension display template")
                    ),
                    text(detail.description.clone().unwrap_or_default()),
                    text(
                        desktop_i18n::__!(
                            i18n,
                            "版本：%s",
                            (detail.version.as_deref().unwrap_or(i18n.__("未提供"))).to_string()
                        )
                        .expect("valid extension display template")
                    ),
                    text(
                        desktop_i18n::__!(
                            i18n,
                            "内容哈希：%s",
                            (detail.content_hash.as_deref().unwrap_or(i18n.__("未提供")))
                                .to_string()
                        )
                        .expect("valid extension display template")
                    ),
                    text(detail.markdown.clone().unwrap_or_default()),
                ]
                .spacing(6),
            ));
        }
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(if s.archive { i18n.__("从 ZIP 安装 Skill") } else { i18n.__("安装技能") }).size(20),
        settings_close_button(close.clone()),
    );
    let footer = row![
        button(if s.archive && !s.preview {
            i18n.__("预览安装请求")
        } else {
            i18n.__("确认安装到此主机")
        })
        .padding([8, 10])
        .style(style::primary)
        .on_press_maybe((!s.busy && s.inventory.is_some()).then_some(message(
            if s.archive && !s.preview { Event::Preview } else { Event::Install }
        ))),
        button(i18n.__("刷新"))
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button(i18n.__("关闭"))
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ]
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(style::dialog)
        .width(560)
        .height(if s.archive { 340 } else { 560 });
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, card)
}
