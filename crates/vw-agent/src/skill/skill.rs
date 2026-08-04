//! 技能管理模块
//!
//! 本模块提供技能的发现、加载和管理功能。技能以 SKILL.md 文件的形式存在，
//! 支持 YAML frontmatter 格式来定义技能的元数据。
//!
//! # 主要功能
//!
//! - 从多个位置扫描和加载技能文件
//! - 解析技能的 frontmatter 元数据（名称、描述）
//! - 提供技能的查询和遍历接口
//! - 支持项目级和全局级的技能管理
//!
//! # 技能文件格式
//!
//! 技能文件应命名为 SKILL.md，支持以下格式：
//!
//! ```markdown
//! ---
//! name: my-skill
//! description: 这是一个示例技能
//! ---
//!
//! # 技能内容
//! 这里是技能的实际内容...
//! ```
//!
//! # 技能查找路径
//!
//! 技能会从以下位置加载：
//! - 配置目录中的 skill/**/SKILL.md 和 skills/**/SKILL.md
//! - 当前技能目录提供方对应的项目、父级和全局 skills 目录

use crate::app::agent::config;
use crate::app::agent::flag;
use crate::app::agent::global;
use crate::app::agent::project::instance;
use crate::app::agent::util::log;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;

/// 技能模块专用日志记录器
///
/// 使用 Lazy 初始化，带有一个 "skill" 服务标识符，
/// 用于记录技能加载、解析和错误信息。
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("skill".to_string()));
        m
    }))
});

/// 技能信息结构体
///
/// 表示一个已加载的技能的完整信息，包括元数据和内容。
///
/// # 字段
///
/// - `name`: 技能的唯一标识名称，从 SKILL.md 的 frontmatter 中解析
/// - `description`: 技能的简要描述，可选，默认为空字符串
/// - `location`: 技能文件的完整路径，用于定位和调试
/// - `content`: 技能文件的实际内容（去除 frontmatter 后的部分）
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill::Info;
///
/// let skill = Info {
///     name: "code-review".to_string(),
///     description: "代码审查技能".to_string(),
///     location: "/path/to/skills/code-review/SKILL.md".to_string(),
///     content: "审查代码时请遵循...".to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Info {
    /// 技能的唯一标识名称
    pub name: String,
    /// 技能的简要描述
    pub description: String,
    /// 技能文件的完整路径
    pub location: String,
    /// 技能文件的实际内容
    pub content: String,
}

/// 技能状态结构体
///
/// 维护当前已加载的所有技能及其相关目录的状态信息。
/// 该结构体通过项目实例状态管理器进行缓存和共享。
///
/// # 字段
///
/// - `skills`: 技能名称到技能信息的映射，使用 HashMap 提供快速查找
/// - `dirs`: 所有包含技能文件的目录路径列表，用于追踪和索引
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill::State;
/// use std::collections::HashMap;
///
/// let state = State {
///     skills: HashMap::new(),
///     dirs: vec!["/path/to/skills".to_string()],
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct State {
    /// 技能名称到技能信息的映射
    pub skills: HashMap<String, Info>,
    /// 所有包含技能文件的目录路径列表
    pub dirs: Vec<String>,
}

/// 创建技能状态实例的工厂函数
///
/// 返回一个闭包，该闭包在被调用时会异步获取当前项目实例的技能状态。
/// 状态通过项目实例管理器进行缓存，避免重复加载。
///
/// # 返回值
///
/// 返回一个实现了 `Fn() -> BoxFuture<Arc<State>> + Send + Sync + 'static` 的闭包。
/// 该闭包可以被多次调用，每次都会返回当前项目实例的技能状态。
///
/// # 实现细节
///
/// - 使用 `instance::state` 函数注册状态加载器
/// - 首次访问时调用 `load_state()` 加载技能
/// - 不提供状态更新回调（使用 `None`）
fn instance_state()
-> impl Fn() -> crate::app::agent::project::BoxFuture<Arc<State>> + Send + Sync + 'static {
    instance::state(
        "skill",
        || async { load_state().await },
        None::<fn(Arc<State>) -> crate::app::agent::project::BoxFuture<()>>,
    )
}

