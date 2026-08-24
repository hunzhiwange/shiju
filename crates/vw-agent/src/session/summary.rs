//! 会话元数据模块
//!
//! 本模块提供会话变更统计和消息标题生成功能。
//! 主要职责包括：
//! - 计算会话中的文件变更差异（新增、删除行数）
//! - 为用户消息生成标题
//! - 管理会话级别的差异存储和事件发布

use crate::app::agent::bus;
use crate::app::agent::project::instance;
use crate::app::agent::snapshot;
use crate::app::agent::storage;
use crate::app::agent::util::log;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::LazyLock;

/// 模块日志记录器
///
/// 用于记录会话元数据更新过程中的关键事件，如标题生成结果。
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("session.summary".to_string()));
        m
    }))
});

/// 会话元数据错误类型
///
/// 封装了会话元数据更新过程中可能发生的各类错误。
#[derive(Debug)]
pub enum Error {
    /// 会话相关错误
    Session(super::session::Error),
    /// 快照差异计算错误
    Snapshot(snapshot::Error),
    /// LLM 调用错误（用于生成标题）
    Llm(super::llm::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Session(e) => write!(f, "{}", e),
            Error::Snapshot(e) => write!(f, "{}", e),
            Error::Llm(e) => write!(f, "{}", e),
        }
    }
}

/// 实现 std::error::Error trait，支持错误链式传递
impl std::error::Error for Error {}

/// 从会话错误转换
impl From<super::session::Error> for Error {
    fn from(value: super::session::Error) -> Self {
        Error::Session(value)
    }
}

/// 从快照错误转换
impl From<snapshot::Error> for Error {
    fn from(value: snapshot::Error) -> Self {
        Error::Snapshot(value)
    }
}

/// 从 LLM 错误转换
impl From<super::llm::Error> for Error {
    fn from(value: super::llm::Error) -> Self {
        Error::Llm(value)
    }
}

/// 从存储错误转换，包装为会话存储错误
impl From<storage::Error> for Error {
    fn from(value: storage::Error) -> Self {
        Error::Session(super::session::Error::Storage(value))
    }
}

/// 会话元数据更新输入参数
///
/// 指定需要更新元数据的会话和消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeInput {
    /// 会话唯一标识符
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// 消息唯一标识符
    #[serde(rename = "messageID")]
    pub message_id: String,
}

/// 差异查询输入参数
///
/// 用于查询指定会话的文件变更差异。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffInput {
    /// 会话唯一标识符
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// 可选的消息唯一标识符，用于筛选特定消息范围的差异
    #[serde(rename = "messageID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

/// 获取实例目录的可选值
///
/// 返回当前项目实例的目录路径，若为空则返回 None。
fn instance_directory_opt() -> Option<String> {
    let d = instance::directory();
    if d.is_empty() { None } else { Some(d) }
}

/// 获取工作树路径
///
/// 返回当前实例的工作树路径，若为空则返回当前目录 "."。
fn worktree_path() -> PathBuf {
    let wt = instance::worktree();
    if wt.is_empty() { PathBuf::from(".") } else { PathBuf::from(wt) }
}

/// 更新会话和消息元数据
///
/// 该函数更新两个层级的元数据：
/// 1. 会话级文件变更统计（新增/删除行数、文件数）
/// 2. 指定用户消息的标题和关联文件差异
///
/// # 参数
///
/// * `input` - 元数据更新输入参数，包含会话ID和消息ID
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回相应的错误类型
///
/// # 错误
///
/// - 会话数据读取失败
/// - 快照差异计算失败
/// - LLM 标题生成失败
pub async fn summarize(input: SummarizeInput) -> Result<(), Error> {
    // 获取会话中的所有消息
    let mut all = super::message::messages(&input.session_id, None).await?;
    // 按消息ID排序，确保时序正确
    all.sort_by(|a, b| a.info.id().cmp(b.info.id()));

    // 更新会话级文件变更元数据
    summarize_session(&input.session_id, &all).await?;
    // 更新消息级元数据
    summarize_message(&input.session_id, &input.message_id, &all).await?;
    Ok(())
}

