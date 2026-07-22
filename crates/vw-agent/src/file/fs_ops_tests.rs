use super::fs_ops::{init_preload, list, publish_edited, read};
use super::{ContentType, Error, NodeType};
use std::fs;

#[test]
fn read_text_file_trims_content() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("note.txt"), " hello \n").expect("write text");

    let content = read(temp.path(), "note.txt").expect("read text");

    assert!(matches!(content.r#type, ContentType::Text));
    assert_eq!(content.content, "hello");
    assert_eq!(content.encoding, None);
    assert_eq!(content.mime_type, None);
}

#[test]
fn read_accepts_owned_root_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("note.txt"), "owned").expect("write text");

    let content = read(temp.path().to_path_buf(), "note.txt").expect("read text");

    assert!(matches!(content.r#type, ContentType::Text));
    assert_eq!(content.content, "owned");
}

#[test]
fn read_missing_text_file_returns_empty_text() {
    let temp = tempfile::tempdir().expect("tempdir");

    let content = read(temp.path(), "missing.txt").expect("missing text");

    assert!(matches!(content.r#type, ContentType::Text));
    assert_eq!(content.content, "");
}

#[test]
fn read_image_encodes_bytes_as_base64_with_mime_type() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("pixel.png"), [0_u8, 1, 2, 3]).expect("write image");

    let content = read(temp.path(), "pixel.png").expect("read image");

    assert!(matches!(content.r#type, ContentType::Text));
    assert_eq!(content.content, "AAECAw==");
    assert_eq!(content.encoding.as_deref(), Some("base64"));
    assert_eq!(content.mime_type.as_deref(), Some("image/png"));
}

#[test]
fn read_image_extension_is_case_insensitive() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("photo.JPEG"), [0xff_u8, 0xd8, 0xff]).expect("write image");

    let content = read(temp.path(), "photo.JPEG").expect("read image");

    assert!(matches!(content.r#type, ContentType::Text));
    assert_eq!(content.content, "/9j/");
    assert_eq!(content.encoding.as_deref(), Some("base64"));
    assert_eq!(content.mime_type.as_deref(), Some("image/jpeg"));
}

#[test]
fn read_missing_image_returns_empty_text_without_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");

    let content = read(temp.path(), "missing.png").expect("missing image");

    assert!(matches!(content.r#type, ContentType::Text));
    assert_eq!(content.content, "");
    assert_eq!(content.encoding, None);
    assert_eq!(content.mime_type, None);
}

#[test]
fn read_binary_extension_returns_binary_content() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("archive.zip"), [0_u8, 159, 146, 150]).expect("write binary");

    let content = read(temp.path(), "archive.zip").expect("read binary");

    assert!(matches!(content.r#type, ContentType::Binary));
    assert_eq!(content.content, "");
}

#[test]
fn read_missing_binary_extension_returns_binary_without_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");

    let content = read(temp.path(), "missing.zip").expect("read binary");

    assert!(matches!(content.r#type, ContentType::Binary));
    assert_eq!(content.content, "");
    assert_eq!(content.diff, None);
    assert_eq!(content.encoding, None);
    assert_eq!(content.mime_type, None);
}

#[test]
fn read_binary_content_returns_binary_content() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("data.txt"), [0_u8, 1, 2, 3]).expect("write binary");

    let content = read(temp.path(), "data.txt").expect("read binary");

    assert!(matches!(content.r#type, ContentType::Binary));
    assert_eq!(content.content, "");
}

#[test]
fn read_rejects_paths_outside_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, "secret").expect("write outside");
    let root = temp.path().join("root");
    fs::create_dir(&root).expect("create root");

    let err = read(&root, "../outside.txt").expect_err("escape should fail");

    assert!(matches!(err, Error::AccessDenied(_)));
}

#[cfg(unix)]
#[test]
fn read_rejects_symlink_that_resolves_outside_root() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    fs::create_dir(&root).expect("create root");
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, "secret").expect("write outside");
    symlink(&outside, root.join("link.txt")).expect("create symlink");

    let err = read(&root, "link.txt").expect_err("symlink escape should fail");

    assert!(matches!(err, Error::AccessDenied(_)));
}

#[test]
fn read_directory_path_returns_empty_text() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("folder")).expect("create folder");

    let content = read(temp.path(), "folder").expect("read directory path");

    assert!(matches!(content.r#type, ContentType::Text));
    assert_eq!(content.content, "");
    assert_eq!(content.diff, None);
    assert_eq!(content.encoding, None);
    assert_eq!(content.mime_type, None);
}

#[test]
fn list_sorts_directories_before_files_and_skips_ignored_names() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("z_dir")).expect("create z dir");
    fs::create_dir(temp.path().join("a_dir")).expect("create a dir");
    fs::create_dir(temp.path().join(".git")).expect("create git");
    fs::write(temp.path().join("b.txt"), "b").expect("write b");
    fs::write(temp.path().join("a.txt"), "a").expect("write a");
    fs::write(temp.path().join(".DS_Store"), "").expect("write ds store");

    let nodes = list(temp.path(), None).expect("list root");
    let names = nodes.iter().map(|node| node.name.as_str()).collect::<Vec<_>>();

    assert_eq!(names, vec!["a_dir", "z_dir", "a.txt", "b.txt"]);
    assert!(nodes[0].absolute.ends_with("a_dir"));
    assert!(matches!(nodes[0].r#type, NodeType::Directory));
    assert!(matches!(nodes[2].r#type, NodeType::File));
    assert!(nodes.iter().all(|node| !node.ignored));
}

#[test]
fn list_accepts_owned_root_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("note.txt"), "note").expect("write note");

    let nodes = list(temp.path().to_path_buf(), None).expect("list root");

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].name, "note.txt");
    assert!(matches!(nodes[0].r#type, NodeType::File));
}

#[test]
fn list_empty_directory_selector_lists_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("src")).expect("create src");
    fs::write(temp.path().join("README.md"), "docs").expect("write readme");

    let nodes = list(temp.path(), Some("")).expect("list root");
    let names = nodes.iter().map(|node| node.name.as_str()).collect::<Vec<_>>();

    assert_eq!(names, vec!["src", "README.md"]);
    assert_eq!(nodes[0].path, "src");
    assert_eq!(nodes[1].path, "README.md");
}

