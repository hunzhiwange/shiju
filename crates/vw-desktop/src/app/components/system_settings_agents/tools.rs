//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::shared::{is_dark_theme, section_card};
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style, settings_panel_style,
    settings_segment_button_style, settings_value_badge,
};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{
    AGENT_TOOLS_BUILTIN_TAB, AGENT_TOOLS_OPTIONAL_TAB, DelegateAgentSettingsEntry,
};
use crate::app::{App, Message};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use vw_config_types::agent::DelegateCapabilityPolicy;
use vw_config_types::tools::is_builtin_required_tool_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolGroup {
    Files,
    Search,
    Execute,
    Web,
    Collaboration,
    Memory,
    Integration,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ToolMeta {
    pub(super) name: &'static str,
    pub(super) description: &'static str,
    pub(super) group: ToolGroup,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ToolGroupMeta {
    pub(super) group: ToolGroup,
    pub(super) label: &'static str,
    pub(super) description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ToolPresetMeta {
    pub(super) key: &'static str,
    pub(super) label: &'static str,
    pub(super) description: &'static str,
}

pub(super) fn tool_card_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_selected: bool,
) -> iced::widget::button::Style {
    let panel_style = settings_panel_style(theme);
    let segment_style = settings_segment_button_style(theme, status, is_selected);
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    let background = if is_selected {
        segment_style.background
    } else {
        match status {
            iced::widget::button::Status::Hovered => Some(Background::Color(if is_dark {
                palette.background.weak.color.scale_alpha(0.30)
            } else {
                Color::WHITE.scale_alpha(0.88)
            })),
            iced::widget::button::Status::Pressed => Some(Background::Color(if is_dark {
                palette.background.strong.color.scale_alpha(0.82)
            } else {
                palette.background.weak.color.scale_alpha(0.92)
            })),
            _ => panel_style.background,
        }
    };

    let border_color = if is_selected {
        segment_style.border.color
    } else if is_dark {
        palette.background.strong.color.scale_alpha(0.78)
    } else {
        panel_style.border.color
    };

    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { width: 1.0, color: border_color, radius: 16.0.into() },
        ..Default::default()
    }
}

pub(super) fn tool_matches_any(tool_id: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| tool_id == *candidate)
}

pub(super) fn tool_group_meta(group: ToolGroup) -> ToolGroupMeta {
    match group {
        ToolGroup::Files => {
            ToolGroupMeta {
                group, label: "文件", description: "读取、写入和修改工作区文件。"
            }
        }
        ToolGroup::Search => {
            ToolGroupMeta {
                group, label: "搜索", description: "查找文件、文本和代码位置。"
            }
        }
        ToolGroup::Execute => {
            ToolGroupMeta {
                group, label: "执行", description: "运行命令、补丁和版本控制操作。"
            }
        }
        ToolGroup::Web => {
            ToolGroupMeta {
                group, label: "联网", description: "浏览页面、联网请求与外部检索。"
            }
        }
        ToolGroup::Collaboration => {
            ToolGroupMeta {
                group, label: "协作", description: "委托子代理、调度计划和任务流转。"
            }
        }
        ToolGroup::Memory => {
            ToolGroupMeta {
                group, label: "记忆", description: "读写长期记忆和上下文数据。"
            }
        }
        ToolGroup::Integration => ToolGroupMeta {
            group,
            label: "系统",
            description: "代理路由、集成能力与系统辅助工具。",
        },
        ToolGroup::Other => {
            ToolGroupMeta {
                group, label: "其他", description: "当前未归类的可调用工具。"
            }
        }
    }
}

pub(super) fn tool_group_order() -> [ToolGroup; 8] {
    [
        ToolGroup::Files,
        ToolGroup::Search,
        ToolGroup::Execute,
        ToolGroup::Web,
        ToolGroup::Collaboration,
        ToolGroup::Memory,
        ToolGroup::Integration,
        ToolGroup::Other,
    ]
}