/// 更新会话级文件变更元数据
///
/// 计算会话中所有文件变更的统计数据，并更新会话元数据。
///
/// # 参数
///
/// * `session_id` - 会话唯一标识符
/// * `messages` - 会话中的所有消息列表
///
/// # 处理流程
///
/// 1. 计算所有消息涉及的文件差异
/// 2. 统计总新增行数、删除行数和文件数
/// 3. 更新会话的摘要字段
/// 4. 持久化差异数据到存储
/// 5. 发布差异变更事件
async fn summarize_session(
    session_id: &str,
    messages: &[super::message::WithParts],
) -> Result<(), Error> {
    // 计算文件差异
    let diffs = compute_diff_inner(messages)?;
    let summary = summary_from_persisted_patch_stats(messages);
    persist_session_diff_summary(session_id, diffs, summary).await?;
    Ok(())
}

fn summary_from_diffs(diffs: &[snapshot::FileDiff]) -> super::session::Summary {
    super::session::Summary {
        additions: diffs.iter().map(|x| x.additions).sum(),
        deletions: diffs.iter().map(|x| x.deletions).sum(),
        files: diffs.len() as i64,
        diffs: None,
    }
}

fn summary_from_persisted_patch_stats(
    messages: &[super::message::WithParts],
) -> Option<super::session::Summary> {
    let mut additions = 0_i64;
    let mut deletions = 0_i64;
    let mut files = HashSet::<&str>::new();
    let mut has_complete_stats = false;

    for item in messages {
        for part in &item.parts {
            let super::message::Part::Patch(patch) = part else {
                continue;
            };
            let (Some(patch_additions), Some(patch_deletions)) = (patch.additions, patch.deletions)
            else {
                continue;
            };

            has_complete_stats = true;
            additions = additions.saturating_add(patch_additions);
            deletions = deletions.saturating_add(patch_deletions);
            files.extend(patch.files.iter().map(String::as_str));
        }
    }

    has_complete_stats.then_some(super::session::Summary {
        additions,
        deletions,
        files: files.len() as i64,
        diffs: None,
    })
}

async fn persist_session_diff_summary(
    session_id: &str,
    diffs: Vec<snapshot::FileDiff>,
    summary: Option<super::session::Summary>,
) -> Result<super::session::Summary, Error> {
    let summary = summary.unwrap_or_else(|| summary_from_diffs(&diffs));

    let summary_for_session = summary.clone();
    let _ = super::session::update(session_id, |draft| {
        draft.summary = Some(summary_for_session.clone());
    })
    .await?;

    storage::write(&["session_diff", session_id], &diffs).await?;
    let _ = bus::publish(
        super::session::event::DIFF,
        json!({ "sessionID": session_id, "diff": diffs }),
        instance_directory_opt(),
    );

    Ok(summary)
}

pub async fn refresh_session_diff_summary(
    session_id: &str,
) -> Result<super::session::Summary, Error> {
    let mut all = super::message::messages(session_id, None).await?;
    all.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    let diffs = compute_diff_inner(&all)?;
    let summary = summary_from_persisted_patch_stats(&all);
    persist_session_diff_summary(session_id, diffs, summary).await
}

