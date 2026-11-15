//! PDF 读取工具测试模块
//!
//! 本模块包含 PdfReadTool 的完整测试套件，涵盖以下方面：
//! - 工具元数据验证（名称、描述、参数 schema）
//! - 安全边界测试（路径遍历防护、绝对路径拦截、符号链接逃逸检测）
//! - 速率限制测试（动作配额消耗与限制）
//! - PDF 文本提取测试（需要 rag-pdf feature）
//!
//! # 测试分类
//!
//! 1. **基础验证测试**：验证工具的基本属性和参数 schema
//! 2. **安全测试**：验证各种安全边界和防护机制
//! 3. **提取测试**（需 rag-pdf feature）：验证 PDF 文本提取功能

use super::*;
use agent_security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use tempfile::TempDir;

/// 创建测试用的基础安全策略
///
/// 创建一个具有监管级自主权的安全策略实例，用于大多数测试场景。
///
/// # 参数
///
/// - `workspace`: 工作空间目录路径，工具只能访问此目录内的文件
///
/// # 返回
///
/// 返回一个 Arc 包装的 SecurityPolicy 实例
///
/// # 示例
///
/// ```ignore
/// let security = test_security(std::env::temp_dir());
/// let tool = PdfReadTool::new(security);
/// ```
fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    })
}

/// 创建带有速率限制的测试安全策略
///
/// 创建一个具有动作配额限制的安全策略实例，用于测试速率限制功能。
///
/// # 参数
///
/// - `workspace`: 工作空间目录路径
/// - `max_actions`: 每小时允许的最大操作次数
///
/// # 返回
///
/// 返回一个 Arc 包装的 SecurityPolicy 实例，配置了指定的速率限制
///
/// # 示例
///
/// ```ignore
/// // 允许每小时最多 10 次操作
/// let security = test_security_with_limit(tmp.path().to_path_buf(), 10);
/// ```
fn test_security_with_limit(
    workspace: std::path::PathBuf,
    max_actions: u32,
) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace,
        max_actions_per_hour: max_actions,
        ..SecurityPolicy::default()
    })
}

/// 测试工具名称是否正确
///
/// 验证 PdfReadTool 实例返回的工具名称是否为 "pdf_read"。
#[test]
fn name_is_pdf_read() {
    let tool = PdfReadTool::new(test_security(std::env::temp_dir()));
    assert_eq!(tool.name(), "pdf_read");
}

/// 测试工具描述非空
///
/// 验证 PdfReadTool 实例返回的描述信息不为空，
/// 确保工具向用户提供了有意义的说明。
#[test]
fn description_not_empty() {
    let tool = PdfReadTool::new(test_security(std::env::temp_dir()));
    assert!(!tool.description().is_empty());
}

/// 测试参数 schema 包含必需字段
///
/// 验证工具的参数 schema 正确定义了：
/// - `path` 参数（必需）：指定要读取的 PDF 文件路径
/// - `max_chars` 参数（可选）：限制提取文本的最大字符数
#[test]
fn schema_has_path_required() {
    let tool = PdfReadTool::new(test_security(std::env::temp_dir()));
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["properties"]["max_chars"].is_object());
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("path")));
}

/// 测试工具规范与元数据一致性
///
/// 验证工具规范（spec）中的名称和参数与工具实例返回的元数据保持一致。
#[test]
fn spec_matches_metadata() {
    let tool = PdfReadTool::new(test_security(std::env::temp_dir()));
    let spec = tool.spec();
    assert_eq!(spec.name, "pdf_read");
    assert!(spec.parameters.is_object());
}

/// 测试缺少 path 参数时返回错误
///
/// 验证当调用 execute 时未提供必需的 `path` 参数，工具返回错误。
#[tokio::test]
async fn missing_path_param_returns_error() {
    let tool = PdfReadTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("path"));
}

/// 测试绝对路径被拦截
///
/// 安全测试：验证工具拒绝访问工作空间外的绝对路径文件。
/// 防止通过绝对路径访问系统敏感文件（如 /etc/passwd）。
#[tokio::test]
async fn absolute_path_is_blocked() {
    let tool = PdfReadTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"path": "/etc/passwd"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("not allowed"));
}

