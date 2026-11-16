//! 文档请求辅助逻辑，负责识别用户的文档列表意图并从工作区收集文档路径。

use std::path::{Path, PathBuf};

/// 执行 is_docs_request 操作，并返回调用方需要的结果。
pub fn is_docs_request(query: &str) -> bool { crate::runner::should_handle_docs_request_locally(query) }

/// 执行 list_docs 操作，并返回调用方需要的结果。
pub fn list_docs(root: Option<&String>) -> Result<Vec<String>, String> {
    let base = match root {
        Some(r) => PathBuf::from(r),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };
    let dir = base.join("docs");
    if !dir.exists() {
        return Err("docs 目录不存在".to_string());
    }
    let mut out = Vec::new();
    walk_docs(&base, &dir, &mut out, 0);
    out.sort();
    Ok(out)
}

fn walk_docs(base: &Path, dir: &Path, out: &mut Vec<String>, depth: usize) {
    if depth > 12 {
        return;
    }
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            walk_docs(base, &path, out, depth + 1);
            continue;
        }
        let rel = path.strip_prefix(base).unwrap_or(&path);
        out.push(rel.to_string_lossy().to_string().replace('\\', "/"));
    }
}
#[cfg(test)]
#[path = "docs_tests.rs"]
mod docs_tests;
