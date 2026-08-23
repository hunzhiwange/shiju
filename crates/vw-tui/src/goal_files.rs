//! 将超长 TUI 目标、粘贴文本和图片物化为应用服务器托管文件。
//! Materializes oversized TUI goal objectives, pastes, and images as app-server-host files.

use std::fs;
use std::path::Path;

use crate::app_server_session::AppServerSession;
use crate::bottom_pane::ChatComposer;
use crate::bottom_pane::LocalImageAttachment;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use codex_app_server_client::AppServerPath;
use codex_protocol::protocol::MAX_THREAD_GOAL_OBJECTIVE_CHARS;
use codex_protocol::user_input::TextElement;
use uuid::Uuid;

const GOAL_ATTACHMENT_DIR: &str = "attachments";
const GOAL_FILE_PREFIX: &str = "Read the Codex goal objective file at ";
const GOAL_FILE_SUFFIX: &str = " before continuing.";
const GOAL_FILE_NAME: &str = "goal-objective.md";

#[derive(Clone, Debug, Default)]
pub(crate) struct GoalDraft {
    /// 保留目标正文及其中尚待替换的文本、图片引用。
    pub(crate) objective: String,
    pub(crate) text_elements: Vec<TextElement>,
    pub(crate) pending_pastes: Vec<(String, String)>,
    pub(crate) local_images: Vec<LocalImageAttachment>,
    pub(crate) remote_image_urls: Vec<String>,
}

pub(crate) type GoalFilePath = AppServerPath;

pub(crate) async fn materialize_goal_draft(
    app_server: &mut AppServerSession,
    codex_home: Option<&GoalFilePath>,
    draft: GoalDraft,
) -> Result<(String, Option<GoalFilePath>)> {
    // 空目标在写入任何附件前拒绝，避免创建无法关联的孤立目录。
    let mut objective = draft.objective;
    if objective.trim().is_empty() {
        bail!("Goal objective must not be empty.");
    }
    let text_elements = draft.text_elements;
    if !draft.pending_pastes.is_empty() {
        let (expanded_objective, _) = ChatComposer::expand_pending_pastes(
            &objective,
            text_elements.clone(),
            &draft.pending_pastes,
        );
        if expanded_objective.trim().is_empty() {
            bail!("Goal objective must not be empty.");
        }
    }

    // 只物化仍在正文中激活的占位符，忽略已经被用户删除的附件。
    let mut active_placeholders = text_elements
        .iter()
        .filter_map(|element| element.placeholder(&objective))
        .filter(|placeholder| !placeholder.is_empty())
        .collect::<Vec<_>>();
    let mut output_dir = None;
    let mut replacements = Vec::new();
    for (placeholder, text) in draft.pending_pastes.iter() {
        let Some(active_idx) = active_placeholders
            .iter()
            .position(|active| *active == placeholder.as_str())
        else {
            continue;
        };
        active_placeholders.swap_remove(active_idx);
        // 附件目录按需创建，粘贴文本使用稳定的一基序号文件名。
        let path = ensure_goal_output_dir(app_server, codex_home, &mut output_dir)
            .await?
            .join(format!("pasted-text-{}.txt", replacements.len() + 1));
        write_goal_file(app_server, path.clone(), text.as_bytes().to_vec()).await?;

        replacements.push((
            placeholder.clone(),
            format!("pasted text file: {path}. Read this file before continuing."),
        ));
    }

    let mut image_lines = Vec::new();
    for (idx, image) in draft.local_images.iter().enumerate() {
        if !image.placeholder.is_empty() {
            let Some(active_idx) = active_placeholders
                .iter()
                .position(|active| *active == image.placeholder.as_str())
            else {
                continue;
            };
            active_placeholders.swap_remove(active_idx);
        }
        let extension = image_extension(&image.path);
        let path = ensure_goal_output_dir(app_server, codex_home, &mut output_dir)
            .await?
            .join(format!("image-{}.{}", idx + 1, extension));
        let bytes = fs::read(&image.path)
            .with_context(|| format!("Could not read goal image {}", image.path.display()))?;
        write_goal_file(app_server, path.clone(), bytes).await?;
        // 无占位符图片追加到引用章节，有占位符图片则就地替换。
        if image.placeholder.is_empty() {
            image_lines.push(format!("- [Image #{}]: {path}", idx + 1));
        } else {
            replacements.push((image.placeholder.clone(), format!("image file: {path}")));
        }
    }

    let (expanded_objective, _) =
        ChatComposer::expand_pending_pastes(&objective, text_elements, &replacements);
    objective = expanded_objective.trim().to_string();
    append_section(&mut objective, "Referenced image files:", image_lines);

    append_section(
        &mut objective,
        "Referenced image URLs:",
        draft
            .remote_image_urls
            .into_iter()
            .enumerate()
            .map(|(idx, url)| format!("- [Image #{}]: {url}", idx + 1))
            .collect(),
    );

    // 超出协议字符上限时保存完整目标，只把受控文件引用交给线程目标接口。
    if objective.chars().count() > MAX_THREAD_GOAL_OBJECTIVE_CHARS {
        let path = ensure_goal_output_dir(app_server, codex_home, &mut output_dir)
            .await?
            .join(GOAL_FILE_NAME);
        let reference = match objective_file_reference(&path) {
            Ok(reference) => reference,
            Err(err) => {
                // 无法构造合法引用时尽力清理本次创建的附件目录。
                if let Some(output_dir) = output_dir.as_ref() {
                    let _ = app_server.fs_remove_path(output_dir).await;
                }
                return Err(err);
            }
        };
        write_goal_file(app_server, path.clone(), objective.as_bytes().to_vec()).await?;
        objective = reference;
    }
    Ok((objective, output_dir))
}

