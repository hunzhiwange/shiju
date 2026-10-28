//! CLI 命令速查：仅展示与复制，不执行命令。
use super::install_cli_modal::hash_copy_text;
use super::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style, settings_segment_button_style,
};
use crate::app::{Message, message::ViewMessage};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CliCommandsTab {
    #[default]
    Common,
    Service,
    All,
}

// 与 vw-cli 的 Commands / RootApiCommands 一级入口对应；子命令通过 --help 查看。
const ALL_COMMANDS: &[(&str, &str)] = &[
    ("shiju acp --help", "本地 ACP Agent"),
    ("shiju agent --help", "终端交互界面（TUI）与单条消息"),
    ("shiju gateway --help", "Gateway 服务"),
    ("shiju daemon --help", "常驻代理运行时"),
    ("shiju service --help", "系统服务管理"),
    ("shiju server --help", "后台服务管理（兼容入口）"),
    ("shiju doctor --help", "运行诊断"),
    ("shiju status --help", "系统状态"),
    ("shiju estop --help", "紧急停止与恢复"),
    ("shiju security --help", "安全控制"),
    ("shiju cron --help", "计划任务"),
    ("shiju providers --help", "支持的 AI 提供商"),
    ("shiju channel --help", "通信通道"),
    ("shiju integrations --help", "集成"),
    ("shiju skills --help", "技能管理"),
    ("shiju plugins --help", "插件管理"),
    ("shiju task --help", "本地项目任务"),
    ("shiju memory --help", "代理记忆"),
    ("shiju config --help", "配置与配置 Schema"),
    ("shiju completions --help", "Shell 补全脚本"),
    ("shiju provider --help", "Gateway 提供商"),
    ("shiju control --help", "认证与 Gateway 控制"),
    ("shiju operations --help", "远程运维"),
    ("shiju interactive --help", "权限、问题与 PTY 交互"),
    ("shiju project --help", "远程项目与 worktree"),
    ("shiju realtime --help", "推理与实时协议"),
    ("shiju knowledge --help", "知识库与检索"),
    ("shiju task-pool --help", "任务 worktree 池"),
    ("shiju task-lifecycle --help", "任务生命周期"),
    ("shiju scheduler --help", "Agent 调度器"),
    ("shiju session --help", "会话管理"),
    ("shiju session-content --help", "会话消息与快照"),
    ("shiju shiju-apps --help", "识矩账号与应用"),
    ("shiju skill-market --help", "技能市场"),
    ("shiju extensions --help", "Gateway 技能与插件"),
    ("shiju desktop --help", "桌面偏好与宿主操作"),
    ("shiju workflow --help", "工作流"),
    ("shiju workspace --help", "工作区文件、搜索与 Git"),
    ("shiju system --help", "Gateway 状态与发现"),
    ("shiju help", "全部命令及帮助"),
];

const COMMON_COMMANDS: &[(&str, &str)] = &[
    ("shiju --help", "列出全部一级命令"),
    ("shiju agent", "启动终端交互界面（TUI）"),
    ("shiju --version", "查看 CLI 版本"),
    ("shiju status", "查看系统状态"),
    ("shiju doctor", "运行诊断"),
    ("shiju service --help", "查看服务管理子命令"),
];

const SERVICE_COMMANDS: &[(&str, &str)] = &[
    ("shiju service status", "查看服务状态"),
    ("shiju service start", "启动服务"),
    ("shiju service stop", "停止服务"),
    ("shiju service restart", "重启服务"),
    ("shiju service install", "注册系统服务"),
    ("shiju service uninstall", "卸载系统服务"),
];

pub(crate) fn view(tab: CliCommandsTab, copied_hash: Option<u64>) -> Element<'static, Message> {
    let mut tabs = row![].spacing(6);
    for (value, label) in [
        (CliCommandsTab::Common, "常用命令"),
        (CliCommandsTab::Service, "服务管理"),
        (CliCommandsTab::All, "全部命令"),
    ] {
        tabs = tabs.push(
            button(label)
                .on_press(Message::View(ViewMessage::CliCommandsTabSelected(value)))
                .padding([7, 12])
                .style(move |theme, status| {
                    settings_segment_button_style(theme, status, tab == value)
                }),
        );
    }
    let commands = match tab {
        CliCommandsTab::Common => COMMON_COMMANDS,
        CliCommandsTab::Service => SERVICE_COMMANDS,
        CliCommandsTab::All => ALL_COMMANDS,
    };
    let mut content = column![
        tabs,
        text("在终端运行；点击标签页切换。全部命令页提供各入口的帮助，可继续查看子命令与参数。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(10);
    for &(command, description) in commands {
        let copied = copied_hash == Some(hash_copy_text(command));
        content = content.push(
            row![
                column![
                    text(command).size(13),
                    text(description).size(12).style(settings_muted_text_style),
                ]
                .spacing(3)
                .width(Length::Fill),
                button(if copied { "已复制" } else { "复制" })
                    .on_press(Message::CopyCode(command.to_owned()))
                    .padding([7, 12])
                    .style(rounded_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }
    content.into()
}
