//! In-memory navigation for the third-party management area.
//!
//! Composition supplies the current catalog fields; this module owns neither
//! domain data nor credentials. Domain handlers still enforce operation support.
use desktop_provider_apps::AgentAppsContext;
use vw_api_app_identity::AppId;

use crate::system_settings_apps::catalog::CatalogRow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppPage {
    #[default]
    Overview,
    Providers,
    Prompts,
    Workspace,
    Memory,
    Routing,
    HighAvailability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourcePage {
    Mcp,
    Skills,
    Sessions,
    Authorization,
    Usage,
    ManagementTools,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Agents,
    App {
        app: AppId,
        page: AppPage,
    },
    Resource(ResourcePage),
}

/// Identifies results that may still be shown after asynchronous work finishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub context: AgentAppsContext,
    pub host: String,
    pub catalog_generation: u64,
    pub page: Page,
    revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationError {
    ContextUnavailable,
    AppUnavailable,
    UnsupportedPage,
    NoSelectedApp,
}

impl std::fmt::Display for NavigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ContextUnavailable => "应用目录上下文尚未就绪",
            Self::AppUnavailable => "当前主机的应用目录中没有此应用",
            Self::UnsupportedPage => "此应用不支持该管理页面",
            Self::NoSelectedApp => "请先选择应用",
        })
    }
}

impl std::error::Error for NavigationError {}

#[derive(Clone, Debug, Default)]
pub struct State {
    page: Page,
    selected_app: Option<AppId>,
    app_page: AppPage,
    context: Option<AgentAppsContext>,
    host: String,
    catalog_generation: u64,
    revision: u64,
}

impl State {
    pub fn page(&self) -> Page {
        self.page
    }

    pub fn selected_app(&self) -> Option<AppId> {
        self.selected_app
    }

    /// Call with AppsCatalogState fields before rendering or handling navigation.
    /// Host/account/project changes discard selection; catalog refreshes preserve
    /// it only while the application remains registered in the current catalog.
    pub fn sync_catalog(
        &mut self,
        context: Option<&AgentAppsContext>,
        host: &str,
        generation: u64,
        rows: &[CatalogRow],
    ) {
        let context_changed = self.context.as_ref() != context || self.host != host;
        if context_changed || self.catalog_generation != generation {
            self.revision += 1;
        }
        self.context = context.cloned();
        self.host = host.to_owned();
        self.catalog_generation = generation;
        if context_changed || context.is_none() || host.trim().is_empty() {
            self.selected_app = None;
            self.app_page = AppPage::Overview;
            self.page = Page::Agents;
        } else if self.selected_app.is_some_and(|id| !rows.iter().any(|r| r.entry.id == id)) {
            self.selected_app = None;
            self.app_page = AppPage::Overview;
            if matches!(self.page, Page::App { .. }) {
                self.page = Page::Agents;
            }
            self.revision += 1;
        }
    }

    pub fn open_app(
        &mut self,
        app: AppId,
        page: AppPage,
        rows: &[CatalogRow],
    ) -> Result<(), NavigationError> {
        self.require_context()?;
        if !rows.iter().any(|row| row.entry.id == app) {
            return Err(NavigationError::AppUnavailable);
        }
        // Match the existing application-specific composition. Installation
        // presence is deliberately not used as evidence of domain support.
        let supported = match page {
            AppPage::Workspace => app == AppId::OpenClaw,
            AppPage::Memory => matches!(app, AppId::OpenClaw | AppId::Hermes),
            _ => true,
        };
        if !supported {
            return Err(NavigationError::UnsupportedPage);
        }
        self.selected_app = Some(app);
        self.app_page = page;
        self.navigate(Page::App { app, page });
        Ok(())
    }

    pub fn open_resource(&mut self, page: ResourcePage) -> Result<(), NavigationError> {
        self.require_context()?;
        self.navigate(Page::Resource(page));
        Ok(())
    }

    pub fn return_to_app(&mut self, rows: &[CatalogRow]) -> Result<(), NavigationError> {
        let app = self.selected_app.ok_or(NavigationError::NoSelectedApp)?;
        self.open_app(app, self.app_page, rows)
    }

    pub fn show_agents(&mut self) {
        self.navigate(Page::Agents);
    }

    pub fn scope(&self) -> Option<Scope> {
        self.require_context().ok()?;
        Some(Scope {
            context: self.context.clone()?,
            host: self.host.clone(),
            catalog_generation: self.catalog_generation,
            page: self.page,
            revision: self.revision,
        })
    }

    /// A result from an earlier app, host, tenant, project or visit is stale.
    pub fn accepts(&self, scope: &Scope) -> bool {
        self.scope().as_ref() == Some(scope)
    }

    fn require_context(&self) -> Result<(), NavigationError> {
        if self.context.is_none() || self.host.trim().is_empty() {
            return Err(NavigationError::ContextUnavailable);
        }
        Ok(())
    }

    fn navigate(&mut self, page: Page) {
        if self.page != page {
            self.revision += 1;
            self.page = page;
        }
    }
}
