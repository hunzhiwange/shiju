//! # 工作空间配置管理模块
//!
//! 本模块负责管理工作空间的配置目录、持久化活动工作空间状态以及解析运行时配置路径。
//!
//! ## 主要功能
//!
//! - **配置目录解析**：根据环境变量、持久化标记或默认值确定配置目录位置
//! - **工作空间状态持久化**：将当前活动的工作空间配置写入标记文件，以便后续恢复
//! - **多平台支持**：为原生和 WebAssembly (WASM) 目标平台提供不同的实现
//!
//! ## 配置目录优先级
//!
//! 配置目录按以下优先级顺序解析：
//! 1. 环境变量 `VIBEWINDOW_CONFIG_DIR`
//! 2. 环境变量 `VIBEWINDOW_WORKSPACE`
//! 3. 持久化的活动工作空间标记文件 (`active_workspace.toml`)
//! 4. 默认配置目录 (`~/.shiju-agent`/`~/.shiju-agent-dev` 或对应 wasm 根目录)
//!
//! ## 文件结构
//!
//! - `active_workspace.toml` - 活动工作空间状态标记文件，记录当前使用的配置目录路径

use anyhow::{Context, Result};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use tokio::fs;

/// 活动工作空间状态标记文件名
///
/// 该文件用于持久化当前活动的工作空间配置目录路径，
/// 以便在下次启动时能够恢复到上次使用的工作空间。
const ACTIVE_WORKSPACE_STATE_FILE: &str = "active_workspace.toml";

/// 活动工作空间状态结构体
///
/// 用于序列化和反序列化活动工作空间的配置信息，
/// 存储在 `active_workspace.toml` 文件中。
#[derive(Debug, Serialize, Deserialize)]
struct ActiveWorkspaceState {
    /// 配置目录路径（可以是绝对路径或相对路径）
    config_dir: String,
}

/// 获取默认的配置目录和工作空间目录
///
/// 返回默认的配置目录路径和对应的工作空间子目录路径。
/// 工作空间目录总是配置目录下的 `workspace` 子目录。
///
/// # 返回值
///
/// 返回元组 `(config_dir, workspace_dir)`：
/// - `config_dir` - 配置目录的完整路径
/// - `workspace_dir` - 工作空间目录的完整路径（`config_dir/workspace`）
///
/// # 错误
///
/// 如果无法确定用户的家目录，将返回错误。
///
/// # 示例
///
/// ```ignore
/// let (config_dir, workspace_dir) = default_config_and_workspace_dirs()?;
/// println!("配置目录: {:?}", config_dir);
/// println!("工作空间目录: {:?}", workspace_dir);
/// ```
pub fn default_config_and_workspace_dirs() -> Result<(PathBuf, PathBuf)> {
    let config_dir = default_config_dir()?;
    Ok((config_dir.clone(), config_dir.join("workspace")))
}

/// 获取默认的配置目录路径（原生平台实现）
///
/// 在原生（非 WASM）平台上，默认配置目录位于用户家目录下的 `.shiju-agent` 文件夹。
///
/// # 平台差异
///
/// - **Linux/macOS**: release `~/.shiju-agent`，dev `~/.shiju-agent-dev`
/// - **Windows**: release `C:\Users\<username>\.shiju-agent`，dev `C:\Users\<username>\.shiju-agent-dev`
///
/// # 错误
///
/// 如果无法找到用户的家目录，将返回错误。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn default_config_dir() -> Result<PathBuf> {
    let home = UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    Ok(vw_config_types::paths::home_config_dir(home))
}

/// 获取默认的配置目录路径（WASM 平台实现）
///
/// 在 WebAssembly 目标平台上，使用固定的根目录路径。
/// 这是由于 WASM 环境中文件系统访问的特殊限制。
#[cfg(target_arch = "wasm32")]
pub(crate) fn default_config_dir() -> Result<PathBuf> {
    Ok(vw_config_types::paths::root_config_dir())
}

