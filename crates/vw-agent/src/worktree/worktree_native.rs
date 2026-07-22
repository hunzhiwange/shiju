use super::naming::random_name;
use super::{Error, Info};
use crate::app::agent::project;
use crate::app::agent::project::instance;
use crate::app::agent::shell::{git_std_command, std_system_command};
use crate::app::agent::util::log;
use directories::UserDirs;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// worktree 模块专用日志记录器
///
/// 使用 "worktree" 作为服务标识，用于追踪 worktree 生命周期事件
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("worktree".to_string()));
        m
    }))
});

/// 获取 worktree 根目录路径。
///
/// 普通 worktree 与任务池共用 home 配置目录下的 `task-worktrees` 根目录；
/// 任务池只接管 `slot-*` 槽位，避免误重置手动或 fork worktree。
pub(super) fn worktree_root(project: &project::Info) -> Result<PathBuf, Error> {
    if project.worktree.trim().is_empty() {
        return Err(Error::MissingProject("missing project worktree root".to_string()));
    }
    let base = UserDirs::new()
        .map_or_else(|| PathBuf::from(&project.worktree), |dirs| dirs.home_dir().to_path_buf());
    Ok(vw_config_types::paths::task_worktrees_dir(base))
}

/// 验证并获取当前 Git 项目信息
pub(super) fn ensure_git_project() -> Result<project::Info, Error> {
    let Some(project) = instance::project() else {
        return Err(Error::MissingProject("missing project context".to_string()));
    };
    if project.vcs != Some(project::Vcs::Git) {
        return Err(Error::NotGit("Worktrees are only supported for git projects".to_string()));
    }
    Ok(project)
}

/// Worktree 列表条目结构
///
/// 用于解析 `git worktree list --porcelain` 的输出
#[derive(Debug, Clone)]
pub(super) struct WorktreeEntry {
    /// Worktree 的文件系统路径
    pub(super) path: String,

    /// 关联的分支引用（如 `refs/heads/vibewindow/brave-eagle`）
    pub(super) branch: Option<String>,
}

/// 解析 git worktree list 的 porcelain 格式输出
pub(super) fn parse_worktree_list(input: &str) -> Vec<WorktreeEntry> {
    let mut out: Vec<WorktreeEntry> = Vec::new();
    for line in input.lines().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Some(rest) = line.strip_prefix("worktree ") {
            out.push(WorktreeEntry { path: rest.trim().to_string(), branch: None });
            continue;
        }
        if let Some(rest) = line.strip_prefix("branch ") {
            if let Some(last) = out.last_mut() {
                last.branch = Some(rest.trim().to_string());
            }
        }
    }
    out
}

/// 在 worktree 条目列表中查找指定路径的条目
pub(super) async fn find_entry(entries: &[WorktreeEntry], target: &Path) -> Option<WorktreeEntry> {
    let target = canonical(target).await;
    for entry in entries {
        let key = canonical(&entry.path).await;
        if key == target {
            return Some(entry.clone());
        }
    }
    None
}

/// 检查路径是否存在
pub(super) async fn path_exists(path: &Path) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

/// 生成候选 worktree 信息
pub(super) async fn candidate(root: &Path, base: Option<&str>) -> Result<Info, Error> {
    let primary = instance::worktree();
    for attempt in 0..26 {
        let name = if let Some(base) = base {
            if attempt == 0 { base.to_string() } else { format!("{}-{}", base, random_name()) }
        } else {
            random_name()
        };
        let branch = format!("vibewindow/{}", name);
        let directory = root.join(&name);

        if path_exists(&directory).await {
            continue;
        }

        let ref_name = format!("refs/heads/{}", branch);
        let check = run_git(&["show-ref", "--verify", "--quiet", &ref_name], &primary).await?;
        if check.success {
            continue;
        }

        return Ok(Info { name, branch, directory: directory.to_string_lossy().to_string() });
    }
    Err(Error::Invalid("Failed to generate a unique worktree name".to_string()))
}

/// Git 命令执行结果
#[derive(Debug, Clone)]
pub(super) struct CmdResult {
    /// 命令是否成功执行（退出码为 0）
    pub(super) success: bool,

    /// 标准输出内容
    pub(super) stdout: String,

    /// 标准错误内容
    pub(super) stderr: String,
}

