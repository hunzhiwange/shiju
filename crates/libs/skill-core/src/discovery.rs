//! # 技能发现模块
//!
//! 本模块负责从远程技能仓库中发现和拉取技能文件到本地缓存。
//!
//! ## 主要功能
//!
//! - **目录管理**：提供技能缓存目录的路径访问
//! - **索引解析**：解析远程技能仓库的 index.json 索引文件
//! - **文件下载**：按需下载技能文件到本地缓存，支持增量更新
//!
//! ## 架构说明
//!
//! 该模块采用惰性下载策略，仅在文件不存在时才进行下载。
//! 支持非 WASM 平台的完整功能，在 WASM 环境下返回空结果。
//!
//! ## 示例
//!
//! ```ignore
//! use skill_core::discovery;
//!
//! // 获取技能缓存目录
//! let cache_dir = discovery::dir();
//!
//! // 从远程仓库拉取技能
//! let skill_paths = discovery::pull("https://example.com/skills").await;
//! ```

use agent_paths as global;
use agent_log as log;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// 技能发现模块的日志记录器
///
/// 使用惰性初始化模式创建，带有固定的服务标识字段，
/// 用于记录技能发现过程中的关键事件（如索引获取、文件下载等）。
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("skill-discovery".to_string()));
        m
    }))
});

/// 技能索引结构
///
/// 表示远程技能仓库的索引文件（index.json）的根结构，
/// 包含仓库中所有可用技能的列表。
///
/// # 字段说明
///
/// * `skills` - 技能列表，包含每个技能的元数据和文件清单
#[derive(Debug, Deserialize)]
struct Index {
    /// 技能列表
    skills: Vec<IndexSkill>,
}

/// 索引中的技能条目
///
/// 描述单个技能的基本信息和包含的文件列表。
/// 该结构从远程 index.json 中反序列化得到。
///
/// # 字段说明
///
/// * `name` - 技能名称，用于构建本地缓存路径
/// * `description` - 技能描述（可选字段，当前未使用）
/// * `files` - 该技能包含的文件路径列表
#[derive(Debug, Deserialize)]
struct IndexSkill {
    /// 技能名称
    name: String,
    /// 技能描述（保留字段，暂未使用）
    #[allow(dead_code)]
    description: Option<String>,
    /// 技能包含的文件路径列表
    files: Vec<String>,
}

/// 获取技能缓存目录路径
///
/// 返回用于存储已下载技能的本地缓存目录。
/// 该目录位于全局缓存路径下的 `skills` 子目录中。
///
/// # 返回值
///
/// 返回技能缓存目录的完整路径（PathBuf）
///
/// # 示例
///
/// ```ignore
/// let cache_dir = dir();
/// // 例如：/home/user/.cache/vibewindow/skills
/// ```
pub fn dir() -> PathBuf {
    global::paths().cache.join("skills")
}

/// 从远程仓库拉取技能（WASM 平台存根实现）
///
/// 在 WASM 目标平台上，此函数始终返回空列表，
/// 因为 WASM 环境不支持阻塞式网络请求。
///
/// # 参数
///
/// * `_url` - 远程技能仓库的 URL（在 WASM 平台上被忽略）
///
/// # 返回值
///
/// 始终返回空的 `Vec<PathBuf>`
#[cfg(target_arch = "wasm32")]
pub async fn pull(_url: &str) -> Vec<PathBuf> {
    Vec::new()
}

