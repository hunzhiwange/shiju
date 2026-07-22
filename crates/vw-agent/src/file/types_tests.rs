use super::types::{Content, ContentType, Error, Info, Node, NodeType, SearchInput, Status};

#[test]
fn file_types_serialize_with_expected_names() {
    let info = Info { path: "a.txt".to_string(), added: 1, removed: 0, status: Status::Added };
    let node = Node {
        name: "src".to_string(),
        path: "src".to_string(),
        absolute: "/tmp/src".to_string(),
        r#type: NodeType::Directory,
        ignored: false,
    };
    let content = Content {
        r#type: ContentType::Text,
        content: "hello".to_string(),
        diff: None,
        encoding: None,
        mime_type: None,
    };

    assert!(serde_json::to_value(info).expect("serialize").to_string().contains("added"));
    assert_eq!(serde_json::to_value(node).expect("serialize")["type"], "directory");
    assert_eq!(serde_json::to_value(content).expect("serialize")["type"], "text");
    assert!(Error::AccessDenied("/root".to_string()).to_string().contains("/root"));
    assert_eq!(
        SearchInput { query: "q".to_string(), limit: 1, dirs: false, r#type: None }.limit,
        1
    );
}

#[test]
fn status_variants_round_trip_as_lowercase() {
    let cases = [
        (Status::Added, "\"added\""),
        (Status::Deleted, "\"deleted\""),
        (Status::Modified, "\"modified\""),
    ];

    for (status, expected) in cases {
        let encoded = serde_json::to_string(&status).expect("serialize status");
        assert_eq!(encoded, expected);
        let decoded: Status = serde_json::from_str(expected).expect("deserialize status");
        assert_eq!(serde_json::to_string(&decoded).expect("serialize decoded"), expected);
    }
}

#[test]
fn node_and_content_type_variants_round_trip_as_lowercase() {
    let node_file = serde_json::to_value(NodeType::File).expect("serialize node type");
    let node_directory = serde_json::to_value(NodeType::Directory).expect("serialize node type");
    let text = serde_json::to_value(ContentType::Text).expect("serialize content type");
    let binary = serde_json::to_value(ContentType::Binary).expect("serialize content type");

    assert_eq!(node_file, "file");
    assert_eq!(node_directory, "directory");
    assert_eq!(text, "text");
    assert_eq!(binary, "binary");

    assert!(matches!(serde_json::from_value::<NodeType>(node_file).unwrap(), NodeType::File));
    assert!(matches!(
        serde_json::from_value::<NodeType>(node_directory).unwrap(),
        NodeType::Directory
    ));
    assert!(matches!(serde_json::from_value::<ContentType>(text).unwrap(), ContentType::Text));
    assert!(matches!(serde_json::from_value::<ContentType>(binary).unwrap(), ContentType::Binary));
}

#[test]
fn content_omits_absent_optional_fields_and_renames_mime_type() {
    let minimal = Content {
        r#type: ContentType::Binary,
        content: "AAEC".to_string(),
        diff: None,
        encoding: None,
        mime_type: None,
    };
    let minimal_json = serde_json::to_value(&minimal).expect("serialize minimal content");
    assert_eq!(minimal_json["type"], "binary");
    assert!(!minimal_json.as_object().unwrap().contains_key("diff"));
    assert!(!minimal_json.as_object().unwrap().contains_key("encoding"));
    assert!(!minimal_json.as_object().unwrap().contains_key("mimeType"));

    let rich = Content {
        r#type: ContentType::Text,
        content: "hello".to_string(),
        diff: Some("-old\n+hello".to_string()),
        encoding: Some("utf-8".to_string()),
        mime_type: Some("text/plain".to_string()),
    };
    let rich_json = serde_json::to_value(&rich).expect("serialize rich content");
    assert_eq!(rich_json["diff"], "-old\n+hello");
    assert_eq!(rich_json["encoding"], "utf-8");
    assert_eq!(rich_json["mimeType"], "text/plain");

    let decoded: Content = serde_json::from_value(rich_json).expect("deserialize rich content");
    assert_eq!(decoded.mime_type.as_deref(), Some("text/plain"));
}

#[test]
fn structs_deserialize_and_clone_without_losing_fields() {
    let info: Info = serde_json::from_value(serde_json::json!({
        "path": "src/lib.rs",
        "added": 3,
        "removed": 2,
        "status": "modified"
    }))
    .expect("deserialize info");
    assert_eq!(info.path, "src/lib.rs");
    assert_eq!(info.added, 3);
    assert_eq!(info.removed, 2);
    assert!(matches!(info.status, Status::Modified));

    let node: Node = serde_json::from_value(serde_json::json!({
        "name": "lib.rs",
        "path": "src/lib.rs",
        "absolute": "/workspace/src/lib.rs",
        "type": "file",
        "ignored": true
    }))
    .expect("deserialize node");
    let cloned = node.clone();
    assert_eq!(cloned.name, "lib.rs");
    assert_eq!(cloned.path, "src/lib.rs");
    assert_eq!(cloned.absolute, "/workspace/src/lib.rs");
    assert!(matches!(cloned.r#type, NodeType::File));
    assert!(cloned.ignored);
}

#[test]
fn error_display_and_source_match_variants() {
    let access = Error::AccessDenied("../secret".to_string());
    assert_eq!(access.to_string(), "Access denied: path escapes project directory: ../secret");
    assert!(std::error::Error::source(&access).is_none());

    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "blocked");
    let wrapped = Error::from(io_error);
    assert_eq!(wrapped.to_string(), "blocked");
    assert!(std::error::Error::source(&wrapped).is_none());
}

#[test]
fn search_input_keeps_all_fields() {
    let input = SearchInput {
        query: "needle".to_string(),
        limit: 25,
        dirs: true,
        r#type: Some("file".to_string()),
    };
    let cloned = input.clone();

    assert_eq!(cloned.query, "needle");
    assert_eq!(cloned.limit, 25);
    assert!(cloned.dirs);
    assert_eq!(cloned.r#type.as_deref(), Some("file"));
    assert!(format!("{:?}", cloned).contains("needle"));
}

#[test]
fn lowercase_enum_types_reject_unknown_or_mis_cased_values() {
    assert!(serde_json::from_str::<Status>("\"Added\"").is_err());
    assert!(serde_json::from_str::<Status>("\"renamed\"").is_err());
    assert!(serde_json::from_str::<NodeType>("\"Directory\"").is_err());
    assert!(serde_json::from_str::<NodeType>("\"symlink\"").is_err());
    assert!(serde_json::from_str::<ContentType>("\"Text\"").is_err());
    assert!(serde_json::from_str::<ContentType>("\"archive\"").is_err());
}

#[test]
fn content_deserialization_defaults_omitted_optional_fields_to_none() {
    let content: Content = serde_json::from_value(serde_json::json!({
        "type": "text",
        "content": ""
    }))
    .expect("deserialize content without optional fields");

    assert!(matches!(content.r#type, ContentType::Text));
    assert!(content.content.is_empty());
    assert!(content.diff.is_none());
    assert!(content.encoding.is_none());
    assert!(content.mime_type.is_none());
}

#[test]
fn struct_deserialization_rejects_missing_required_fields() {
    let info = serde_json::json!({
        "path": "src/lib.rs",
        "added": 1,
        "removed": 0
    });
    let node = serde_json::json!({
        "name": "lib.rs",
        "path": "src/lib.rs",
        "absolute": "/workspace/src/lib.rs",
        "type": "file"
    });
    let content = serde_json::json!({ "type": "binary" });

    assert!(serde_json::from_value::<Info>(info).is_err());
    assert!(serde_json::from_value::<Node>(node).is_err());
    assert!(serde_json::from_value::<Content>(content).is_err());
}

#[test]
fn io_error_conversion_preserves_kind_and_message() {
    let converted = Error::from(std::io::Error::new(std::io::ErrorKind::NotFound, "missing file"));

    let Error::Io(error) = converted else {
        panic!("io errors must convert to Error::Io");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(error.to_string(), "missing file");
}
