#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsSettingsTab {
    Skills,
    DiscoveryOrder,
    SystemConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsDirectoryScope {
    Project,
    Ancestor,
    Global,
    Bundled,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsCatalogKind {
    Recommended,
    System,
    Personal,
}

#[derive(Debug, Clone)]
pub struct SkillsSelectedDetail {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: SkillsCatalogKind,
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

#[derive(Debug, Clone)]
pub struct SkillsCatalogItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: SkillsCatalogKind,
    pub resource_count: usize,
    pub installed: bool,
    pub enabled: bool,
    pub source: String,
    pub source_path: Option<String>,
}