/// 构建活动工作空间状态标记文件的完整路径
///
/// # 参数
///
/// - `marker_root` - 标记文件所在的根目录
///
/// # 返回值
///
/// 返回标记文件的完整路径：`marker_root/active_workspace.toml`
fn active_workspace_state_path(marker_root: &Path) -> PathBuf {
    marker_root.join(ACTIVE_WORKSPACE_STATE_FILE)
}

/// 检查给定路径是否位于操作系统的临时目录下
///
/// 用于防止将临时目录持久化为活动工作空间，
/// 避免测试或一次性调用污染守护进程的配置解析。
///
/// # 参数
///
/// - `path` - 要检查的路径
///
/// # 返回值
///
/// 如果路径位于临时目录下，返回 `true`；否则返回 `false`
///
/// # 实现细节
///
/// - 使用路径规范化（canonicalize）来正确处理符号链接
/// - 例如在 macOS 上，`/var` 可能是 `/private/var` 的符号链接
pub(super) fn is_temp_directory(path: &Path) -> bool {
    let temp = std::env::temp_dir();
    // 使用规范化路径来正确处理符号链接（例如 macOS 的 /var → /private/var）
    let canon_temp = temp.canonicalize().unwrap_or_else(|_| temp.clone());
    let canon_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canon_path.starts_with(&canon_temp)
}

/// 从持久化标记文件中加载工作空间目录（原生平台实现）
///
/// 读取并解析 `active_workspace.toml` 文件，从中恢复之前保存的
/// 配置目录和工作空间目录路径。
///
/// # 参数
///
/// - `default_config_dir` - 默认配置目录，用于解析相对路径
///
/// # 返回值
///
/// - `Ok(Some((config_dir, workspace_dir)))` - 成功加载持久化路径
/// - `Ok(None)` - 标记文件不存在、读取失败或解析失败
/// - `Err(_)` - 其他错误
///
/// # 错误处理
///
/// - 文件不存在时返回 `Ok(None)`（正常情况）
/// - 读取或解析失败时记录警告并返回 `Ok(None)`（容错处理）
/// - 空路径会被忽略并返回 `Ok(None)`
#[cfg(not(target_arch = "wasm32"))]
async fn load_persisted_workspace_dirs(
    default_config_dir: &Path,
) -> Result<Option<(PathBuf, PathBuf)>> {
    let state_path = active_workspace_state_path(default_config_dir);

    // 标记文件不存在，直接返回 None
    if !state_path.exists() {
        return Ok(None);
    }

    // 读取标记文件内容
    let contents = match fs::read_to_string(&state_path).await {
        Ok(contents) => contents,
        Err(error) => {
            tracing::warn!(
                "Failed to read active workspace marker {}: {error}",
                state_path.display()
            );
            return Ok(None);
        }
    };

    // 解析 TOML 格式的状态文件
    let state: ActiveWorkspaceState = match toml::from_str(&contents) {
        Ok(state) => state,
        Err(error) => {
            tracing::warn!(
                "Failed to parse active workspace marker {}: {error}",
                state_path.display()
            );
            return Ok(None);
        }
    };

    // 验证配置目录路径不为空
    let raw_config_dir = state.config_dir.trim();
    if raw_config_dir.is_empty() {
        tracing::warn!(
            "Ignoring active workspace marker {} because config_dir is empty",
            state_path.display()
        );
        return Ok(None);
    }

    // 解析路径：支持绝对路径和相对路径
    let parsed_dir = PathBuf::from(raw_config_dir);
    let config_dir =
        if parsed_dir.is_absolute() { parsed_dir } else { default_config_dir.join(parsed_dir) };

    Ok(Some((config_dir.clone(), config_dir.join("workspace"))))
}

/// 从持久化标记文件中加载工作空间目录（WASM 平台实现）
///
/// 在 WASM 环境中，持久化功能不可用，直接返回 `None`。
#[cfg(target_arch = "wasm32")]
async fn load_persisted_workspace_dirs(
    _default_config_dir: &Path,
) -> Result<Option<(PathBuf, PathBuf)>> {
    Ok(None)
}