#[test]
fn list_skips_default_ignored_entries() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("target")).expect("create target");
    fs::create_dir(temp.path().join("src")).expect("create src");
    fs::write(temp.path().join("src/main.rs"), "main").expect("write main");
    fs::write(temp.path().join("app.log"), "logs").expect("write logs");

    let nodes = list(temp.path(), None).expect("list root");
    let names = nodes.iter().map(|node| node.name.as_str()).collect::<Vec<_>>();

    assert_eq!(names, vec!["src"]);
}

#[test]
fn list_nested_directory_uses_forward_slash_relative_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src/nested")).expect("create nested");
    fs::write(temp.path().join("src/nested/lib.rs"), "lib").expect("write lib");

    let nodes = list(temp.path(), Some("src")).expect("list src");

    assert!(nodes.iter().any(|node| {
        node.name == "nested"
            && node.path == "src/nested"
            && matches!(node.r#type, NodeType::Directory)
    }));
}

#[test]
fn list_nested_file_reports_complete_node_fields() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("src")).expect("create src");
    fs::write(temp.path().join("src/lib.rs"), "lib").expect("write lib");

    let nodes = list(temp.path(), Some("src")).expect("list src");

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].name, "lib.rs");
    assert_eq!(nodes[0].path, "src/lib.rs");
    assert_eq!(nodes[0].absolute, temp.path().join("src/lib.rs").to_string_lossy().into_owned());
    assert!(matches!(nodes[0].r#type, NodeType::File));
    assert!(!nodes[0].ignored);
}

#[test]
fn list_empty_directory_returns_no_nodes() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("empty")).expect("create empty");

    let nodes = list(temp.path(), Some("empty")).expect("list empty");

    assert!(nodes.is_empty());
}

#[test]
fn list_rejects_paths_outside_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    fs::create_dir(&root).expect("create root");

    let err = list(&root, Some("..")).expect_err("escape should fail");

    assert!(matches!(err, Error::AccessDenied(_)));
}

#[cfg(unix)]
#[test]
fn list_rejects_symlink_directory_that_resolves_outside_root() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    let outside = temp.path().join("outside");
    fs::create_dir(&root).expect("create root");
    fs::create_dir(&outside).expect("create outside");
    symlink(&outside, root.join("link")).expect("create symlink");

    let err = list(&root, Some("link")).expect_err("symlink escape should fail");

    assert!(matches!(err, Error::AccessDenied(_)));
}

#[test]
fn list_propagates_read_dir_errors() {
    let temp = tempfile::tempdir().expect("tempdir");

    let err = list(temp.path(), Some("missing")).expect_err("missing dir should fail");

    assert!(matches!(err, Error::Io(_)));
}

#[test]
fn list_file_path_returns_io_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("note.txt"), "note").expect("write note");

    let err = list(temp.path(), Some("note.txt")).expect_err("file is not a directory");

    assert!(matches!(err, Error::Io(_)));
}

#[test]
fn publish_edited_accepts_file_path() {
    publish_edited("src/lib.rs");
}

#[test]
fn publish_edited_accepts_owned_file_path() {
    publish_edited("src/main.rs".to_string());
}

#[test]
fn init_preload_accepts_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("note.txt"), "note").expect("write note");

    init_preload(temp.path());
}

#[test]
fn init_preload_ignores_missing_directory() {
    let temp = tempfile::tempdir().expect("tempdir");

    init_preload(temp.path().join("missing"));
}
