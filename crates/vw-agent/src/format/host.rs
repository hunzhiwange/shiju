//! 连接现有项目上下文与格式化领域。
use agent_format::{State, host::{BoxFuture, FormatHost}};
use crate::{project::instance, util::filesystem};
use std::{path::PathBuf, sync::Arc};

struct ProjectFormatHost;

impl FormatHost for ProjectFormatHost {
    fn directory(&self) -> String { instance::directory() }
    fn worktree(&self) -> String { instance::worktree() }
    fn state(&self) -> BoxFuture<Arc<State>> {
        instance::state("format", State::load, None::<fn(Arc<State>) -> BoxFuture<()>>)()
    }
    fn find_up(&self, target: String, start: String, stop: Option<String>) -> BoxFuture<Vec<PathBuf>> {
        Box::pin(async move { filesystem::find_up(&target, start, stop).await })
    }
}

pub(super) fn install() {
    agent_format::host::install(Arc::new(ProjectFormatHost));
}
