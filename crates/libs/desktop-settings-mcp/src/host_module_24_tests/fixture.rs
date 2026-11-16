//! Scoped MCP fixtures: real domain state/reducers with captured gateway authority.
//! No desktop App, persistent bootstrap, native credentials, or window is constructed.
pub use crate::settings_message::McpMessage;
use crate::{
    management_context::Context, management_effect::Effect, management_state::McpSettingsState,
};
use desktop_provider_apps::AgentAppsContext;
pub use desktop_settings_apps::system_settings_apps::catalog::CatalogRow;
use iced::{Element, Task, widget::column};
use std::{
    cell::RefCell,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Mutex, MutexGuard, mpsc},
    time::Duration,
};
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
static LOCK: Mutex<()> = Mutex::new(());
thread_local! { static CLIENT: RefCell<Option<GatewayClient>> = const { RefCell::new(None) }; }
pub struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        CLIENT.with(|c| *c.borrow_mut() = None);
    }
}
pub fn isolated_settings() -> (MutexGuard<'static, ()>, (), Reset) {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    CLIENT.with(|c| *c.borrow_mut() = None);
    (guard, (), Reset)
}
fn client() -> GatewayClient {
    CLIENT.with(|c| c.borrow().clone()).unwrap_or_else(|| {
        GatewayClient::new(
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1).with_tenant_scope(
                vw_api_transport::TenantScope::new("10000", "100000001").unwrap(),
            ),
        )
        .unwrap()
    })
}
pub fn with_settings_gateway(
    test: impl FnOnce(mpsc::Receiver<String>, mpsc::Sender<Option<String>>),
) {
    let (_guard, _, _reset) = isolated_settings();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    CLIENT.with(|c| {
        *c.borrow_mut() = Some(
            GatewayClient::new(
                gateway_endpoint::GatewayEndpoint::new("127.0.0.1", address.port())
                    .with_tenant_scope(
                        vw_api_transport::TenantScope::new("10000", "100000001").unwrap(),
                    ),
            )
            .unwrap(),
        )
    });
    let (tx, rx) = mpsc::channel();
    let (responses, replies) = mpsc::channel::<Option<String>>();
    let server = std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut bytes = Vec::new();
            let mut block = [0u8; 4096];
            loop {
                let n = stream.read(&mut block).unwrap_or(0);
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&block[..n]);
                if let Some(header) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..header]);
                    let length: usize = headers
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= header + 4 + length {
                        break;
                    }
                }
            }
            if bytes.is_empty() || tx.send(String::from_utf8(bytes).unwrap()).is_err() {
                break;
            }
            let body = match replies.recv_timeout(Duration::from_secs(5)) {
                Ok(Some(body)) => body,
                Ok(None) => continue,
                Err(_) => break,
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| test(rx, responses)));
    drop(TcpStream::connect(address));
    server.join().unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
