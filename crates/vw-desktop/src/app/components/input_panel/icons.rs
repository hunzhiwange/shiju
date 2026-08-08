//! 输入面板局部组件。
//!
//! 本模块负责输入区附件、文件搜索或图标展示相关的可复用逻辑。

use iced::widget::{image, svg};
/// 重新导出 use iced::{Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Color, Element, Length, Theme};

/// 重新导出 use crate::app::Message，让上层模块通过稳定路径访问。
use crate::app::Message;
/// 重新导出 use crate::app::assets::{self, Icon}，让上层模块通过稳定路径访问。
use crate::app::assets::{self, Icon};

/// 处理 provider logo handle 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn provider_logo_handle(provider_id: &str) -> svg::Handle {
    // crate 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    crate::app::assets::get_provider_icon(provider_id)
}

/// 处理 auto icon 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn auto_icon() -> svg::Handle {
    // assets 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    assets::get_icon(Icon::Star)
}

/// 处理 max icon 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn max_icon() -> svg::Handle {
    // assets 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    assets::get_icon(Icon::Speedometer2)
}

/// 处理 icon svg 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn icon_svg<'a>(icon: Icon, size: f32) -> svg::Svg<'a> {
    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    svg::Svg::<'a, iced::Theme>::new(assets::get_icon(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}

/// 处理 themed svg handle 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn themed_svg_handle<'a>(handle: svg::Handle, size: f32) -> svg::Svg<'a> {
    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    svg::Svg::<'a, iced::Theme>::new(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}

/// 处理 raw svg handle 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn raw_svg_handle<'a>(handle: svg::Handle, size: f32) -> svg::Svg<'a> {
    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    svg::Svg::<'a, iced::Theme>::new(handle).width(Length::Fixed(size)).height(Length::Fixed(size))
}

/// 处理 is dark mode 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn is_dark_mode(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

/// 处理 default acp icon 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn default_acp_icon<'a>(size: f32) -> Element<'a, Message> {
    // 默认 ACP 后端与 slash 面板 ACP 分组一致，使用机器人图标，而非专家中心的连接器图标。
    svg::Svg::<'a, iced::Theme>::new(assets::get_icon(Icon::Robot))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|theme: &Theme, _| svg::Style {
            color: Some(if is_dark_mode(theme) { Color::WHITE } else { theme.palette().text }),
        })
        .into()
}

/// 处理 svg icon 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[allow(dead_code)]
pub(super) fn svg_icon<'a>(icon: Icon, size: f32) -> Element<'a, Message> {
    icon_svg(icon, size).into()
}

/// 处理 image icon 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[allow(dead_code)]
pub(super) fn image_icon<'a>(icon: Icon, size: f32) -> Element<'a, Message> {
    image(assets::get_image(icon)).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

/// 处理 acp agent icon 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn acp_agent_icon<'a>(agent: &str, size: f32) -> Element<'a, Message> {
    let normalized = normalize_acp_agent_icon_name(agent);
    match normalized {
        "auggie" => raw_svg_handle(assets::get_icon(Icon::AppAuggie), size).into(),
        "claude" => raw_svg_handle(assets::get_icon(Icon::AppClaudeCode), size).into(),
        "codex" => raw_svg_handle(assets::get_icon(Icon::AppCodex), size).into(),
        "copilot" => raw_svg_handle(assets::get_icon(Icon::AppGitHubCopilot), size).into(),
        "cursor" => raw_svg_handle(assets::get_icon(Icon::AppCursor), size).into(),
        "droid" => raw_svg_handle(assets::get_icon(Icon::AppFactoryDroid), size).into(),
        "gemini" => raw_svg_handle(assets::get_icon(Icon::AppGeminiCli), size).into(),
        "kiro" => raw_svg_handle(assets::get_icon(Icon::AppKiro), size).into(),
        "kilocode" => raw_svg_handle(assets::get_icon(Icon::AppKiloCode), size).into(),
        "kimi" => raw_svg_handle(assets::get_icon(Icon::AppKimiCode), size).into(),
        "opencode" => raw_svg_handle(assets::get_icon(Icon::AppOpenCode), size).into(),
        "openclaw" => raw_svg_handle(assets::get_icon(Icon::AppOpenClaw), size).into(),
        "pi" => raw_svg_handle(assets::get_icon(Icon::AppPi), size).into(),
        "qoder" => raw_svg_handle(assets::get_icon(Icon::AppQoder), size).into(),
        "qwen" => raw_svg_handle(assets::get_icon(Icon::AppQwenCode), size).into(),
        "trae" => raw_svg_handle(assets::get_icon(Icon::AppTrae), size).into(),
        _ => default_acp_icon(size),
    }
}

/// 归一化 acp agent icon name，让后续路径或文本比较保持确定性。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn normalize_acp_agent_icon_name(agent: &str) -> &str {
    match agent.trim().to_ascii_lowercase().as_str() {
        "agentclientprotocol-claude" | "claude" | "claude code" | "claude-code" | "claudecode" => {
            "claude"
        }
        "auggie" | "auggie cli" | "auggie-cli" => "auggie",
        "codex" | "codex cli" | "codex-cli" => "codex",
        "copilot" | "copilot-cli" | "github copilot" | "github-copilot" | "githubcopilot" => {
            "copilot"
        }
        "cursor" => "cursor",
        "droid" | "factory droid" | "factory-droid" | "factorydroid" => "droid",
        "gemini" | "gemini cli" | "gemini-cli" => "gemini",
        "kiro" | "kiro agent" | "kiro-agent" | "kiro-cli-chat" => "kiro",
        "kilocode" => "kilocode",
        "kimi" | "kimi code" | "kimi-code" | "kimi code cli" | "kimi-code-cli" => "kimi",
        "opencode" | "open-code" => "opencode",
        "openclaw" => "openclaw",
        "pi" | "pi-acp" => "pi",
        "qoder" | "qoder cli" | "qoder-cli" => "qoder",
        "qwen" | "qwen code" | "qwen-code" => "qwen",
        "trae" | "trae cli" | "trae-cli" => "trae",
        _ => "",
    }
}
