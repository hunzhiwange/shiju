use super::*;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::{FileReadTool, PdfReadTool, Tool};
use base64::Engine as _;
use pdf_extract::{Document, Object, Stream, dictionary};
use serde_json::json;
use std::sync::Arc;

fn two_page_pdf() -> Vec<u8> {
    let mut document = Document::with_version("1.4");
    let pages_id = document.new_object_id();
    let mut kids = Vec::new();
    for color in ["1 0 0", "0 0 1"] {
        let content = document.add_object(Stream::new(
            dictionary! {},
            format!("{color} rg 0 0 200 100 re f").into_bytes(),
        ));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content,
        });
        kids.push(Object::Reference(page));
    }
    document.objects.insert(
        pages_id,
        dictionary! {
            "Type" => "Pages", "Kids" => kids, "Count" => 2,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {},
        }
        .into(),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn page_selection_preserves_second_page_and_inherited_resources() {
    let (bytes, total) = select_page(&two_page_pdf(), 2).unwrap();
    assert_eq!(total, 2);
    let document = Document::load_mem(&bytes).unwrap();
    let pages = document.get_pages();
    assert_eq!(pages.len(), 1);
    let content = document.get_page_content(pages[&1]).unwrap();
    assert_eq!(String::from_utf8(content).unwrap().trim(), "0 0 1 rg 0 0 200 100 re f");
    assert!(document.get_dictionary(pages[&1]).unwrap().has(b"Parent"));
}

#[test]
fn page_selection_rejects_invalid_pdf_and_page_ranges() {
    assert!(select_page(b"invalid", 1).is_err());
    for page in [0, 3, u32::MAX] {
        assert!(
            select_page(&two_page_pdf(), page).unwrap_err().to_string().contains("out of range")
        );
    }
}

#[tokio::test]
async fn image_mode_rejects_invalid_arguments_before_rendering() {
    let tool = PdfReadTool::new(Arc::new(SecurityPolicy::default()));
    for arguments in [
        json!({"path":"a.pdf", "mode":"unknown"}),
        json!({"path":"a.pdf", "mode":true}),
        json!({"path":"a.pdf", "mode":"image", "page":0}),
        json!({"path":"a.pdf", "mode":"image", "page":-1}),
        json!({"path":"a.pdf", "mode":"image", "page":"2"}),
    ] {
        assert!(tool.execute(arguments).await.is_err());
    }
}

#[tokio::test]
async fn file_read_suggests_image_mode_for_pdf_without_text() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("scan.pdf"), two_page_pdf()).unwrap();
    let tool = FileReadTool::new(Arc::new(SecurityPolicy {
        workspace_dir: workspace.path().to_path_buf(),
        ..SecurityPolicy::default()
    }));
    let result = tool.execute(json!({"path":"scan.pdf"})).await.unwrap();
    assert!(result.success, "{:?}", result.error);
    assert!(result.output.contains("pdf_read with mode='image' and page=1"));
}

#[test]
fn image_call_keeps_large_image_out_of_text_result() {
    let encoded = "A".repeat(10_000);
    let output = format!("PDF page 1 of 1; PNG 1600x800\n[IMAGE:data:image/png;base64,{encoded}]");
    let result = PdfReadTool::call_result(crate::app::agent::tools::ToolResult {
        success: true,
        output: output.clone(),
        error: None,
    });
    assert_eq!(result.model_result, json!("PDF page 1 of 1; PNG 1600x800"));
    assert_eq!(result.extra_messages.len(), 1);
    assert_eq!(result.extra_messages[0].role, "user");
    assert_eq!(result.extra_messages[0].content, json!(output));
}

#[cfg(unix)]
#[tokio::test]
async fn image_mode_denies_symlink_escape() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let source = outside.path().join("source.pdf");
    std::fs::write(&source, two_page_pdf()).unwrap();
    std::os::unix::fs::symlink(source, workspace.path().join("linked.pdf")).unwrap();
    let tool = PdfReadTool::new(Arc::new(SecurityPolicy {
        workspace_dir: workspace.path().to_path_buf(),
        ..SecurityPolicy::default()
    }));
    let result = tool.execute(json!({"path":"linked.pdf", "mode":"image"})).await.unwrap();
    assert!(!result.success);
    assert!(result.output.is_empty());
}

#[tokio::test]
#[ignore = "Requires macOS sips or Poppler pdftoppm"]
async fn image_mode_renders_requested_page_as_inline_png() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("扫描件.pdf"), two_page_pdf()).unwrap();
    let tool = PdfReadTool::new(Arc::new(SecurityPolicy {
        workspace_dir: workspace.path().to_path_buf(),
        ..SecurityPolicy::default()
    }));
    let result = tool.call(json!({"path":"扫描件.pdf", "mode":"image", "page":2})).await.unwrap();
    assert!(result.is_success(), "{:?}", result.error_text());
    assert!(result.model_result.as_str().unwrap().starts_with("PDF page 2 of 2; PNG"));
    let encoded = result.extra_messages[0]
        .content
        .as_str()
        .unwrap()
        .split("[IMAGE:data:image/png;base64,")
        .nth(1)
        .unwrap()
        .strip_suffix(']')
        .unwrap();
    let png = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
    let image = image::load_from_memory(&png).unwrap().to_rgb8();
    assert!(image.width() <= 1600 && image.height() <= 1600);
    let pixel = image.get_pixel(image.width() / 2, image.height() / 2);
    assert!(pixel[2] > 240 && pixel[0] < 15 && pixel[1] < 15, "{pixel:?}");
}