/// 从远程仓库拉取技能
///
/// 从指定的远程技能仓库拉取技能索引，并下载所有技能文件到本地缓存。
/// 采用惰性下载策略：仅当文件不存在时才进行下载。
///
/// # 工作流程
///
/// 1. 规范化 URL 并构建索引文件路径
/// 2. 创建本地缓存目录
/// 3. 获取并解析远程 index.json
/// 4. 遍历每个技能，下载其包含的所有文件
/// 5. 返回成功下载且包含 SKILL.md 文件的技能目录列表
///
/// # 参数
///
/// * `url` - 远程技能仓库的基础 URL（如 `https://example.com/skills`）
///
/// # 返回值
///
/// 返回成功拉取的技能目录路径列表。
/// 如果 URL 为空或索引获取失败，返回空列表。
///
/// # 错误处理
///
/// - 网络请求失败：跳过该文件，记录警告日志
/// - 索引解析失败：返回空列表
/// - 文件下载失败：跳过该文件，记录警告日志
///
/// # 示例
///
/// ```ignore
/// let skill_paths = pull("https://skills.example.com/repo").await;
/// for path in skill_paths {
///     println!("已下载技能: {:?}", path);
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub async fn pull(url: &str) -> Vec<PathBuf> {
    // 去除 URL 前后空白字符
    let url = url.trim();
    // 空字符串 URL 直接返回空列表
    if url.is_empty() {
        return Vec::new();
    }

    // 规范化基础 URL，确保以斜杠结尾
    let base = if url.ends_with('/') { url.to_string() } else { format!("{}/", url) };
    // 构建索引文件的完整 URL
    let index_url = format!("{}index.json", base);
    // 提取主机部分（不含尾部斜杠），用于构建文件下载链接
    let host = base.trim_end_matches('/').to_string();
    // 获取本地缓存目录路径
    let cache = dir();

    // 在阻塞上下文中创建缓存目录
    let _ = tokio::task::spawn_blocking({
        let cache = cache.clone();
        move || std::fs::create_dir_all(cache)
    })
    .await;

    // 记录开始获取索引的日志
    LOGGER.info(
        "fetching index",
        Some({
            let mut m = Map::new();
            m.insert("url".to_string(), Value::String(index_url.clone()));
            m
        }),
    );

    // 在阻塞上下文中执行 HTTP 请求获取索引
    let data: Option<Index> = tokio::task::spawn_blocking({
        let index_url = index_url.clone();
        move || {
            // 构建 HTTP 客户端，设置 30 秒超时
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .ok()?;

            // 发送 GET 请求获取索引
            let resp = client.get(&index_url).send().ok()?;
            // 检查响应状态码
            if !resp.status().is_success() {
                return None;
            }
            // 解析 JSON 响应为 Index 结构
            resp.json::<Index>().ok()
        }
    })
    .await
    .ok()
    .flatten();

    // 如果索引获取或解析失败，记录警告并返回空列表
    let Some(index) = data else {
        LOGGER.warn(
            "invalid index",
            Some({
                let mut m = Map::new();
                m.insert("url".to_string(), Value::String(index_url));
                m
            }),
        );
        return Vec::new();
    };

    // 初始化结果列表，用于存储成功下载的技能目录
    let mut result: Vec<PathBuf> = Vec::new();
    let mut skills = index.skills;

    // 过滤掉无效的技能条目：名称为空或文件列表为空
    skills.retain(|s| !s.name.trim().is_empty() && !s.files.is_empty());

    // 遍历每个技能，下载其包含的所有文件
    for skill in skills {
        // 构建该技能的本地根目录
        let root = cache.join(&skill.name);

        // 遍历技能包含的每个文件
        for file in skill.files {
            let file = file.trim();
            // 跳过空文件路径
            if file.is_empty() {
                continue;
            }

            // 构建远程文件的完整 URL
            let link = format!("{}/{}/{}", host, skill.name, file);
            // 构建本地目标文件路径
            let dest = root.join(file);

            // 尝试下载文件（如果本地不存在）
            let ok = download_if_missing(&link, &dest).await;

            // 下载失败时记录警告
            if !ok {
                LOGGER.warn(
                    "failed to download skill file",
                    Some({
                        let mut m = Map::new();
                        m.insert("url".to_string(), Value::String(link));
                        m.insert(
                            "dest".to_string(),
                            Value::String(dest.to_string_lossy().to_string()),
                        );
                        m
                    }),
                );
            }
        }

        // 检查技能目录是否包含 SKILL.md 文件
        // 只有包含该文件的目录才被认为是有效的技能
        let md = root.join("SKILL.md");
        let exists = tokio::task::spawn_blocking({
            let md = md.clone();
            move || md.is_file()
        })
        .await
        .ok()
        .unwrap_or(false);

        // 将有效技能目录添加到结果列表
        if exists {
            result.push(root);
        }
    }

    result
}

/// 按需下载文件（仅当本地不存在时下载）
///
/// 检查目标文件是否存在，如果存在则跳过下载；
/// 如果不存在，则从指定 URL 下载并保存到目标路径。
///
/// # 参数
///
/// * `url` - 远程文件的 URL
/// * `dest` - 本地目标文件路径
///
/// # 返回值
///
/// - `true` - 文件已存在或下载成功
/// - `false` - 下载失败（网络错误、写入失败等）
///
/// # 实现细节
///
/// - 使用 30 秒超时的 HTTP 客户端
/// - 自动创建目标目录的父目录结构
/// - 在阻塞上下文中执行文件系统操作
#[cfg(not(target_arch = "wasm32"))]
async fn download_if_missing(url: &str, dest: &Path) -> bool {
    // 在阻塞上下文中检查文件是否已存在
    let exists = tokio::task::spawn_blocking({
        let dest = dest.to_path_buf();
        move || dest.is_file()
    })
    .await
    .ok()
    .unwrap_or(false);

    // 文件已存在，跳过下载
    if exists {
        return true;
    }

    // 准备下载所需的参数
    let url = url.to_string();
    let dest = dest.to_path_buf();

    // 在阻塞上下文中执行下载和文件写入
    tokio::task::spawn_blocking(move || {
        // 创建目标目录的父目录结构
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // 构建 HTTP 客户端，设置 30 秒超时
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;

        // 发送 GET 请求
        let resp = client.get(&url).send().ok()?;

        // 检查响应状态码
        if !resp.status().is_success() {
            return Some(false);
        }

        // 获取响应文本内容
        let text = resp.text().ok()?;

        // 将内容写入本地文件
        std::fs::write(dest, text).ok()?;

        Some(true)
    })
    .await
    .ok()
    .flatten()
    .unwrap_or(false)
}
#[cfg(test)]
#[path = "discovery_tests.rs"]
mod discovery_tests;
