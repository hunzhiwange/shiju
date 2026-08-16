//! 维护运行时工具选择器状态及其回归测试。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::SkillsDirectoryScope;
use std::collections::{BTreeMap, BTreeSet};

/// Slash 面板条目类别，顺序固定为 Skills → Tools → MCP → Models → ACP。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlashPaletteKind {
    Skill,
    Tool,
    Mcp,
    Model,
    Acp,
}

impl SlashPaletteKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Skill => "Skills",
            Self::Tool => "Tools",
            Self::Mcp => "MCP",
            Self::Model => "Models",
            Self::Acp => "ACP",
        }
    }

    pub(crate) fn all() -> [Self; 5] {
        [Self::Skill, Self::Tool, Self::Mcp, Self::Model, Self::Acp]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 描述 SessionToolBucket 支持的离散状态或消息分支。
pub enum SessionToolBucket {
    ReadOnly,
    Edit,
    Execution,
    Browser,
    Agent,
    Other,
}

impl SessionToolBucket {
    /// ALL 使用的固定配置值。
    pub(crate) const ALL: [Self; 6] =
        [Self::ReadOnly, Self::Edit, Self::Execution, Self::Browser, Self::Agent, Self::Other];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 描述 SessionToolGroup 支持的离散状态或消息分支。
pub enum SessionToolGroup {
    Files,
    Search,
    Execute,
    Web,
    Collaboration,
    Memory,
    Integration,
    Other,
}

impl SessionToolGroup {
    /// ALL 使用的固定配置值。
    pub(crate) const ALL: [Self; 8] = [
        Self::Files,
        Self::Search,
        Self::Execute,
        Self::Web,
        Self::Collaboration,
        Self::Memory,
        Self::Integration,
        Self::Other,
    ];

    /// 执行 label 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Files => "文件",
            Self::Search => "搜索",
            Self::Execute => "执行",
            Self::Web => "联网",
            Self::Collaboration => "协作",
            Self::Memory => "记忆",
            Self::Integration => "系统",
            Self::Other => "其他",
        }
    }

    /// 执行 description 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Files => "读取、写入和修改工作区文件。",
            Self::Search => "查找文件、文本和代码位置。",
            Self::Execute => "运行命令、补丁和版本控制操作。",
            Self::Web => "浏览页面、联网请求和搜索。",
            Self::Collaboration => "委托代理、提问、计划和任务协作。",
            Self::Memory => "读写长期记忆与上下文记录。",
            Self::Integration => "系统配置、状态和集成能力。",
            Self::Other => "当前未归类的工具。",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 描述 SessionToolSelectorTab 支持的离散状态或消息分支。
pub enum SessionToolSelectorTab {
    Agent,
    Tools,
    Skills,
}

impl SessionToolSelectorTab {
    /// 执行 label 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Agent => "代理",
            Self::Tools => "工具",
            Self::Skills => "技能",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 描述 AdvancedToolSurfaceState 支持的离散状态或消息分支。
pub(crate) enum AdvancedToolSurfaceState {
    Available,
    Planned,
}

impl AdvancedToolSurfaceState {
    /// 执行 label 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Planned => "planned",
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// 表示 AdvancedToolSurfaceSpec 相关的应用状态或派生数据。
pub(crate) struct AdvancedToolSurfaceSpec {
    /// str 使用的共享静态数据。
    pub(crate) label: &'static str,
    pub(crate) state: AdvancedToolSurfaceState,
}

const ADVANCED_TOOL_SURFACES: [AdvancedToolSurfaceSpec; 7] = [
    AdvancedToolSurfaceSpec {
        label: "进入规划模式", state: AdvancedToolSurfaceState::Available
    },
    AdvancedToolSurfaceSpec {
        label: "退出规划模式", state: AdvancedToolSurfaceState::Available
    },
    AdvancedToolSurfaceSpec {
        label: "校验计划执行", state: AdvancedToolSurfaceState::Available
    },
    AdvancedToolSurfaceSpec {
        label: "进入 worktree", state: AdvancedToolSurfaceState::Available
    },
    AdvancedToolSurfaceSpec {
        label: "退出 worktree", state: AdvancedToolSurfaceState::Available
    },
    AdvancedToolSurfaceSpec { label: "mcp_* 集成", state: AdvancedToolSurfaceState::Planned },
    AdvancedToolSurfaceSpec { label: "tool_search", state: AdvancedToolSurfaceState::Available },
];

/// 执行 explicit_advanced_tool_surface_spec 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn explicit_advanced_tool_surface_spec(
    tool_id: &str,
) -> Option<AdvancedToolSurfaceSpec> {
    match tool_id {
        "enter_plan_mode" => Some(ADVANCED_TOOL_SURFACES[0]),
        "exit_plan_mode" => Some(ADVANCED_TOOL_SURFACES[1]),
        "verify_plan_execution" => Some(ADVANCED_TOOL_SURFACES[2]),
        "enter_worktree" => Some(ADVANCED_TOOL_SURFACES[3]),
        "exit_worktree" => Some(ADVANCED_TOOL_SURFACES[4]),
        "tool_search" => Some(ADVANCED_TOOL_SURFACES[6]),
        _ if tool_id.starts_with("mcp_") => Some(ADVANCED_TOOL_SURFACES[5]),
        _ => None,
    }
}

#[derive(Debug, Clone)]
/// 表示 SessionToolSelectorState 相关的应用状态或派生数据。
pub(crate) struct SessionToolSelectorState {
    enabled_buckets: BTreeSet<SessionToolBucket>,
    explicit_allowed_tools: Option<BTreeSet<String>>,
    manual_tools: BTreeSet<String>,
    manual_skills: BTreeSet<String>,
    selected_shiju_skills: BTreeMap<u64, ShijuSkillSelection>,
    collapsed_groups: BTreeSet<SessionToolGroup>,
    active_tab: SessionToolSelectorTab,
    query: String,
    skill_directory_scope: SkillsDirectoryScope,
}

impl Default for SessionToolSelectorState {
    fn default() -> Self {
        Self {
            enabled_buckets: SessionToolBucket::ALL.into_iter().collect(),
            explicit_allowed_tools: None,
            manual_tools: BTreeSet::new(),
            manual_skills: BTreeSet::new(),
            selected_shiju_skills: BTreeMap::new(),
            collapsed_groups: BTreeSet::new(),
            active_tab: SessionToolSelectorTab::Agent,
            query: String::new(),
            skill_directory_scope: SkillsDirectoryScope::All,
        }
    }
}

impl SessionToolSelectorState {
    /// 执行 active_tab 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn active_tab(&self) -> SessionToolSelectorTab {
        self.active_tab
    }

    /// 执行 select_tab 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn select_tab(&mut self, tab: SessionToolSelectorTab) {
        self.active_tab = tab;
    }

    /// 执行 query 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    /// 执行 set_query 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn set_query(&mut self, query: String) {
        self.query = query;
    }

    /// 执行 skill_directory_scope 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn skill_directory_scope(&self) -> SkillsDirectoryScope {
        self.skill_directory_scope
    }

    /// 执行 select_skill_directory_scope 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn select_skill_directory_scope(&mut self, scope: SkillsDirectoryScope) {
        self.skill_directory_scope = scope;
    }

    /// 执行 is_bucket_enabled 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn is_bucket_enabled(&self, bucket: SessionToolBucket) -> bool {
        self.enabled_buckets.contains(&bucket)
    }

    /// 执行 reset 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn reset(&mut self) {
        self.enabled_buckets = SessionToolBucket::ALL.into_iter().collect();
        self.explicit_allowed_tools = None;
        self.manual_tools.clear();
        self.manual_skills.clear();
        self.selected_shiju_skills.clear();
        self.query.clear();
    }

    /// 执行 toggle_bucket 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn toggle_bucket(&mut self, bucket: SessionToolBucket) -> bool {
        if self.enabled_buckets.contains(&bucket) {
            if self.enabled_buckets.len() == 1 {
                return false;
            }
            self.enabled_buckets.remove(&bucket);
            return true;
        }
        self.enabled_buckets.insert(bucket);
        true
    }

    /// 执行 has_custom_tool_selection 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    #[cfg(test)]
    pub(crate) fn has_custom_tool_selection(&self) -> bool {
        self.explicit_allowed_tools.is_some()
    }

    pub(crate) fn has_tool_filter(&self) -> bool {
        self.explicit_allowed_tools.is_some()
            || self.enabled_buckets.len() != SessionToolBucket::ALL.len()
    }

    /// 执行 has_manual_context_selection 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn has_manual_context_selection(&self) -> bool {
        !self.manual_tools.is_empty()
            || !self.manual_skills.is_empty()
            || !self.selected_shiju_skills.is_empty()
    }

    pub(crate) fn clear_manual_context_selection(&mut self) {
        self.manual_tools.clear();
        self.manual_skills.clear();
        self.selected_shiju_skills.clear();
    }

    /// 执行 toggle_manual_tool 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn toggle_manual_tool(&mut self, tool_id: &str) {
        let tool_id = tool_id.trim();
        if tool_id.is_empty() {
            return;
        }
        if !self.manual_tools.insert(tool_id.to_string()) {
            self.manual_tools.remove(tool_id);
        }
    }

    /// 执行 is_manual_tool_selected 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn is_manual_tool_selected(&self, tool_id: &str) -> bool {
        self.manual_tools.contains(tool_id)
    }

    /// 执行 selected_manual_tools 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn selected_manual_tools(&self) -> Vec<String> {
        self.manual_tools.iter().cloned().collect()
    }

    /// 执行 toggle_manual_skill 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn toggle_manual_skill(&mut self, skill_id: &str) {
        let skill_id = skill_id.trim();
        if skill_id.is_empty() {
            return;
        }
        if !self.manual_skills.insert(skill_id.to_string()) {
            self.manual_skills.remove(skill_id);
        }
    }

    /// 执行 is_manual_skill_selected 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn is_manual_skill_selected(&self, skill_id: &str) -> bool {
        self.manual_skills.contains(skill_id)
    }

    /// 执行 selected_manual_skills 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn selected_manual_skills(&self) -> Vec<String> {
        self.manual_skills.iter().cloned().collect()
    }

    pub(crate) fn toggle_shiju_skill(&mut self, selection: ShijuSkillSelection) {
        let skill_id = selection.skill_id();
        if self.selected_shiju_skills.remove(&skill_id).is_none() {
            self.selected_shiju_skills.insert(skill_id, selection);
        }
    }

    pub(crate) fn is_shiju_skill_selected(&self, skill_id: u64) -> bool {
        self.selected_shiju_skills.contains_key(&skill_id)
    }

    pub(crate) fn selected_shiju_skills(&self) -> Vec<ShijuSkillSelection> {
        self.selected_shiju_skills.values().cloned().collect()
    }

    /// 执行 select_all_tools 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn select_all_tools(&mut self, tools: &[String]) {
        let bucket_filtered_tools = self.bucket_filtered_tools(tools);
        if bucket_filtered_tools.is_empty() {
            self.explicit_allowed_tools = None;
            return;
        }

        self.explicit_allowed_tools = None;
        self.normalize_explicit_tools(&bucket_filtered_tools);
    }

    /// 执行 invert_tools 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn invert_tools(&mut self, tools: &[String]) -> bool {
        let bucket_filtered_tools = self.bucket_filtered_tools(tools);
        if bucket_filtered_tools.len() <= 1 {
            return false;
        }

        let enabled_tools = self.filter_tools(tools);
        let inverted_tools = bucket_filtered_tools
            .iter()
            .filter(|tool_id| !enabled_tools.iter().any(|candidate| candidate == *tool_id))
            .cloned()
            .collect::<BTreeSet<_>>();

        if inverted_tools.is_empty() {
            return false;
        }

        self.explicit_allowed_tools = Some(inverted_tools);
        self.normalize_explicit_tools(&bucket_filtered_tools);
        true
    }

    /// 执行 reconcile_tools 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn reconcile_tools(&mut self, tools: &[String]) {
        let bucket_filtered_tools = self.bucket_filtered_tools(tools);
        self.normalize_explicit_tools(&bucket_filtered_tools);
    }

    /// 执行 is_group_collapsed 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn is_group_collapsed(&self, group: SessionToolGroup) -> bool {
        self.collapsed_groups.contains(&group)
    }

    /// 执行 toggle_group_collapsed 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn toggle_group_collapsed(&mut self, group: SessionToolGroup) {
        if !self.collapsed_groups.insert(group) {
            self.collapsed_groups.remove(&group);
        }
    }

    fn bucket_filtered_tools(&self, tools: &[String]) -> Vec<String> {
        tools
            .iter()
            .filter(|tool_id| self.is_bucket_enabled(tool_bucket(tool_id)))
            .cloned()
            .collect()
    }

    fn normalize_explicit_tools(&mut self, bucket_filtered_tools: &[String]) {
        let Some(selected_tools) = self.explicit_allowed_tools.as_mut() else {
            return;
        };

        selected_tools
            .retain(|tool_id| bucket_filtered_tools.iter().any(|candidate| candidate == tool_id));

        if selected_tools.is_empty() || selected_tools.len() == bucket_filtered_tools.len() {
            self.explicit_allowed_tools = None;
        }
    }

    /// 执行 available_tools_for_group 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn available_tools_for_group(
        &self,
        tools: &[String],
        group: SessionToolGroup,
    ) -> Vec<String> {
        self.bucket_filtered_tools(tools)
            .into_iter()
            .filter(|tool_id| tool_group(tool_id) == group)
            .collect()
    }

    /// 执行 toggle_tool 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn toggle_tool(&mut self, tools: &[String], tool_id: &str) -> bool {
        let bucket_filtered_tools = self.bucket_filtered_tools(tools);
        if !bucket_filtered_tools.iter().any(|candidate| candidate == tool_id) {
            return true;
        }

        let enabled_tools = self.filter_tools(tools);
        let selected_tools = self
            .explicit_allowed_tools
            .get_or_insert_with(|| bucket_filtered_tools.iter().cloned().collect());

        if selected_tools.contains(tool_id) {
            if enabled_tools.len() == 1 {
                return false;
            }
            selected_tools.remove(tool_id);
        } else {
            selected_tools.insert(tool_id.to_string());
        }

        self.normalize_explicit_tools(&bucket_filtered_tools);
        true
    }

    /// 执行 toggle_group_tools 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn toggle_group_tools(&mut self, tools: &[String], group: SessionToolGroup) -> bool {
        let group_tools = self.available_tools_for_group(tools, group);
        if group_tools.is_empty() {
            return true;
        }

        let bucket_filtered_tools = self.bucket_filtered_tools(tools);
        let enabled_tools = self.filter_tools(tools);
        let enabled_in_group = group_tools
            .iter()
            .filter(|tool_id| enabled_tools.iter().any(|candidate| candidate == *tool_id))
            .count();

        let selected_tools = self
            .explicit_allowed_tools
            .get_or_insert_with(|| bucket_filtered_tools.iter().cloned().collect());

        if enabled_in_group == group_tools.len() {
            if enabled_tools.len() == enabled_in_group {
                return false;
            }
            for tool_id in &group_tools {
                selected_tools.remove(tool_id);
            }
        } else {
            for tool_id in &group_tools {
                selected_tools.insert(tool_id.clone());
            }
        }

        self.normalize_explicit_tools(&bucket_filtered_tools);
        true
    }

    /// 执行 filter_tools 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn filter_tools(&self, tools: &[String]) -> Vec<String> {
        let bucket_filtered_tools = self.bucket_filtered_tools(tools);
        let Some(selected_tools) = self.explicit_allowed_tools.as_ref() else {
            return bucket_filtered_tools;
        };

        let filtered = bucket_filtered_tools
            .iter()
            .filter(|tool_id| selected_tools.contains(tool_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        if filtered.is_empty() && !bucket_filtered_tools.is_empty() {
            bucket_filtered_tools
        } else {
            filtered
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuSkillSelection {
    skill_id: u64,
    slug: String,
    display_name: String,
}

impl ShijuSkillSelection {
    pub(crate) fn new(skill_id: u64, slug: String, display_name: String) -> Self {
        Self { skill_id, slug, display_name }
    }

    pub(crate) fn skill_id(&self) -> u64 {
        self.skill_id
    }

    pub(crate) fn slug(&self) -> &str {
        &self.slug
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }
}

#[derive(Debug, Clone, Default)]
/// 表示 SessionToolInventory 相关的应用状态或派生数据。
pub(crate) struct SessionToolInventory {
    pub(crate) base_tools: Vec<String>,
    pub(crate) restricts_tools: bool,
}

/// 执行 tool_bucket 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn tool_bucket(tool_id: &str) -> SessionToolBucket {
    match tool_id {
        "read" | "file_read" | "read_file" | "pdf_read" | "ls" | "grep" | "content_search"
        | "glob" | "glob_search" | "code_search" | "codesearch" | "lsp" | "memory_recall" => {
            SessionToolBucket::ReadOnly
        }
        "write" | "file_write" | "apply_patch" | "edit" | "edit_file" | "editfile"
        | "file_edit" | "notebook_edit" => SessionToolBucket::Edit,
        "bash" | "shell" | "process" | "git_operations" => SessionToolBucket::Execution,
        "browser" | "browser_open" | "http_request" | "web_fetch" | "fetch_webpage"
        | "web_search" | "websearch" | "web_search_tool" | "screenshot" | "image_info" => {
            SessionToolBucket::Browser
        }
        "AgentTool"
        | "Agent"
        | "agent"
        | "schedule"
        | "cron_add"
        | "cron_list"
        | "cron_remove"
        | "cron_update"
        | "cron_run"
        | "cron_runs"
        | "todoread"
        | "todowrite"
        | "plan_enter"
        | "enter_plan_mode"
        | "plan_exit"
        | "exit_plan_mode"
        | "verify_plan_execution"
        | "enter_worktree"
        | "exit_worktree"
        | "tool_search"
        | "question"
        | "skill"
        | "memory_store"
        | "memory_forget" => SessionToolBucket::Agent,
        _ => SessionToolBucket::Other,
    }
}

/// 执行 tool_group 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn tool_group(tool_id: &str) -> SessionToolGroup {
    match tool_id {
        "read" | "file_read" | "pdf_read" | "ls" | "write" | "file_write" => {
            SessionToolGroup::Files
        }
        "grep" | "content_search" | "glob" | "glob_search" | "code_search" | "codesearch"
        | "lsp" => SessionToolGroup::Search,
        "apply_patch" | "edit" | "edit_file" | "editfile" | "file_edit" | "notebook_edit"
        | "bash" | "shell" | "process" | "git_operations" => SessionToolGroup::Execute,
        "browser" | "browser_open" | "http_request" | "web_fetch" | "fetch_webpage"
        | "web_search" | "websearch" | "web_search_tool" | "screenshot" | "image_info" => {
            SessionToolGroup::Web
        }
        "AgentTool"
        | "Agent"
        | "agent"
        | "question"
        | "skill"
        | "schedule"
        | "cron_add"
        | "cron_list"
        | "cron_remove"
        | "cron_update"
        | "cron_run"
        | "cron_runs"
        | "todoread"
        | "todowrite"
        | "plan_enter"
        | "enter_plan_mode"
        | "plan_exit"
        | "exit_plan_mode"
        | "verify_plan_execution"
        | "enter_worktree"
        | "exit_worktree" => SessionToolGroup::Collaboration,
        "memory_recall" | "memory_store" | "memory_forget" => SessionToolGroup::Memory,
        "tool_search"
        | "proxy_config"
        | "model_routing_config"
        | "pushover"
        | "batch"
        | "wasm_module" => SessionToolGroup::Integration,
        _ => SessionToolGroup::Other,
    }
}

/// 执行 tool_display_name 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn tool_display_name(tool_id: &str) -> String {
    match tool_id {
        "read" | "file_read" => "读取文件".to_string(),
        "pdf_read" => "读取文档".to_string(),
        "ls" => "目录列表".to_string(),
        "write" | "file_write" => "写入文件".to_string(),
        "apply_patch" => "补丁编辑".to_string(),
        "grep" => "文本搜索".to_string(),
        "content_search" => "内容搜索".to_string(),
        "glob" => "文件匹配".to_string(),
        "glob_search" => "组合搜索".to_string(),
        "code_search" | "codesearch" => "代码检索".to_string(),
        "bash" | "shell" => "命令行".to_string(),
        "process" => "后台进程".to_string(),
        "git_operations" => "Git 操作".to_string(),
        "browser" => "浏览器交互".to_string(),
        "browser_open" => "打开页面".to_string(),
        "http_request" => "网络请求".to_string(),
        "web_fetch" => "网页抓取".to_string(),
        "web_search" | "websearch" | "web_search_tool" => "联网搜索".to_string(),
        "AgentTool" | "Agent" | "agent" => "委托代理".to_string(),
        "question" => "用户提问".to_string(),
        "skill" => "加载技能".to_string(),
        "memory_recall" => "读取记忆".to_string(),
        "memory_store" => "写入记忆".to_string(),
        "memory_forget" => "删除记忆".to_string(),
        "tool_search" => "工具搜索".to_string(),
        _ => tool_id
            .split(['_', '-'])
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => {
                        let mut word = first.to_uppercase().to_string();
                        word.push_str(chars.as_str());
                        word
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}
