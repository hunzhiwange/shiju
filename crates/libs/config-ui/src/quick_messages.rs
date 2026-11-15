use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuickMessageGroup {
    #[default]
    General,
    Git,
}

impl QuickMessageGroup {
    pub const ALL: [Self; 2] = [Self::General, Self::Git];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "常用操作",
            Self::Git => "Git 操作",
        }
    }
}

impl std::fmt::Display for QuickMessageGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuickMessageKind {
    #[default]
    Message,
    FillInput,
    CreateNewSession,
    SummarizeAndCreateNewSession,
    RefreshGitChanges,
    SelectAllGitChanges,
    InvertGitSelection,
    StageAllGitChanges,
    StageAllAndCommitGitChanges,
    CommitSelectedGitChanges,
}

impl QuickMessageKind {
    pub const ALL: [Self; 10] = [
        Self::FillInput,
        Self::Message,
        Self::CreateNewSession,
        Self::SummarizeAndCreateNewSession,
        Self::RefreshGitChanges,
        Self::SelectAllGitChanges,
        Self::InvertGitSelection,
        Self::StageAllGitChanges,
        Self::StageAllAndCommitGitChanges,
        Self::CommitSelectedGitChanges,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Message => "发送消息",
            Self::FillInput => "填入输入框",
            Self::CreateNewSession => "创建新会话",
            Self::SummarizeAndCreateNewSession => "压缩当前会话",
            Self::RefreshGitChanges => "刷新 Git 变更",
            Self::SelectAllGitChanges => "全选 Git 变更",
            Self::InvertGitSelection => "反选 Git 变更",
            Self::StageAllGitChanges => "暂存全部 Git 变更",
            Self::StageAllAndCommitGitChanges => "暂存并提交全部 Git 变更",
            Self::CommitSelectedGitChanges => "提交已选 Git 变更",
        }
    }

    pub fn action_badge(self) -> Option<&'static str> {
        match self {
            Self::Message => Some("发送"),
            Self::FillInput => None,
            Self::CreateNewSession => Some("新会话"),
            Self::SummarizeAndCreateNewSession => Some("压缩"),
            Self::RefreshGitChanges => Some("刷新"),
            Self::SelectAllGitChanges => Some("全选"),
            Self::InvertGitSelection => Some("反选"),
            Self::StageAllGitChanges => Some("暂存"),
            Self::StageAllAndCommitGitChanges => Some("暂存并提交"),
            Self::CommitSelectedGitChanges => Some("提交"),
        }
    }
}

impl std::fmt::Display for QuickMessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(default)]
pub struct QuickMessageConfig {
    pub text: String,
    pub group: QuickMessageGroup,
    pub kind: QuickMessageKind,
}

pub fn builtin_git_quick_messages() -> Vec<QuickMessageConfig> {
    vec![
        QuickMessageConfig {
            text: "刷新 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::RefreshGitChanges,
        },
        QuickMessageConfig {
            text: "全选 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::SelectAllGitChanges,
        },
        QuickMessageConfig {
            text: "反选 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::InvertGitSelection,
        },
        QuickMessageConfig {
            text: "暂存全部变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::StageAllGitChanges,
        },
        QuickMessageConfig {
            text: "暂存并提交全部变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::StageAllAndCommitGitChanges,
        },
        QuickMessageConfig {
            text: "提交已选变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::CommitSelectedGitChanges,
        },
    ]
}

