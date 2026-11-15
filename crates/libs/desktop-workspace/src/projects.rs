use crate::project_message::edit_meta::RecentProjectMeta;
#[derive(Debug, Clone)]
pub struct ProjectBranchSnapshot {
    pub project_path: String,
    pub selected_branch: Option<String>,
    pub branches: Vec<String>,
}

pub fn save_recent_projects_background(dirs: Option<directories::ProjectDirs>, recent: Vec<String>) {
    let _ = std::thread::Builder::new()
        .name("vw-save-recent-projects".to_string())
        .spawn(move || save_recent_projects(dirs.clone(), &recent));
}

pub fn save_recent_projects_meta_background(dirs: Option<directories::ProjectDirs>, v: Vec<RecentProjectMeta>) {
    let _ = std::thread::Builder::new()
        .name("vw-save-recent-projects-meta".to_string())
        .spawn(move || save_recent_projects_meta(dirs.clone(), &v));
}

/// 返回最近项目列表存储路径
///
/// 该路径基于平台数据目录（`data_local_dir`）下的 `recent_projects.json` 文件。
/// 如果无法获取项目目录（例如平台不支持），则返回 [`None`]。
///
/// # 返回值
///
/// - `Some(PathBuf)` - 指向 `recent_projects.json` 的完整路径
/// - `None` - 无法确定项目数据目录时返回
fn recent_projects_path(dirs: Option<directories::ProjectDirs>) -> Option<std::path::PathBuf> {
    // 优先使用平台本地数据目录，确保与 XDG/Data Local 等约定一致
    dirs.clone()
        .map(|d: directories::ProjectDirs| d.data_local_dir().join("recent_projects.json"))
}

/// 从磁盘加载最近项目路径列表
///
/// 按优先级依次尝试以下候选路径：
/// 1. `data_local_dir/recent_projects.json`（首选）
/// 2. `data_dir/recent_projects.json`
/// 3. `config_dir/recent_projects.json`
///
/// 找到首个可解析的文件后立即返回；如果文件位于非首选路径，会将其迁移到首选路径。
/// 若所有候选路径均不存在或解析失败，则返回空列表。
///
/// # 返回值
///
/// - `Vec<String>` - 最近打开的项目路径列表（已去重与规范化）
pub fn load_recent_projects(dirs: Option<directories::ProjectDirs>) -> Vec<String> {
    // 获取项目目录，失败则返回空列表
    let Some(dirs) = dirs.clone() else {
        return vec![];
    };

    // 按优先级定义候选路径：local data > data > config
    let candidate_paths: [std::path::PathBuf; 3] = [
        dirs.data_local_dir().join("recent_projects.json"),
        dirs.data_dir().join("recent_projects.json"),
        dirs.config_dir().join("recent_projects.json"),
    ];

    // 依次尝试读取并解析，成功即返回
    for path in candidate_paths {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(recent) = parse_recent_projects(&content) else {
            continue;
        };

        // 如果是从旧路径加载，则迁移到首选路径
        if path != dirs.data_local_dir().join("recent_projects.json") {
            save_recent_projects_background(Some(dirs.clone()), recent.clone());
        }

        return recent;
    }

    // 所有候选路径均未找到可解析内容
    vec![]
}

/// 将最近项目路径列表持久化到磁盘
///
/// 写入到平台 `data_local_dir` 下的 `recent_projects.json` 文件。
/// 如果父目录不存在会自动创建；写入失败时静默忽略错误。
///
/// # 参数
///
/// - `recent` - 要保存的项目路径列表（仅包含路径字符串）
pub fn save_recent_projects(dirs: Option<directories::ProjectDirs>, recent: &Vec<String>) {
    // 无法获取目标路径时直接返回
    let Some(path) = recent_projects_path(dirs.clone()) else {
        return;
    };
    // 确保父目录存在
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // 序列化为 JSON；失败时回退为空数组
    let content = serde_json::to_string(recent).unwrap_or_else(|_| "[]".to_string());
    let _ = std::fs::write(path, content);
}