/// 根据名称获取指定的技能
///
/// 从当前项目实例的技能状态中查找指定名称的技能信息。
///
/// # 参数
///
/// - `name`: 技能的名称（对应 SKILL.md frontmatter 中的 name 字段）
///
/// # 返回值
///
/// - `Some(Info)`: 如果找到匹配的技能，返回技能信息的克隆
/// - `None`: 如果未找到指定名称的技能
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill;
///
/// // 异步获取技能
/// if let Some(skill) = skill::get("code-review").await {
///     println!("技能描述: {}", skill.description);
/// }
/// ```
///
/// # 性能
///
/// - 从缓存的技能状态中查找，时间复杂度为 O(1)
/// - 返回 Info 的克隆，适合多次使用
pub async fn get(name: &str) -> Option<Info> {
    instance_state()().await.skills.get(name).cloned()
}

/// 获取所有已加载的技能列表
///
/// 返回当前项目实例中所有已加载技能的信息列表。
///
/// # 返回值
///
/// 返回一个 `Vec<Info>`，包含所有技能的信息克隆。
/// 如果没有加载任何技能，返回空向量。
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill;
///
/// // 获取并遍历所有技能
/// for skill in skill::all().await {
///     println!("技能名称: {}", skill.name);
///     println!("技能描述: {}", skill.description);
///     println!("位置: {}", skill.location);
/// }
/// ```
///
/// # 性能
///
/// - 从缓存的技能状态中收集，避免重新扫描文件系统
/// - 返回所有技能的克隆，适合遍历和处理
pub async fn all() -> Vec<Info> {
    instance_state()().await.skills.values().cloned().collect()
}

/// 获取所有包含技能文件的目录路径列表
///
/// 返回当前已加载技能所在的所有目录路径。
/// 可用于了解技能的来源分布和文件系统布局。
///
/// # 返回值
///
/// 返回一个 `Vec<String>`，包含所有技能所在目录的路径。
/// 如果没有加载任何技能，返回空向量。
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill;
///
/// // 获取所有技能目录
/// for dir in skill::dirs().await {
///     println!("技能目录: {}", dir);
/// }
/// ```
///
/// # 用途
///
/// - 调试和日志：了解技能加载的来源
/// - 索引和监控：追踪技能文件的位置
/// - 工具集成：提供技能目录给外部工具使用
pub async fn dirs() -> Vec<String> {
    instance_state()().await.dirs.clone()
}

/// 加载技能状态（WASM 目标平台）
///
/// 在 WebAssembly 环境中，技能加载功能被禁用，
/// 返回默认的空状态。
///
/// # 返回值
///
/// 始终返回 `State::default()`，即空技能映射和空目录列表。
///
/// # 限制
///
/// WASM 环境下不支持文件系统访问，因此无法加载技能文件。
#[cfg(target_arch = "wasm32")]
async fn load_state() -> State {
    State::default()
}