pub fn builtin_general_quick_messages() -> Vec<QuickMessageConfig> {
    let send = |text: &str| QuickMessageConfig {
        text: text.to_string(),
        group: QuickMessageGroup::General,
        kind: QuickMessageKind::Message,
    };
    let fill = |text: &str| QuickMessageConfig {
        text: text.to_string(),
        group: QuickMessageGroup::General,
        kind: QuickMessageKind::FillInput,
    };
    let mut messages = vec![
        send("继续推进"),
        send("执行下一步"),
        fill("摘要并生成下一个任务的提示词"),
        QuickMessageConfig {
            text: "压缩当前会话".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::SummarizeAndCreateNewSession,
        },
        fill("为剩余任务制定开发计划"),
    ];
    messages.extend(
        [
            "不要兼容旧的代码",
            "修复单元测试",
            "给代码增加单元测试",
            "尽你最大的能力突破阻塞",
            "不对啊，完全不对啊，你仔细检查一下",
            "你实现得太简单了，做得细节一点",
            "多次修改都不对，你这一次需要仔细深度思考解决问题",
            "不要修改代码，这次只做分析和计划",
            "从其他语言移植代码，需要尽量一一对齐，不要尝试简化实现",
            "从第一个任务开始，直到所有任务全部完成才结束，不要轻易结束。",
            "这是一个开放性的需求，你可以自由发挥，做得细节一点，希望给我惊喜",
            "请选择合适的技能，给我制定计划，但是不要急着修改代码",
            "先复述你理解的目标、约束和验收标准，确认没有遗漏后再开始",
            "先定位根因和影响范围，不要用临时绕过掩盖底层问题",
            "请沿完整调用链检查数据流、状态流和错误流，再决定修改位置",
            "请主动覆盖边界条件、并发时序、空状态和失败恢复",
            "完成后逐条对照需求做自我验收，有偏差就继续修正",
            "请检查是否破坏现有行为，并补充能防止回归的测试",
            "请优先复用现有架构和组件，避免无必要的新抽象与依赖",
            "请站在最终用户视角打磨交互、文案、反馈和异常体验",
            "对不确定的假设先用代码和测试验证，不要凭记忆直接实现",
            "交付前检查修改范围、格式、静态检查和相关测试，明确报告结果",
        ]
        .into_iter()
        .map(fill),
    );
    messages
}

pub fn append_missing_builtin_general_quick_messages(messages: &mut Vec<QuickMessageConfig>) {
    if messages.is_empty() {
        return;
    }

    let mut insert_at = messages
        .iter()
        .position(|item| item.group == QuickMessageGroup::Git)
        .unwrap_or(messages.len());
    for builtin in builtin_general_quick_messages() {
        if !messages.iter().any(|item| {
            item.group == builtin.group
                && item.kind == builtin.kind
                && item.text.trim() == builtin.text
        }) {
            messages.insert(insert_at, builtin);
            insert_at += 1;
        }
    }
}

pub fn append_missing_builtin_git_quick_messages(messages: &mut Vec<QuickMessageConfig>) {
    if messages.is_empty() {
        return;
    }

    for builtin in builtin_git_quick_messages() {
        if !messages.iter().any(|item| item.kind == builtin.kind) {
            messages.push(builtin);
        }
    }
}

pub fn upgrade_legacy_quick_message_actions(messages: &mut [QuickMessageConfig]) {
    let fill_input_texts = builtin_general_quick_messages()
        .into_iter()
        .filter(|item| item.kind == QuickMessageKind::FillInput)
        .map(|item| item.text)
        .collect::<Vec<_>>();

    for item in messages {
        if matches!(item.text.trim(), "摘要并创建新会话" | "压缩当前会话")
            && matches!(
                item.kind,
                QuickMessageKind::CreateNewSession | QuickMessageKind::SummarizeAndCreateNewSession
            )
        {
            item.kind = QuickMessageKind::SummarizeAndCreateNewSession;
            item.text = "压缩当前会话".to_string();
        } else if item.group == QuickMessageGroup::General
            && item.kind == QuickMessageKind::Message
            && fill_input_texts.iter().any(|text| item.text.trim() == text)
        {
            item.kind = QuickMessageKind::FillInput;
        }
    }
}

pub(super) fn default_quick_messages() -> Vec<QuickMessageConfig> {
    let mut messages = builtin_general_quick_messages();
    messages.extend(builtin_git_quick_messages());
    messages
}

#[cfg(test)]
#[path = "quick_messages_tests.rs"]
mod tests;
