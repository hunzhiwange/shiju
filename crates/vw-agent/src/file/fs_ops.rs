use crate::app::agent::bus;
use base64::Engine;
use serde_json::{Map, Value};
use std::path::Path;

use super::pathing::{
    contains_path, image_mime_type, is_binary_by_extension, is_image_by_extension,
};
use super::{Content, ContentType, Error, Node, NodeType, event, ignore, ripgrep};

/// 初始化预加载。
pub fn init_preload(cwd: impl AsRef<Path>) {
    let _ = ripgrep::files(ripgrep::FilesInput {
        cwd: cwd.as_ref().to_path_buf(),
        glob: None,
        hidden: Some(true),
        follow: Some(false),
        max_depth: None,
    });
}

/// 发布文件编辑事件。
pub fn publish_edited(file: impl Into<String>) {
    let mut props = Map::new();
    props.insert("file".to_string(), Value::String(file.into()));
    bus::publish_value(event::EDITED, Value::Object(props), None);
}

/// 读取文件内容。
pub fn read(root: impl AsRef<Path>, file: &str) -> Result<Content, Error> {
    let root = root.as_ref();
    let full = root.join(file);

    if !contains_path(root, &full) {
        return Err(Error::AccessDenied(full.to_string_lossy().to_string()));
    }

    if is_image_by_extension(file) {
        if let Ok(bytes) = std::fs::read(&full) {
            let content = base64::engine::general_purpose::STANDARD.encode(bytes);
            return Ok(Content {
                r#type: ContentType::Text,
                content,
                diff: None,
                encoding: Some("base64".to_string()),
                mime_type: Some(image_mime_type(file)),
            });
        }

        return Ok(Content {
            r#type: ContentType::Text,
            content: String::new(),
            diff: None,
            encoding: None,
            mime_type: None,
        });
    }

    if is_binary_by_extension(file) {
        return Ok(Content {
            r#type: ContentType::Binary,
            content: String::new(),
            diff: None,
            encoding: None,
            mime_type: None,
        });
    }

    let content = match super::read_text(root, file) {
        Ok(content) => content.trim().to_string(),
        Err(Error::BinaryFile(_)) => {
            return Ok(Content {
                r#type: ContentType::Binary,
                content: String::new(),
                diff: None,
                encoding: None,
                mime_type: None,
            });
        }
        // Preserve the legacy content endpoint's missing-file/directory behavior.
        Err(Error::Io(err)) if err.kind() == std::io::ErrorKind::NotFound || full.is_dir() => {
            String::new()
        }
        Err(err) => return Err(err),
    };
    Ok(Content { r#type: ContentType::Text, content, diff: None, encoding: None, mime_type: None })
}

/// 列出目录内容。
pub fn list(root: impl AsRef<Path>, dir: Option<&str>) -> Result<Vec<Node>, Error> {
    let root = root.as_ref();
    let resolved = if let Some(dir) = dir { root.join(dir) } else { root.to_path_buf() };

    if !contains_path(root, &resolved) {
        return Err(Error::AccessDenied(resolved.to_string_lossy().to_string()));
    }

    let mut nodes = Vec::new();
    let entries = std::fs::read_dir(&resolved)?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" || name == ".DS_Store" {
            continue;
        }

        let full = entry.path();
        let rel = full.strip_prefix(root).unwrap_or(full.as_path());
        let rel = rel.to_string_lossy().to_string().replace('\\', "/");
        let is_dir = entry.file_type().ok().is_some_and(|file_type| file_type.is_dir());
        let ty = if is_dir { NodeType::Directory } else { NodeType::File };
        let ignore_path =
            if is_dir { format!("{}/", rel.trim_end_matches('/')) } else { rel.clone() };
        let ignored = ignore::matches(&ignore_path, None, None);
        if ignored {
            continue;
        }

        nodes.push(Node {
            name,
            path: rel,
            absolute: full.to_string_lossy().to_string(),
            r#type: ty,
            ignored,
        });
    }

    nodes.sort_by(|a, b| match (&a.r#type, &b.r#type) {
        (NodeType::Directory, NodeType::File) => std::cmp::Ordering::Less,
        (NodeType::File, NodeType::Directory) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(nodes)
}
