//! File-read domain scenarios migrated from the agent host.
use tool_types::Tool;
use serde_json::json;
use std::sync::Arc;
use agent_security::{AutonomyLevel, SecurityPolicy};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
mod host;
type FileReadTool = crate::file_read::FileReadTool<host::FileReadHost>;
/// 创建基础测试安全策略
///
/// 使用默认配置创建一个监督级别的安全策略，用于大多数基本测试。
///
/// # 参数
/// - `workspace`: 测试工作区目录路径
///
/// # 返回
/// 返回包装在 `Arc` 中的安全策略实例
fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        allowed_roots: vec![workspace.clone()],
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    })
}

/// 创建可配置的测试安全策略
///
/// 允许自定义自主级别和速率限制参数，用于需要特定权限配置的测试。
///
/// # 参数
/// - `workspace`: 测试工作区目录路径
/// - `autonomy`: 自主级别（Supervised/ReadOnly/Autonomous 等）
/// - `max_actions_per_hour`: 每小时允许的最大操作数
///
/// # 返回
/// 返回包装在 `Arc` 中的自定义安全策略实例
fn test_security_with(
    workspace: std::path::PathBuf,
    autonomy: AutonomyLevel,
    max_actions_per_hour: u32,
) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy,
        allowed_roots: vec![workspace.clone()],
        workspace_dir: workspace,
        max_actions_per_hour,
        ..SecurityPolicy::default()
    })
}

fn minimal_pdf_bytes() -> Vec<u8> {
    let stream = "BT\n/F1 12 Tf\n72 720 Td\n(Hello PDF) Tj\nET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0usize);

    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    pdf
}

fn tiny_png_bytes() -> Vec<u8> {
    BASE64_STANDARD
        .decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aU+0AAAAASUVORK5CYII=",
        )
        .expect("decode tiny png")
}


