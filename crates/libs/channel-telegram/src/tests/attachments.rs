//! Telegram 附件处理测试模块
//!
//! 本模块包含针对 Telegram 附件处理相关函数的单元测试，
//! 覆盖附件标记解析、路径验证、文件名清理以及安全防护等功能。
//!
//! # 测试范围
//!
//! - **附件标记解析**: 测试从消息文本中提取各种类型的附件标记
//! - **路径附件检测**: 测试纯路径字符串的附件类型推断
//! - **文件名安全**: 测试文件名清理和路径遍历攻击防护
//! - **工作区路径解析**: 测试工作区内外的路径访问控制
//! - **符号链接安全**: 测试对符号链接的安全检查

use super::*;
use crate::tests::helpers::*;

/// 测试解析附件标记能够提取多种类型的附件
///
/// # 测试场景
///
/// - 输入消息包含一个 IMAGE 类型的本地文件路径标记
/// - 输入消息同时包含一个 DOCUMENT 类型的 URL 标记
/// - 验证清理后的文本中标记被移除
/// - 验证提取出的附件数量、类型和目标路径正确
#[test]
fn parse_attachment_markers_extracts_multiple_types() {
    // 准备包含两种不同类型附件标记的测试消息
    let message = "Here are files [IMAGE:/tmp/a.png] and [DOCUMENT:https://example.com/a.pdf]";

    // 解析附件标记
    let (cleaned, attachments) = parse_attachment_markers(message);

    // 验证清理后的文本（标记被替换为空）
    assert_eq!(cleaned, "Here are files  and");
    // 验证成功提取两个附件
    assert_eq!(attachments.len(), 2);
    // 验证第一个附件是 IMAGE 类型且目标路径正确
    assert_eq!(attachments[0].kind, TelegramAttachmentKind::Image);
    assert_eq!(attachments[0].target, "/tmp/a.png");
    // 验证第二个附件是 DOCUMENT 类型且目标 URL 正确
    assert_eq!(attachments[1].kind, TelegramAttachmentKind::Document);
    assert_eq!(attachments[1].target, "https://example.com/a.pdf");
}

/// 测试解析附件标记保留未知类型标记在文本中
///
/// # 测试场景
///
/// - 输入消息包含一个未知类型（UNKNOWN）的标记
/// - 验证未知标记不被解析，保留在原文本中
/// - 验证提取出的附件列表为空
#[test]
fn parse_attachment_markers_keeps_invalid_markers_in_text() {
    // 准备包含未知类型标记的测试消息
    let message = "Report [UNKNOWN:/tmp/a.bin]";

    // 尝试解析附件标记
    let (cleaned, attachments) = parse_attachment_markers(message);

    // 验证未知标记保留在文本中（未被移除）
    assert_eq!(cleaned, "Report [UNKNOWN:/tmp/a.bin]");
    // 验证没有提取出任何附件
    assert!(attachments.is_empty());
}

/// 测试解析附件标记能够处理文件名中的括号
///
/// # 测试场景
///
/// - 输入消息的文件路径包含方括号字符
/// - 验证解析器能够正确匹配标记的起始和结束括号
/// - 验证文件名中的括号不会导致解析错误
#[test]
fn parse_attachment_markers_handles_brackets_in_filename() {
    // 准备文件名中包含方括号的测试消息
    let message = "Here it is [VIDEO:/mnt/clips/Butters - What What [G4PvTrTp7Tc].mp4]";

    // 解析附件标记
    let (cleaned, attachments) = parse_attachment_markers(message);

    // 验证清理后的文本
    assert_eq!(cleaned, "Here it is");
    // 验证成功提取一个附件
    assert_eq!(attachments.len(), 1);
    // 验证附件类型为 VIDEO
    assert_eq!(attachments[0].kind, TelegramAttachmentKind::Video);
    // 验证文件名中的括号被正确保留
    assert_eq!(attachments[0].target, "/mnt/clips/Butters - What What [G4PvTrTp7Tc].mp4");
}

/// 测试未闭合括号时回退到原始文本
///
/// # 测试场景
///
/// - 输入消息包含不完整的标记（缺少闭合括号）
/// - 验证解析失败时不破坏原始文本
/// - 验证提取出的附件列表为空
#[test]
fn parse_attachment_markers_unclosed_bracket_falls_back_to_text() {
    // 准备包含未闭合括号的测试消息
    let message = "send [VIDEO:/path/file[broken.mp4";

    // 尝试解析附件标记
    let (cleaned, attachments) = parse_attachment_markers(message);

    // 验证原始文本被完整保留（未做修改）
    assert_eq!(cleaned, "send [VIDEO:/path/file[broken.mp4");
    // 验证没有提取出任何附件
    assert!(attachments.is_empty());
}

