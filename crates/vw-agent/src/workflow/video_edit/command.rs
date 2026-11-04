use super::{VideoEditCommand, VideoEditRequest};
use crate::tools::ToolRuntimeContext;
use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

const VIDEO_EXTENSIONS: &[&str] =
    &["mp4", "mov", "mkv", "webm", "avi", "m4v", "mpeg", "mpg", "mts", "m2ts", "flv", "wmv"];

fn take_word(input: &str) -> anyhow::Result<(&str, &str)> {
    let input = input.trim_start();
    if let Some(quote @ ('\'' | '"')) = input.chars().next() {
        let content = &input[quote.len_utf8()..];
        let Some(end) = content.find(quote) else { bail!("视频命令引号未闭合") };
        let rest = &content[end + quote.len_utf8()..];
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            bail!("引号路径后必须使用空白分隔剪辑要求");
        }
        return Ok((&content[..end], rest.trim_start()));
    }
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    Ok((&input[..end], input[end..].trim_start()))
}

fn has_video_extension(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| {
        VIDEO_EXTENSIONS.iter().any(|supported| extension.eq_ignore_ascii_case(supported))
    })
}

/// Parse slash input or the equivalent input emitted by the client menu.
/// Backslashes, dollar signs and backticks are literal characters, never shell syntax.
pub fn parse_video_edit_command(query: &str) -> anyhow::Result<Option<VideoEditCommand>> {
    let query = query.trim();
    let (command, tail) = query.split_once(char::is_whitespace).unwrap_or((query, ""));
    if command != "/shiju" {
        return Ok(None);
    }
    let tail = tail.trim_start();
    let (action, tail) = tail.split_once(char::is_whitespace).unwrap_or((tail, ""));
    if action != "剪辑" {
        return Ok(None);
    }
    let (path, requirements) = take_word(tail)?;
    if path.is_empty() {
        bail!("请提供一个视频路径");
    }
    let mut rest = requirements;
    while !rest.is_empty() {
        let (word, tail) = take_word(rest)?;
        if has_video_extension(Path::new(word)) {
            bail!("只支持一个视频；多个文件路径含糊不清");
        }
        rest = tail;
    }
    Ok(Some(VideoEditCommand {
        video_path: PathBuf::from(path),
        requirements: (!requirements.is_empty()).then(|| requirements.to_owned()),
    }))
}

/// Canonicalize and authorize a nonempty file using the native tool security factory.
/// Extension checks are preliminary; a cloud media probe must still confirm the input.
pub fn prepare_video_edit_request(
    query: &str,
    context: &ToolRuntimeContext,
) -> anyhow::Result<Option<VideoEditRequest>> {
    let Some(command) = parse_video_edit_command(query)? else { return Ok(None) };
    let native = context.tool_use_context();
    let project_root = native.workspace_root();
    let path = if command.video_path.is_absolute() {
        command.video_path
    } else {
        project_root.as_ref().context("相对视频路径需要当前项目目录")?.join(command.video_path)
    };
    let path = path.canonicalize().context("视频路径不存在或无法解析")?;
    let security = context.path_security_policy();
    if !security.is_resolved_path_allowed(&path) {
        bail!("原生安全策略拒绝访问视频路径");
    }
    let metadata = path.metadata().context("无法读取视频文件信息")?;
    if !metadata.is_file() {
        bail!("视频路径必须为普通文件");
    }
    if metadata.len() == 0 {
        bail!("视频文件为空");
    }
    if !has_video_extension(&path) {
        bail!("不支持的视频扩展名；媒体内容仍须云端探测");
    }
    Ok(Some(VideoEditRequest {
        video_path: path,
        original_request: query.to_owned(),
        requirements: command.requirements,
        tenant_scope: native.tenant_scope(),
        project_root,
    }))
}
