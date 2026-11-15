//! 文件树选择独立于编辑器预览；修饰键释放不会清空选择。

use std::collections::BTreeSet;

use super::model::FileTreeNode;

#[derive(Debug, Default)]
pub struct FileTreeSelection {
    project: Option<String>,
    pub paths: BTreeSet<String>,
    anchor: Option<String>,
}

impl FileTreeSelection {
    pub fn contains(&self, project: &str, path: &str) -> bool {
        self.project.as_deref() == Some(project) && self.paths.contains(path)
    }

    /// 移动后保留选择与连选锚点，且只更新原项目内的路径。
    pub fn relocate(&mut self, project: &str, source: &str, target: &str) {
        if self.project.as_deref() != Some(project) {
            return;
        }
        let relocate = |path: &str| {
            std::path::Path::new(path).strip_prefix(source).ok().map(|suffix| {
                if suffix.as_os_str().is_empty() {
                    target.to_string()
                } else {
                    std::path::Path::new(target).join(suffix).to_string_lossy().into_owned()
                }
            })
        };
        self.paths =
            self.paths.iter().map(|path| relocate(path).unwrap_or_else(|| path.clone())).collect();
        if let Some(anchor) = &mut self.anchor
            && let Some(updated) = relocate(anchor)
        {
            *anchor = updated;
        }
    }

    pub fn select(
        &mut self,
        project: &str,
        path: String,
        additive: bool,
        range: bool,
        visible: &[String],
    ) {
        if self.project.as_deref() != Some(project) {
            *self = Self { project: Some(project.to_string()), ..Self::default() };
        }
        self.paths.retain(|path| visible.contains(path));
        if range
            && let Some(start) =
                self.anchor.as_ref().and_then(|p| visible.iter().position(|v| v == p))
            && let Some(end) = visible.iter().position(|v| v == &path)
        {
            if !additive {
                self.paths.clear();
            }
            self.paths.extend(visible[start.min(end)..=start.max(end)].iter().cloned());
            return;
        }
        if !additive {
            self.paths.clear();
        }
        if !additive || !self.paths.remove(&path) {
            self.paths.insert(path.clone());
        }
        self.anchor = Some(path);
    }
}

/// 顺序与列表渲染一致：目录、展开的子节点、文件；折叠节点不参与连选。
pub fn visible_paths(tree: &FileTreeNode, root: &str, expanded: &std::collections::HashSet<String>) -> Vec<String> {
    fn visit(tree: &FileTreeNode, root: &str, prefix: &str, expanded: &std::collections::HashSet<String>, paths: &mut Vec<String>) {
        for (name, child) in &tree.children {
            let relative = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            paths.push(std::path::Path::new(root).join(&relative).to_string_lossy().into_owned());
            if expanded.contains(&relative) { visit(child, root, &relative, expanded, paths); }
        }
        paths.extend(tree.files.iter().map(|path| std::path::Path::new(root).join(path).to_string_lossy().into_owned()));
    }
    let mut paths = Vec::new();
    visit(tree, root, "", expanded, &mut paths);
    paths
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
