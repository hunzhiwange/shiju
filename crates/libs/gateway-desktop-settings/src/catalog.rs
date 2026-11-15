use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopSkillCatalogEntryDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub resource_count: usize,
    pub installed: bool,
    pub enabled: bool,
    pub source: String,
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopSkillDetailDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub installed: bool,
    pub enabled: bool,
    pub source: String,
    pub source_path: Option<String>,
    pub document_name: String,
    pub document_content: String,
    pub can_install: bool,
    pub can_toggle: bool,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopSkillPathDto {
    pub path: String,
}

/// A read-only dependency observation; presence does not imply successful execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginDependencyDto {
    pub component: String,
    pub dependency: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginCatalogEntryDto {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub component_count: usize,
    pub source_path: Option<String>,
    #[serde(default)]
    pub dependency_checks: Vec<DesktopPluginDependencyDto>,
    #[serde(default)]
    pub origin: DesktopPluginSourceDto,
    #[serde(default)]
    pub hook_review: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginCapabilitiesDto {
    pub skills: usize,
    pub mcp_servers: usize,
    pub hooks: usize,
    pub interface: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginDetailDto {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub source_path: Option<String>,
    pub manifest_path: String,
    pub capabilities: DesktopPluginCapabilitiesDto,
    pub can_toggle: bool,
    pub can_delete: bool,
    #[serde(default)]
    pub dependency_checks: Vec<DesktopPluginDependencyDto>,
    #[serde(default)]
    pub origin: DesktopPluginSourceDto,
    #[serde(default)]
    pub hook_review: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginPathDto {
    pub path: String,
}

/// Locked package provenance without credentials or raw registry responses.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginSourceDto {
    pub kind: String,
    pub resolved_version: Option<String>,
    pub revision: Option<String>,
    pub sha256: Option<String>,
    #[serde(default)]
    pub marketplace: Option<String>,
}

/// Audited package preview awaiting explicit, single-use activation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginPreviewDto {
    pub token: String,
    pub plugin_id: String,
    pub source: String,
    pub old_version: Option<String>,
    pub version: Option<String>,
    pub capabilities: DesktopPluginCapabilitiesDto,
    pub audit_status: String,
    pub dependency_status: String,
    pub origin: DesktopPluginSourceDto,
    #[serde(default)]
    pub dependency_checks: Vec<DesktopPluginDependencyDto>,
    #[serde(default)]
    pub previous_capabilities: Option<DesktopPluginCapabilitiesDto>,
}

/// One statically audited hook declaration from an installed plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookCatalogEntryDto {
    pub plugin_id: String,
    pub event: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: String,
    pub mode: String,
    pub hook_hash: String,
    pub trusted: bool,
}

/// Trust mutation request containing no executable declaration fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookMutationRequestDto {
    pub plugin_id: String,
    pub hook_hash: String,
}

/// Result of trusting or revoking one current hook declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookMutationResponseDto {
    pub plugin_id: String,
    pub hook_hash: String,
    pub trusted: bool,
}

/// Bounded command audit record without tool arguments, output, or error text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookAuditEntryDto {
    pub timestamp: String,
    pub tool_name: String,
    pub duration_ms: u64,
    pub success: bool,
}

/// 外部应用探测结果的精简表示。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalAppsStateDto {
    /// 当前桌面平台标识。
    pub platform: Option<String>,
    /// 外部应用 ID 与是否可用的状态列表。
    pub apps: Vec<(String, bool)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopServiceCommandResponseDto {
    pub command: String,
    pub output: String,
}

/// Gateway 宿主允许执行的封闭 service 生命周期命令。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopServiceCommand {
    Status,
    Start,
    Stop,
    Restart,
}

impl DesktopServiceCommand {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }
}

impl TryFrom<&str> for DesktopServiceCommand {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "status" => Ok(Self::Status),
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            "restart" => Ok(Self::Restart),
            _ => Err(format!("unsupported desktop service command: {value}")),
        }
    }
}
