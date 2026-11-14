use std::borrow::Cow;
use std::path::{self, Component, Path};

use http::{Request, Response, StatusCode};

use crate::paths::percent_decode;

fn is_safe_relative_path(p: &Path) -> bool {
    !p.components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_) | Component::RootDir))
}

fn guess_mime(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub fn local_protocol_response(root: &Path, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let raw_path = request.uri().path();
    let req_path = if raw_path.is_empty() { "/" } else { raw_path };
    let rel = percent_decode(req_path.trim_start_matches('/'));
    let rel_path = Path::new(&rel);

    if !rel.is_empty() && !is_safe_relative_path(rel_path) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Cow::Owned(Vec::new()))
            .unwrap();
    }

    let mut file_path = if rel.is_empty() { root.join("index.html") } else { root.join(rel_path) };
    if file_path.is_dir() {
        file_path = file_path.join("index.html");
    }

    let (status, body, mime_path) = if let Ok(bytes) = std::fs::read(&file_path) {
        (StatusCode::OK, bytes, file_path)
    } else {
        let fallback_index = root.join("index.html");
        let should_fallback = path::Path::new(req_path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| !s.contains('.'))
            .unwrap_or(true);
        if should_fallback {
            match std::fs::read(&fallback_index) {
                Ok(bytes) => (StatusCode::OK, bytes, fallback_index),
                Err(_) => (StatusCode::NOT_FOUND, Vec::new(), file_path),
            }
        } else {
            (StatusCode::NOT_FOUND, Vec::new(), file_path)
        }
    };

    Response::builder()
        .status(status)
        .header("Content-Type", guess_mime(&mime_path))
        .body(Cow::Owned(body))
        .unwrap()
}

#[cfg(test)]
mod response_tests;