impl CmdResult {
    /// 生成错误文本
    pub(super) fn error_text(&self, fallback: &str) -> String {
        let message = [self.stderr.trim(), self.stdout.trim()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if message.is_empty() { fallback.to_string() } else { message }
    }
}

/// 执行 git 命令
pub(super) async fn run_git(args: &[&str], cwd: &str) -> Result<CmdResult, Error> {
    let args = args.iter().map(|value| value.to_string()).collect::<Vec<_>>();
    let cwd = cwd.to_string();
    let output = tokio::task::spawn_blocking(move || {
        let mut cmd = git_std_command();
        cmd.current_dir(&cwd);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.output()
    })
    .await??;

    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n").replace('\r', "\n");
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n").replace('\r', "\n");
    Ok(CmdResult { success: output.status.success(), stdout, stderr })
}

/// 将路径转换为规范路径
pub(super) async fn canonical(input: impl AsRef<Path>) -> PathBuf {
    let path = input.as_ref().to_path_buf();
    let fallback = path.clone();
    tokio::task::spawn_blocking(move || {
        let absolute = if path.is_absolute() {
            path
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
        };
        let real = std::fs::canonicalize(&absolute).unwrap_or(absolute);
        if cfg!(windows) {
            PathBuf::from(real.to_string_lossy().to_ascii_lowercase())
        } else {
            real
        }
    })
    .await
    .unwrap_or(fallback)
}

/// 运行 worktree 的启动脚本
pub(super) async fn run_start_scripts(
    worktree_dir: &str,
    project_id: &str,
    extra: &str,
) -> Result<(), Error> {
    let project_info = match project::project_scope_for_default_tenant(project_id) {
        Ok(scope) => project::get(&scope).await.ok().flatten(),
        Err(_) => None,
    };
    let project_start = project_info
        .and_then(|project| project.commands.and_then(|commands| commands.start))
        .map(|command| command.trim().to_string())
        .unwrap_or_default();

    if !project_start.trim().is_empty() {
        let ok = run_shell(worktree_dir, &project_start).await?;
        if !ok {
            LOGGER.error(
                "worktree start command failed",
                Some({
                    let mut m = Map::new();
                    m.insert("kind".to_string(), Value::String("project".to_string()));
                    m.insert("directory".to_string(), Value::String(worktree_dir.to_string()));
                    m.insert(
                        "message".to_string(),
                        Value::String("project start failed".to_string()),
                    );
                    m
                }),
            );
            return Ok(());
        }
    }

    if !extra.trim().is_empty() {
        let _ = run_shell(worktree_dir, extra).await?;
    }

    Ok(())
}

/// 在指定目录中执行 shell 命令
async fn run_shell(cwd: &str, cmd: &str) -> Result<bool, Error> {
    let cwd = cwd.to_string();
    let cmd = cmd.to_string();
    let output = tokio::task::spawn_blocking(move || {
        let mut command = if cfg!(windows) {
            let mut command = std_system_command("cmd");
            command.arg("/c").arg(cmd);
            command
        } else {
            let mut command = std_system_command("bash");
            command.arg("-lc").arg(cmd);
            command
        };
        command.current_dir(cwd);
        command.output()
    })
    .await??;

    Ok(output.status.success())
}

/// 默认重置目标信息
pub(super) struct DefaultTarget {
    /// 重置目标引用（分支名、标签或远程分支）
    pub(super) target: String,

    /// 远程名称（如果是远程分支）
    pub(super) remote: Option<String>,

    /// 远程分支名称（如果是远程分支）
    pub(super) remote_branch: Option<String>,
}

/// 解析重置目标
pub(super) async fn resolve_reset_target(
    primary: &Path,
    base_ref: Option<&str>,
) -> Result<DefaultTarget, Error> {
    let Some(base_ref) = base_ref.map(str::trim).filter(|value| !value.is_empty()) else {
        return default_branch_target(primary).await;
    };

    let primary_s = primary.to_string_lossy().to_string();
    let verify = run_git(&["rev-parse", "--verify", base_ref], &primary_s).await?;
    if !verify.success {
        return Err(Error::Invalid(verify.error_text(&format!("Invalid base ref: {}", base_ref))));
    }

    let (remote, remote_branch) = if let Some((remote, branch)) = base_ref.split_once('/') {
        if !remote.is_empty() && !branch.is_empty() {
            (Some(remote.to_string()), Some(branch.to_string()))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    Ok(DefaultTarget { target: base_ref.to_string(), remote, remote_branch })
}

/// 确定项目的默认分支目标
async fn default_branch_target(primary: &Path) -> Result<DefaultTarget, Error> {
    let primary_s = primary.to_string_lossy().to_string();

    let remote_list = run_git(&["remote"], &primary_s).await?;
    if !remote_list.success {
        return Err(Error::Invalid(remote_list.error_text("Failed to list git remotes")));
    }

    let remotes = remote_list
        .stdout
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    let remote = if remotes.iter().any(|remote| remote == "origin") {
        Some("origin".to_string())
    } else if remotes.len() == 1 {
        Some(remotes[0].clone())
    } else if remotes.iter().any(|remote| remote == "upstream") {
        Some("upstream".to_string())
    } else {
        None
    };

    let remote_branch = if let Some(remote) = remote.as_deref() {
        let head = run_git(&["symbolic-ref", &format!("refs/remotes/{}/HEAD", remote)], &primary_s)
            .await?;
        if head.success {
            let remote_ref = head.stdout.trim();
            let remote_target =
                remote_ref.strip_prefix("refs/remotes/").unwrap_or(remote_ref).trim();
            remote_target.strip_prefix(&format!("{}/", remote)).map(|branch| branch.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let main = run_git(&["show-ref", "--verify", "--quiet", "refs/heads/main"], &primary_s).await?;
    let master =
        run_git(&["show-ref", "--verify", "--quiet", "refs/heads/master"], &primary_s).await?;
    let local_branch = if main.success {
        Some("main".to_string())
    } else if master.success {
        Some("master".to_string())
    } else {
        None
    };

    if let (Some(remote), Some(remote_branch)) = (remote.clone(), remote_branch.clone()) {
        return Ok(DefaultTarget {
            target: format!("{}/{}", remote, remote_branch),
            remote: Some(remote),
            remote_branch: Some(remote_branch),
        });
    }

    let Some(local_branch) = local_branch else {
        return Err(Error::Invalid("Default branch not found".to_string()));
    };
    Ok(DefaultTarget { target: local_branch, remote: None, remote_branch: None })
}
