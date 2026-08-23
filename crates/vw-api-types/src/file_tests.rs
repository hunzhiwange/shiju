//! 验证文件 API DTO 的默认值与节点类型线协议。

use crate::file::{FileNodeDto, FileNodeKind, ListFilesRequest, WriteFileRequest};
use crate::id::ProjectId;
use serde_json::json;

// 文件节点和请求 fixture 验证类型名称、递归子节点及写入策略的兼容默认值。
#[test]
fn file_dtos_apply_defaults_and_snake_case_kinds() {
    let node: FileNodeDto = serde_json::from_value(json!({
        "path": "src/lib.rs",
        "name": "lib.rs",
        "kind": "file"
    }))
    .expect("valid file node");
    assert_eq!(node.kind, FileNodeKind::File);
    assert_eq!(node.children, None);

    let request: ListFilesRequest =
        serde_json::from_value(json!({ "project_id": "project-1" })).expect("valid list");
    assert_eq!(request.project_id, ProjectId::from("project-1"));
    assert_eq!(request.depth, None);

    let write: WriteFileRequest = serde_json::from_value(json!({
        "project_id": "project-1",
        "path": "README.md",
        "content": "hello"
    }))
    .expect("valid write request");
    assert!(!write.create_if_missing);
}
