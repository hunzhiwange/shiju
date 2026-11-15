pub fn unique_name_in_dir(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let original = dir.join(name);
    if !original.exists() {
        return original;
    }

    let p = std::path::Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|s| s.to_str());

    for i in 1..=999 {
        let suffix = if i == 1 { "copy".to_string() } else { format!("copy {}", i) };
        let candidate_name = if let Some(ext) = ext {
            format!("{stem} {suffix}.{ext}")
        } else {
            format!("{stem} {suffix}")
        };
        let candidate = dir.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    original
}

pub fn copy_recursively(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let from = entry.path();
            let to = dst.join(entry.file_name());
            copy_recursively(&from, &to)?;
        }
        Ok(())
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dst)?;
        Ok(())
    }
}

pub fn relative_to_project(project_root: &str, path: &str) -> Option<String> {
    let root = std::path::Path::new(project_root);
    let target = std::path::Path::new(path);

    let normalize = |p: &std::path::Path| {
        let s = p.to_string_lossy().replace('\\', "/");
        if s.is_empty() { ".".to_string() } else { s }
    };

    if let Ok(rel) = target.strip_prefix(root) {
        return Some(normalize(rel));
    }

    if let (Ok(root_canon), Ok(target_canon)) =
        (std::fs::canonicalize(root), std::fs::canonicalize(target))
        && let Ok(rel) = target_canon.strip_prefix(root_canon)
    {
        return Some(normalize(rel));
    }

    let root_norm = project_root.replace('\\', "/");
    let root_norm = root_norm.trim_end_matches('/');
    let target_norm = path.replace('\\', "/");
    if target_norm == root_norm {
        return Some(".".to_string());
    }
    let prefix = format!("{}/", root_norm);
    if let Some(rel) = target_norm.strip_prefix(&prefix) {
        return Some(if rel.is_empty() { ".".to_string() } else { rel.to_string() });
    }

    None
}