/// 删除活动工作空间标记文件（原生平台实现）
///
/// 删除 `active_workspace.toml` 文件，并同步目录以确保更改持久化到磁盘。
///
/// # 参数
///
/// - `marker_root` - 标记文件所在的根目录
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误
///
/// # 错误处理
///
/// - 文件不存在时直接返回 `Ok(())`（幂等操作）
/// - 删除失败时返回错误
#[cfg(not(target_arch = "wasm32"))]
async fn remove_active_workspace_marker(marker_root: &Path) -> Result<()> {
    let state_path = active_workspace_state_path(marker_root);

    // 文件不存在，无需操作
    if !state_path.exists() {
        return Ok(());
    }

    // 删除标记文件
    fs::remove_file(&state_path).await.with_context(|| {
        format!("Failed to clear active workspace marker: {}", state_path.display())
    })?;

    // 同步目录以确保删除操作持久化
    if marker_root.exists() {
        sync_directory(marker_root).await?;
    }
    Ok(())
}

/// 删除活动工作空间标记文件（WASM 平台实现）
///
/// 在 WASM 环境中，该操作为空操作。
#[cfg(target_arch = "wasm32")]
async fn remove_active_workspace_marker(_marker_root: &Path) -> Result<()> {
    Ok(())
}

/// 写入活动工作空间标记文件（WASM 平台实现）
///
/// 在 WASM 环境中，该操作为空操作。
#[cfg(target_arch = "wasm32")]
async fn write_active_workspace_marker(_marker_root: &Path, _config_dir: &Path) -> Result<()> {
    Ok(())
}

/// 写入活动工作空间标记文件（原生平台实现）
///
/// 使用原子写入操作创建或更新 `active_workspace.toml` 文件，
/// 记录当前活动的配置目录路径。
///
/// # 参数
///
/// - `marker_root` - 标记文件所在的根目录
/// - `config_dir` - 要持久化的配置目录路径
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误
///
/// # 原子写入流程
///
/// 1. 在标记根目录创建临时文件（使用 UUID 确保唯一性）
/// 2. 将序列化的状态写入临时文件
/// 3. 原子性地重命名临时文件为目标文件
/// 4. 同步目录以确保更改持久化
///
/// # 错误处理
///
/// - 创建目录失败时返回错误
/// - 序列化失败时返回错误
/// - 写入临时文件失败时返回错误
/// - 原子重命名失败时删除临时文件并返回错误
#[cfg(not(target_arch = "wasm32"))]
async fn write_active_workspace_marker(marker_root: &Path, config_dir: &Path) -> Result<()> {
    // 确保标记根目录存在
    fs::create_dir_all(marker_root).await.with_context(|| {
        format!("Failed to create active workspace marker root: {}", marker_root.display())
    })?;

    // 构建状态并序列化为 TOML
    let state = ActiveWorkspaceState { config_dir: config_dir.to_string_lossy().into_owned() };
    let serialized =
        toml::to_string_pretty(&state).context("Failed to serialize active workspace marker")?;

    // 使用临时文件进行原子写入，UUID 确保文件名唯一
    let temp_path =
        marker_root.join(format!(".{ACTIVE_WORKSPACE_STATE_FILE}.tmp-{}", uuid::Uuid::new_v4()));

    fs::write(&temp_path, serialized).await.with_context(|| {
        format!("Failed to write temporary active workspace marker: {}", temp_path.display())
    })?;

    // 原子性地重命名临时文件为目标文件
    let state_path = active_workspace_state_path(marker_root);
    if let Err(error) = fs::rename(&temp_path, &state_path).await {
        // 重命名失败时清理临时文件
        let _ = fs::remove_file(&temp_path).await;
        anyhow::bail!(
            "Failed to atomically persist active workspace marker {}: {error}",
            state_path.display()
        );
    }

    // 同步目录以确保写入持久化
    sync_directory(marker_root).await?;
    Ok(())
}

