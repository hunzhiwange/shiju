//! Semantic feedback retained alongside the legacy raw error fields.
use super::{
    I18n,
    format::{Argument, Message},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorField {
    General,
    WorktreeDelete,
    WorktreeReset,
}

#[derive(Debug, Clone)]
pub struct Feedback {
    pub fallback: String,
    message: Message,
}
impl Feedback {
    pub fn new(source: &str, arguments: Vec<Argument>) -> Self {
        let message = Message::new(source, arguments);
        let fallback = message
            .render(I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
            .expect("valid feedback template");
        Self { fallback, message }
    }
    pub fn render(&self, i18n: I18n, raw: &str) -> String {
        if self.fallback == raw {
            self.message.render(i18n).expect("valid feedback template")
        } else {
            raw.to_owned()
        }
    }
}

/// Localized input errors and generated session labels shared across host routes.
#[derive(Default)]
pub struct State {
    /// 错误消息
    pub error_message: Option<String>,
    pub localized_input_errors:
        std::collections::BTreeMap<crate::input::ErrorField, crate::input::Feedback>,
    /// 用于模型聊天时间上下文的 IANA 时区
    pub generated_session_titles: std::collections::HashMap<String, crate::input::Feedback>,
}

impl State {
    pub fn render_error(&self, field: ErrorField, i18n: I18n, raw: &str) -> String {
        self.localized_input_errors
            .get(&field)
            .map(|feedback| feedback.render(i18n, raw))
            .unwrap_or_else(|| feedback(raw).render(i18n).expect("valid input feedback"))
    }
}

/// Only known application feedback is translated; arbitrary service diagnostics stay literal.
pub fn feedback(raw: &str) -> Message {
    let source = match raw {
        "启用工作流执行需要先选择项目根目录" => Some("启用工作流执行需要先选择项目根目录"),
        "识矩登录凭据无效" => Some("识矩登录凭据无效"),
        "工作流执行受阻" => Some("工作流执行受阻"),
        "请先选择项目目录" => Some("请先选择项目目录"),
        "此智能体不支持所选模型来源" => Some("此智能体不支持所选模型来源"),
        "会话选项切换或读取失败，请刷新或重试；当前能力尚未确认" => Some("会话选项切换或读取失败，请刷新或重试；当前能力尚未确认"),
        "后端未确认所选模型来源" => Some("后端未确认所选模型来源"),
        "请等待当前请求结束后切换模型来源" => Some("请等待当前请求结束后切换模型来源"),
        "剪贴板图片宽度超出范围" => Some("剪贴板图片宽度超出范围"),
        "剪贴板图片高度超出范围" => Some("剪贴板图片高度超出范围"),
        "读取系统剪贴板失败或访问被拒绝" => Some("读取系统剪贴板失败或访问被拒绝"),
        "会话历史不存在，未发送消息" => Some("会话历史不存在，未发送消息"),
        "请先从历史列表打开数字员工会话" => Some("请先从历史列表打开数字员工会话"),
        "网关返回了其他会话的历史" => Some("网关返回了其他会话的历史"),
        "模型来源正在保存，请稍后发送" => Some("模型来源正在保存，请稍后发送"),
        "会话历史正在加载，请稍后发送" => Some("会话历史正在加载，请稍后发送"),
        "CLI 未安装，请通过输入区的安装 / 运行设置安装" => Some("CLI 未安装，请通过输入区的安装 / 运行设置安装"),
        "网关未提供该智能体的安装状态，请检查运行设置" => Some("网关未提供该智能体的安装状态，请检查运行设置"),
        "网关返回了其他会话的历史，未发送消息" => Some("网关返回了其他会话的历史，未发送消息"),
        "会话正在回复且历史尚未就绪，请稍后发送" => Some("会话正在回复且历史尚未就绪，请稍后发送"),
        "当前会话没有可用于压缩的用户消息" => Some("当前会话没有可用于压缩的用户消息"),
        "会话保存任务异常终止" => Some("会话保存任务异常终止"),
        "请求内容过大（图片或会话历史超出网关限制），请压缩图片、减少附件数量或开启新会话后重试" => Some("请求内容过大（图片或会话历史超出网关限制），请压缩图片、减少附件数量或开启新会话后重试"),
        "原始数据自动重试 3 次后仍未就绪，可稍后重试" => Some("原始数据自动重试 3 次后仍未就绪，可稍后重试"),
        "无法定位置顶会话目录" => Some("无法定位置顶会话目录"),
        "plan-run task id 不能为空" => Some("plan-run task id 不能为空"),
        "终端工作目录必须是目录" => Some("终端工作目录必须是目录"),
        "当前平台不支持打开本机终端" => Some("当前平台不支持打开本机终端"),
        "会话开始后不能切换智能体" => Some("会话开始后不能切换智能体"),
        "智能体未启用或不支持当前入口" => Some("智能体未启用或不支持当前入口"),
        "恢复快照缺少本地历史，已保留现有消息，请重新加载会话" => Some("恢复快照缺少本地历史，已保留现有消息，请重新加载会话"),
        "请在本机网关启用数字员工智能体后重试" => Some("请在本机网关启用数字员工智能体后重试"),
        "识矩登录凭据已失效，请重新登录后重试" => Some("识矩登录凭据已失效，请重新登录后重试"),
        "当前网关或账号无法访问数字员工会话" => Some("当前网关或账号无法访问数字员工会话"),
        "数字员工或会话当前不可用，请刷新员工列表" => Some("数字员工或会话当前不可用，请刷新员工列表"),
        "读取置顶会话任务失败" => Some("读取置顶会话任务失败"),
        "远端终端只能通过网关 PTY 打开，不能启动本机终端" => Some("远端终端只能通过网关 PTY 打开，不能启动本机终端"),
        "会话开始后不能切换模型" => Some("会话开始后不能切换模型"),
        "请先处理现有草稿" => Some("请先处理现有草稿"),
        "当前没有可压缩的会话" => Some("当前没有可压缩的会话"),
        "当前会话没有可用的项目目录" => Some("当前会话没有可用的项目目录"),
        "请先打开项目，再创建新会话" => Some("请先打开项目，再创建新会话"),
        "快捷消息内容不能为空" => Some("快捷消息内容不能为空"),
        _ => None,
    };
    if let Some(source) = source { return Message::new(source, vec![]); }
    if let Some(error) = raw.strip_prefix("加载会话历史失败，未发送消息：") {
        return Message::new("加载会话历史失败，未发送消息：%s", vec![error.into()]);
    }
    Message::literal(raw.to_owned())
}