/// 加载技能状态（非 WASM 目标平台）
///
/// 从文件系统扫描并加载所有可用的技能文件。支持从多个位置加载：
/// 1. 全局主目录下的 .claude 和 .agents 目录
/// 2. 项目目录树中的 .claude 和 .agents 目录
/// 3. 配置文件中指定的目录
///
/// # 返回值
///
/// 返回一个 `State`，包含：
/// - 所有已加载技能的名称到信息的映射
/// - 所有包含技能文件的目录路径列表
///
/// # 加载优先级
///
/// 如果存在重名技能，后加载的技能会覆盖先前的，
/// 并记录警告日志。
///
/// # 性能
///
/// - 首次调用时会扫描文件系统，可能耗时较长
/// - 通过项目实例状态管理器缓存结果
/// - 使用异步 IO 提高并发性能
#[cfg(not(target_arch = "wasm32"))]
async fn load_state() -> State {
    let mut skills: HashMap<String, Info> = HashMap::new();
    let mut dirs: HashSet<String> = HashSet::new();
    let cfg = config::get().await;

    /// 内部辅助函数：添加单个技能到技能映射
    ///
    /// # 参数
    ///
    /// - `match_path`: 技能文件的路径
    /// - `skills`: 技能映射（可变引用）
    /// - `dirs`: 目录集合（可变引用）
    ///
    /// # 行为
    ///
    /// - 解析 SKILL.md 文件提取元数据和内容
    /// - 检测并记录重名技能警告
    /// - 记录技能文件所在的目录
    async fn add_skill(
        match_path: PathBuf,
        skills: &mut HashMap<String, Info>,
        dirs: &mut HashSet<String>,
    ) {
        // 尝试解析技能文件，如果格式错误则直接返回
        let Some((name, description, content)) = parse_skill_md(&match_path).await else {
            return;
        };

        // 检查是否存在重名技能，记录警告日志
        if let Some(existing) = skills.get(&name) {
            LOGGER.warn(
                "duplicate skill name",
                Some({
                    let mut m = Map::new();
                    m.insert("name".to_string(), Value::String(name.clone()));
                    m.insert("existing".to_string(), Value::String(existing.location.clone()));
                    m.insert(
                        "duplicate".to_string(),
                        Value::String(match_path.to_string_lossy().to_string()),
                    );
                    m
                }),
            );
        }

        // 记录技能文件所在的目录
        if let Some(parent) = match_path.parent() {
            dirs.insert(parent.to_string_lossy().to_string());
        }

        // 将技能信息添加到映射中
        let location = match_path.to_string_lossy().to_string();
        skills.insert(name.clone(), Info { name, description, location, content });
    }

    // 第一步：加载外部技能（如果未禁用）
    if !*flag::VIBEWINDOW_DISABLE_EXTERNAL_SKILLS {
        let start = instance::directory();
        let workspace_dir = if start.is_empty() { None } else { Some(PathBuf::from(&start)) };
        for source in crate::app::agent::skills::discover_local_skill_source_dirs_with_provider(
            workspace_dir.as_deref(),
            cfg.skills.directory_provider,
        ) {
            for match_path in glob_files(&source.path, "**/SKILL.md") {
                add_skill(match_path, &mut skills, &mut dirs).await;
            }
            log_external_scan(&source.path, "configured");
        }
    }

    // 第二步：加载配置目录中的技能
    // 支持 skill/**/SKILL.md 和 skills/**/SKILL.md 两种模式
    for dir in config::directories().await {
        let base = PathBuf::from(dir);
        for pat in ["skill/**/SKILL.md", "skills/**/SKILL.md"] {
            for match_path in glob_files(&base, pat) {
                add_skill(match_path, &mut skills, &mut dirs).await;
            }
        }
    }

    // 构建最终状态，将 HashSet 转换为 Vec
    State { skills, dirs: dirs.into_iter().collect() }
}

#[cfg(not(target_arch = "wasm32"))]
fn log_external_scan(root: &Path, scope: &str) {
    LOGGER.debug(
        "scanned external skills",
        Some({
            let mut m = Map::new();
            m.insert("scope".to_string(), Value::String(scope.to_string()));
            m.insert("dir".to_string(), Value::String(root.to_string_lossy().to_string()));
            m
        }),
    );
}

/// 使用 glob 模式查找匹配的文件
///
/// 在指定根目录下使用 glob 模式搜索文件，仅返回普通文件（不包括目录）。
///
/// # 参数
///
/// - `root`: 搜索的根目录路径
/// - `pattern`: glob 匹配模式（相对于根目录）
///
/// # 返回值
///
/// 返回所有匹配且为普通文件的路径列表 `Vec<PathBuf>`。
/// 如果 glob 模式无效或没有匹配项，返回空向量。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// // 查找所有 SKILL.md 文件
/// let files = glob_files(Path::new("/path/to/skills"), "**/SKILL.md");
///
/// // 查找特定模式的文件
/// let files = glob_files(Path::new("/config"), "skill/**/SKILL.md");
/// ```
///
/// # 错误处理
///
/// - 如果 glob 模式语法错误，静默忽略并返回空向量
/// - 如果读取目录项失败，跳过该项继续处理其他项
#[cfg(not(target_arch = "wasm32"))]
fn glob_files(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let pat = root.join(pattern).to_string_lossy().to_string();
    let mut out = Vec::new();
    if let Ok(iter) = glob::glob(&pat) {
        for entry in iter.flatten() {
            // 仅添加普通文件，忽略目录和符号链接等
            if entry.is_file() {
                out.push(entry);
            }
        }
    }
    out
}