/// 持久化当前活动的配置目录
///
/// 将指定的配置目录路径写入活动工作空间标记文件。
/// 如果配置目录是默认目录，则清除标记文件。
/// 临时目录永远不会被持久化。
///
/// # 参数
///
/// - `config_dir` - 要持久化的配置目录路径
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误
///
/// # 安全措施
///
/// - **临时目录保护**：拒绝持久化临时目录路径，防止测试或一次性调用污染配置
/// - **默认目录处理**：使用默认目录时清除标记，保持环境清洁
/// - **双重标记**：在选定的配置根目录和默认 HOME 配置根目录都写入标记，
///   以提高在不同环境中的可发现性
///
/// # 错误处理
///
/// - 主标记写入失败时返回错误
/// - 镜像标记写入失败时仅记录警告，不中断流程
pub(crate) async fn persist_active_workspace_config_dir(config_dir: &Path) -> Result<()> {
    let default_config_dir = default_config_dir()?;
    persist_active_workspace_config_dir_with_default(config_dir, &default_config_dir).await
}

pub(super) async fn persist_active_workspace_config_dir_with_default(
    config_dir: &Path,
    default_config_dir: &Path,
) -> Result<()> {
    // 安全守卫：拒绝将临时目录持久化为活动工作空间
    // 防止测试运行或一次性调用劫持守护进程的配置解析
    #[cfg(not(test))]
    if is_temp_directory(config_dir) {
        tracing::warn!(
            path = %config_dir.display(),
            "Refusing to persist temp directory as active workspace marker"
        );
        return Ok(());
    }

    // 如果使用默认配置目录，则清除标记文件
    if config_dir == default_config_dir {
        remove_active_workspace_marker(&default_config_dir).await?;
        return Ok(());
    }

    // 主标记写入选定的配置根目录，保持自定义主目录布局的自包含性和可写性
    write_active_workspace_marker(config_dir, config_dir).await?;

    // 尝试镜像到默认 HOME 范围的根目录，作为最佳努力的指针
    if let Err(error) = write_active_workspace_marker(&default_config_dir, config_dir).await {
        tracing::warn!(
            selected_config_dir = %config_dir.display(),
            default_config_dir = %default_config_dir.display(),
            "Failed to mirror active workspace marker to default HOME config root; continuing with selected-root marker only: {error}"
        );
    }

    Ok(())
}

/// 根据工作空间目录解析配置目录
///
/// 根据给定的工作空间目录路径，智能推断配置目录的位置。
/// 支持多种布局模式，包括现代布局和旧版布局。
///
/// # 参数
///
/// - `workspace_dir` - 工作空间目录路径
///
/// # 返回值
///
/// 返回元组 `(config_dir, resolved_workspace_dir)`：
/// - `config_dir` - 解析后的配置目录路径
/// - `resolved_workspace_dir` - 解析后的工作空间目录路径
///
/// # 解析策略
///
/// 按以下顺序尝试解析：
///
/// 1. **工作空间目录本身就是配置目录**：
///    - 检查是否存在 `config.json`
///
/// 2. **旧版布局**：
///    - 检查父目录下的 `.vibewindow` 子目录
///    - 适用于工作空间目录名为 `workspace` 的情况
///
/// 3. **默认回退**：
///    - 将工作空间目录作为配置目录，工作空间子目录为 `workspace`
pub(crate) fn resolve_config_dir_for_workspace(workspace_dir: &Path) -> (PathBuf, PathBuf) {
    let workspace_config_dir = workspace_dir.to_path_buf();

    // 情况1：工作空间目录直接包含配置文件（现代布局）
    if workspace_config_dir.join("config.json").exists() {
        return (workspace_config_dir.clone(), workspace_config_dir.join("workspace"));
    }
    // 情况2：检查父目录下的 .vibewindow（旧版布局）
    let legacy_config_dir = workspace_dir.parent().map(|parent| parent.join(".vibewindow"));
    if let Some(legacy_dir) = legacy_config_dir {
        // 检查旧版配置目录中是否存在配置文件
        if legacy_dir.join("config.json").exists() {
            return (legacy_dir, workspace_config_dir);
        }
        // 如果工作空间目录名为 "workspace"，假定使用旧版布局
        if workspace_dir.file_name().is_some_and(|name| name == std::ffi::OsStr::new("workspace")) {
            return (legacy_dir, workspace_config_dir);
        }
    }

    // 情况3：默认回退，将工作空间目录作为配置目录
    (workspace_config_dir.clone(), workspace_config_dir.join("workspace"))
}

