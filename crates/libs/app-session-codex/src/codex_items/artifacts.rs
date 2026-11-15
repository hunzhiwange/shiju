use super::*;

pub(crate) fn diff_summary(files: Vec<DiffFile>, full: Option<ContentRef>) -> DiffSummary {
    DiffSummary {
        added: files.iter().map(|f| f.added).sum(),
        removed: files.iter().map(|f| f.removed).sum(),
        files,
        full,
    }
}

/// apply_patch 的输出表明补丁没有落盘：校验失败（`apply_patch verification failed: …`，
/// 模型写坏的补丁）或被用户拒绝。这类输出没有退出码，不认出来会按成功显示。
pub(crate) fn apply_patch_rejected(output: &str) -> bool {
    let output = output.trim_start();
    output.starts_with("apply_patch verification failed") || output.starts_with("patch rejected")
}

/// apply_patch 文本（`*** Begin Patch` 格式）→ 文件列表与增删行数
pub(crate) fn parse_apply_patch(patch: &str) -> Vec<DiffFile> {
    let mut files: Vec<DiffFile> = Vec::new();
    for line in patch.lines() {
        let header = |prefix: &str| line.strip_prefix(prefix).map(str::trim);
        if let Some(path) = header("*** Add File:") {
            files.push(DiffFile {
                path: path.to_string(),
                op: DiffOp::Add,
                added: 0,
                removed: 0,
            });
        } else if let Some(path) = header("*** Update File:") {
            files.push(DiffFile {
                path: path.to_string(),
                op: DiffOp::Update,
                added: 0,
                removed: 0,
            });
        } else if let Some(path) = header("*** Delete File:") {
            files.push(DiffFile {
                path: path.to_string(),
                op: DiffOp::Delete,
                added: 0,
                removed: 0,
            });
        } else if let Some(path) = header("*** Move to:") {
            if let Some(file) = files.last_mut() {
                file.op = DiffOp::Rename;
                file.path = path.to_string();
            }
        } else if line.starts_with("*** End Patch") {
            break;
        } else if line.starts_with("***") {
            continue;
        } else if let Some(file) = files.last_mut() {
            if line.starts_with('+') {
                file.added += 1;
            } else if line.starts_with('-') && file.op != DiffOp::Add {
                // 新增文件只有 `+` 行；`-` 开头的是写坏补丁里漏了前缀的正文
                file.removed += 1;
            }
        }
    }
    files
}

/// 文件改动的 kind：全是新增 → Write，其余 Edit
pub(crate) fn diff_kind(diff: &DiffSummary) -> ToolKind {
    if !diff.files.is_empty() && diff.files.iter().all(|f| f.op == DiffOp::Add) {
        ToolKind::Write
    } else {
        ToolKind::Edit
    }
}

/// 文件改动的标题：单文件为路径，多文件为 `首个路径 等 N 个文件`
pub(crate) fn diff_title(diff: &DiffSummary) -> String {
    match diff.files.as_slice() {
        [] => String::new(),
        [file] => title_path(&file.path),
        [first, ..] => one_line_title(&format!("{} 等 {} 个文件", first.path, diff.files.len())),
    }
}

// ─── 图片 ────────────────────────────────────────────────────────────────

/// `data:image/png;base64,…` → 内联图片引用；不是 data URL 时：本地路径 → LocalFile，其余忽略。
pub(crate) fn image_from_url(url: &str, span: JsonlSpan, pointer: String) -> Option<ImageRef> {
    if let Some(rest) = url.strip_prefix("data:") {
        let (meta, data) = rest.split_once(',')?;
        let media_type = meta.split(';').next().filter(|m| !m.is_empty());
        return Some(ImageRef {
            source: ImageSource::Inline {
                content: span.content_ref(pointer),
            },
            media_type: media_type.unwrap_or("image/png").to_string(),
            size: app_session_core::blocks::estimate_base64_size(data.len()),
            alt: None,
        });
    }
    if url.starts_with("file://") || url.starts_with('/') {
        return Some(local_image(strip_file_url(url)));
    }
    None
}

/// 本地图片文件引用；大小取自文件元数据（文件已不存在时为 0）
pub(crate) fn local_image(path: &str) -> ImageRef {
    let ext = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let media_type = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };
    let size = 0;
    ImageRef {
        source: ImageSource::LocalFile {
            path: path.to_string(),
        },
        media_type: media_type.to_string(),
        size,
        alt: None,
    }
}

// ─── block 构造 ───────────────────────────────────────────────────────────

/// 工具调用块的公共部分
pub(crate) struct CallSpec<'s> {
    pub id: String,
    pub raw_name: String,
    pub kind: ToolKind,
    pub title: String,
    pub detail: Option<String>,
    pub server: Option<String>,
    pub input: &'s str,
    /// 参数全文的引用（只在预览截断时带上）
    pub input_ref: Option<ContentRef>,
    pub diff: Option<DiffSummary>,
    pub by_user: bool,
}

impl<'s> CallSpec<'s> {
    /// 只填必需字段的调用，其余字段按需用 `..` 覆盖
    pub fn new(
        id: String,
        raw_name: impl Into<String>,
        kind: ToolKind,
        title: String,
        input: &'s str,
    ) -> Self {
        Self {
            id,
            raw_name: raw_name.into(),
            kind,
            title,
            detail: None,
            server: None,
            input,
            input_ref: None,
            diff: None,
            by_user: false,
        }
    }

    pub fn into_block(self) -> SessionBlock {
        let input = preview_chars(self.input, INPUT_PREVIEW_CHARS);
        SessionBlock::ToolCall {
            id: self.id,
            raw_name: self.raw_name,
            kind: self.kind,
            title: self.title,
            detail: self.detail,
            server: self.server,
            input_full: if input.truncated {
                self.input_ref
            } else {
                None
            },
            input_preview: input.text,
            input_total_len: input.total_len,
            diff: self.diff,
            by_user: self.by_user,
        }
    }
}

/// 工具结果块
pub(crate) struct ResultSpec {
    pub call_id: String,
    pub status: ToolStatus,
    pub output: Preview,
    pub full: Option<ContentRef>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub images: Vec<ImageRef>,
}

impl ResultSpec {
    /// 没有输出的结果
    pub fn empty(call_id: String, status: ToolStatus) -> Self {
        Self {
            call_id,
            status,
            output: preview(""),
            full: None,
            exit_code: None,
            duration_ms: None,
            images: Vec::new(),
        }
    }

    pub fn into_block(self) -> SessionBlock {
        let truncated = self.output.truncated;
        SessionBlock::ToolResult {
            call_id: self.call_id,
            status: self.status,
            preview: self.output.text,
            total_len: self.output.total_len,
            line_count: self.output.line_count,
            truncated,
            full: if truncated { self.full } else { None },
            exit_code: self.exit_code,
            duration_ms: self.duration_ms,
            images: self.images,
            saved_path: None,
        }
    }
}

impl CommandInfo {
    /// 单独成块的 CommandExecution 工具调用（没有对应 exec 时）
    pub fn call_spec(&self) -> CallSpec<'_> {
        CallSpec {
            detail: self.cwd.clone(),
            by_user: self.by_user,
            ..CallSpec::new(
                self.id.clone(),
                "CommandExecution",
                self.kind,
                title_shell(&self.command),
                &self.command,
            )
        }
    }

    pub fn result_spec(&self, call_id: String) -> ResultSpec {
        ResultSpec {
            output: self.output.clone(),
            full: self.output_ref.clone(),
            exit_code: self.exit_code,
            duration_ms: self.duration_ms,
            ..ResultSpec::empty(call_id, self.status)
        }
    }
}