/// 解析文件路径
///
/// 将用户提供的路径字符串解析为绝对路径，支持以下格式：
/// - `~/path`: 展开为用户主目录下的路径
/// - `/path`: 绝对路径，直接使用
/// - `path`: 相对路径，相对于当前项目目录
///
/// # 参数
///
/// - `p`: 待解析的路径字符串
///
/// # 返回值
///
/// 返回解析后的 `PathBuf`。如果输入为空或仅包含空白，返回空路径。
///
/// # 示例
///
/// ```ignore
/// // 假设用户主目录为 /home/user，项目目录为 /home/user/project
///
/// resolve_path("~/skills");      // -> /home/user/skills
/// resolve_path("/etc/config");   // -> /etc/config
/// resolve_path("local/skills");  // -> /home/user/project/local/skills
/// resolve_path("");              // -> (空路径)
/// ```
///
/// # 注意事项
///
/// - 路径中的 `~` 只在开头时才会被展开
/// - 相对路径基于项目目录而非当前工作目录
/// - 如果项目目录为空，相对路径直接返回而不进行拼接
#[cfg(not(target_arch = "wasm32"))]
fn resolve_path(p: &str) -> PathBuf {
    let p = p.trim();
    if p.is_empty() {
        return PathBuf::new();
    }

    // 展开波浪号为用户主目录
    let expanded = if let Some(rest) = p.strip_prefix("~/") {
        global::paths().home.join(rest)
    } else {
        PathBuf::from(p)
    };

    // 如果是绝对路径直接返回，否则相对于项目目录
    if expanded.is_absolute() {
        expanded
    } else {
        let base = instance::directory();
        if base.is_empty() { expanded } else { PathBuf::from(base).join(expanded) }
    }
}

/// 解析 SKILL.md 文件
///
/// 读取并解析技能 markdown 文件，提取 YAML frontmatter 中的元数据
/// 和 markdown 内容。文件格式应为：
///
/// ```markdown
/// ---
/// name: skill-name
/// description: 技能描述
/// ---
///
/// # 技能标题
/// 技能内容...
/// ```
///
/// # 参数
///
/// - `path`: SKILL.md 文件的路径
///
/// # 返回值
///
/// - `Some((name, description, content))`: 成功解析时返回元组
///   - `name`: 技能名称（必需，frontmatter 中必须存在且非空）
///   - `description`: 技能描述（可选，默认为空字符串）
///   - `content`: markdown 内容（去除 frontmatter 后的部分，已去除首尾空白）
/// - `None`: 解析失败时返回（文件读取失败、缺少 name 字段等）
///
/// # 异步处理
///
/// 使用 `spawn_blocking` 在单独的线程中执行文件读取，
/// 避免阻塞异步运行时。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// if let Some((name, desc, content)) = parse_skill_md(Path::new("SKILL.md")).await {
///     println!("技能名称: {}", name);
///     println!("描述: {}", desc);
///     println!("内容长度: {} 字节", content.len());
/// }
/// ```
///
/// # 错误情况
///
/// 以下情况会返回 `None`：
/// - 文件不存在或无法读取
/// - frontmatter 格式错误（无效的 YAML）
/// - 缺少 `name` 字段或 `name` 为空字符串
#[cfg(not(target_arch = "wasm32"))]
async fn parse_skill_md(path: &Path) -> Option<(String, String, String)> {
    // 使用 spawn_blocking 异步读取文件，避免阻塞
    let raw = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || std::fs::read_to_string(path)
    })
    .await
    .ok()
    .and_then(|x| x.ok())?;

    // 统一换行符为 \n，处理不同操作系统的换行符差异
    let raw = raw.replace("\r\n", "\n").replace('\r', "\n");

    // 分离 frontmatter 和内容
    let (frontmatter, content) = split_frontmatter(&raw);

    // 解析 YAML frontmatter，如果不存在则使用 Null 值
    let yaml = frontmatter
        .map(|s| serde_yaml::from_str::<serde_yaml::Value>(s).ok())
        .flatten()
        .unwrap_or(serde_yaml::Value::Null);

    /// 从 YAML 文档中获取字符串字段
    ///
    /// # 参数
    ///
    /// - `doc`: YAML 文档引用
    /// - `key`: 要获取的字段名称
    ///
    /// # 返回值
    ///
    /// 如果文档是 Mapping 且包含指定的字符串键，返回对应的字符串值。
    fn yaml_get_str<'a>(doc: &'a serde_yaml::Value, key: &str) -> Option<&'a str> {
        let serde_yaml::Value::Mapping(map) = doc else {
            return None;
        };
        map.get(&serde_yaml::Value::String(key.to_string())).and_then(|v| v.as_str())
    }

    // 提取必需的 name 字段（不能为空）
    let name =
        yaml_get_str(&yaml, "name").map(|s| s.trim().to_string()).filter(|s| !s.is_empty())?;

    // 提取可选的 description 字段（可以为空）
    let description =
        yaml_get_str(&yaml, "description").map(|s| s.trim().to_string()).unwrap_or_default();

    Some((name, description, content.trim().to_string()))
}

