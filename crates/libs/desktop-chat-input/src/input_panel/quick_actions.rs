//! 快捷消息只修改草稿或产生明确的跨域操作。
use config_ui::{QuickMessageConfig, QuickMessageKind};
use iced::widget::text_editor;

pub struct Context<'a> {
    pub project: Option<&'a str>,
    pub session: Option<&'a str>,
    pub session_directory: Option<&'a str>,
    pub git_path: Option<&'a str>,
    pub has_git_changes: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    Draft { text: String, send: bool },
    CreateSession(String),
    Compact { session: String, project: String },
    RefreshGit,
    SelectGit,
    InvertGit,
    StageGit,
    CommitGit { project: String, stage_all: bool },
    Warning(&'static str),
}
fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.trim().is_empty())
}
pub fn apply(item: &QuickMessageConfig, context: Context<'_>) -> Effect {
    use QuickMessageKind as Kind;
    match item.kind {
        Kind::Message | Kind::FillInput => {
            if item.text.trim().is_empty() {
                return Effect::Warning("快捷消息内容不能为空");
            }
            Effect::Draft { text: item.text.clone(), send: item.kind == Kind::Message }
        }
        Kind::CreateNewSession => nonempty(context.project)
            .map(|v| Effect::CreateSession(v.into()))
            .unwrap_or(Effect::Warning("请先打开项目，再创建新会话")),
        Kind::SummarizeAndCreateNewSession => {
            let Some(session) = context.session else {
                return Effect::Warning("当前没有可压缩的会话");
            };
            let Some(project) =
                nonempty(context.session_directory).or_else(|| nonempty(context.project))
            else {
                return Effect::Warning("当前会话没有可用的项目目录");
            };
            Effect::Compact { session: session.into(), project: project.into() }
        }
        Kind::RefreshGitChanges => {
            if nonempty(context.project).is_some() {
                Effect::RefreshGit
            } else {
                Effect::Warning("请先打开项目，再刷新 Git 变更")
            }
        }
        Kind::SelectAllGitChanges => {
            if context.has_git_changes {
                Effect::SelectGit
            } else {
                Effect::Warning("当前没有可选择的 Git 变更")
            }
        }
        Kind::InvertGitSelection => {
            if context.has_git_changes {
                Effect::InvertGit
            } else {
                Effect::Warning("当前没有可反选的 Git 变更")
            }
        }
        Kind::StageAllGitChanges => Effect::StageGit,
        Kind::StageAllAndCommitGitChanges | Kind::CommitSelectedGitChanges => {
            let stage_all = item.kind == Kind::StageAllAndCommitGitChanges;
            match context.git_path {
                Some(project) => Effect::CommitGit { project: project.into(), stage_all },
                None => Effect::Warning(if stage_all {
                    "请先打开项目，再暂存并提交 Git 变更"
                } else {
                    "请先打开项目，再提交已选 Git 变更"
                }),
            }
        }
    }
}

pub fn apply_draft(editor: &mut text_editor::Content, text: &str, send: bool) {
    if send {
        *editor = text_editor::Content::with_text(text);
    } else {
        desktop_chat_view::chat::append_line(editor, text);
    }
}

/// 推荐消息只填入空草稿；请求生命周期不属于此入口。
pub fn fill_recommendation(editor: &mut text_editor::Content, prompt: Option<&str>) -> Result<Option<String>, &'static str> {
    if !editor.text().is_empty() { return Err("请先处理现有草稿"); }
    let Some(prompt) = prompt.filter(|prompt| !prompt.trim().is_empty()) else { return Ok(None); };
    *editor = text_editor::Content::with_text(prompt);
    Ok(Some(prompt.to_owned()))
}
