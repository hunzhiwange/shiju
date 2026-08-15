//! # 组件模块
//!
//! 本模块是 VibeWindow 应用程序 UI 组件的统一导出入口。
//!
//! ## 模块概述
//!
//! 该模块汇总并导出所有 UI 组件，包括：
//! - **编辑器组件**：主编辑器、Markdown 编辑器、编辑器工具栏
//! - **面板组件**：聊天面板、文件树、Git 面板、预览面板、搜索面板、终端面板
//! - **设置组件**：完整的系统设置模块集合（代理、模型、安全、调度等）
//! - **交互组件**：通知、对话框、覆盖层
//! - **辅助组件**：输入面板、动画文本、提及高亮器、标签栏、顶部栏
//!
//! ## 设计原则
//!
//! - 模块化：每个组件独立封装，职责单一
//! - 可复用：组件设计为可在不同上下文中复用
//! - 可组合：组件之间通过明确定义的接口进行组合
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use crate::app::components::{
//!     chat_panel::ChatPanel,
//!     editor::Editor,
//!     system_settings::SystemSettings,
//! };
//! ```

/// 关于对话框组件
///
/// 提供应用程序版本信息、许可协议和致谢信息的模态对话框。
pub mod about_modal;

/// 动画文本组件
///
/// 提供文本动画效果，用于增强用户界面的视觉反馈。
pub mod animated_text;

/// 运行状态动画辅助组件
pub mod status_animation;

#[cfg(test)]
#[path = "status_animation_tests.rs"]
mod status_animation_tests;

/// 小宠物任务提示组件
pub mod task_pet;

/// 聊天面板组件
///
/// 核心交互界面，用于用户与 AI 代理之间的对话交互。
/// 支持消息显示、滚动历史、消息格式化等功能。
pub mod chat_panel;

/// 代码审查组件
///
/// 提供代码差异查看、审查意见输入、代码标注等代码审查功能。
pub mod code_review;
#[cfg(test)]
#[path = "code_review_tests.rs"]
mod code_review_tests;

/// 通用颜色选择器组件
pub mod color_picker;

/// 通用数字输入组件
pub mod number_input;

/// SVG path 解析到 Iced canvas path 的轻量 helper
pub mod svg_path;

/// 主编辑器组件
///
/// 应用程序的核心编辑器，支持代码编辑、语法高亮、
/// 自动补全等基础编辑功能。
pub mod editor;
#[cfg(test)]
#[path = "editor_tests.rs"]
mod editor_tests;

/// 编辑器工具栏组件
///
/// 提供编辑器的工具按钮栏，包括格式化、保存、撤销等常用操作。
pub mod editor_toolbar;
#[cfg(test)]
#[path = "editor_toolbar_tests.rs"]
mod editor_toolbar_tests;

/// 文件树组件
///
/// 展示项目目录结构的树形视图，支持文件浏览、
/// 新建/删除/重命名文件等操作。
pub mod file_tree;

/// Git 面板组件
///
/// 提供版本控制相关功能，包括查看变更、提交、分支管理等 Git 操作界面。
pub mod git_panel;

/// 输入提及高亮器组件
///
/// 为输入框中的 @提及、#标签等特殊语法提供高亮显示。
pub mod input_mention_highlighter;
#[cfg(test)]
#[path = "input_mention_highlighter_tests.rs"]
mod input_mention_highlighter_tests;

/// 输入面板组件
///
/// 用户输入界面，支持文本输入、多行编辑、快捷键等功能。
pub mod input_panel;

/// 安装 CLI 对话框组件
///
/// 引导用户安装和配置命令行工具的模态对话框。
pub mod install_cli_modal;
#[cfg(test)]
#[path = "install_cli_modal_tests.rs"]
mod install_cli_modal_tests;

/// Markdown 编辑器组件
///
/// 专门用于编辑 Markdown 文档的编辑器，支持实时预览、
/// Markdown 语法高亮和快捷操作。
pub mod markdown_editor;

/// 模型悬浮提示组件
///
/// 提供模型选择面板共用的右侧悬浮提示触发器和覆盖层封装。
pub mod model_hover_tooltip;
#[cfg(test)]
#[path = "model_hover_tooltip_tests.rs"]
mod model_hover_tooltip_tests;

/// 通知组件
///
/// 提供系统通知的显示、管理和交互功能。
pub mod notification;
#[cfg(test)]
#[path = "notification_tests.rs"]
mod notification_tests;

/// 轻量提示组件
///
/// 提供自动消失的短消息提示，可在各功能模块复用。
pub mod toast;
#[cfg(test)]
#[path = "toast_tests.rs"]
mod toast_tests;

pub mod text_editor_context_menu;
#[cfg(test)]
#[path = "text_editor_context_menu_tests.rs"]
mod text_editor_context_menu_tests;

pub mod text_editor_scroll_panel;

/// 覆盖层组件
///
/// 提供模态框、下拉菜单、工具提示等覆盖层 UI 元素的基础设施。
pub mod overlays;

/// 预览面板组件
///
/// 提供文件内容预览、渲染结果预览等功能。
pub mod preview_panel;

/// 搜索面板组件
///
/// 提供全局搜索、文件内搜索、搜索结果展示等功能。
pub mod search_panel;

#[cfg(test)]
#[path = "search_panel_tests.rs"]
mod search_panel_tests;

/// 系统设置主模块
///
/// 系统设置的入口和容器组件，协调各设置子模块。
pub mod system_settings;
#[cfg(test)]
#[path = "system_settings_tests.rs"]
mod system_settings_tests;

/// 系统设置：多代理配置
///
/// 配置 researcher、coder、reviewer 等委托代理的独立模型与工具权限。
pub mod system_settings_agents;

/// 系统设置：员工配置
///
/// 配置任务看板员工名册、agent 绑定和能力入口。
pub mod system_settings_staff;

/// 系统设置：代理间通信
///
/// 配置多个代理之间的进程间通信（IPC）机制。
pub mod system_settings_agents_ipc;

/// 系统设置：自主性配置
///
/// 配置代理的自主决策级别和行为边界。
pub mod system_settings_autonomy;

/// 系统设置：ACP 配置
///
/// 管理 Agent Client Protocol 后端目录、初始化说明和启用状态。
pub mod system_settings_acp;

/// 系统设置：MCP 配置
///
/// 管理全局 MCP 服务器配置。
pub mod system_settings_mcp;

/// 系统设置：通用组件
///
/// 各设置面板共用的 UI 组件和工具函数。
pub mod system_settings_common;

/// 系统设置：缺失页帮助文档
///
/// 为尚未内联实现帮助模态框的系统设置页提供统一帮助入口与文案。
pub mod system_settings_help;

/// 系统设置：多通道集成
///
/// 配置 CLI 与多种消息/语音通道的启用状态和参数。
pub mod system_settings_channels;

/// 系统设置：协调配置
///
/// 配置多代理协作、任务分发和结果汇总机制。
pub mod system_settings_coordination;

/// 系统设置：定时任务
///
/// 配置和管理 Cron 定时任务的执行计划。
pub mod system_settings_cron;

/// 系统设置：对话流程
///
/// 配置对话流程、上下文管理、会话持久化等。
pub mod system_settings_dialogue_flow;

/// 系统设置：聊天快捷消息。
pub mod system_settings_quick_messages;

/// 系统设置：编辑器偏好
///
/// 配置编辑器的主题、字体、缩进、快捷键等偏好设置。
pub mod system_settings_editor;

/// 系统设置：嵌入路由
///
/// 配置嵌入模型的 provider、model 和 dimensions 路由规则。
pub mod system_settings_embedding_routes;

/// 系统设置：常规设置
///
/// 应用程序的常规设置，如语言、主题、启动行为等。
pub mod system_settings_general;

/// 系统设置：网关配置
///
/// 配置 HTTP gateway 的监听地址、配对、安全限流与 node-control 行为。
pub mod system_settings_gateway;

/// 系统设置：客户端网关连接
///
/// 配置桌面客户端连接哪个网关，以及访问认证参数。
pub mod system_settings_gateway_client;

/// 系统设置：目标循环
///
/// 配置自主 goal loop 的执行开关、节奏与事件投递目标。
pub mod system_settings_goal_loop;

/// 系统设置：心跳监控
///
/// 配置代理健康检查、心跳间隔、故障检测等。
pub mod system_settings_heartbeat;

/// 系统设置：钩子配置
///
/// 配置运行时 hooks 总开关和内置钩子行为。
pub mod system_settings_hooks;

/// 系统设置：模型配置
///
/// 配置 AI 模型的参数、上下文长度、响应策略等。
pub mod system_settings_models;

/// 系统设置：模型路由
///
/// 配置消息模式到 Provider / Model 的路由规则。
pub mod system_settings_model_routes;

/// 系统设置：查询分类
///
/// 配置查询分类启用状态和分类规则。
pub mod system_settings_query_classification;

/// 系统设置：可观测性
///
/// 配置日志、指标、追踪等可观测性功能。
pub mod system_settings_observability;

/// 系统设置：存储配置
///
/// 配置持久化存储 provider、连接参数与 TLS 选项。
pub mod system_settings_storage;

/// 系统设置：项目管理
///
/// 管理项目列表、项目配置、工作区设置等。
pub mod system_settings_projects;

/// 系统设置：服务提供商
///
/// 配置 AI 服务提供商、API 密钥、端点等。
pub mod system_settings_providers;

/// 系统设置：代理设置
///
/// 配置网络代理、HTTP/SOCKS 代理等网络设置。
pub mod system_settings_proxy;

/// 系统设置：隧道设置
///
/// 配置公网暴露网关时使用的 tunnel provider 与凭据。
pub mod system_settings_tunnel;

/// 系统设置：Composio 集成
///
/// 配置 Composio OAuth 工具集成的启用状态、API 密钥和实体标识。
pub mod system_settings_composio;

/// 系统设置：记忆系统配置
///
/// 配置 memory 后端、嵌入参数、缓存与 Qdrant 连接选项。
pub mod system_settings_memory;

/// 系统设置：可靠性配置
///
/// 配置重试策略、超时设置、故障恢复等可靠性参数。
pub mod system_settings_reliability;

/// 系统设置：多模态配置
///
/// 配置图像输入数量、大小限制与远程抓取开关。
pub mod system_settings_multimodal;

/// 系统设置：运行时配置
///
/// 配置 runtime 类型以及 Docker/WASM 执行环境参数。
pub mod system_settings_runtime;

/// 系统设置：研究功能
///
/// 配置代理的研究能力、信息检索、知识库访问等。
pub mod system_settings_research;

/// 系统设置：Web 搜索
///
/// 配置 web_search 工具的 provider、凭据、结果数和超时等参数。
pub mod system_settings_web_search;

/// 系统设置：调度器配置
///
/// 配置任务调度器、优先级队列、并发控制等。
pub mod system_settings_scheduler;


/// 系统设置：浏览器配置
///
/// 配置 browser / browser_open 工具、原生后端与 computer-use sidecar。
pub mod system_settings_browser;

/// 系统设置：HTTP 请求配置
///
/// 配置 http_request 工具的启用状态、域名白名单和请求限制。
/// 配置 http_request 工具的白名单域名、超时、响应大小与 User-Agent。
pub mod system_settings_http_request;

/// 系统设置：安全配置
///
/// 配置安全策略、权限控制、敏感数据保护等。
pub mod system_settings_security;

/// 系统设置：技能管理
///
/// 管理和配置代理可使用的技能和工具集。
pub mod system_settings_skills;

/// 系统设置：语音转写
///
/// 配置语音识别、音频转文字等转写功能。
pub mod system_settings_transcription;

/// 系统设置：主题管理
///
/// 展示内置与自定义主题画廊，支持应用与删除自定义主题。
pub mod system_settings_themes;

/// 标签栏组件
///
/// 提供多标签页界面，支持标签切换、关闭、拖拽排序等。
pub mod tab_bar;

/// 终端面板组件
///
/// 提供嵌入式终端功能，支持命令执行和输出显示。
pub mod terminal_panel;
#[cfg(test)]
#[path = "terminal_panel_tests.rs"]
mod terminal_panel_tests;

/// 顶部工具栏组件
///
/// 应用程序顶部的工具栏，包含窗口控制、全局操作等。
pub mod top_bar;
#[cfg(test)]
#[path = "top_bar_tests.rs"]
mod top_bar_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod account_status;
/// 通用小部件集合
///
/// 提供按钮、输入框、下拉框等可复用的基础 UI 小部件。
pub mod widgets;
#[cfg(test)]
#[path = "widgets_tests.rs"]
mod widgets_tests;