/// 配置解析来源枚举
///
/// 标识配置目录是通过哪种方式解析得到的，
/// 用于日志记录和调试目的。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConfigResolutionSource {
    /// 通过环境变量 `VIBEWINDOW_CONFIG_DIR` 指定
    EnvConfigDir,
    /// 通过环境变量 `VIBEWINDOW_WORKSPACE` 指定
    EnvWorkspace,
    /// 通过持久化的活动工作空间标记文件 (`active_workspace.toml`) 恢复
    ActiveWorkspaceMarker,
    /// 使用默认配置目录
    DefaultConfigDir,
}

impl ConfigResolutionSource {
    /// 将配置来源转换为字符串标识
    ///
    /// # 返回值
    ///
    /// 返回人类可读的来源标识：
    /// - `EnvConfigDir` → `"VIBEWINDOW_CONFIG_DIR"`
    /// - `EnvWorkspace` → `"VIBEWINDOW_WORKSPACE"`
    /// - `ActiveWorkspaceMarker` → `"active_workspace.toml"`
    /// - `DefaultConfigDir` → `"default"`
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::EnvConfigDir => "VIBEWINDOW_CONFIG_DIR",
            Self::EnvWorkspace => "VIBEWINDOW_WORKSPACE",
            Self::ActiveWorkspaceMarker => "active_workspace.toml",
            Self::DefaultConfigDir => "default",
        }
    }
}

/// 解析运行时配置目录
///
/// 按优先级顺序确定运行时使用的配置目录和工作空间目录。
/// 这是配置解析的核心函数，被启动流程调用。
///
/// # 参数
///
/// - `default_vibewindow_dir` - 默认的 VibeWindow 配置目录
/// - `default_workspace_dir` - 默认的工作空间目录
///
/// # 返回值
///
/// 返回元组 `(vibewindow_dir, workspace_dir, source)`：
/// - `vibewindow_dir` - 解析后的 VibeWindow 配置目录
/// - `workspace_dir` - 解析后的工作空间目录
/// - `source` - 配置来源标识
///
/// # 解析优先级
///
/// 1. **环境变量 `VIBEWINDOW_CONFIG_DIR`**（最高优先级）
///    - 直接指定配置目录路径
///    - 工作空间目录为配置目录下的 `workspace` 子目录
///
/// 2. **环境变量 `VIBEWINDOW_WORKSPACE`**
///    - 指定工作空间目录，通过 [`resolve_config_dir_for_workspace`] 推断配置目录
///
/// 3. **持久化的活动工作空间标记**
///    - 从 `active_workspace.toml` 文件加载之前保存的路径
///
/// 4. **默认配置目录**（最低优先级）
///    - 使用传入的默认值
///
/// # 示例
///
/// ```ignore
/// let (config_dir, workspace_dir, source) = resolve_runtime_config_dirs(
///     &default_config,
///     &default_workspace
/// ).await?;
/// println!("配置来源: {}", source.as_str());
/// ```
pub(crate) async fn resolve_runtime_config_dirs(
    default_vibewindow_dir: &Path,
    default_workspace_dir: &Path,
) -> Result<(PathBuf, PathBuf, ConfigResolutionSource)> {
    let env_config_dir = std::env::var("VIBEWINDOW_CONFIG_DIR").ok();
    let env_workspace = std::env::var("VIBEWINDOW_WORKSPACE").ok();
    resolve_runtime_config_dirs_from_env(
        default_vibewindow_dir,
        default_workspace_dir,
        env_config_dir.as_deref(),
        env_workspace.as_deref(),
    )
    .await
}