pub(super) fn tool_preset_meta() -> [ToolPresetMeta; 5] {
    [
        ToolPresetMeta {
            key: "minimal", label: "最小", description: "只保留只读分析能力。"
        },
        ToolPresetMeta {
            key: "coding",
            label: "编码",
            description: "补齐读写、编辑、命令和 Git。",
        },
        ToolPresetMeta {
            key: "research",
            label: "研究",
            description: "补齐浏览器、联网和页面取证。",
        },
        ToolPresetMeta {
            key: "collab", label: "协作", description: "补齐委托、调度和记忆写入。"
        },
        ToolPresetMeta {
            key: "full", label: "全部", description: "启用当前可见的全部工具。"
        },
    ]
}

pub(super) fn tool_in_preset(tool_id: &str, preset_key: &str) -> bool {
    match preset_key {
        "minimal" => tool_matches_any(
            tool_id,
            &[
                "ls",
                "read",
                "file_read",
                "pdf_read",
                "grep",
                "glob",
                "lsp",
                "memory_recall",
                "question",
            ],
        ),
        "coding" => {
            tool_in_preset(tool_id, "minimal")
                || tool_matches_any(
                    tool_id,
                    &["write", "file_write", "apply_patch", "bash", "shell", "git_operations"],
                )
        }
        "research" => {
            tool_in_preset(tool_id, "minimal")
                || tool_matches_any(
                    tool_id,
                    &[
                        "browser",
                        "browser_open",
                        "http_request",
                        "web_fetch",
                        "web_search",
                        "websearch",
                        "web_search_tool",
                        "screenshot",
                        "image_info",
                    ],
                )
        }
        "collab" => {
            tool_in_preset(tool_id, "minimal")
                || tool_matches_any(
                    tool_id,
                    &[
                        "AgentTool",
                        "delegate_coordination_status",
                        "schedule",
                        "cron_add",
                        "cron_list",
                        "cron_remove",
                        "cron_update",
                        "cron_run",
                        "cron_runs",
                        "memory_store",
                        "memory_forget",
                        "todoread",
                        "todowrite",
                        "plan_enter",
                        "plan_exit",
                    ],
                )
        }
        "full" => true,
        _ => false,
    }
}

pub(super) fn preset_tool_count(available_tools: &[String], preset_key: &str) -> usize {
    available_tools.iter().filter(|tool_id| tool_in_preset(tool_id, preset_key)).count()
}

/// 按工具列表子 Tab 过滤可见工具：可选不含内置，内置仅含必选目录。
pub(super) fn tools_for_list_tab<'a>(available_tools: &'a [String], tab: &str) -> Vec<&'a str> {
    available_tools
        .iter()
        .map(String::as_str)
        .filter(|tool_id| {
            let is_builtin = is_builtin_required_tool_id(tool_id);
            if tab == AGENT_TOOLS_BUILTIN_TAB { is_builtin } else { !is_builtin }
        })
        .collect()
}

pub(super) fn tool_card_is_locked(tool_id: &str) -> bool {
    is_builtin_required_tool_id(tool_id)
}

pub(super) fn builtin_tool_badge_label() -> &'static str {
    "内置"
}

pub(super) fn capability_policy_label(policy: DelegateCapabilityPolicy) -> &'static str {
    match policy {
        DelegateCapabilityPolicy::Strict => "严格限制",
        DelegateCapabilityPolicy::UserOverride => "限制但允许用户选择",
        DelegateCapabilityPolicy::Unrestricted => "不限制",
    }
}

pub(super) fn tool_policy_message(agent_key: String, policy: DelegateCapabilityPolicy) -> Message {
    Message::Settings(SettingsMessage::Agents(AgentsMessage::ToolPolicyChanged(agent_key, policy)))
}

fn tool_policy_button(
    entry: &DelegateAgentSettingsEntry,
    policy: DelegateCapabilityPolicy,
) -> Element<'static, Message> {
    let selected = entry.tool_policy == policy;
    button(text(capability_policy_label(policy)).size(12))
        .padding([8, 12])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, selected))
        .on_press(tool_policy_message(entry.key.clone(), policy))
        .into()
}

