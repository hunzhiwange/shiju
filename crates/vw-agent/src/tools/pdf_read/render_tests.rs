use crate::app::agent::tools::{FileReadTool, Tool};
use agent_security::SecurityPolicy;
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