/// 分离 YAML frontmatter 和内容
///
/// 从 markdown 文本中分离 YAML frontmatter 块和主体内容。
/// Frontmatter 必须以 `---` 开始和结束。
///
/// # 格式
///
/// ```markdown
/// ---
/// key: value
/// ---
///
/// 内容从这里开始
/// ```
///
/// # 参数
///
/// - `input`: 包含可能的 frontmatter 的完整文本
///
/// # 返回值
///
/// 返回元组 `(Option<&str>, String)`:
/// - 第一个元素：如果存在有效的 frontmatter，返回 `Some(frontmatter_text)`，否则返回 `None`
/// - 第二个元素：frontmatter 之后的内容（已去除前后空白）
///
/// # 匹配规则
///
/// - Frontmatter 必须以 `---\n` 开头（注意：必须有换行符）
/// - Frontmatter 必须以 `\n---\n` 结束（前后都必须有换行符）
/// - 如果格式不匹配，整个输入作为内容返回，frontmatter 为 `None`
///
/// # 示例
///
/// ```ignore
/// let input = "---\nname: test\n---\n\n# Content";
/// let (fm, content) = split_frontmatter(input);
/// assert_eq!(fm, Some("name: test"));
/// assert_eq!(content, "\n# Content");
///
/// let input2 = "No frontmatter here";
/// let (fm, content) = split_frontmatter(input2);
/// assert_eq!(fm, None);
/// assert_eq!(content, "No frontmatter here");
/// ```
///
/// # 边界情况
///
/// - 空 frontmatter（`---\n\n---\n`）是有效的，返回空字符串
/// - 如果只有起始 `---` 没有结束符，整个文本作为内容返回
/// - Windows 换行符（`\r\n`）应在调用前转换为 `\n`
#[cfg(not(target_arch = "wasm32"))]
fn split_frontmatter(input: &str) -> (Option<&str>, String) {
    // 检查是否以 frontmatter 开始标记开头
    if !input.starts_with("---\n") {
        return (None, input.to_string());
    }

    let rest = &input[4..]; // 跳过 "---\n"

    // 查找结束标记，必须在行首
    let Some(end) = rest.find("\n---\n") else {
        // 没有找到结束标记，整个输入作为内容
        return (None, input.to_string());
    };

    // 提取 frontmatter 内容（去除首尾的 --- 标记）
    let fm = &rest[..end];

    // 提取 frontmatter 之后的内容，跳过 "\n---\n"（5个字符）
    let after = &rest[(end + 5)..];

    (Some(fm), after.to_string())
}
#[cfg(test)]
#[path = "skill_tests.rs"]
mod skill_tests;
