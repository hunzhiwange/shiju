//! 项目技能元数据与文件扫描；目录由宿主按当前项目配置提供。
#[cfg(not(target_arch = "wasm32"))]
use agent_log as log;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Map, Value};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::LazyLock;

/// 技能模块专用日志记录器
///
/// 使用 Lazy 初始化，带有一个 "skill" 服务标识符，
/// 用于记录技能加载、解析和错误信息。
#[cfg(not(target_arch = "wasm32"))]
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
/// use skill_core::project::Info;
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
/// use skill_core::project::State;
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

/// 扫描宿主授权的外部目录，然后扫描配置目录；同名覆盖顺序保持不变。
#[cfg(not(target_arch = "wasm32"))]
pub async fn load_state<F, Fut>(external_dirs: &[PathBuf], config_dirs: F) -> State
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Vec<String>>,
{
    let mut skills: HashMap<String, Info> = HashMap::new();
    let mut dirs: HashSet<String> = HashSet::new();
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

    for source in external_dirs {
        for match_path in glob_files(source, "**/SKILL.md") {
            add_skill(match_path, &mut skills, &mut dirs).await;
        }
        log_external_scan(source, "configured");
    }

    // 第二步：加载配置目录中的技能
    // 支持 skill/**/SKILL.md 和 skills/**/SKILL.md 两种模式
    for dir in config_dirs().await {
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
#[cfg(all(test, not(target_arch = "wasm32")))]
fn resolve_path(p: &str, home: &Path, project_dir: &str) -> PathBuf {
    let p = p.trim();
    if p.is_empty() {
        return PathBuf::new();
    }

    // 展开波浪号为用户主目录
    let expanded = if let Some(rest) = p.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(p)
    };

    // 如果是绝对路径直接返回，否则相对于项目目录
    if expanded.is_absolute() {
        expanded
    } else {
        let base = project_dir;
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
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "project_tests.rs"]
mod skill_tests;