pub(super) async fn resolve_runtime_config_dirs_from_env(
    default_vibewindow_dir: &Path,
    default_workspace_dir: &Path,
    env_config_dir: Option<&str>,
    env_workspace: Option<&str>,
) -> Result<(PathBuf, PathBuf, ConfigResolutionSource)> {
    // 优先级1：检查 VIBEWINDOW_CONFIG_DIR 环境变量
    if let Some(custom_config_dir) = env_config_dir {
        let custom_config_dir = custom_config_dir.trim();
        if !custom_config_dir.is_empty() {
            let vibewindow_dir = PathBuf::from(custom_config_dir);
            return Ok((
                vibewindow_dir.clone(),
                vibewindow_dir.join("workspace"),
                ConfigResolutionSource::EnvConfigDir,
            ));
        }
    }

    // 优先级2：检查 VIBEWINDOW_WORKSPACE 环境变量
    if let Some(custom_workspace) = env_workspace {
        if !custom_workspace.is_empty() {
            let (vibewindow_dir, workspace_dir) =
                resolve_config_dir_for_workspace(&PathBuf::from(custom_workspace));
            return Ok((vibewindow_dir, workspace_dir, ConfigResolutionSource::EnvWorkspace));
        }
    }

    // 优先级3：检查持久化的活动工作空间标记
    if let Some((vibewindow_dir, workspace_dir)) =
        load_persisted_workspace_dirs(default_vibewindow_dir).await?
    {
        return Ok((vibewindow_dir, workspace_dir, ConfigResolutionSource::ActiveWorkspaceMarker));
    }

    // 优先级4：使用默认配置目录
    Ok((
        default_vibewindow_dir.to_path_buf(),
        default_workspace_dir.to_path_buf(),
        ConfigResolutionSource::DefaultConfigDir,
    ))
}

/// 持久化活动工作空间标记
///
/// 在指定的标记根目录写入配置目录路径。
/// 这是 [`write_active_workspace_marker`] 的公开封装。
///
/// # 参数
///
/// - `default_config_dir` - 标记文件所在的根目录
/// - `config_dir` - 要持久化的配置目录路径
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误
pub(crate) async fn persist_active_workspace_marker(
    default_config_dir: &Path,
    config_dir: &Path,
) -> Result<()> {
    write_active_workspace_marker(default_config_dir, config_dir).await
}

/// 清除活动工作空间标记
///
/// 删除指定根目录下的活动工作空间标记文件。
/// 这是 [`remove_active_workspace_marker`] 的公开封装。
///
/// # 参数
///
/// - `default_config_dir` - 标记文件所在的根目录
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误
pub(crate) async fn clear_active_workspace_marker(default_config_dir: &Path) -> Result<()> {
    remove_active_workspace_marker(default_config_dir).await
}

/// 同步目录到磁盘（Unix 平台实现）
///
/// 在 Unix 平台上，通过打开目录并调用 `sync_all` 来确保
/// 目录的元数据（包括文件创建、删除等操作）持久化到磁盘。
///
/// # 参数
///
/// - `path` - 要同步的目录路径
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误
///
/// # 用途
///
/// 主要用于确保原子写入操作（如重命名文件）的结果持久化，
/// 防止系统崩溃后丢失文件系统更改。
#[cfg(unix)]
pub(crate) async fn sync_directory(path: &Path) -> Result<()> {
    let file = tokio::fs::File::open(path).await?;
    file.sync_all().await?;
    Ok(())
}

/// 同步目录到磁盘（非 Unix 平台实现）
///
/// 在非 Unix 平台（如 Windows）上，该操作为空操作。
/// 这是因为不同平台的文件系统同步机制不同，
/// 且在大多数情况下不需要显式同步目录。
#[cfg(not(unix))]
pub(crate) async fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}