/// 更新消息级元数据
///
/// 为指定的用户消息更新文件差异和标题。
/// 如果消息尚未有标题，会调用内部标题能力生成一个简洁标题。
///
/// # 参数
///
/// * `session_id` - 会话唯一标识符
/// * `message_id` - 目标用户消息的唯一标识符
/// * `messages` - 会话中的所有消息列表
///
/// # 处理流程
///
/// 1. 筛选出目标消息及其关联的助手回复
/// 2. 计算这些消息涉及的文件差异
/// 3. 更新消息的文件差异摘要
/// 4. 若消息无标题，使用内部标题能力生成标题
async fn summarize_message(
    session_id: &str,
    message_id: &str,
    messages: &[super::message::WithParts],
) -> Result<(), Error> {
    // 筛选相关消息：目标消息本身及其子消息（助手回复）
    let related = messages
        .iter()
        .filter(|m| {
            // 包含目标消息本身
            if m.info.id() == message_id {
                return true;
            }
            // 包含以该消息为父消息的助手回复
            match &m.info {
                super::message::Info::Assistant(a) => a.parent_id == message_id,
                _ => false,
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    // 查找目标用户消息
    let mut msg_with_parts = related.iter().find(|m| m.info.id() == message_id).cloned();
    let Some(msg_with_parts) = msg_with_parts.take() else { return Ok(()) };

    // 确保目标消息是用户消息
    let super::message::Info::User(user) = msg_with_parts.info.clone() else { return Ok(()) };
    let mut user = *user;

    // 计算相关消息的文件差异
    let diffs = compute_diff_inner(&related)?;

    // 初始化或更新消息摘要
    let mut summary = user.summary.clone().unwrap_or(super::message::FileDiffSummary {
        title: None,
        body: None,
        diffs: Vec::new(),
    });
    summary.diffs = diffs;
    user.summary = Some(summary);

    // 持久化消息更新
    super::message::update_message(&super::message::Info::User(Box::new(user.clone()))).await?;

    // 提取消息的文本内容（排除合成消息）
    let text_part = msg_with_parts.parts.iter().find_map(|p| match p {
        super::message::Part::Text(t) if t.synthetic != Some(true) => Some(t.text.clone()),
        _ => None,
    });

    // 若消息尚无标题，使用内部标题能力生成
    if user.summary.as_ref().and_then(|s| s.title.as_ref()).is_none() {
        let Some(text) = text_part else { return Ok(()) };
        let title = super::title::generate_message_title(session_id, user.clone(), text).await?;

        // 若生成了有效标题，更新消息。
        if let Some(title) = title {
            if let Some(s) = user.summary.as_mut() {
                s.title = Some(title.clone());
            }
            // 记录标题生成日志
            LOGGER.info("title", Some(extra([("title", Value::String(title))])));
            // 持久化消息更新
            super::message::update_message(&super::message::Info::User(Box::new(user))).await?;
        }
    }

    Ok(())
}

/// 获取会话的文件变更差异
///
/// 从存储中读取指定会话的文件差异列表，并处理 Git 路径转义。
/// 持久化消息可重新计算出非空差异时优先返回该结果；重算为空或失败时
/// 兼容回退已持久化缓存。
///
/// # 参数
///
/// * `input` - 差异查询输入参数，包含会话ID
///
/// # 返回值
///
/// 返回文件差异列表，若无数据则返回空列表
///
/// # 处理细节
///
/// Git 在输出含特殊字符的文件路径时会使用 C 风格的转义格式（如 `"test\303\251.txt"`），
/// 本函数会自动检测并解码这些转义序列，确保返回正确的文件路径。
pub async fn diff(input: DiffInput) -> Vec<snapshot::FileDiff> {
    // 从存储读取差异数据，失败则返回空列表
    let cached = storage::read::<Vec<snapshot::FileDiff>>(&["session_diff", &input.session_id])
        .await
        .unwrap_or_default();

    let (diffs, from_cache) = match recompute_session_diff_for_read(&input.session_id).await {
        Ok(computed) if !computed.is_empty() => (computed, false),
        Ok(_) => (cached, true),
        Err(err) => {
            LOGGER.warn(
                "session diff read recompute failed",
                Some(extra([
                    ("sessionID", Value::String(input.session_id.clone())),
                    ("error", Value::String(err.to_string())),
                ])),
            );
            (cached, true)
        }
    };

    // 处理 Git 路径转义
    let next = diffs
        .iter()
        .map(|item| {
            let file = unquote_git_path(&item.file);
            if file == item.file {
                item.clone()
            } else {
                let mut cloned = item.clone();
                cloned.file = file;
                cloned
            }
        })
        .collect::<Vec<_>>();

    // 若有路径变更，更新存储
    let changed = next.iter().zip(diffs.iter()).any(|(a, b)| a.file != b.file);
    if from_cache && changed {
        let _ = storage::write(&["session_diff", &input.session_id], &next).await;
    }
    next
}

/// 基于持久化会话消息重新计算文件差异。
///
/// 该读取入口不更新会话摘要、差异缓存或发布事件。
async fn recompute_session_diff_for_read(
    session_id: &str,
) -> Result<Vec<snapshot::FileDiff>, Error> {
    let mut all = super::message::messages(session_id, None).await?;
    all.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    Ok(compute_diff_inner(&all)?)
}

/// 计算消息列表的文件差异
///
/// 分析消息中的快照信息，计算从第一个快照到最后一个快照之间的文件变更。
///
/// # 参数
///
/// * `input` - 消息及其部件列表
///
/// # 返回值
///
/// 成功返回文件差异列表，失败返回错误
pub fn compute_diff(input: &[super::message::WithParts]) -> Result<Vec<snapshot::FileDiff>, Error> {
    Ok(compute_diff_inner(input)?)
}

/// 计算消息列表的文件差异（内部实现）
///
/// 遍历消息列表，提取步骤开始（StepStart）和步骤结束（StepFinish）中的快照引用，
/// 然后调用快照模块计算两个快照之间的完整差异。
///
/// # 参数
///
/// * `messages` - 消息及其部件列表
///
/// # 返回值
///
/// 成功返回文件差异列表，若无有效快照对则返回空列表
///
/// # 处理逻辑
///
/// 1. 从消息中提取第一个 StepStart 的快照作为起始点
/// 2. 提取最后一个 StepFinish 的快照作为结束点
/// 3. 计算两个快照之间的文件差异
fn compute_diff_inner(
    messages: &[super::message::WithParts],
) -> Result<Vec<snapshot::FileDiff>, snapshot::Error> {
    // 起始快照引用
    let mut from: Option<String> = None;
    // 结束快照引用
    let mut to: Option<String> = None;

    // 遍历消息提取快照引用
    for item in messages {
        // 查找第一个 StepStart 中的快照作为起始点
        if from.is_none() {
            for part in &item.parts {
                if let super::message::Part::StepStart(p) = part {
                    if let Some(s) = p.snapshot.as_ref() {
                        from = Some(s.clone());
                        break;
                    }
                }
            }
        }
        // 查找最后一个 StepFinish 中的快照作为结束点
        for part in &item.parts {
            if let super::message::Part::StepFinish(p) = part {
                if let Some(s) = p.snapshot.as_ref() {
                    to = Some(s.clone());
                }
            }
        }
    }

    // 若找到了有效的快照对，计算完整差异
    if let (Some(from), Some(to)) = (from, to) {
        let worktree = worktree_path();
        let diffs = snapshot::diff_full(&worktree, &from, &to)?;
        if !diffs.is_empty() {
            return Ok(diffs);
        }
    }
    Ok(file_diffs_from_message_summaries(messages))
}

fn file_diffs_from_patch_parts(messages: &[super::message::WithParts]) -> Vec<snapshot::FileDiff> {
    let mut files = Vec::<String>::new();
    for item in messages {
        for part in &item.parts {
            let super::message::Part::Patch(patch) = part else {
                continue;
            };
            for file in &patch.files {
                if !files.iter().any(|existing| existing == file) {
                    files.push(file.clone());
                }
            }
        }
    }
    files
        .into_iter()
        .map(|file| snapshot::FileDiff {
            file,
            before: String::new(),
            after: String::new(),
            additions: 0,
            deletions: 0,
            status: Some(snapshot::DiffStatus::Modified),
        })
        .collect()
}

fn file_diffs_from_message_summaries(
    messages: &[super::message::WithParts],
) -> Vec<snapshot::FileDiff> {
    let mut diffs = Vec::<snapshot::FileDiff>::new();
    for item in messages {
        let super::message::Info::User(user) = &item.info else {
            continue;
        };
        let Some(summary) = &user.summary else {
            continue;
        };
        for diff in &summary.diffs {
            merge_file_diff(&mut diffs, diff);
        }
    }

    for diff in file_diffs_from_patch_parts(messages) {
        if !diffs.iter().any(|existing| existing.file == diff.file) {
            diffs.push(diff);
        }
    }

    diffs
}

fn merge_file_diff(diffs: &mut Vec<snapshot::FileDiff>, diff: &snapshot::FileDiff) {
    if let Some(existing) = diffs.iter_mut().find(|existing| existing.file == diff.file) {
        existing.additions += diff.additions;
        existing.deletions += diff.deletions;
        if existing.before.is_empty() && !diff.before.is_empty() {
            existing.before = diff.before.clone();
        }
        if !diff.after.is_empty() {
            existing.after = diff.after.clone();
        }
        if existing.status.is_none() {
            existing.status = diff.status.clone();
        }
        return;
    }
    diffs.push(diff.clone());
}

/// 解码 Git 路径中的转义序列
///
/// Git 在输出含特殊字符（如空格、非 ASCII 字符）的文件路径时，
/// 会使用 C 风格的双引号包裹和转义序列。此函数将其还原为原始路径。
///
/// # 参数
///
/// * `input` - 可能包含转义序列的文件路径
///
/// # 返回值
///
/// 解码后的原始文件路径
///
/// # 支持的转义序列
///
/// - 八进制转义：`\ooo`（如 `\303\251` 表示 UTF-8 字节序列）
/// - 特殊字符：`\n`（换行）、`\r`（回车）、`\t`（制表符）等
/// - 字面量：`\\`（反斜杠）、`\"`（双引号）
///
/// # 示例
///
/// - 输入 `"test\303\251.txt"` -> 输出 `"testé.txt"`
/// - 输入 `"path with spaces"` -> 输出 `"path with spaces"`
/// - 输入 `normal_path.txt` -> 输出 `normal_path.txt`（无变化）
fn unquote_git_path(input: &str) -> String {
    let s = input.trim();

    // 检查是否为双引号包裹的路径
    if !s.starts_with('"') || !s.ends_with('"') || s.len() < 2 {
        return input.to_string();
    }

    // 提取引号内的内容
    let body = &s[1..s.len() - 1];
    let mut bytes: Vec<u8> = Vec::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0usize;

    // 逐字符解析转义序列
    while i < chars.len() {
        let c = chars[i];

        // 非转义字符直接编码
        if c != '\\' {
            let mut buf = [0u8; 4];
            let out = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(out.as_bytes());
            i += 1;
            continue;
        }

        // 处理转义序列
        let next = chars.get(i + 1).copied();
        let Some(next) = next else {
            // 反斜杠后无字符，保留反斜杠
            bytes.push(b'\\');
            i += 1;
            continue;
        };

        // 处理八进制转义（\ooo 形式，最多3位）
        if next >= '0' && next <= '7' {
            let mut j = i + 1;
            let mut oct = String::new();
            // 收集最多3个八进制数字
            while j < chars.len() && oct.len() < 3 {
                let ch = chars[j];
                if ch < '0' || ch > '7' {
                    break;
                }
                oct.push(ch);
                j += 1;
            }
            // 解析八进制数为字节
            if let Ok(v) = u8::from_str_radix(&oct, 8) {
                bytes.push(v);
                i = i + 1 + oct.len();
                continue;
            }
        }

        // 处理特殊字符转义
        let escaped = match next {
            'n' => '\n',       // 换行
            'r' => '\r',       // 回车
            't' => '\t',       // 制表符
            'b' => '\u{0008}', // 退格
            'f' => '\u{000c}', // 换页
            'v' => '\u{000b}', // 垂直制表符
            '\\' => '\\',      // 反斜杠
            '"' => '"',        // 双引号
            other => other,    // 其他字符保留原样
        };
        let mut buf = [0u8; 4];
        let out = escaped.encode_utf8(&mut buf);
        bytes.extend_from_slice(out.as_bytes());
        i += 2;
    }

    // 将字节序列转换为字符串，处理无效 UTF-8
    String::from_utf8(bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).to_string())
}

/// 构造日志额外字段映射
///
/// 将键值对数组转换为 JSON Map，用于日志记录。
///
/// # 类型参数
///
/// * `N` - 键值对数量
///
/// # 参数
///
/// * `pairs` - 键值对数组
///
/// # 返回值
///
/// 包含所有键值对的 JSON Map
fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}
#[cfg(test)]
#[path = "summary_tests.rs"]
mod summary_tests;