pub(super) fn tool_card_press_message(
    agent_key: String,
    tool_id: String,
    currently_checked: bool,
    locked: bool,
) -> Option<Message> {
    if locked {
        None
    } else {
        Some(Message::Settings(SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
            agent_key,
            tool_id,
            !currently_checked,
        ))))
    }
}

fn tools_list_tab_button<'a>(
    label: &'a str,
    key: &'a str,
    active_key: &'a str,
) -> Element<'a, Message> {
    let is_active = key == active_key;
    button(text(label).size(14))
        .padding([10, 18])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
        .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::ToolsListTabSelected(
            key.to_string(),
        ))))
        .into()
}

pub(super) fn tool_meta(tool_id: &str) -> ToolMeta {
    match tool_id {
        "read" | "file_read" => ToolMeta {
            name: "读取文件",
            description: "按路径读取文件，返回带行号的内容片段与元数据；支持分页，PDF 会抽取文本。",
            group: ToolGroup::Files,
        },
        "pdf_read" => ToolMeta {
            name: "读取文档",
            description: "从工作区 PDF 中提取纯文本，可限制返回字符数；纯图片或加密 PDF 可能为空。",
            group: ToolGroup::Files,
        },
        "ls" => ToolMeta {
            name: "目录列表",
            description: "列出目录树结构，支持忽略模式过滤，适合快速查看项目层级。",
            group: ToolGroup::Files,
        },
        "write" | "file_write" => ToolMeta {
            name: "写入文件",
            description: "向目标路径写入完整内容；已有文件会整体覆盖，通常要求先读取再写入。",
            group: ToolGroup::Files,
        },
        "apply_patch" => ToolMeta {
            name: "补丁编辑",
            description: "按补丁块原子地新增、更新或删除文件，并校验路径、格式和安全策略。",
            group: ToolGroup::Execute,
        },
        "grep" => ToolMeta {
            name: "文本搜索",
            description: "用正则快速搜索文件内容，适合先定位命中文件再继续分析。",
            group: ToolGroup::Search,
        },
        "content_search" => ToolMeta {
            name: "内容搜索",
            description: "在工作区内按正则搜索内容，支持返回匹配行、命中文件或计数统计。",
            group: ToolGroup::Search,
        },
        "glob" => ToolMeta {
            name: "文件匹配",
            description: "按 glob 模式匹配文件路径，并按最近修改时间排序返回结果。",
            group: ToolGroup::Search,
        },
        "glob_search" => ToolMeta {
            name: "组合搜索",
            description: "先按 glob 过滤文件，再在命中文件集中继续搜索内容。",
            group: ToolGroup::Search,
        },
        "code_search" | "codesearch" => ToolMeta {
            name: "代码检索",
            description: "调用语义代码搜索服务，返回与查询最相关的代码片段和上下文。",
            group: ToolGroup::Search,
        },
        "bash" | "shell" => ToolMeta {
            name: "命令行",
            description: "在受限工作目录执行 Bash 命令，支持超时、审批和风险控制。",
            group: ToolGroup::Execute,
        },
        "PowerShell" | "powershell" => ToolMeta {
            name: "PowerShell",
            description: "在 Windows 上执行 PowerShell 命令，受工作区与安全策略约束。",
            group: ToolGroup::Execute,
        },
        "process" => ToolMeta {
            name: "后台进程",
            description: "启动长时间运行的后台命令、查看输出并终止进程。",
            group: ToolGroup::Execute,
        },
        "git_operations" => ToolMeta {
            name: "Git 操作",
            description: "以结构化参数执行 status、diff、log、add、commit、checkout、stash 等操作。",
            group: ToolGroup::Execute,
        },
        "notebook_edit" | "NotebookEdit" => ToolMeta {
            name: "编辑笔记本",
            description: "按 cell 插入、替换或删除已有 .ipynb；编辑前需先用 file_read 读取。",
            group: ToolGroup::Files,
        },
        "browser" | "Browser" => ToolMeta {
            name: "浏览器交互",
            description: "在受控浏览器会话中打开页面、抓取快照、点击、输入、滚动和截图。",
            group: ToolGroup::Web,
        },
        "browser_open" | "BrowserOpen" => ToolMeta {
            name: "打开页面",
            description: "在系统默认浏览器中打开已批准的 HTTPS 链接；不负责 DOM 交互。",
            group: ToolGroup::Web,
        },
        "http_request" => ToolMeta {
            name: "网络请求",
            description: "向外部 API 发起 HTTP 请求，支持 method、headers 和 body，并受域名白名单约束。",
            group: ToolGroup::Web,
        },
        "web_fetch" | "WebFetch" | "webfetch" => ToolMeta {
            name: "网页抓取",
            description: "抓取网页正文并转成 text、markdown 或 html，适合读取页面内容。",
            group: ToolGroup::Web,
        },
        "web_search" | "websearch" | "web_search_tool" | "WebSearch" => ToolMeta {
            name: "联网搜索",
            description: "执行互联网搜索并返回结果摘要，适合找外部资料和线索。",
            group: ToolGroup::Web,
        },
        "AgentTool" => ToolMeta {
            name: "委托子代理",
            description: "把复杂工作交给持久子代理：launch/spawn 并行启动，run 同步等待，status/wait/stop 管理任务与结果。",
            group: ToolGroup::Collaboration,
        },
        "delegate" => ToolMeta {
            name: "委托执行",
            description: "把子任务交给专门智能体；默认单次提示，agentic=true 时可带过滤工具循环迭代。",
            group: ToolGroup::Collaboration,
        },
        "delegate_coordination_status" => ToolMeta {
            name: "委托状态",
            description: "检查委托协调运行时状态，包括收件箱积压、上下文状态和死信记录。",
            group: ToolGroup::Collaboration,
        },
        "schedule" => ToolMeta {
            name: "任务调度",
            description: "创建、列出、暂停、恢复或取消基于 Shell 的计划任务。",
            group: ToolGroup::Collaboration,
        },
        "cron_add" => ToolMeta {
            name: "新增定时任务",
            description: "添加新的 cron 任务，可配置执行表达式、任务类型和投递行为。",
            group: ToolGroup::Collaboration,
        },
        "cron_list" => ToolMeta {
            name: "定时任务列表",
            description: "列出当前所有已配置的 cron 任务。",
            group: ToolGroup::Collaboration,
        },
        "cron_remove" => ToolMeta {
            name: "删除定时任务",
            description: "按任务 ID 移除已有 cron 任务。",
            group: ToolGroup::Collaboration,
        },
        "cron_update" => ToolMeta {
            name: "更新定时任务",
            description: "修改已有 cron 任务的配置、表达式或执行内容。",
            group: ToolGroup::Collaboration,
        },
        "cron_run" => ToolMeta {
            name: "执行定时任务",
            description: "立即触发一次已有 cron 任务，方便手动验证。",
            group: ToolGroup::Collaboration,
        },
        "cron_runs" => ToolMeta {
            name: "运行历史",
            description: "查看指定 cron 任务最近的运行记录和输出摘要。",
            group: ToolGroup::Collaboration,
        },
        "todoread" | "TodoRead" => ToolMeta {
            name: "读取任务计划",
            description: "读取当前会话的任务清单与执行状态，返回规范化 JSON。",
            group: ToolGroup::Collaboration,
        },
        "todowrite" | "TodoWrite" => ToolMeta {
            name: "写入任务计划",
            description: "覆盖或合并当前会话任务清单，用于持续更新执行进度。",
            group: ToolGroup::Collaboration,
        },
        "plan_enter" | "EnterPlanMode" | "enter_plan_mode" => ToolMeta {
            name: "进入规划模式",
            description: "进入显式规划工作流，开始先列计划再执行。",
            group: ToolGroup::Collaboration,
        },
        "plan_exit" | "ExitPlanMode" | "exit_plan_mode" => ToolMeta {
            name: "退出规划模式",
            description: "结束规划模式并回到正常执行流程；默认会先检查是否满足进入执行的最小条件。",
            group: ToolGroup::Collaboration,
        },
        "enter_worktree" | "EnterWorktree" => ToolMeta {
            name: "进入 Worktree",
            description: "创建并绑定真实 Git worktree，后续工具调用切换到该隔离工作区。",
            group: ToolGroup::Execute,
        },
        "exit_worktree" | "ExitWorktree" => ToolMeta {
            name: "退出 Worktree",
            description: "退出当前绑定的 worktree；默认要求工作树干净，force 可强制移除。",
            group: ToolGroup::Execute,
        },
        "question" | "AskUserQuestion" => ToolMeta {
            name: "用户提问",
            description: "向用户发起澄清问题，支持单选、多选和自定义输入。",
            group: ToolGroup::Collaboration,
        },
        "Brief" | "brief" => ToolMeta {
            name: "发送简报",
            description: "向用户发送一条可见消息，可附带本地文件附件。",
            group: ToolGroup::Collaboration,
        },
        "Sleep" | "sleep" => ToolMeta {
            name: "短暂等待",
            description: "暂停执行一小段时间，适合等待外部状态收敛或短暂退避。",
            group: ToolGroup::Collaboration,
        },
        "SendMessage" | "send_message" => ToolMeta {
            name: "发送 IPC 消息",
            description: "向单个 agent、团队成员或全部在线 agent 发送 IPC 消息。",
            group: ToolGroup::Integration,
        },
        "SendUserFile" | "send_user_file" => ToolMeta {
            name: "准备用户附件",
            description: "为当前会话准备可发送给用户的本地文件附件标记。",
            group: ToolGroup::Collaboration,
        },
        "TeamCreate" | "team_create" => ToolMeta {
            name: "创建团队",
            description: "创建或覆盖一个团队对象，并定义成员边界。",
            group: ToolGroup::Integration,
        },
        "TeamDelete" | "team_delete" => ToolMeta {
            name: "删除团队",
            description: "删除一个已存在的团队对象。",
            group: ToolGroup::Integration,
        },
        "skill" => ToolMeta {
            name: "加载技能",
            description: "把指定技能内容注入当前上下文，适合临时加载专项工作流或知识。",
            group: ToolGroup::Collaboration,
        },
        "memory_recall" => ToolMeta {
            name: "读取记忆",
            description: "在长期记忆中按相关性检索事实、偏好和上下文。",
            group: ToolGroup::Memory,
        },
        "memory_store" => ToolMeta {
            name: "写入记忆",
            description: "把事实、偏好或笔记写入长期记忆，并按类别归档。",
            group: ToolGroup::Memory,
        },
        "memory_forget" => ToolMeta {
            name: "删除记忆",
            description: "按键删除记忆，适合清理过时事实或敏感内容。",
            group: ToolGroup::Memory,
        },
        "proxy_config" => ToolMeta {
            name: "代理配置",
            description: "管理环境、VibeWindow 和服务级代理设置，并支持应用或清理环境变量。",
            group: ToolGroup::Integration,
        },
        "model_routing_config" => ToolMeta {
            name: "模型路由",
            description: "管理默认模型、场景路由规则和委托子代理的 provider/model 配置。",
            group: ToolGroup::Integration,
        },
        "Config" => ToolMeta {
            name: "配置管理",
            description: "读取或调整运行时配置项，用于会话内配置查询与变更。",
            group: ToolGroup::Integration,
        },
        "composio" => ToolMeta {
            name: "外部集成",
            description: "通过 Composio 调用 Gmail、Notion、GitHub、Slack 等外部应用操作。",
            group: ToolGroup::Integration,
        },
        "pushover" => ToolMeta {
            name: "推送通知",
            description: "向 Pushover 设备发送通知，支持标题、优先级和提示音。",
            group: ToolGroup::Integration,
        },
        "screenshot" => ToolMeta {
            name: "屏幕截图",
            description: "截取当前屏幕并返回文件路径及 Base64 PNG 数据。",
            group: ToolGroup::Web,
        },
        "image_info" => ToolMeta {
            name: "图像信息",
            description: "读取图像格式、尺寸、大小等元数据，并可选返回 Base64 数据。",
            group: ToolGroup::Web,
        },
        "image_asset_read" => ToolMeta {
            name: "重载图像资源",
            description: "按 image_id 请求在下一步模型调用中重新加载已存储图像；不直接返回图像字节。",
            group: ToolGroup::Web,
        },
        "file_edit" | "edit" => ToolMeta {
            name: "编辑文件",
            description: "对已有文件做局部编辑，适合小范围精确修改。",
            group: ToolGroup::Files,
        },
        "tool_search" | "ToolSearch" => ToolMeta {
            name: "工具搜索",
            description: "搜索当前上下文中真实可见的工具，并解释每个结果为何命中。",
            group: ToolGroup::Integration,
        },
        "verify_plan_execution" | "VerifyPlanExecution" => ToolMeta {
            name: "验证计划执行",
            description: "校验当前计划模式是否已满足进入执行阶段的最小条件。",
            group: ToolGroup::Collaboration,
        },
        "lsp" => ToolMeta {
            name: "语义分析",
            description: "实验性语言服务入口，用于文件级语义分析与导航，当前仍在开发中。",
            group: ToolGroup::Integration,
        },
        "ListMcpResources" | "list_mcp_resources" => ToolMeta {
            name: "MCP 资源列表",
            description: "列出当前会话可真实连通的 MCP 服务器资源。",
            group: ToolGroup::Integration,
        },
        "ListMcpResourceTemplates" | "list_mcp_resource_templates" => ToolMeta {
            name: "MCP 资源模板",
            description: "列出当前会话可真实连通的 MCP 服务器资源模板。",
            group: ToolGroup::Integration,
        },
        "ReadMcpResource" | "read_mcp_resource" => ToolMeta {
            name: "读取 MCP 资源",
            description: "读取指定 MCP 资源的真实内容。",
            group: ToolGroup::Integration,
        },
        "ListMcpPrompts" | "list_mcp_prompts" => ToolMeta {
            name: "MCP Prompt 列表",
            description: "列出当前会话可真实连通的 MCP 服务器 prompts。",
            group: ToolGroup::Integration,
        },
        "GetMcpPrompt" | "get_mcp_prompt" => ToolMeta {
            name: "获取 MCP Prompt",
            description: "按 server 与 prompt 名称获取 MCP prompt 消息。",
            group: ToolGroup::Integration,
        },
        "McpAuth" | "mcp_auth" => ToolMeta {
            name: "MCP 认证信息",
            description: "读取当前会话的 MCP 认证桥接信息，包括策略、认证方法和目标 server。",
            group: ToolGroup::Integration,
        },
        "McpStatus" | "mcp_status" => ToolMeta {
            name: "MCP 状态",
            description: "读取 MCP server 的启用状态、连接状态、工具数量和认证方法摘要。",
            group: ToolGroup::Integration,
        },
        "RemoteTrigger" | "remote_trigger" => ToolMeta {
            name: "远端触发",
            description: "触发远端任务或环境动作；稳定后端未接入时会显式报错。",
            group: ToolGroup::Integration,
        },
        "invalid" => ToolMeta {
            name: "无效调用",
            description: "工具错误回退入口，用于报告无效或未识别的工具请求。",
            group: ToolGroup::Other,
        },
        "agents_list" => ToolMeta {
            name: "代理列表",
            description: "列出 IPC 代理及其运行状态。",
            group: ToolGroup::Integration,
        },
        "agents_send" => ToolMeta {
            name: "发送代理消息",
            description: "向指定 IPC 代理发送消息或任务。",
            group: ToolGroup::Integration,
        },
        "agents_inbox" => ToolMeta {
            name: "代理收件箱",
            description: "查看 IPC 代理的收件箱消息。",
            group: ToolGroup::Integration,
        },
        "state_get" => ToolMeta {
            name: "读取状态",
            description: "读取共享状态存储中的键值。",
            group: ToolGroup::Integration,
        },
        "state_set" => ToolMeta {
            name: "写入状态",
            description: "写入共享状态存储中的键值。",
            group: ToolGroup::Integration,
        },
        "batch" => ToolMeta {
            name: "批处理",
            description: "将多个工具调用打包在一次批处理请求中执行。",
            group: ToolGroup::Integration,
        },
        "wasm_module" => ToolMeta {
            name: "WASM 模块",
            description: "调用已注册的 WebAssembly 模块能力。",
            group: ToolGroup::Integration,
        },
        _ => ToolMeta {
            name: "未分类工具",
            description: "尚未收录该工具的专用说明；请结合下方工具 ID，或在运行时工具实现中查看其 description。",
            group: ToolGroup::Other,
        },
    }
}

