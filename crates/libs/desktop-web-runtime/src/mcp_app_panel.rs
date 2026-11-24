//! A session owns each isolated application process and revokes it on disposal.
use crate::mcp_app_bridge::McpAppProcess;
use crate::mcp_app_download::select_destination;
use anyhow::{Result, bail};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
};
use vw_api_mcp_network::McpAppNetworkPolicy;

#[derive(Debug, Clone)]
pub struct McpAppPanel(Arc<PanelLease>);
#[derive(Debug)]
struct PanelLease {
    cancelled: Arc<AtomicBool>,
    status: Arc<AtomicU8>,
    locale: Arc<AtomicU8>,
}
impl Drop for PanelLease {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

impl McpAppPanel {
    pub fn validate(app: &Value) -> Result<(&str, &str, &str, &str)> {
        let field = |name| {
            app[name]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("missing MCP app {name}"))
        };
        let server = field("server")?;
        let tool = field("tool")?;
        let binding = field("execution_binding")?;
        let uri = field("resource_uri")?;
        let contents = app["resource"]["contents"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing app contents"))?;
        if !uri.starts_with("ui://")
            || contents.len() != 1
            || contents[0]["uri"] != uri
            || contents[0]["mimeType"] != "text/html;profile=mcp-app"
        {
            bail!("invalid MCP app resource association");
        }
        let html = contents[0]["text"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 4 * 1024 * 1024)
            .ok_or_else(|| anyhow::anyhow!("invalid MCP app HTML"))?;
        Self::network_policy(app)?;
        Ok((server, tool, binding, html))
    }

    pub fn network_policy(app: &Value) -> Result<McpAppNetworkPolicy> {
        McpAppNetworkPolicy::parse(app.pointer("/resource/contents/0/_meta/ui/csp"))
            .map_err(anyhow::Error::msg)
    }

    pub fn launch(
        locale: config_ui::DesktopLocale,
        helper: &Path,
        session: String,
        app: &Value,
        arguments: Value,
        result: Value,
        client: vw_gateway_client::GatewayClient,
    ) -> Result<Self> {
        let (server, tool, binding, html) = Self::validate(app)?;
        let binding = binding.to_owned();
        let mut process = McpAppProcess::launch_with_policy(
            locale,
            helper,
            session,
            server.to_owned(),
            html,
            BTreeSet::from([tool.to_owned()]),
            Self::network_policy(app)?,
        )?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let status = Arc::new(AtomicU8::new(0));
        let selected_locale = Arc::new(AtomicU8::new(locale_id(locale)));
        let lease = Self(Arc::new(PanelLease {
            cancelled: cancelled.clone(), status: status.clone(), locale: selected_locale.clone(),
        }));
        std::thread::Builder::new().name("mcp-app-panel".into()).spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
            if let Ok(runtime) = runtime {
                runtime.block_on(async {
                    let mut delivered = false;
                    let mut current_locale = locale;
                    let started = std::time::Instant::now();
                    loop {
                        if cancelled.load(Ordering::Acquire) {
                            break;
                        }
                        if !process.is_running() {
                            if !delivered {
                                status.store(3, Ordering::Release);
                            }
                            break;
                        }
                        let locale = selected_locale.load(Ordering::Acquire);
                        let next_locale = locale_value(locale);
                        if locale_id(current_locale) != locale {
                            if process.set_locale(next_locale).is_err() { status.store(3, Ordering::Release); break; }
                            current_locale = next_locale;
                        }
                        let poll = process.poll_gateway(
                            &client,
                            &binding,
                            move |origin| confirm_external_link(desktop_i18n::I18n::new(current_locale), origin),
                            open_system_browser,
                            move |prompt| select_destination(desktop_i18n::I18n::new(current_locale), prompt),
                            || cancelled.load(Ordering::Acquire),
                        );
                        let stop = async {
                            while !cancelled.load(Ordering::Acquire) {
                                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                            }
                        };
                        let outcome =
                            tokio::select! { result = poll => result, () = stop => break };
                        if outcome.is_err() {
                            status.store(3, Ordering::Release);
                            break;
                        }
                        if !process.is_ready()
                            && started.elapsed() > std::time::Duration::from_secs(15)
                        {
                            status.store(3, Ordering::Release);
                            break;
                        }
                        if !delivered && process.is_ready() {
                            if process.tool_result(arguments.clone(), result.clone()).is_err() {
                                status.store(3, Ordering::Release);
                                break;
                            }
                            delivered = true;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                    }
                });
            } else {
                status.store(3, Ordering::Release);
            }
            if status.load(Ordering::Acquire) != 3 {
                status.store(2, Ordering::Release);
            }
            let _ = process.close();
        })?;
        Ok(lease)
    }
    pub fn set_locale(&self, locale: config_ui::DesktopLocale) {
        self.0.locale.store(locale_id(locale), Ordering::Release);
    }
    pub fn failed(&self) -> bool {
        self.0.status.load(Ordering::Acquire) == 3
    }
    pub fn finished(&self) -> bool {
        self.0.status.load(Ordering::Acquire) >= 2
    }
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
    }
}

#[cfg(target_os = "macos")]
async fn confirm_external_link( i18n: desktop_i18n::I18n, origin: String) -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertSecondButtonReturn};
    use objc2_foundation::NSString;

    let (decision, response) = tokio::sync::oneshot::channel();
    dispatch2::DispatchQueue::main().exec_async(move || {
        if decision.is_closed() {
            return;
        }
        let mtm = MainThreadMarker::new().expect("confirmation runs on the main thread");
        // Use an app-owned alert so a system notification's default response cannot approve.
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(i18n.__("打开 MCP 外部链接")));
        alert.setInformativeText(&NSString::from_str(&i18n.sprintf("是否使用系统浏览器打开此来源的链接？\n%s", &[origin.to_string().into()])));
        let cancel = alert.addButtonWithTitle(&NSString::from_str(i18n.__("取消")));
        cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        let open = alert.addButtonWithTitle(&NSString::from_str(i18n.__("打开")));
        open.setKeyEquivalent(&NSString::from_str(""));
        let _ = decision.send(alert.runModal() == NSAlertSecondButtonReturn);
    });
    response.await.unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
async fn confirm_external_link( i18n: desktop_i18n::I18n, origin: String) -> bool {
    rfd::AsyncMessageDialog::new()
        .set_title(i18n.__("打开 MCP 外部链接"))
        .set_description(i18n.sprintf("是否使用系统浏览器打开此来源的链接？\n%s", &[format!("{}", origin).into()]))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        .await
        == rfd::MessageDialogResult::Yes
}

// On Windows, open::that uses cmd in the pinned version. Use ShellExecute
// so the confirmed URL is passed as data, without shell expansion.
fn open_system_browser(url: &str) -> bool {
    #[cfg(windows)]
    {
        open::that_detached(url).is_ok()
    }
    #[cfg(not(windows))]
    {
        open::that(url).is_ok()
    }
}

fn locale_id(locale: config_ui::DesktopLocale) -> u8 {
    match locale {
        config_ui::DesktopLocale::SimplifiedChinese => 0,
        config_ui::DesktopLocale::TraditionalChinese => 1,
        config_ui::DesktopLocale::English => 2,
    }
}
fn locale_value(id: u8) -> config_ui::DesktopLocale {
    match id {
        1 => config_ui::DesktopLocale::TraditionalChinese,
        2 => config_ui::DesktopLocale::English,
        _ => config_ui::DesktopLocale::SimplifiedChinese,
    }
}