/// 测试工具名称是否正确
///
/// 验证 `FileReadTool` 返回的工具名称为 `"file_read"`，
/// 这是工具注册和调用的关键标识符。
#[test]
fn file_read_name() {
    let tool = FileReadTool::new(test_security(std::env::temp_dir()));
    assert_eq!(tool.name(), "file_read");
}
/// 测试参数 schema 包含必要字段
///
/// 验证工具的 JSON Schema 定义了以下参数：
/// - `path`（必需）：文件路径
/// - `offset`（可选）：起始行号
/// - `limit`（可选）：最大行数
#[test]
fn file_read_schema_has_path() {
    let tool = FileReadTool::new(test_security(std::env::temp_dir()));
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["properties"]["offset"].is_object());
    assert!(schema["properties"]["limit"].is_object());
    assert!(schema["required"].as_array().unwrap().contains(&json!("path")));
    assert_eq!(schema["additionalProperties"], json!(false));
    // offset 和 limit 是可选参数
    assert!(!schema["required"].as_array().unwrap().contains(&json!("offset")));
}
/// 测试读取存在的文件
///
/// 验证能够成功读取工作区内的普通文件，输出应包含：
/// - 带行号前缀的内容（如 `1: hello world`）
/// - 行数统计信息（如 `[1 lines total]`）
#[tokio::test]
async fn file_read_existing_file() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("test.txt"), "hello world").await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("<file_link>"));
    assert!(result.output.contains("00001| hello world"));
    assert!(result.output.contains("(End of file: 1 lines)"));
    assert!(result.error.is_none());

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
#[tokio::test]
async fn file_read_call_returns_structured_text_result() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_structured_text");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("test.txt"), "alpha\nbeta").await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.call(json!({"path": "test.txt", "limit": 1})).await.unwrap();

    assert_eq!(result.data["kind"], json!("text"));
    assert_eq!(result.data["start_line"], json!(1));
    assert_eq!(result.data["end_line"], json!(1));
    assert_eq!(result.data["total_lines"], json!(2));
    assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("text"));
    assert_eq!(result.render_hint.as_ref().unwrap().summary.as_deref(), Some("lines 1-1 of 2"));

    let model = result.model_result.as_str().unwrap_or("");
    assert!(model.contains("00001| alpha"));
    assert!(model.contains("(File has more lines. Use 'offset' to continue after line 1)"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试读取不存在的文件
///
/// 当尝试读取不存在的文件时，应返回失败结果，
/// 错误信息应包含 "Failed to resolve" 提示。
#[tokio::test]
async fn file_read_nonexistent_file() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_missing");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "nope.txt"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("File does not exist"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试阻止路径遍历攻击
///
/// 验证工具拒绝包含 `../` 序列的路径，防止攻击者访问工作区外的文件。
/// 这是核心安全边界之一。
#[tokio::test]
async fn file_read_blocks_path_traversal() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_traversal");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "../../../etc/passwd"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("not allowed"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试阻止绝对路径访问
///
/// 验证工具拒绝以 `/` 开头的绝对路径，
/// 强制所有文件访问必须相对于工作区目录。
#[tokio::test]
async fn file_read_blocks_absolute_path() {
    let tool = FileReadTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"path": "/etc/passwd"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("not allowed"));
}
/// 测试速率限制生效
///
/// 当配置 `max_actions_per_hour` 为 0 时，
/// 任何文件读取操作都应被速率限制器拦截。
#[tokio::test]
async fn file_read_blocks_when_rate_limited() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_rate_limited");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("test.txt"), "hello world").await.unwrap();

    let tool = FileReadTool::new(test_security_with(dir.clone(), AutonomyLevel::Supervised, 0));
    let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Rate limit exceeded"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试只读模式允许读取操作
///
/// 在 `ReadOnly` 自主级别下，文件读取是允许的操作，
/// 因为它不会修改文件系统状态。
#[tokio::test]
async fn file_read_allows_readonly_mode() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_readonly");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("test.txt"), "readonly ok").await.unwrap();

    let tool = FileReadTool::new(test_security_with(dir.clone(), AutonomyLevel::ReadOnly, 20));
    let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("00001| readonly ok"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试缺少必需参数时返回错误
///
/// 当请求中不包含必需的 `path` 参数时，
/// 工具应返回错误而非静默失败。
#[tokio::test]
async fn file_read_missing_path_param() {
    let tool = FileReadTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}
#[tokio::test]
async fn file_read_rejects_unknown_fields() {
    let tool = FileReadTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"path": "a.txt", "filePath": "b.txt"})).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown field `filePath`"));
}
/// 测试读取空文件
///
/// 空文件应成功读取，输出为空字符串而非报错。
#[tokio::test]
async fn file_read_empty_file() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_empty");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("empty.txt"), "").await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "empty.txt"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("<file_link>"));
    assert!(result.output.contains("(End of file: 0 lines)"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试读取嵌套路径文件
///
/// 验证能够正确处理工作区内的深层嵌套路径（如 `sub/dir/deep.txt`）。
#[tokio::test]
async fn file_read_nested_path() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_nested");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(dir.join("sub/dir")).await.unwrap();
    tokio::fs::write(dir.join("sub/dir/deep.txt"), "deep content").await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "sub/dir/deep.txt"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("00001| deep content"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试阻止符号链接逃逸（仅 Unix 平台）
///
/// 攻击者可能在工作区内创建指向工作区外敏感文件的符号链接。
/// 本测试验证工具能够检测并阻止此类逃逸尝试。
#[cfg(unix)]
#[tokio::test]
async fn file_read_blocks_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join("vibewindow_test_file_read_symlink_escape");
    let workspace = root.join("workspace");
    let outside = root.join("outside");

    let _ = tokio::fs::remove_dir_all(&root).await;
    tokio::fs::create_dir_all(&workspace).await.unwrap();
    tokio::fs::create_dir_all(&outside).await.unwrap();

    tokio::fs::write(outside.join("secret.txt"), "outside workspace").await.unwrap();

    symlink(outside.join("secret.txt"), workspace.join("escape.txt")).unwrap();

    let tool = FileReadTool::new(test_security(workspace.clone()));
    let result = tool.execute(json!({"path": "escape.txt"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("escapes workspace"));

    let _ = tokio::fs::remove_dir_all(&root).await;
}
/// 测试禁用 workspace_only 时允许读取工作区外文件
///
/// 当安全策略中 `workspace_only` 设为 `false` 时，
/// 允许读取工作区目录外的文件（受其他约束限制）。
#[tokio::test]
async fn file_read_outside_workspace_allowed_when_workspace_only_disabled() {
    let root = std::env::temp_dir().join("vibewindow_test_file_read_allowed_roots_hint");
    let workspace = root.join("workspace");
    let outside = root.join("outside");
    let outside_file = outside.join("notes.txt");

    let _ = tokio::fs::remove_dir_all(&root).await;
    tokio::fs::create_dir_all(&workspace).await.unwrap();
    tokio::fs::create_dir_all(&outside).await.unwrap();
    tokio::fs::write(&outside_file, "outside").await.unwrap();

    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace,
        workspace_only: false,
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    });
    let tool = FileReadTool::new(security);

    let result = tool
        .execute(json!({"path": outside_file.to_string_lossy().to_string()}))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.error.is_none());
    assert!(result.output.contains("outside"));

    let _ = tokio::fs::remove_dir_all(&root).await;
}
/// 测试失败的文件读取也会消耗速率配额
///
/// 即使文件不存在，读取尝试仍会计入速率限制配额。
/// 这可以防止攻击者通过探测不存在的文件来绕过速率限制。
#[tokio::test]
async fn file_read_nonexistent_consumes_rate_limit_budget() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_probe");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    // 仅允许总共 2 次操作
    let tool = FileReadTool::new(test_security_with(dir.clone(), AutonomyLevel::Supervised, 2));

    // 两次读取都失败（文件不存在）但会消耗配额
    let r1 = tool.execute(json!({"path": "nope1.txt"})).await.unwrap();
    assert!(!r1.success);
    assert!(r1.error.as_ref().unwrap().contains("File does not exist"));

    let r2 = tool.execute(json!({"path": "nope2.txt"})).await.unwrap();
    assert!(!r2.success);
    assert!(r2.error.as_ref().unwrap().contains("File does not exist"));

    // 第三次尝试应该被速率限制，即使文件不存在
    let r3 = tool.execute(json!({"path": "nope3.txt"})).await.unwrap();
    assert!(!r3.success);
    assert!(
        r3.error.as_ref().unwrap().contains("Rate limit"),
        "Expected rate limit error, got: {:?}",
        r3.error
    );

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试 offset 和 limit 参数的分页功能
///
/// 验证以下分页场景：
/// - 同时使用 offset 和 limit 读取指定行范围
/// - 仅使用 offset 从指定行读取到文件末尾
/// - 仅使用 limit 读取前 N 行
/// - 不使用分页参数读取全部内容
#[tokio::test]
async fn file_read_with_offset_and_limit() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_offset");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("lines.txt"), "aaa\nbbb\nccc\nddd\neee").await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));

    // 读取第 2-3 行
    let result =
        tool.execute(json!({"path": "lines.txt", "offset": 2, "limit": 2})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("00002| bbb"));
    assert!(result.output.contains("00003| ccc"));
    assert!(!result.output.contains("00001| aaa"));
    assert!(!result.output.contains("00004| ddd"));
    assert!(
        result.output.contains("(File has more lines. Use 'offset' to continue after line 3)")
    );

    // 从第 4 行读取到末尾
    let result = tool.execute(json!({"path": "lines.txt", "offset": 4})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("00004| ddd"));
    assert!(result.output.contains("00005| eee"));
    assert!(result.output.contains("(End of file: 5 lines)"));

    // 仅限制行数（前 2 行）
    let result = tool.execute(json!({"path": "lines.txt", "limit": 2})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("00001| aaa"));
    assert!(result.output.contains("00002| bbb"));
    assert!(!result.output.contains("00003| ccc"));
    assert!(
        result.output.contains("(File has more lines. Use 'offset' to continue after line 2)")
    );

    // 完整读取（无 offset/limit）显示所有行
    let result = tool.execute(json!({"path": "lines.txt"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("00001| aaa"));
    assert!(result.output.contains("00005| eee"));
    assert!(result.output.contains("(End of file: 5 lines)"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试 offset 超出文件总行数
///
/// 当请求的起始行号大于文件总行数时，
/// 应返回成功但带有提示信息，而非报错。
#[tokio::test]
async fn file_read_offset_beyond_end() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_offset_end");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("short.txt"), "one\ntwo").await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "short.txt", "offset": 100})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("(No lines in range, file has 2 lines)"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试拒绝超大文件
///
/// 文件大小超过 10MB 限制时，应返回错误而非尝试读取，
/// 避免内存耗尽攻击。
#[tokio::test]
async fn file_read_rejects_oversized_file() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_large");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    // 创建一个刚好超过 10MB 的文件
    let big = vec![b'x'; 10 * 1024 * 1024 + 1];
    tokio::fs::write(dir.join("huge.bin"), &big).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "huge.bin"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("File too large"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试 PDF 文件文本提取
///
/// PDF 文件应通过 pdf-extract 库提取文本内容，
/// 而非返回原始二进制数据。
#[tokio::test]
async fn file_read_extracts_pdf_text() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_pdf");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "report.pdf"})).await.unwrap();

    assert!(result.success, "PDF read must succeed, error: {:?}", result.error);
    assert!(
        result.output.contains("Hello"),
        "extracted text must contain 'Hello', got: {}",
        result.output
    );

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
#[tokio::test]
async fn file_read_call_returns_structured_pdf_result() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_pdf_structured");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.call(json!({"path": "report.pdf"})).await.unwrap();

    assert_eq!(result.data["kind"], json!("pdf"));
    assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("pdf"));
    assert!(result.model_result.as_str().unwrap_or("").contains("<pdf>"));
    assert!(result.model_result.as_str().unwrap_or("").contains("Hello"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试非 UTF-8 二进制文件的容错读取
///
/// 对于非 UTF-8 编码的二进制文件（非 PDF），
/// 应使用 lossy 转换方式读取，无效字节替换为 Unicode 替换字符。
#[tokio::test]
async fn file_read_lossy_reads_binary_file() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_lossy");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    // 写入非有效 UTF-8 字节且非 PDF 格式的数据
    let binary_data: Vec<u8> = vec![0x00, 0x80, 0xFF, 0xFE, b'h', b'i', 0x80];
    tokio::fs::write(dir.join("data.bin"), &binary_data).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "data.bin"})).await.unwrap();

    assert!(result.success, "lossy read must succeed, error: {:?}", result.error);
    assert!(
        result.output.contains('\u{FFFD}'),
        "lossy output must contain replacement character, got: {:?}",
        result.output
    );
    assert!(
        result.output.contains("hi"),
        "lossy output must preserve valid ASCII, got: {:?}",
        result.output
    );

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
#[tokio::test]
async fn file_read_reads_image_as_structured_result() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_image");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("pixel.png"), tiny_png_bytes()).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.call(json!({"path": "pixel.png"})).await.unwrap();

    assert_eq!(result.data["kind"], json!("image"));
    assert_eq!(result.data["mime_type"], json!("image/png"));
    assert_eq!(result.data["width"], json!(1));
    assert_eq!(result.data["height"], json!(1));
    assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("image"));

    let model = result.model_result.as_str().unwrap_or("");
    assert!(model.contains("<image>"));
    assert!(model.contains("[IMAGE:data:image/png;base64,"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
#[tokio::test]
async fn file_read_reads_notebook_as_structured_result() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_notebook");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(
        dir.join("demo.ipynb"),
        serde_json::to_vec(&json!({
            "cells": [
                {
                    "cell_type": "markdown",
                    "metadata": {"language": "markdown"},
                    "source": ["# Title\n", "Body"]
                },
                {
                    "cell_type": "code",
                    "metadata": {"language": "python"},
                    "source": ["print('hello')\n"],
                    "outputs": [{"data": {"text/plain": ["hello"]}}]
                }
            ]
        }))
        .unwrap(),
    )
    .await
    .unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.call(json!({"path": "demo.ipynb"})).await.unwrap();

    assert_eq!(result.data["kind"], json!("notebook"));
    assert_eq!(result.data["total_cells"], json!(2));
    assert_eq!(result.data["cells"][0]["cell_type"], json!("markdown"));
    assert_eq!(result.data["cells"][1]["language"], json!("python"));
    assert_eq!(result.data["cells"][1]["output_mime_types"][0], json!("text/plain"));
    assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("notebook"));

    let model = result.model_result.as_str().unwrap_or("");
    assert!(model.contains("<notebook>"));
    assert!(model.contains("[Cell 1] markdown (markdown)"));
    assert!(model.contains("[Cell 2] code (python)"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
/// 测试阻止路径中的空字节注入
///
/// 空字节（`\0`）可能被用于截断路径字符串绕过安全检查。
/// 验证工具拒绝包含空字节的路径。
#[tokio::test]
async fn file_read_blocks_null_byte_in_path() {
    let dir = std::env::temp_dir().join("vibewindow_test_file_read_null_byte");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    let tool = FileReadTool::new(test_security(dir.clone()));
    let result = tool.execute(json!({"path": "test\0evil.txt"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("not allowed"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
