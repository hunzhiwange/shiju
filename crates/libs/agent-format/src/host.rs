//! 项目生命周期和文件查找由组合层提供。
use std::{future::Future, path::PathBuf, pin::Pin, sync::{Arc, OnceLock}};
use crate::State;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait FormatHost: Send + Sync + 'static {
    fn directory(&self) -> String;
    fn worktree(&self) -> String;
    fn state(&self) -> BoxFuture<Arc<State>>;
    fn find_up(&self, target: String, start: String, stop: Option<String>) -> BoxFuture<Vec<PathBuf>>;
}

static HOST: OnceLock<Arc<dyn FormatHost>> = OnceLock::new();

/// 由组合入口安装一次，后续调用保留原宿主。
pub fn install(host: Arc<dyn FormatHost>) {
    let _ = HOST.set(host);
}

fn host() -> &'static dyn FormatHost {
    #[cfg(test)]
    crate::test_support::install();
    HOST.get().expect("format host must be installed before use").as_ref()
}

pub(crate) fn directory() -> String { host().directory() }
pub(crate) fn worktree() -> String { host().worktree() }
pub(crate) fn state() -> BoxFuture<Arc<State>> { host().state() }
pub(crate) async fn find_up(target: &str, start: &str, stop: Option<&String>) -> Vec<PathBuf> {
    host().find_up(target.to_owned(), start.to_owned(), stop.cloned()).await
}
