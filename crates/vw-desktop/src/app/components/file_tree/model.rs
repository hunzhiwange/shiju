//! 文件树模型构建。
//!
//! 本模块把项目文件路径归一化为层级树，供文件树视图按目录逐层展示。

use std::collections::BTreeMap;

/// FileTreeNode 保存 model 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Debug, Clone, Default)]
pub(crate) struct FileTreeNode {
    // children 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(crate) children: BTreeMap<String, FileTreeNode>,
    // files 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(crate) files: Vec<String>,
}

impl FileTreeNode {
    /// 处理 has entries 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// `true` 表示当前输入满足该辅助函数描述的条件。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    pub(crate) fn has_entries(&self) -> bool {
        !self.files.is_empty() || !self.children.is_empty()
    }
}

/// 处理 subtree 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(test)]
fn subtree<'a>(node: &'a FileTreeNode, prefix: &str) -> Option<&'a FileTreeNode> {
    if prefix.is_empty() {
        return Some(node);
    }

    let mut current = node;
    for segment in prefix.split('/').filter(|segment| !segment.is_empty()) {
        current = current.children.get(segment)?;
    }

    Some(current)
}

/// 处理 to rel 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn to_rel(project_root: &str, path: &str) -> String {
    /// 处理 normalize 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回字符串已经按界面展示或比较需求做过必要整理。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn normalize(value: &str) -> String {
        value.trim().replace('\\', "/")
    }

    /// 处理 strip root prefix 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn strip_root_prefix<'a>(path: &'a str, root: &str) -> Option<&'a str> {
        path.strip_prefix(root)
            .map(|rest| rest.trim_start_matches('/'))
            .filter(|rest| !rest.is_empty())
    }

    let normalized_path = normalize(path);
    if normalized_path.is_empty() || normalized_path == "." {
        return String::new();
    }

    let normalized_root = normalize(project_root).trim_end_matches('/').to_string();
    if normalized_root.is_empty() {
        return normalized_path.trim_start_matches("./").trim_start_matches('/').to_string();
    }

    // 优先移除完整项目根路径，保证绝对路径和相对路径最终落到同一棵树。
    if let Some(relative) = strip_root_prefix(&normalized_path, &normalized_root) {
        return relative.trim_start_matches("./").trim_start_matches('/').to_string();
    }

    // 有些调用方传入的路径已经去掉父目录，只保留项目目录名，这里保留兼容。
    let root_name = std::path::Path::new(&normalized_root)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !root_name.is_empty() {
        let root_prefix = format!("{root_name}/");
        if let Some(relative) = normalized_path.strip_prefix(&root_prefix) {
            return relative.to_string();
        }
    }

    normalized_path.trim_start_matches("./").trim_start_matches('/').to_string()
}

/// 构建 recursive file tree 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn build_recursive_file_tree(rel_files: &[String]) -> FileTreeNode {
    let mut tree = FileTreeNode::default();

    for rel in rel_files {
        let parts = rel.split('/').filter(|segment| !segment.is_empty()).collect::<Vec<_>>();
        if parts.is_empty() {
            continue;
        }

        if parts.len() == 1 {
            tree.files.push(rel.clone());
            continue;
        }

        let mut node = &mut tree;
        for dir_name in &parts[..parts.len() - 1] {
            node = node.children.entry((*dir_name).to_string()).or_default();
        }

        node.files.push(rel.clone());
    }

    tree
}

/// 构建 file tree model 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn build_file_tree_model(project_root: &str, files: &[String]) -> FileTreeNode {
    let rel_files = files.iter().map(|path| to_rel(project_root, path)).collect::<Vec<_>>();
    build_recursive_file_tree(&rel_files)
}

/// 构建 file tree subtree 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(test)]
pub(crate) fn build_file_tree_subtree(
    project_root: &str,
    // files 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    files: &[String],
    // prefix 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    prefix: &str,
) -> FileTreeNode {
    subtree(&build_file_tree_model(project_root, files), prefix).cloned().unwrap_or_default()
}

/// 从网关目录节点构建模型，保留没有文件的目录。
pub(crate) fn build_file_tree_model_from_dto(
    project_root: &str,
    root: &vw_gateway_client::vw_api_types::file::FileNodeDto,
) -> FileTreeNode {
    use vw_gateway_client::vw_api_types::file::{FileNodeDto, FileNodeKind};
    fn insert(project_root: &str, dto: &FileNodeDto, tree: &mut FileTreeNode) {
        // DTO 路径已经相对项目根；不能剥掉恰好与项目同名的目录。
        let normalized = dto.path.replace('\\', "/");
        let normalized_root = project_root.replace('\\', "/");
        let root_prefix = format!("{}/", normalized_root.trim_end_matches('/'));
        let relative = if normalized == normalized_root {
            String::new()
        } else {
            to_rel("", normalized.strip_prefix(&root_prefix).unwrap_or(&normalized))
        };
        let parts = relative.split('/').filter(|part| !part.is_empty()).collect::<Vec<_>>();
        let mut node = &mut *tree;
        match dto.kind {
            FileNodeKind::Directory => {
                for part in &parts {
                    node = node.children.entry((*part).to_string()).or_default();
                }
                if let Some(children) = &dto.children {
                    for child in children {
                        insert(project_root, child, tree);
                    }
                }
            }
            FileNodeKind::File => {
                if parts.is_empty() {
                    return;
                }
                for part in &parts[..parts.len() - 1] {
                    node = node.children.entry((*part).to_string()).or_default();
                }
                node.files.push(relative);
            }
        }
    }
    let mut tree = FileTreeNode::default();
    insert(project_root, root, &mut tree);
    fn sort_files(node: &mut FileTreeNode) {
        node.files.sort();
        for child in node.children.values_mut() {
            sort_files(child);
        }
    }
    sort_files(&mut tree);
    tree
}
