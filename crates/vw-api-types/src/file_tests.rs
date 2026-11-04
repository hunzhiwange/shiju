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

#[test]
fn local_import_request_roundtrip() {
    use crate::file::{LocalImportRequest, LocalImportResponse};
    let wire = json!({"project_id":"project", "target_directory":"",
        "source_paths":["/external/中文.txt", "/external/folder"]});
    let request: LocalImportRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(request.worktree_id, None);
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
    let wire = json!({"items":[
        {"source_path":"/external/中文.txt","destination_path":"中文.txt","ok":true},
        {"source_path":"/external/folder","ok":false,"error":"target exists"}
    ]});
    let response: LocalImportResponse = serde_json::from_value(wire.clone()).unwrap();
    assert!(response.items[0].ok);
    assert!(!response.items[1].ok);
    assert_eq!(response.items[1].destination_path, None);
    assert_eq!(serde_json::to_value(response).unwrap(), wire);
}