pub(super) fn tool_english_name(tool_id: &str) -> String {
    tool_id
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
        .join(" ")
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn view<'a>(
    app: &'a App,
    entry: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let settings = &app.agents_settings;
    let active_tab = settings.active_tools_list_tab.as_str();
    let is_builtin_tab = active_tab == AGENT_TOOLS_BUILTIN_TAB;
    let visible_tool_ids = tools_for_list_tab(&settings.available_tools, active_tab);
    let visible_tools: Vec<String> = visible_tool_ids.iter().map(|id| (*id).to_string()).collect();

    let tools_list: Element<'_, Message> = if settings.available_tools.is_empty() {
        text("暂无可用工具列表").size(12).into()
    } else {
        let tabs_row = row![
            tools_list_tab_button("可选", AGENT_TOOLS_OPTIONAL_TAB, active_tab),
            tools_list_tab_button("内置", AGENT_TOOLS_BUILTIN_TAB, active_tab),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let group_sections = tool_group_order()
            .into_iter()
            .filter_map(|group| {
                let group_meta = tool_group_meta(group);
                let cards = visible_tools
                    .iter()
                    .filter(|tool_id| tool_meta(tool_id).group == group_meta.group)
                    .map(|tool_id| {
                        let locked = is_builtin_tab || tool_card_is_locked(tool_id);
                        let checked =
                            locked || entry.allowed_tools.iter().any(|tool| tool == tool_id);
                        let agent_key = entry.key.clone();
                        let tool_id = tool_id.clone();
                        let meta = tool_meta(&tool_id);
                        let fallback_name = tool_english_name(&tool_id);
                        let display_name = if meta.name == "未分类工具" {
                            fallback_name
                        } else {
                            meta.name.to_string()
                        };

                        let title: Element<'_, Message> = if locked {
                            row![
                                text(display_name)
                                    .size(14)
                                    .font(iced::Font {
                                        weight: iced::font::Weight::Bold,
                                        ..Default::default()
                                    })
                                    .style(move |theme: &Theme| iced::widget::text::Style {
                                        color: Some(theme.palette().text.scale_alpha(0.98)),
                                    }),
                                settings_value_badge(builtin_tool_badge_label()),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .into()
                        } else {
                            text(display_name)
                                .size(14)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .style(move |theme: &Theme| iced::widget::text::Style {
                                    color: Some(theme.palette().text.scale_alpha(if checked {
                                        0.98
                                    } else {
                                        0.94
                                    })),
                                })
                                .into()
                        };

                        let mut card = button(
                            column![
                                title,
                                text(meta.description).size(12).style(move |theme: &Theme| {
                                    iced::widget::text::Style {
                                        color: Some(theme.palette().text.scale_alpha(if checked {
                                            0.72
                                        } else {
                                            0.66
                                        })),
                                    }
                                }),
                                text(tool_id.clone()).size(11).style(move |theme: &Theme| {
                                    iced::widget::text::Style {
                                        color: Some(theme.palette().text.scale_alpha(if checked {
                                            0.56
                                        } else {
                                            0.46
                                        })),
                                    }
                                }),
                            ]
                            .spacing(6)
                            .width(Length::Fill),
                        )
                        .padding([10, 14])
                        .width(Length::Fixed(220.0))
                        .style(move |theme: &Theme, status| {
                            tool_card_button_style(theme, status, checked)
                        });

                        if let Some(message) =
                            tool_card_press_message(agent_key, tool_id, checked, locked)
                        {
                            card = card.on_press(message);
                        }

                        card.into()
                    })
                    .collect::<Vec<Element<'_, Message>>>();

                if cards.is_empty() {
                    None
                } else {
                    Some(
                        column![
                            row![
                                text(group_meta.label).size(14),
                                settings_value_badge(format!("{} 项", cards.len())),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                            text(group_meta.description).size(12).style(settings_muted_text_style),
                            row(cards).spacing(12).wrap(),
                        ]
                        .spacing(10)
                        .into(),
                    )
                }
            })
            .collect::<Vec<Element<'_, Message>>>();

        let groups = column(group_sections).spacing(16);

        if is_builtin_tab {
            column![tabs_row, groups].spacing(12).into()
        } else {
            let selected_optional = visible_tools
                .iter()
                .filter(|tool_id| entry.allowed_tools.iter().any(|tool| tool == *tool_id))
                .count();

            let preset_row = row(tool_preset_meta()
                .into_iter()
                .map(|preset| {
                    let count = preset_tool_count(&visible_tools, preset.key);
                    let is_useful = count > 0;
                    let agent_key = entry.key.clone();

                    let button = button(
                        column![
                            text(preset.label).size(14),
                            text(format!("{} · {} 项", preset.description, count))
                                .size(11)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(4),
                    )
                    .padding([10, 14])
                    .style(rounded_action_btn_style);

                    let button = if is_useful {
                        button.on_press(Message::Settings(SettingsMessage::Agents(
                            AgentsMessage::AllowedToolsApplyPreset(
                                agent_key,
                                preset.key.to_string(),
                            ),
                        )))
                    } else {
                        button
                    };

                    button.into()
                })
                .collect::<Vec<Element<'_, Message>>>())
            .spacing(10)
            .wrap();

            column![
                tabs_row,
                row![
                    button(text("全选").size(12))
                        .padding([8, 14])
                        .on_press(Message::Settings(SettingsMessage::Agents(
                            AgentsMessage::AllowedToolsSelectAll(entry.key.clone()),
                        )))
                        .style(rounded_action_btn_style),
                    button(text("反选").size(12))
                        .padding([8, 14])
                        .on_press(Message::Settings(SettingsMessage::Agents(
                            AgentsMessage::AllowedToolsInvertSelection(entry.key.clone()),
                        )))
                        .style(rounded_action_btn_style),
                    settings_value_badge(format!(
                        "已选 {}/{}",
                        selected_optional,
                        visible_tools.len(),
                    )),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                section_card("快捷预选", "按当前可用工具自动套用常见权限组合。"),
                preset_row,
                groups,
            ]
            .spacing(12)
            .into()
        }
    };

    column![
        section_card("默认工具", "选择会在委托代理首步默认注入的工具；它们不是工具白名单。",),
        column![
            text("工具访问策略").size(14),
            text("控制委托代理可见工具的范围；不限制为默认策略。")
                .size(12)
                .style(settings_muted_text_style),
            row![
                tool_policy_button(entry, DelegateCapabilityPolicy::Strict),
                tool_policy_button(entry, DelegateCapabilityPolicy::UserOverride),
                tool_policy_button(entry, DelegateCapabilityPolicy::Unrestricted),
            ]
            .spacing(8)
            .wrap(),
        ]
        .spacing(8),
        tools_list,
    ]
    .spacing(14)
    .into()
}
