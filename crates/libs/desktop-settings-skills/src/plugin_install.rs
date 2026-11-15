use iced::Task;
pub use crate::plugin_state::{PluginInstallForm, PluginInstallPhase};
#[derive(Clone, Debug)]
pub enum ToastKind { Info, Success, Warning, Error }
fn notice(kind: ToastKind, text: &str, _: Vec<()>) -> Task<Event> { Task::done(Event::Toast(kind, text.to_owned())) }
pub struct State<'a> { pub plugin_operation_id: &'a mut u64, pub plugin_install: &'a mut Option<PluginInstallForm> }
fn discard(token: String) -> Task<Event> { Task::done(Event::Discard(token)) }
fn next_id(app: &mut State<'_>) -> u64 {
    *app.plugin_operation_id =
        app.plugin_operation_id.wrapping_add(1);
    *app.plugin_operation_id
}
pub fn update(app: &mut State<'_>, message: Event) -> Task<Event> {
    match message {
        Event::PluginsInstallOpen | Event::PluginsUpdateOpen(_) => {
            if app.plugin_install.is_some() {
                return Task::none();
            }
            let update = match message {
                Event::PluginsUpdateOpen(id) => Some(id),
                _ => None,
            };
            let id = next_id(app);
            *app.plugin_install = Some(PluginInstallForm::new(id, update));
            Task::none()
        }
        Event::PluginsInstallSourceChanged(value) => {
            let Some(form) = app.plugin_install.as_mut() else {
                return Task::none();
            };
            if form.busy() {
                return Task::none();
            }
            let token = form.preview().map(|p| p.token.clone());
            form.source = value;
            form.phase = PluginInstallPhase::Input;
            token.map(discard).unwrap_or_else(Task::none)
        }
        Event::PluginsInstallPrepare => {
            if app.plugin_install.as_ref().is_none_or(|form| form.busy()) {
                return Task::none();
            }
            let id = next_id(app);
            let form = app.plugin_install.as_mut().unwrap();
            let old = form.preview().map(|p| p.token.clone());
            form.request_id = id;
            if let Err(error) = form.begin_prepare() {
                form.phase = PluginInstallPhase::Failed(error.clone());
                return notice(
                    ToastKind::Error,
                    &error,
                    vec![],
                );
            }
            let source = form.source.clone();
            let update = form.update_id.clone();
            let prepare = Task::done(Event::Prepare(id, source, update));
            Task::batch([prepare, old.map(discard).unwrap_or_else(Task::none)])
        }
        Event::PluginsInstallPrepared { request_id, result } => {
            let failure = result.as_ref().err().cloned();
            let stale = match app.plugin_install.as_mut() {
                Some(form) => form.receive_preview(request_id, result),
                None => result.ok(),
            };
            if let Some(stale) = stale {
                return discard(stale.token);
            }
            if let Some(error) = failure {
                if app
                    .plugin_install
                    .as_ref()
                    .is_some_and(|f| f.request_id == request_id)
                {
                    return Task::done(Event::Toast(ToastKind::Error, error));
                }
            }
            if app.plugin_install.as_ref().is_some_and(|form| {
                form.request_id == request_id && matches!(form.phase, PluginInstallPhase::Review(_))
            }) {
                return notice(
                    ToastKind::Info,
                    "审计预览已就绪；请核对来源与声明后确认",
                    vec![],
                );
            }
            Task::none()
        }
        Event::PluginsInstallConfirm => {
            let Some(form) = app.plugin_install.as_mut() else {
                return Task::none();
            };
            let Some(preview) = form.confirm() else {
                return Task::none();
            };
            let id = form.request_id;
            let update = form.update_id.is_some();
            Task::done(Event::Activate(id, preview.token, update))
        }
        Event::PluginsInstallCompleted { request_id, result } => {
            if !app
                .plugin_install
                .as_ref()
                .is_some_and(|f| f.request_id == request_id)
            {
                return Task::none();
            }
            match result {
                Ok(_) => {
                    *app.plugin_install = None;
                    let toast = notice(
                        ToastKind::Success,
                        "插件操作已完成；未自动运行引导技能或钩子",
                        vec![],
                    );
                    let refresh = Task::done(Event::Refresh);
                    Task::batch([toast, refresh])
                }
                Err(error) => {
                    app.plugin_install.as_mut().unwrap().phase =
                        PluginInstallPhase::Failed(error.clone());
                    Task::done(Event::Toast(ToastKind::Error, error))
                }
            }
        }
        Event::PluginsInstallClose => {
            if app
                .plugin_install
                .as_ref()
                .is_some_and(|f| matches!(f.phase, PluginInstallPhase::Committing(_)))
            {
                return notice(
                    ToastKind::Info,
                    "正在提交插件操作，完成后关闭",
                    vec![],
                );
            }
            let token = app
                .plugin_install
                .take()
                .and_then(|f| f.preview().map(|p| p.token.clone()));
            next_id(app);
            Task::batch([
                notice(
                    ToastKind::Info,
                    "已取消插件操作",
                    vec![],
                ),
                token.map(discard).unwrap_or_else(Task::none),
            ])
        }
        Event::PluginsPreviewDiscarded(result) => match result {
            Ok(()) => Task::none(),
            Err(_) => notice(
                ToastKind::Warning,
                "预览已关闭；暂存清理未确认，服务端将在过期后清理",
                vec![],
            ),
        },
        _ => Task::none(),
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    PluginsInstallOpen,
    PluginsUpdateOpen(String),
    PluginsInstallSourceChanged(String),
    PluginsInstallPrepare,
    PluginsInstallPrepared {
        request_id: u64,
        result: Result<gateway_desktop_settings::DesktopPluginPreviewDto, String>,
    },
    PluginsInstallConfirm,
    PluginsInstallCompleted {
        request_id: u64,
        result: Result<String, String>,
    },
    PluginsInstallClose,
    PluginsPreviewDiscarded(Result<(), String>),
Prepare(u64, String, Option<String>), Activate(u64, String, bool), Discard(String), Refresh, Toast(ToastKind, String),
}