#[derive(Default)]
pub struct Authority {
    pub host: String,
    pub token: String,
    pub context: Option<AgentAppsContext>,
    pub generation: u64,
    pub loading: bool,
    pub rows: Vec<CatalogRow>,
}
pub struct Fixture {
    pub state: McpSettingsState,
    pub authority: Authority,
    pub project_id: Option<String>,
    pub on_page: bool,
    pub effects: Vec<Effect>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            state: Default::default(),
            authority: Default::default(),
            project_id: None,
            on_page: false,
            effects: Vec::new(),
        }
    }
}
impl Fixture {
    fn input(&mut self) -> crate::management_update::Update<'_> {
        let context = catalog::context(self);
        let authorized =
            context.as_ref().is_some_and(|c| self.authority.context.as_ref() == Some(&c.endpoint));
        let targets = imports::targets(self);
        crate::management_update::Update {
            state: &mut self.state,
            context,
            authorized,
            token: self.authority.token.clone(),
            client: Ok(client()),
            targets,
            effects: Vec::new(),
        }
    }
    fn apply(&mut self, task: Task<McpMessage>, effects: Vec<Effect>) -> Task<McpMessage> {
        let mut tasks = vec![task];
        for effect in effects {
            if matches!(effect, Effect::Refresh) {
                tasks.push(catalog::update(self, catalog::CatalogMessage::Refresh));
            }
            self.effects.push(effect);
        }
        Task::batch(tasks)
    }
    pub fn route(&mut self, message: McpMessage) -> Task<McpMessage> {
        if let McpMessage::TabSelected(tab) = message {
            let _ = crate::configuration_update::update(
                &mut self.state.page,
                crate::configuration_update::Message::TabSelected(tab),
            );
        }
        invalidate(self)
    }
}
pub mod catalog {
    use super::*;
use iced::widget::column;
    pub use crate::management_list::CatalogMessage;
    pub fn context(f: &Fixture) -> Option<Context> {
        let client = client();
        Some(Context {
            endpoint: AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: None,
            },
            host: f.authority.host.clone(),
            credentials_generation: f.authority.generation,
        })
    }
    pub fn visible(f: &Fixture) -> Option<&Result<McpCatalogResponse, String>> {
        crate::management_list::visible(&f.state.catalog, context(f).as_ref())
    }
    pub fn loading(f: &Fixture) -> bool {
        crate::management_list::loading(&f.state.catalog, context(f).as_ref())
    }
    pub fn update(f: &mut Fixture, event: CatalogMessage) -> Task<McpMessage> {
        let context = context(f);
        let authorized =
            context.as_ref().is_some_and(|c| f.authority.context.as_ref() == Some(&c.endpoint));
        let mut input = crate::management_list::Update {
            state: &mut f.state.catalog,
            context,
            authorized,
            token: f.authority.token.clone(),
            client: Ok(client()),
            effects: Vec::new(),
        };
        let task = crate::management_list::update(&mut input, event).map(McpMessage::Catalog);
        let effects = std::mem::take(&mut input.effects);
        f.apply(task, effects)
    }
}
pub mod imports {
    use super::*;
use iced::widget::column;
    pub use crate::management_import::*;
    pub fn targets(f: &Fixture) -> Vec<McpAppTarget> {
        let context = catalog::context(f);
        let authorized = context.as_ref().is_some_and(|c| {
            f.authority.context.as_ref() == Some(&c.endpoint) && !f.authority.token.is_empty()
        });
        crate::management_import_update::trusted_targets(
            context.as_ref(),
            authorized,
            f.authority.loading,
            f.authority.rows.iter().flat_map(|r| {
                r.observation
                    .as_ref()
                    .ok()
                    .into_iter()
                    .flat_map(move |o| o.installations.iter().map(move |i| (r.entry.id, i)))
            }),
        )
    }
    pub fn visible(f: &Fixture) -> Option<&ImportState> {
        crate::management_import::visible(f.state.import.as_ref(), catalog::context(f).as_ref())
    }
    pub fn open_link(f: &mut Fixture, drafts: Vec<Result<McpCreateRequest, String>>) -> bool {
        crate::management_import_update::open_link(&mut f.input(), drafts)
    }
    pub fn update(f: &mut Fixture, event: ImportMessage) -> Task<McpMessage> {
        let mut input = f.input();
        let task =
            crate::management_import_update::update(&mut input, event).map(McpMessage::Import);
        let effects = std::mem::take(&mut input.effects);
        f.apply(task, effects)
    }
}
pub mod resync {
    use super::*;
use iced::widget::column;
    pub use crate::management_resync::{
        ResyncFailure, ResyncMessage, ResyncResult, ResyncState, locked,
    };
    pub fn update(f: &mut Fixture, event: ResyncMessage) -> Task<McpMessage> {
        let context = catalog::context(f);
        let targets = imports::targets(f);
        let authorized =
            context.as_ref().is_some_and(|c| f.authority.context.as_ref() == Some(&c.endpoint));
        let current = context.as_ref() == f.state.catalog.context.as_ref();
        let mut input = crate::management_resync::Update {
            context,
            authorized,
            targets,
            token: f.authority.token.clone(),
            client: Ok(client()),
            modal_open: f.state.modal_open(),
            selected: &f.state.bindings.selected,
            catalog: current.then_some(f.state.catalog.result.as_ref()).flatten(),
            state: &mut f.state.resync,
            effects: Vec::new(),
        };
        let task = crate::management_resync::update(&mut input, event).map(McpMessage::Resync);
        let effects = std::mem::take(&mut input.effects);
        f.apply(task, effects)
    }
}
pub mod probe {
    use super::*;
use iced::widget::column;
    pub use crate::diagnostic_context::Context;
    pub use desktop_apps_ui::agent_apps_diagnostics::{
        mcp::McpDiagnosticMessage,
        mcp_state::{AppChoice, Phase},
    };
    pub fn context(f: &Fixture) -> Option<crate::diagnostic_context::Context> {
        crate::diagnostic_context::current(
            catalog::context(f),
            f.project_id.as_deref(),
            f.state.diagnostics.app.as_ref(),
            &f.authority.token,
            f.authority.context.as_ref(),
            f.authority.rows.iter().map(|r| r.entry.id),
            catalog::visible(f),
        )
    }
    pub fn update(f: &mut Fixture, event: McpDiagnosticMessage) -> Task<McpMessage> {
        let current = context(f);
        let visible = catalog::visible(f).is_some();
        let mut notice = None;
        let task = desktop_apps_ui::agent_apps_diagnostics::mcp::update(
            &mut f.state.diagnostics,
            current,
            visible.then_some(f.state.catalog.result.as_ref()).flatten(),
            &f.authority.token,
            matches!(event, McpDiagnosticMessage::Start(_)).then(|| Ok(client())),
            &mut notice,
            event,
        )
        .map(McpMessage::Diagnostic);
        task
    }
}
pub fn invalidate(f: &mut Fixture) -> Task<McpMessage> {
    let changed = probe::context(f).as_ref() != f.state.diagnostics.context.as_ref();
    let mutating = f.state.mutating();
    crate::management_probe::invalidate(&mut f.state.diagnostics, f.on_page, changed, mutating)
        .map(|r| McpMessage::Diagnostic(probe::McpDiagnosticMessage::Cleaned(r)))
}
pub mod tools {
    use super::*;
use iced::widget::column;
    pub use crate::management_tools::{Context, ToolsMessage};
    pub fn visible(f: &Fixture) -> Option<&crate::management_tools::ToolsState> {
        crate::management_tools::visible(&f.state.tools, probe::context(f).as_ref())
    }
    fn run(f: &mut Fixture, event: Option<ToolsMessage>) -> Task<McpMessage> {
        let current = probe::context(f);
        let mutating = f.state.mutating();
        let mut input = crate::management_tools::Update {
            state: &mut f.state.tools,
            current,
            on_page: f.on_page,
            mutating,
            token: f.authority.token.clone(),
            client: Ok(client()),
            effects: Vec::new(),
        };
        let task = match event {
            Some(e) => crate::management_tools::update(&mut input, e),
            None => crate::management_tools::invalidate(&mut input),
        }
        .map(McpMessage::Tools);
        let effects = std::mem::take(&mut input.effects);
        f.apply(task, effects)
    }
    pub fn update(f: &mut Fixture, event: ToolsMessage) -> Task<McpMessage> {
        run(f, Some(event))
    }
    pub fn invalidate(f: &mut Fixture) -> Task<McpMessage> {
        run(f, None)
    }
}
pub fn tool_card(f: &Fixture) -> Element<'_, McpMessage> {
    tools::visible(f)
        .map(|s| crate::management_tools::card(s, catalog::visible(f)).map(McpMessage::Tools))
        .unwrap_or_else(|| column![].into())
}
pub fn import_card<'a>(
    f: &Fixture,
    state: &'a crate::management_import::ImportState,
) -> Element<'a, McpMessage> {
    crate::management_import::panel(state, imports::targets(f)).map(McpMessage::Import)
}
pub fn diagnostic_panel(f: &Fixture) -> Element<'_, McpMessage> {
    use desktop_apps_ui::agent_apps_diagnostics::{mcp_panel, mcp_results};
    let s = &f.state.diagnostics;
    mcp_panel::view(
        mcp_panel::Panel {
            visible: probe::context(f).as_ref() == s.context.as_ref(),
            active: s.active(),
            cancelling: s.cancelling,
            choices: f
                .authority
                .rows
                .iter()
                .map(|r| probe::AppChoice(r.entry.id, r.entry.name.clone()))
                .collect(),
            app: s.app.clone(),
            catalog: catalog::visible(f).and_then(|r| r.as_ref().ok()),
            labels: s.targets.iter().map(|(r, p)| (r.id.clone(), p.label())).collect(),
            error: &s.error,
        },
        |event| {
            McpMessage::Diagnostic(match event {
                mcp_panel::Event::Select(c) => probe::McpDiagnosticMessage::Select(c),
                mcp_panel::Event::Start(ids) => probe::McpDiagnosticMessage::Start(ids),
                mcp_panel::Event::Cancel => probe::McpDiagnosticMessage::Cancel,
                mcp_panel::Event::Refresh => probe::McpDiagnosticMessage::Refresh,
            })
        },
        mcp_results::view(s),
    )
}
pub fn view(f: &Fixture) -> Element<'_, McpMessage> {
    column![
        crate::catalog::panel(catalog::visible(f), catalog::loading(f))
            .map(|_| McpMessage::Refresh),
        crate::management_resync::panel(Some(&f.state.resync)).map(McpMessage::Resync),
        crate::bindings::panel(
            catalog::visible(f),
            Some(crate::bindings::BindingsView {
                selected: &f.state.bindings.selected,
                running: f.state.bindings.running,
                result: &f.state.bindings.result
            }),
            f.authority
                .rows
                .iter()
                .flat_map(|r| r
                    .observation
                    .as_ref()
                    .ok()
                    .into_iter()
                    .flat_map(|o| o.installations.iter()))
                .collect(),
            &f.authority.host,
            catalog::loading(f)
        )
        .map(|_| McpMessage::Refresh),
        diagnostic_panel(f),
        crate::tools::panel(catalog::visible(f)).map(|_| McpMessage::Refresh),
    ]
    .into()
}
pub fn overlay<'a>(f: &'a Fixture, dialog: Element<'a, McpMessage>) -> Element<'a, McpMessage> {
    if f.state.resync.open {
        return crate::management_resync::overlay(
            &f.state.resync,
            &f.state.bindings.selected,
            imports::targets(f),
            column![].into(),
        )
        .map(McpMessage::Resync);
    }
    if let Some(state) = imports::visible(f) {
        return desktop_ui_controls::system_settings_common::settings_modal_overlay(
            Some(dialog),
            McpMessage::Import(crate::management_import::ImportMessage::Close),
            import_card(f, state),
        );
    }
    dialog
}

pub mod form {
    use super::*;
use iced::widget::column;
    pub use crate::management_form::*;
    pub fn update(f: &mut Fixture, event: FormMessage) -> Task<McpMessage> {
        let mut input = f.input();
        let task = crate::management_form_update::update(&mut input, event)
            .map(McpMessage::ConnectionForm);
        let effects = std::mem::take(&mut input.effects);
        f.apply(task, effects)
    }
}
pub mod bindings {
    use super::*;
use iced::widget::column;
    pub use crate::management_bindings::*;
    pub fn update(f: &mut Fixture, event: BindingsMessage) -> Task<McpMessage> {
        let mut input = f.input();
        let task =
            crate::management_bindings_update::update(&mut input, event).map(McpMessage::Bindings);
        let effects = std::mem::take(&mut input.effects);
        f.apply(task, effects)
    }
}