/// 测试纯路径附件解析能够检测已存在的文件
///
/// # 测试场景
///
/// - 创建临时目录并写入一个虚拟的 PNG 文件
/// - 使用该文件路径调用纯路径附件解析函数
/// - 验证返回的附件类型被正确推断为 IMAGE
/// - 验证返回的目标路径与输入路径一致
#[test]
fn parse_path_only_attachment_detects_existing_file() {
    // 创建临时目录
    let dir = tempfile::tempdir().unwrap();
    // 在临时目录中创建一个虚拟的 PNG 文件
    let image_path = dir.path().join("snap.png");
    std::fs::write(&image_path, b"fake-png").unwrap();

    // 解析纯路径附件
    let parsed = parse_path_only_attachment(image_path.to_string_lossy().as_ref())
        .expect("expected attachment");

    // 验证附件类型被推断为 IMAGE
    assert_eq!(parsed.kind, TelegramAttachmentKind::Image);
    // 验证目标路径正确
    assert_eq!(parsed.target, image_path.to_string_lossy());
}

/// 测试纯路径附件解析拒绝句子文本
///
/// # 测试场景
///
/// - 输入是一个包含路径的句子而非纯路径
/// - 验证函数返回 None（无法识别为有效的附件路径）
#[test]
fn parse_path_only_attachment_rejects_sentence_text() {
    // 输入是包含路径的句子，不是纯路径
    assert!(parse_path_only_attachment("Screenshot saved to /tmp/snap.png").is_none());
}

/// 测试清理附件文件名能够阻止路径遍历攻击
///
/// # 测试场景
///
/// - 测试使用 `../` 进行路径遍历的文件名
/// - 测试使用 Windows 风格 `..\\` 进行路径遍历的文件名
/// - 测试仅包含 `..` 的无效文件名
/// - 测试空字符串文件名
///
/// # 预期结果
///
/// - `../../tmp/evil.txt` 应被清理为 `evil.txt`
/// - Windows 风格的路径分隔符应被替换为安全字符
/// - 仅 `..` 或空字符串应返回 None
#[test]
fn sanitize_attachment_filename_strips_path_traversal() {
    // 测试 Unix 风格的路径遍历攻击
    assert_eq!(sanitize_attachment_filename("../../tmp/evil.txt").as_deref(), Some("evil.txt"));
    // 测试 Windows 风格的路径遍历攻击（反斜杠被替换）
    assert_eq!(
        sanitize_attachment_filename(r"..\\..\\secrets\\token.env").as_deref(),
        Some("..__..__secrets__token.env")
    );
    // 测试仅包含父目录引用的无效情况
    assert!(sanitize_attachment_filename("..").is_none());
    // 测试空字符串
    assert!(sanitize_attachment_filename("").is_none());
}

/// 测试工作区附件路径解析拒绝越界访问并接受工作区文件
///
/// # 测试场景
///
/// - 创建临时目录结构，包含工作区目录和外部文件
/// - 测试工作区内的相对路径能够正确解析
/// - 测试工作区外的绝对路径被拒绝
///
/// # 安全考虑
///
/// 此测试确保用户无法通过附件路径访问工作区外的敏感文件。
#[test]
fn resolve_workspace_attachment_path_rejects_escape_and_accepts_workspace_file() {
    // 创建临时目录结构
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace should exist");

    // 在工作区内创建文件
    let in_workspace = workspace.join("report.txt");
    std::fs::write(&in_workspace, b"ok").expect("workspace fixture should be written");

    // 测试工作区内相对路径能够正确解析
    let resolved = resolve_workspace_attachment_path(&workspace, "report.txt")
        .expect("workspace relative path should resolve");
    // 验证解析后的路径位于工作区内
    assert!(resolved.starts_with(workspace.canonicalize().unwrap_or(workspace.clone())));

    // 在工作区外创建文件
    let outside = temp.path().join("outside.txt");
    std::fs::write(&outside, b"secret").expect("outside fixture should be written");

    // 测试工作区外的绝对路径被拒绝
    let escaped = resolve_workspace_attachment_path(&workspace, outside.to_string_lossy().as_ref());
    assert!(escaped.is_err(), "outside workspace path must be rejected");
}

