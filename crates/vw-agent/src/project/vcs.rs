//! 项目 VCS 状态跟踪。
//!
//! 当前实现关注 Git 分支名：初始化时读取一次分支，并订阅文件 watcher 事件，在
//! `.git/HEAD` 变化时重新读取并发布分支更新事件。
//!
//! 分支读取走 git2，不启动系统 Git。仍依赖 Git CLI 的范围外能力：
//! `/v1/git/command`、普通 worktree fetch/submodule、skill clone/pull、snapshot 等。

use crate::app::agent::bus;
use crate::app::agent::file::watcher as file_watcher;
use crate::app::agent::util::log;
use crate::git_repository;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("vcs".to_string()));
    log::create(Some(tags))
});

pub mod event {
    //! VCS 子系统发布的事件定义。

    use crate::app::agent::bus;

    /// Git 分支发生变化时发布。
    pub const BRANCH_UPDATED: bus::Definition = bus::Definition { r#type: "vcs.branch.updated" };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 当前 VCS 摘要信息。
pub struct Info {
    /// 当前 Git 分支名；无法读取或处于非普通分支状态时为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

fn current_branch(worktree: &str) -> Option<String> {
    git_repository::current_branch(Path::new(worktree))
}

/// 初始化工作树的 VCS 状态跟踪。
///
/// 函数会启动文件 watcher，读取当前分支，并在 HEAD 变化时发布分支更新事件。
///
/// # 返回值
///
/// 返回共享的 VCS 信息，调用方可读取最新分支缓存。
pub async fn init(worktree: PathBuf) -> Arc<Mutex<Info>> {
    let wt = worktree.to_string_lossy().to_string();
    file_watcher::init(&worktree);

    let info = Arc::new(Mutex::new(Info { branch: current_branch(&wt) }));
    LOGGER.info(
        "initialized",
        Some({
            let mut m = Map::new();
            m.insert(
                "branch".to_string(),
                info.lock()
                    .ok()
                    .and_then(|x| x.branch.clone())
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            m
        }),
    );

    let info2 = info.clone();
    bus::subscribe(file_watcher::event::UPDATED, move |evt| {
        let Some(file) =
            evt.get("properties").and_then(|props| props.get("file")).and_then(|v| v.as_str())
        else {
            return;
        };
        if !file.ends_with("HEAD") {
            return;
        }
        // HEAD 变化后异步重读分支，避免在 watcher 回调里阻塞事件处理。
        let wt2 = wt.clone();
        let info3 = info2.clone();
        tokio::spawn(async move {
            let next = current_branch(&wt2);
            let mut lock = info3.lock().unwrap_or_else(|e| e.into_inner());
            if next != lock.branch {
                lock.branch = next.clone();
                let _ = bus::publish(event::BRANCH_UPDATED, Info { branch: next }, None);
            }
        });
    });

    info
}

#[cfg(test)]
#[path = "vcs_tests.rs"]
mod vcs_tests;
