pub fn mention_display_path(project_path: Option<&str>, path: &str) -> String {
    if let Some(project_root) = project_path {
        std::path::Path::new(path)
            .strip_prefix(project_root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or(path)
            .replace('\\', "/")
    } else {
        path.replace('\\', "/")
    }
}


/// Composer text generated from a preview selection or an explicit append action.
pub enum ContextInput<'a> {
    Position { path: &'a str, line: usize, column: usize },
    Selection { path: &'a str, start_line: usize, start_column: usize, end_line: usize, end_column: usize },
    Append(&'a str),
}

pub fn context_input_text(project_path: Option<&str>, input: ContextInput<'_>) -> Option<String> {
    match input {
        ContextInput::Position { path, line, column } => Some(format!(
            "文件:{} 行:{} 列:{}", mention_display_path(project_path, path), line, column,
        )),
        ContextInput::Selection { path, start_line, start_column, end_line, end_column } => Some(format!(
            "@{}:{}:{}-{}:{}", mention_display_path(project_path, path), start_line, start_column, end_line, end_column,
        )),
        ContextInput::Append(text) => (!text.is_empty()).then(|| text.to_owned()),
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
