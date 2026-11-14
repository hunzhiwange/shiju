//! Fixed workspace filenames; drafts and tokens never enter diagnostics.
use crate::app::components::system_settings_common::*;
use crate::app::{App, Message, state::AgentAppsContext};
use iced::{
    Element, Length, Task,
    widget::{button, column, container, row, text, text_editor},
};
use vw_gateway_client::{WorkspaceMemoryScope, vw_api_types::workspace_memory::*};

pub(super) const FILES: [WorkspaceFilename; 9] = [
    WorkspaceFilename::Agents,
    WorkspaceFilename::Soul,
    WorkspaceFilename::User,
    WorkspaceFilename::Identity,
    WorkspaceFilename::Tools,
    WorkspaceFilename::Memory,
    WorkspaceFilename::Heartbeat,
    WorkspaceFilename::Bootstrap,
    WorkspaceFilename::Boot,
];
#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    generation: u64,
    request: u64,
    listing: u64,
    files: Vec<(WorkspaceFilename, Result<bool, String>)>,
    selected: Option<WorkspaceFilename>,
    loaded: bool,
    busy: bool,
    editor: text_editor::Content,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
    pub(super) fn select(&mut self, filename: WorkspaceFilename) -> u64 {
        self.request = self.request.wrapping_add(1);
        self.selected = Some(filename);
        self.loaded = false;
        self.request
    }
    pub(super) fn accept_read(&mut self, request: u64, content: WorkspaceFileContent) -> bool {
        if !self.open || request != self.request || self.selected.is_none() {
            return false;
        }
        self.editor = text_editor::Content::with_text(content.0.as_deref().unwrap_or(""));
        self.loaded = true;
        true
    }
}
#[derive(Clone)]
pub struct Scope {
    context: AgentAppsContext,
    host: String,
    project: String,
    catalog: u64,
    generation: u64,
}
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    CloseEditor,
    Refresh,
    Select(WorkspaceFilename),
    Edit(text_editor::Action),
    Save,
    Directory(WorkspaceDirectory),
    DirectoryDone(Scope, Result<(), String>),
    Listed(Scope, u64, Vec<(WorkspaceFilename, Result<bool, String>)>),
    Read(Scope, u64, Result<WorkspaceFileContent, String>),
    Written(Scope, u64, Result<(), String>),
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::WorkspaceFiles(event),
    ))
}
pub(crate) fn entry() -> Element<'static, Message> {
    button(text("工作区文件"))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(message(Event::Open))
        .into()
}
pub(crate) fn overlay<'a>(
    state: &'a State,
    host: &'a str,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !state.open {
        return base;
    }
    let mut body = column![
        text(format!("执行主机：{host}")),
        row![
            button(text("工作区目录"))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(message(Event::Directory(WorkspaceDirectory::Workspace))),
            button(text("记忆目录"))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(message(Event::Directory(WorkspaceDirectory::Memory)))
        ]
        .spacing(8)
    ]
    .spacing(8);
    for filename in FILES {
        let status = match state.files.iter().find(|(f, _)| *f == filename).map(|(_, r)| r) {
            Some(Ok(true)) => "存在",
            Some(Ok(false)) => "缺失",
            Some(Err(_)) => "读取失败",
            None => "待刷新",
        };
        body = body.push(
            button(text(format!("{} · {status}", filename.as_str())))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe((!state.busy).then(|| message(Event::Select(filename)))),
        );
    }
    let mut footer = row![
        button(text("刷新"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then(|| message(Event::Refresh)))
    ]
    .spacing(8);
    if state.loaded {
        if let Some(filename) = state.selected {
            body = body.push(text(filename.as_str()));
        }
        let mut editor = settings_text_editor(&state.editor).height(260);
        if !state.busy {
            editor = editor.on_action(|a| message(Event::Edit(a)));
        }
        body = body.push(editor);
        footer = footer
            .push(
                button(text("保存"))
                    .padding([8, 12])
                    .style(primary_action_btn_style)
                    .on_press_maybe((!state.busy).then(|| message(Event::Save))),
            )
            .push(
                button(text("关闭编辑"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then(|| message(Event::CloseEditor))),
            );
    }
    let close = message(Event::Close);
    let card = settings_modal_layout(
        settings_modal_header(
            text("OpenClaw 工作区").size(20),
            settings_close_button(close.clone()),
        ),
        body,
        Some(footer.into()),
    )
    .width(760)
    .height(680);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.apps_catalog.workspace_files.invalidate();
        return Task::none();
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用，草稿已保留");
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    let Some(project) = app.project_id.clone().filter(|p| !p.is_empty()) else {
        return app.show_error_toast("请先选择项目");
    };
    let state = &mut app.apps_catalog.workspace_files;
    let reply = match &event {
        Event::Listed(s, ..)
        | Event::Read(s, ..)
        | Event::Written(s, ..)
        | Event::DirectoryDone(s, _) => Some(s),
        _ => None,
    };
    if let Some(s) = reply {
        if !state.open
            || s.context != context
            || s.host != app.apps_catalog.host
            || s.project != project
            || s.catalog != app.apps_catalog.generation
            || s.generation != state.generation
        {
            return Task::none();
        }
    } else if app.apps_catalog.context.as_ref() != Some(&context)
        || app.apps_catalog.token.is_empty()
    {
        return app.show_error_toast("请先连接应用服务");
    }
    match event.clone() {
        Event::Listed(_, request, files) => {
            if request == state.listing {
                state.files = files;
            }
            return Task::none();
        }
        Event::Read(_, request, result) => {
            if request != state.request {
                return Task::none();
            }
            match result {
                Ok(content) => {
                    state.accept_read(request, content);
                }
                Err(_) => {
                    state.loaded = false;
                    return app.show_error_toast("读取失败，请重试");
                }
            }
            return Task::none();
        }
        Event::Written(_, request, result) => {
            if request != state.request {
                return Task::none();
            }
            state.busy = false;
            if result.is_err() {
                return app.show_error_toast("保存失败，草稿已保留");
            }
        }
        Event::DirectoryDone(_, result) => {
            return if result.is_ok() {
                app.show_success_toast("已在执行主机打开目录")
            } else {
                app.show_error_toast("执行主机无法打开目录")
            };
        }
        Event::Edit(action) => {
            if state.loaded && !state.busy {
                state.editor.perform(action);
            }
            return Task::none();
        }
        Event::CloseEditor => {
            if !state.busy {
                state.request = state.request.wrapping_add(1);
                state.selected = None;
                state.loaded = false;
                state.editor = text_editor::Content::new();
            }
            return Task::none();
        }
        Event::Open => {
            if state.open {
                return Task::none();
            }
            state.open = true;
        }
        _ => {}
    }
    if state.busy {
        return Task::none();
    }
    let scope = Scope {
        context,
        host: app.apps_catalog.host.clone(),
        project: project.to_string(),
        catalog: app.apps_catalog.generation,
        generation: state.generation,
    };
    let query =
        WorkspaceMemoryScope { project_id: project.into(), target_host_id: scope.host.clone() };
    let token = app.apps_catalog.token.clone();
    match event {
        Event::Select(filename) => {
            let request = state.select(filename);
            return Task::perform(
                async move {
                    message(Event::Read(
                        scope,
                        request,
                        client
                            .read_workspace_file(&query, filename, &token)
                            .await
                            .map_err(|e| e.to_string()),
                    ))
                },
                |m| m,
            );
        }
        Event::Save => {
            if !state.loaded {
                return Task::none();
            }
            let Some(filename) = state.selected else {
                return Task::none();
            };
            state.busy = true;
            let request = state.request;
            let content = state.editor.text();
            return Task::perform(
                async move {
                    message(Event::Written(
                        scope,
                        request,
                        client
                            .write_workspace_file(&query, filename, content, &token)
                            .await
                            .map_err(|e| e.to_string()),
                    ))
                },
                |m| m,
            );
        }
        Event::Directory(directory) => {
            return Task::perform(
                async move {
                    message(Event::DirectoryDone(
                        scope,
                        client
                            .open_workspace_directory(&query, directory, &token)
                            .await
                            .map_err(|e| e.to_string()),
                    ))
                },
                |m| m,
            );
        }
        _ => {}
    }
    state.listing = state.listing.wrapping_add(1);
    let listing = state.listing;
    let refresh = Task::perform(
        async move {
            let mut files = Vec::new();
            for filename in FILES {
                files.push((
                    filename,
                    client
                        .read_workspace_file(&query, filename, &token)
                        .await
                        .map(|v| v.0.is_some())
                        .map_err(|e| e.to_string()),
                ));
            }
            message(Event::Listed(scope, listing, files))
        },
        |m| m,
    );
    if matches!(event, Event::Written(..)) {
        Task::batch([refresh, app.show_success_toast("工作区文件已保存")])
    } else {
        refresh
    }
}

#[cfg(test)]
#[path = "workspace_files_tests.rs"]
mod tests;