/// 解析最近项目列表内容（兼容多种历史格式）
///
/// 支持以下 JSON 格式：
/// 1. 字符串数组：`["/path1", "/path2"]`
/// 2. 元数据对象数组：`[{"path": "/path1", ...}, ...]`
/// 3. 包含 `recent_projects` 字段的对象：`{"recent_projects": [...]}`
///
/// 解析成功后会对结果进行规范化（去重、去空白）。
///
/// # 参数
///
/// - `content` - 从文件读取的原始 JSON 字符串
///
/// # 返回值
///
/// - `Some(Vec<String>)` - 解析成功且经过规范化的路径列表
/// - `None` - 无法识别的格式或解析失败
fn parse_recent_projects(content: &str) -> Option<Vec<String>> {
    // 尝试解析为简单的字符串数组
    if let Ok(v) = serde_json::from_str::<Vec<String>>(content) {
        return Some(normalize_recent_projects(v));
    }

    // 尝试解析为元数据对象数组（旧版本格式）
    if let Ok(v) = serde_json::from_str::<Vec<RecentProjectMeta>>(content) {
        return Some(normalize_recent_projects(v.into_iter().map(|m| m.path).collect()));
    }

    // 尝试解析为通用 JSON 值，以处理更复杂的结构
    let Ok(v) = serde_json::from_str::<serde_json::Value>(content) else {
        return None;
    };

    let mut out: Vec<String> = vec![];

    match v {
        // 直接数组：逐项提取路径
        serde_json::Value::Array(items) => {
            for item in items {
                match item {
                    serde_json::Value::String(s) => out.push(s),
                    // 对象中尝试提取 "path" 字段
                    serde_json::Value::Object(map) => {
                        if let Some(serde_json::Value::String(path)) = map.get("path") {
                            out.push(path.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        // 嵌套对象：从 "recent_projects" 字段提取数组
        serde_json::Value::Object(map) => {
            let Some(serde_json::Value::Array(items)) = map.get("recent_projects") else {
                return None;
            };
            for item in items {
                match item {
                    serde_json::Value::String(s) => out.push(s.clone()),
                    serde_json::Value::Object(map) => {
                        if let Some(serde_json::Value::String(path)) = map.get("path") {
                            out.push(path.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    Some(normalize_recent_projects(out))
}

fn recent_project_dir_name(path: &str) -> Option<&str> {
    let path = path.trim().trim_end_matches(['/', '\\']);
    if path.is_empty() {
        return None;
    }

    path.rsplit(['/', '\\']).find(|part| !part.is_empty())
}

pub fn is_visible_recent_project_path(path: &str) -> bool {
    recent_project_dir_name(path).is_some_and(|name| !name.starts_with('.'))
}

/// 规范化最近项目列表
///
/// 处理逻辑：
/// - 去除每项首尾空白
/// - 过滤空字符串
/// - 过滤隐藏目录项目
/// - 去重（保留首次出现的顺序）
///
/// # 参数
///
/// - `recent` - 原始路径列表
///
/// # 返回值
///
/// - `Vec<String>` - 经过规范化的路径列表
fn normalize_recent_projects(recent: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(recent.len());
    for p in recent {
        let p = p.trim().to_string();
        // 跳过空项
        if p.is_empty() {
            continue;
        }
        // 隐藏目录通常是系统/运行时临时工作区，不展示为用户最近项目
        if !is_visible_recent_project_path(&p) {
            continue;
        }
        // 跳过重复项（保持首次出现顺序）
        if out.iter().any(|x| x == &p) {
            continue;
        }
        out.push(p);
    }
    out
}

fn normalize_recent_projects_meta(recent: Vec<RecentProjectMeta>) -> Vec<RecentProjectMeta> {
    let mut out: Vec<RecentProjectMeta> = Vec::with_capacity(recent.len());
    for mut meta in recent {
        let path = meta.path.trim().to_string();
        if path.is_empty() || !is_visible_recent_project_path(&path) {
            continue;
        }
        if out.iter().any(|item| item.path == path) {
            continue;
        }

        meta.path = path;
        out.push(meta);
    }
    out
}

/// 将项目路径加入最近列表（去重、置顶、截断、持久化）
///
/// 操作步骤：
/// 1. 如果路径已存在，先移除旧位置
/// 2. 将路径插入到列表头部
/// 3. 截断列表至最多 10 项
/// 4. 持久化到磁盘
///
/// # 参数
///
/// - `recent` - 可变的最近项目列表引用
/// - `path` - 要添加的项目路径
pub fn push_recent_project(dirs: Option<directories::ProjectDirs>, recent: &mut Vec<String>, path: String) {
    // 移除已存在的同名路径（确保置顶）
    if let Some(pos) = recent.iter().position(|p| p == &path) {
        recent.remove(pos);
    }
    // 插入到头部
    recent.insert(0, path);
    // 限制列表长度为 10
    recent.truncate(10);
    // 后台持久化，避免切换项目时阻塞 UI
    save_recent_projects_background(dirs.clone(), recent.clone());
}

/// 确保项目路径存在于最近列表，已存在时保持原顺序。
///
/// 该函数用于普通项目切换，避免用户点击已有项目时列表位置变化。
pub fn ensure_recent_project(dirs: Option<directories::ProjectDirs>, recent: &mut Vec<String>, path: String) {
    if recent.iter().any(|p| p == &path) {
        return;
    }
    recent.insert(0, path);
    recent.truncate(10);
    save_recent_projects_background(dirs.clone(), recent.clone());
}

/// 返回最近项目元数据存储路径
///
/// 该路径基于平台数据目录（`data_local_dir`）下的 `recent_projects_meta.json` 文件。
/// 如果无法获取项目目录，则返回 [`None`]。
///
/// # 返回值
///
/// - `Some(PathBuf)` - 指向 `recent_projects_meta.json` 的完整路径
/// - `None` - 无法确定项目数据目录时返回
fn recent_projects_meta_path(dirs: Option<directories::ProjectDirs>) -> Option<std::path::PathBuf> {
    // 与 recent_projects.json 保持同一目录
    dirs.clone()
        .map(|d: directories::ProjectDirs| d.data_local_dir().join("recent_projects_meta.json"))
}

/// 从磁盘加载最近项目元数据列表
///
/// 按优先级依次尝试以下候选路径：
/// 1. `data_local_dir/recent_projects_meta.json`（首选）
/// 2. `data_dir/recent_projects_meta.json`
/// 3. `config_dir/recent_projects_meta.json`
///
/// 找到首个可解析的文件后立即返回；如果文件位于非首选路径，会将其迁移到首选路径。
/// 若所有候选路径均不存在或解析失败，则返回空列表。
///
/// # 返回值
///
/// - `Vec<RecentProjectMeta>` - 最近项目的元数据列表
pub fn load_recent_projects_meta(dirs: Option<directories::ProjectDirs>) -> Vec<RecentProjectMeta> {
    // 获取项目目录，失败则返回空列表
    let Some(dirs) = dirs.clone() else {
        return vec![];
    };

    // 按优先级定义候选路径
    let candidate_paths = [
        dirs.data_local_dir().join("recent_projects_meta.json"),
        dirs.data_dir().join("recent_projects_meta.json"),
        dirs.config_dir().join("recent_projects_meta.json"),
    ];

    // 依次尝试读取并解析
    for path in candidate_paths {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Vec<RecentProjectMeta>>(&content) else {
            continue;
        };
        let v = normalize_recent_projects_meta(v);

        // 如果是从旧路径加载，则迁移到首选路径
        if path != dirs.data_local_dir().join("recent_projects_meta.json") {
            save_recent_projects_meta_background(Some(dirs.clone()), v.clone());
        }

        return v;
    }

    // 所有候选路径均未找到可解析内容
    vec![]
}

/// 将最近项目元数据列表持久化到磁盘
///
/// 写入到平台 `data_local_dir` 下的 `recent_projects_meta.json` 文件。
/// 如果父目录不存在会自动创建；写入失败时静默忽略错误。
///
/// # 参数
///
/// - `v` - 要保存的元数据列表引用
pub fn save_recent_projects_meta(dirs: Option<directories::ProjectDirs>, v: &Vec<RecentProjectMeta>) {
    // 无法获取目标路径时直接返回
    let Some(path) = recent_projects_meta_path(dirs.clone()) else {
        return;
    };
    // 确保父目录存在
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // 序列化为 JSON；失败时回退为空数组
    let content = serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string());
    let _ = std::fs::write(path, content);
}


#[cfg(test)]
#[path = "projects_tests.rs"]
mod projects_tests;

/// Stable tab identity and display title for a project path.
pub fn project_tab(path: &str) -> (String, String) {
    let title = std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string();
    (format!("project_{}", path), title)
}
pub fn recent_project_labels(recent: &[String], meta: &[RecentProjectMeta]) -> Vec<String> {
    recent.iter().map(|path| {
        if let Some(meta) = meta.iter().find(|meta| &meta.path == path) { meta.name.clone() }
        else { std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string() }
    }).collect()
}
pub fn project_board_settings(meta: &[RecentProjectMeta], path: &str) -> desktop_task_model::models::TaskBoardSettings {
    meta.iter().find(|meta| meta.path == path).and_then(|meta| meta.task_board_settings.clone())
        .unwrap_or_else(desktop_task_model::models::TaskBoardSettings::new).sanitized()
}
pub fn refresh_branches(project_path: Option<&str>, selected_branch: &mut Option<String>,
    branches: &mut Vec<String>, updated_at: &mut Option<u64>) {
    if project_path.is_some() { *selected_branch = None; branches.clear(); *updated_at = None; }
}

pub fn apply_branch_snapshot(current_project: Option<&str>, snapshot: ProjectBranchSnapshot,
    selected_branch: &mut Option<String>, branches: &mut Vec<String>) {
    if current_project != Some(snapshot.project_path.as_str()) { return; }
    *selected_branch = snapshot.selected_branch;
    *branches = snapshot.branches;
}
