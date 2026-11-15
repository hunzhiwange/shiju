use crate::state::{McpSettingsTab, McpNewServerTransport};
use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub enum McpMessage {
    Tools(crate::management_tools::ToolsMessage),
    Diagnostic(desktop_apps_ui::agent_apps_diagnostics::mcp::McpDiagnosticMessage),
    Catalog(crate::management_list::CatalogMessage),
    ConnectionForm(crate::management_form::FormMessage),
    Bindings(crate::management_bindings::BindingsMessage),
    Resync(crate::management_resync::ResyncMessage),
    Import(crate::management_import::ImportMessage),
    TabSelected(McpSettingsTab),
    Refresh,
    Loaded(Result<serde_json::Value, String>),
    InspectRefresh,
    Inspected(Result<vw_api_mcp_inspect::McpInspectResponse, String>),
    EditorAction(text_editor::Action),
    Format,
    InsertTemplate,
    NewServerPanelToggled,
    NewServerNameChanged(String),
    NewServerTransportChanged(McpNewServerTransport),
    NewServerCommandChanged(String),
    NewServerArgsChanged(String),
    NewServerUrlChanged(String),
    NewServerTimeoutChanged(String),
    NewServerEnabledToggled(bool),
    NewServerRequiredToggled(bool),
    AddServer,
    ServerEditRequested(usize),
    ServerEditCanceled,
    SaveServerEdit,
    ServerEnabledToggled(usize, bool),
    RuntimeServerEnabledToggled(String, bool),
    PreviewExpandedToggled(String),
    /// 请求删除指定索引的 MCP 服务器（弹出确认）。
    RequestDelete(usize),
    /// 确认删除指定的 MCP 服务器；处理时仍需匹配当前待确认索引。
    ConfirmDelete(usize),
    /// 取消删除操作。
    CancelDelete,
    Save,
    Saved(Result<(), String>),
}