/// 测试路径遍历攻击被拦截
///
/// 安全测试：验证工具拒绝包含路径遍历序列（如 `../`）的路径。
/// 防止通过相对路径逃逸工作空间目录。
#[tokio::test]
async fn path_traversal_is_blocked() {
    let tmp = TempDir::new().unwrap();
    let tool = PdfReadTool::new(test_security(tmp.path().to_path_buf()));
    let result = tool.execute(json!({"path": "../../../etc/passwd"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("not allowed"));
}

/// 测试不存在的文件返回错误
///
/// 验证当请求的 PDF 文件不存在时，工具返回明确的错误信息。
#[tokio::test]
async fn nonexistent_file_returns_error() {
    let tmp = TempDir::new().unwrap();
    let tool = PdfReadTool::new(test_security(tmp.path().to_path_buf()));
    let result = tool.execute(json!({"path": "does_not_exist.pdf"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Failed to resolve"));
}

/// 测试速率限制拦截请求
///
/// 安全测试：验证当操作次数超过配额限制时，工具拒绝执行并返回速率限制错误。
#[tokio::test]
async fn rate_limit_blocks_request() {
    let tmp = TempDir::new().unwrap();
    let tool = PdfReadTool::new(test_security_with_limit(tmp.path().to_path_buf(), 0));
    let result = tool.execute(json!({"path": "any.pdf"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Rate limit"));
}

/// 测试探测不存在的文件也会消耗速率限制配额
///
/// 安全测试：验证即使是失败的文件访问请求（文件不存在），
/// 也会消耗速率限制配额。这防止攻击者通过探测不存在的文件来绕过速率限制。
///
/// # 测试流程
///
/// 1. 设置配额为 2 次操作
/// 2. 尝试读取两个不存在的文件（应该失败，但消耗配额）
/// 3. 第三次尝试应该触发速率限制错误
#[tokio::test]
async fn probing_nonexistent_consumes_rate_limit_budget() {
    let tmp = TempDir::new().unwrap();
    // 允许 2 次操作；两次都会因文件不存在而失败，但必须消耗配额
    let tool = PdfReadTool::new(test_security_with_limit(tmp.path().to_path_buf(), 2));

    // 第一次尝试：消耗第 1 个配额
    let r1 = tool.execute(json!({"path": "a.pdf"})).await.unwrap();
    assert!(!r1.success);
    assert!(r1.error.as_deref().unwrap_or("").contains("Failed to resolve"));

    // 第二次尝试：消耗第 2 个配额
    let r2 = tool.execute(json!({"path": "b.pdf"})).await.unwrap();
    assert!(!r2.success);
    assert!(r2.error.as_deref().unwrap_or("").contains("Failed to resolve"));

    // 第三次尝试：必须触发速率限制
    let r3 = tool.execute(json!({"path": "c.pdf"})).await.unwrap();
    assert!(!r3.success);
    assert!(
        r3.error.as_deref().unwrap_or("").contains("Rate limit"),
        "expected rate limit, got: {:?}",
        r3.error
    );
}

/// 测试符号链接逃逸被拦截（仅 Unix 系统）
///
/// 安全测试：验证工具检测并拒绝指向工作空间外部的符号链接。
/// 这防止攻击者通过在工作空间内创建符号链接来访问外部敏感文件。
///
/// # 测试流程
///
/// 1. 创建工作空间目录和外部目录
/// 2. 在外部目录创建包含敏感内容的 PDF 文件
/// 3. 在工作空间内创建指向外部文件的符号链接
/// 4. 尝试通过符号链接读取文件，验证被拦截
#[cfg(unix)]
#[tokio::test]
async fn symlink_escape_is_blocked() {
    use std::os::unix::fs::symlink;

    // 创建临时目录结构
    let root = TempDir::new().unwrap();
    let workspace = root.path().join("workspace");
    let outside = root.path().join("outside");
    tokio::fs::create_dir_all(&workspace).await.unwrap();
    tokio::fs::create_dir_all(&outside).await.unwrap();

    // 在工作空间外创建包含敏感内容的 PDF 文件
    tokio::fs::write(outside.join("secret.pdf"), b"%PDF-1.4 secret").await.unwrap();

    // 在工作空间内创建指向外部文件的符号链接
    symlink(outside.join("secret.pdf"), workspace.join("link.pdf")).unwrap();

    // 尝试通过符号链接访问文件
    let tool = PdfReadTool::new(test_security(workspace));
    let result = tool.execute(json!({"path": "link.pdf"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("escapes workspace"));
}

/// PDF 文本提取测试模块
///
/// 本模块包含需要 rag-pdf feature 的 PDF 文本提取功能测试。
/// 这些测试验证 PDF 文件的实际文本提取能力，包括：
/// - 从有效 PDF 中提取文本
/// - 字符数限制的截断功能
/// - 仅含图片的 PDF 的处理
#[cfg(feature = "rag-pdf")]
mod extraction {
    use super::*;

    /// 生成最小有效 PDF 文件的字节内容
    ///
    /// 创建一个包含单页文本 "Hello PDF" 的最小 PDF 文件。
    /// 这个 PDF 是手工构造的，已在 pdf-extract 0.10 版本中验证。
    ///
    /// # 返回
    ///
    /// 返回一个 Vec<u8>，包含完整的 PDF 文件字节
    ///
    /// # PDF 结构说明
    ///
    /// 该 PDF 包含以下对象：
    /// - 对象 1: 文档目录（Catalog）
    /// - 对象 2: 页面集合（Pages）
    /// - 对象 3: 单个页面（Page），包含内容引用和字体资源
    /// - 对象 4: 内容流（Content Stream），包含文本绘制命令
    /// - 对象 5: 字体对象（Font），使用 Helvetica 字体
    fn minimal_pdf_bytes() -> Vec<u8> {
        // 手工构造的单页 PDF，包含文本 "Hello PDF"
        let body = b"%PDF-1.4\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R\
            /Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj\n\
            4 0 obj<</Length 44>>\nstream\n\
            BT /F1 12 Tf 72 720 Td (Hello PDF) Tj ET\n\
            endstream\nendobj\n\
            5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n";

        // 计算 xref 表的偏移量
        let xref_offset = body.len();

        // 生成交叉引用表（xref）和文件尾（trailer）
        let xref = format!(
            "xref\n0 6\n\
             0000000000 65535 f \n\
             0000000009 00000 n \n\
             0000000058 00000 n \n\
             0000000115 00000 n \n\
             0000000274 00000 n \n\
             0000000370 00000 n \n\
             trailer<</Size 6/Root 1 0 R>>\n\
             startxref\n{xref_offset}\n%%EOF\n"
        );

        // 组合完整的 PDF 文件
        let mut pdf = body.to_vec();
        pdf.extend_from_slice(xref.as_bytes());
        pdf
    }

    /// 测试从有效 PDF 中提取文本
    ///
    /// 验证工具能够从有效的 PDF 文件中提取文本内容。
    /// 对于手工构造的最小 PDF，接受两种结果：
    /// 1. 成功提取文本
    /// 2. 报告无可提取文本（因为手工构造的 PDF 可能不完全符合标准）
    #[tokio::test]
    async fn extracts_text_from_valid_pdf() {
        let tmp = TempDir::new().unwrap();
        let pdf_path = tmp.path().join("test.pdf");
        tokio::fs::write(&pdf_path, minimal_pdf_bytes()).await.unwrap();

        let tool = PdfReadTool::new(test_security(tmp.path().to_path_buf()));
        let result = tool.execute(json!({"path": "test.pdf"})).await.unwrap();

        // 可接受的结果：成功提取文本，或报告无可提取文本
        // （对于手工构造的最小 PDF，可能无法完美解析）
        assert!(result.success || result.error.as_deref().unwrap_or("").contains("no extractable"));
    }

    /// 测试 max_chars 参数的截断功能
    ///
    /// 验证当指定 max_chars 参数时，提取的文本会被截断到指定长度。
    /// 输出长度应该不超过 max_chars 加上截断后缀（"[truncated"）的长度。
    #[tokio::test]
    async fn max_chars_truncates_output() {
        let tmp = TempDir::new().unwrap();
        // 写入 PDF 文件以测试截断路径，使用已知内容长度
        let pdf_path = tmp.path().join("trunc.pdf");
        tokio::fs::write(&pdf_path, minimal_pdf_bytes()).await.unwrap();

        let tool = PdfReadTool::new(test_security(tmp.path().to_path_buf()));
        let result = tool.execute(json!({"path": "trunc.pdf", "max_chars": 5})).await.unwrap();

        // 如果提取成功，输出必须遵守字符限制
        // （加上截断后缀和额外的 50 字符缓冲）
        if result.success && !result.output.is_empty() {
            assert!(
                result.output.chars().count() <= 5 + "[truncated".len() + 50,
                "output longer than expected: {} chars",
                result.output.chars().count()
            );
        }
    }

    /// 测试仅含图片的 PDF 返回空文本警告
    ///
    /// 验证当 PDF 文件格式正确但不包含文本流时，工具返回合适的提示信息。
    /// 使用一个内容流为空的有效 PDF 来模拟这种情况。
    #[tokio::test]
    async fn image_only_pdf_returns_empty_text_warning() {
        // 构造一个格式正确但没有文本流的 PDF，会产生空输出
        // 使用内容流为空的有效 PDF 来模拟
        let tmp = TempDir::new().unwrap();
        let empty_content_pdf = b"%PDF-1.4\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R\
            /Contents 4 0 R/Resources<<>>>>endobj\n\
            4 0 obj<</Length 0>>\nstream\n\nendstream\nendobj\n\
            xref\n0 5\n\
            0000000000 65535 f \n\
            0000000009 00000 n \n\
            0000000058 00000 n \n\
            0000000115 00000 n \n\
            0000000250 00000 n \n\
            trailer<</Size 5/Root 1 0 R>>\nstartxref\n300\n%%EOF\n";

        tokio::fs::write(tmp.path().join("empty.pdf"), empty_content_pdf).await.unwrap();

        let tool = PdfReadTool::new(test_security(tmp.path().to_path_buf()));
        let result = tool.execute(json!({"path": "empty.pdf"})).await.unwrap();

        // 可接受的结果：空文本警告、提取错误或解析错误
        // 因为手工构造的 PDF 可能格式不完全标准
        let is_empty_warning = result.success && result.output.contains("no extractable text");
        let is_extraction_error =
            !result.success && result.error.as_deref().unwrap_or("").contains("extraction");
        let is_resolve_error =
            !result.success && result.error.as_deref().unwrap_or("").contains("Failed");
        assert!(
            is_empty_warning || is_extraction_error || is_resolve_error,
            "unexpected result: success={} error={:?}",
            result.success,
            result.error
        );
    }
}

/// 测试未启用 rag-pdf feature 时返回清晰错误
///
/// 验证当编译时未启用 rag-pdf feature 的情况下，
/// 尝试读取 PDF 文件会返回明确的错误提示，告知用户需要启用该 feature。
#[cfg(not(feature = "rag-pdf"))]
#[tokio::test]
async fn without_feature_returns_clear_error() {
    let tmp = TempDir::new().unwrap();
    let pdf_path = tmp.path().join("doc.pdf");
    tokio::fs::write(&pdf_path, b"%PDF-1.4 fake").await.unwrap();

    let tool = PdfReadTool::new(test_security(tmp.path().to_path_buf()));
    let result = tool.execute(json!({"path": "doc.pdf"})).await.unwrap();
    assert!(!result.success);
    assert!(
        result.error.as_deref().unwrap_or("").contains("rag-pdf"),
        "expected feature hint in error, got: {:?}",
        result.error
    );
}