pub(crate) async fn objective_text_for_edit(
    app_server: &mut AppServerSession,
    codex_home: Option<&GoalFilePath>,
    objective: &str,
) -> Result<String> {
    // 只有严格匹配本模块生成格式的目标才从服务器读取；普通文本原样返回。
    let Some(path) = objective_file_path(objective, codex_home) else {
        return Ok(objective.to_string());
    };
    let bytes = app_server
        .fs_read_file_path(&path)
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))
        .with_context(|| format!("Could not read goal objective file {path}"))?;
    String::from_utf8(bytes)
        .with_context(|| format!("Goal objective file {path} is not valid UTF-8"))
}

pub(crate) fn objective_file_path(
    objective: &str,
    codex_home: Option<&GoalFilePath>,
) -> Option<GoalFilePath> {
    // 同时校验固定前后缀、绝对路径、附件 UUID 和预期 Codex 主目录层级。
    let path = objective
        .strip_prefix(GOAL_FILE_PREFIX)
        .and_then(|path| path.strip_suffix(GOAL_FILE_SUFFIX))?;
    let path = AppServerPath::from_absolute_str(path)?;
    let parts = path.components();
    let attachment_id = parts.get(parts.len().checked_sub(2)?)?;
    let expected = codex_home?
        .join(GOAL_ATTACHMENT_DIR)
        .join(attachment_id)
        .join(GOAL_FILE_NAME);
    (path == expected && Uuid::parse_str(attachment_id).is_ok()).then_some(path)
}

pub(crate) fn objective_file_reference(path: &GoalFilePath) -> Result<String> {
    // 文件引用本身也必须满足目标字符上限，否则明确拒绝伪支持。
    let reference = format!("{GOAL_FILE_PREFIX}{path}{GOAL_FILE_SUFFIX}");
    let actual_chars = reference.chars().count();
    if actual_chars > MAX_THREAD_GOAL_OBJECTIVE_CHARS {
        bail!(
            "Goal objective file reference is too long: {actual_chars} characters. Limit: {MAX_THREAD_GOAL_OBJECTIVE_CHARS} characters."
        );
    }
    Ok(reference)
}

async fn ensure_goal_output_dir(
    app_server: &mut AppServerSession,
    codex_home: Option<&GoalFilePath>,
    output_dir: &mut Option<GoalFilePath>,
) -> Result<GoalFilePath> {
    // 同一份草稿的所有附件复用一个 UUID 目录，并仅在首次需要时创建。
    if let Some(output_dir) = output_dir {
        return Ok(output_dir.clone());
    }
    let codex_home = codex_home
        .context("App server did not report $CODEX_HOME; cannot materialize goal files")?;
    let path = codex_home
        .join(GOAL_ATTACHMENT_DIR)
        .join(Uuid::new_v4().to_string());
    app_server
        .fs_create_directory_all_path(&path)
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))
        .with_context(|| format!("Could not create goal attachment directory {path}"))?;
    *output_dir = Some(path.clone());
    Ok(path)
}

async fn write_goal_file(
    app_server: &mut AppServerSession,
    path: GoalFilePath,
    bytes: Vec<u8>,
) -> Result<()> {
    app_server
        .fs_write_file_path(&path, bytes)
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))
        .with_context(|| format!("Could not write goal file {path}"))
}
fn append_section(objective: &mut String, heading: &str, lines: Vec<String>) {
    // 仅在存在引用行时追加章节，并用空行与原目标分隔。
    if lines.is_empty() {
        return;
    }
    if !objective.ends_with('\n') {
        objective.push_str("\n\n");
    }
    objective.push_str(heading);
    objective.push('\n');
    objective.push_str(&lines.join("\n"));
}

fn image_extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .take(8)
                .collect::<String>()
        })
        .filter(|extension| !extension.is_empty())
        .unwrap_or_else(|| "png".to_string())
}
