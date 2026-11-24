use agent_bus as bus;
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
    #[cfg(not(target_arch = "wasm32"))]
    let git_ignore = git_ignore_for_directory(&resolved);
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().into_owned();
        if name == ".git" || name == ".DS_Store" {
            continue;
        }

        let is_dir = entry.file_type().ok().is_some_and(|file_type| file_type.is_dir());
        #[cfg(not(target_arch = "wasm32"))]
        if git_ignore.as_ref().is_some_and(|ignore| ignore(&file_name, is_dir)) {
            continue;
        }

        let full = entry.path();
        let rel = full.strip_prefix(root).unwrap_or(full.as_path());
        let rel = rel.to_string_lossy().to_string().replace('\\', "/");
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

#[cfg(not(target_arch = "wasm32"))]
fn git_ignore_for_directory(dir: &Path) -> Option<impl Fn(&std::ffi::OsStr, bool) -> bool> {
    let dir = dir.canonicalize().ok()?;
    let repo = git2::Repository::discover(&dir).ok()?;
    let workdir = repo.workdir()?.canonicalize().ok()?;
    let relative_dir = dir.strip_prefix(&workdir).ok()?.to_path_buf();
    let index = std::cell::OnceCell::new();

    Some(move |name: &std::ffi::OsStr, is_dir| {
        let path = relative_dir.join(name);
        if !repo.status_should_ignore(&path).unwrap_or(false) {
            return false;
        }

        // Git ignore patterns only hide untracked entries. If the index is
        // unreadable, keep the entry visible.
        let Some(index) = index.get_or_init(|| repo.index().ok()) else {
            return false;
        };
        if (0..=3).any(|stage| index.get_path(&path, stage).is_some()) {
            return false;
        }
        if is_dir {
            let mut prefix = path.as_os_str().as_encoded_bytes().to_vec();
            #[cfg(windows)]
            for byte in &mut prefix {
                if *byte == b'\\' {
                    *byte = b'/';
                }
            }
            prefix.push(b'/');
            return matches!(index.find_prefix(prefix), Err(err) if err.code() == git2::ErrorCode::NotFound);
        }
        true
    })
}