/// 测试工作区附件路径解析接受工作区前缀映射
///
/// # 测试场景
///
/// - 创建嵌套的工作区目录结构
/// - 测试使用 `/workspace/` 前缀的绝对路径能够映射到实际工作区根目录
///
/// # 用例说明
///
/// 某些场景下，路径可能包含 `/workspace/` 前缀（如容器环境），
/// 此测试验证该前缀能够被正确映射到实际的工作区目录。
#[test]
fn resolve_workspace_attachment_path_accepts_workspace_prefix_mapping() {
    // 创建临时目录结构
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(workspace.join("sub")).expect("workspace dir should exist");

    // 在工作区子目录中创建文件
    let nested = workspace.join("sub/file.txt");
    std::fs::write(&nested, b"content").expect("fixture should be written");

    // 测试带有 /workspace 前缀的路径能够正确映射
    let resolved = resolve_workspace_attachment_path(&workspace, "/workspace/sub/file.txt")
        .expect("/workspace prefix should map to workspace root");

    // 验证解析后的路径与实际文件路径一致
    assert_eq!(resolved, nested.canonicalize().expect("canonical path should resolve"));
}

/// 测试工作区附件输出路径解析拒绝符号链接的目标目录
///
/// # 测试场景
///
/// - 创建工作区目录和外部目录
/// - 在工作区内创建指向外部目录的符号链接
/// - 验证符号链接的保存目录被拒绝
///
/// # 安全考虑
///
/// 符号链接可能被用于绕过工作区边界，访问工作区外的文件。
/// 此测试确保输出路径解析会检测并拒绝指向外部的符号链接目录。
#[tokio::test]
async fn resolve_workspace_attachment_output_path_rejects_symlinked_save_dir() {
    // 创建临时目录结构
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    tokio::fs::create_dir_all(&workspace).await.expect("workspace dir should exist");

    // 创建外部目录
    let outside = temp.path().join("outside");
    tokio::fs::create_dir_all(&outside).await.expect("outside dir should exist");

    // 在工作区内创建指向外部目录的符号链接
    symlink_dir(&outside, &workspace.join("telegram_files"));

    // 尝试解析输出路径，验证符号链接目录被拒绝
    let result = resolve_workspace_attachment_output_path(&workspace, "doc.txt").await;
    assert!(result.is_err(), "symlinked save dir must be rejected");
}

/// 测试工作区附件输出路径解析拒绝符号链接的目标文件
///
/// # 测试场景
///
/// - 创建工作区目录和保存目录
/// - 在工作区外创建一个文件
/// - 在保存目录中创建指向外部文件的符号链接
/// - 验证输出路径解析检测到符号链接并拒绝
///
/// # 安全考虑
///
/// 即使目录本身位于工作区内，如果目标文件是符号链接且指向工作区外，
/// 也应该被拒绝以防止数据泄露。
#[tokio::test]
async fn resolve_workspace_attachment_output_path_rejects_symlink_target_file() {
    // 创建临时目录结构
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let save_dir = workspace.join("telegram_files");
    tokio::fs::create_dir_all(&save_dir).await.expect("save dir should exist");

    // 在工作区外创建文件
    let outside = temp.path().join("outside.txt");
    tokio::fs::write(&outside, b"secret").await.expect("outside fixture should be written");

    // 在保存目录中创建指向外部文件的符号链接
    symlink_file(&outside, &save_dir.join("doc.txt"));

    // 尝试解析输出路径，验证符号链接文件被拒绝
    let result = resolve_workspace_attachment_output_path(&workspace, "doc.txt").await;
    assert!(result.is_err(), "symlink target file must be rejected");
}

/// 测试从目标路径推断附件类型能够识别文档扩展名
///
/// # 测试场景
///
/// - 输入一个包含查询参数的 PDF 文件 URL
/// - 验证函数能够忽略查询参数并正确识别为 DOCUMENT 类型
///
/// # 实现说明
///
/// 类型推断基于 URL 或路径的文件扩展名，
/// 此测试确保查询参数不会干扰扩展名的识别。
#[test]
fn infer_attachment_kind_from_target_detects_document_extension() {
    // 测试包含查询参数的 PDF URL
    assert_eq!(
        infer_attachment_kind_from_target("https://example.com/files/specs.pdf?download=1"),
        Some(TelegramAttachmentKind::Document)
    );
}
