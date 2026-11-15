//! 迁移测试使用的项目上下文夹具，不执行项目发现或持久化。
use crate::{State, host::{BoxFuture, FormatHost}};
use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, LazyLock, Mutex}};
use tokio::sync::OnceCell;

tokio::task_local! { static DIRECTORY: String; }
static STATES: LazyLock<Mutex<HashMap<String, Arc<OnceCell<Arc<State>>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn global_env_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

struct TestHost;
pub(crate) fn install() { crate::host::install(Arc::new(TestHost)); }

impl FormatHost for TestHost {
    fn directory(&self) -> String { DIRECTORY.try_with(Clone::clone).unwrap_or_default() }
    fn worktree(&self) -> String { self.directory() }
    fn state(&self) -> BoxFuture<Arc<State>> {
        let key = self.directory();
        Box::pin(async move {
            let cell = STATES.lock().unwrap_or_else(|e| e.into_inner())
                .entry(key).or_default().clone();
            cell.get_or_init(|| async { Arc::new(State::load().await) }).await.clone()
        })
    }
    fn find_up(&self, target: String, start: String, stop: Option<String>) -> BoxFuture<Vec<PathBuf>> {
        Box::pin(async move {
            let mut current = PathBuf::from(start);
            let stop = stop.map(PathBuf::from);
            let mut found = Vec::new();
            loop {
                let path = current.join(&target);
                if std::fs::metadata(&path).is_ok() { found.push(path); }
                if stop.as_ref() == Some(&current) { break; }
                let Some(parent) = current.parent().map(Path::to_path_buf) else { break; };
                if parent == current { break; }
                current = parent;
            }
            found
        })
    }
}

pub(crate) async fn provide<R>(
    directory: impl AsRef<Path>,
    init: Option<Box<dyn FnOnce() -> BoxFuture<()> + Send + 'static>>,
    f: impl FnOnce() -> BoxFuture<R> + Send + 'static,
) -> Result<R, std::convert::Infallible> {
    install();
    DIRECTORY.scope(directory.as_ref().to_string_lossy().into_owned(), async move {
        if let Some(init) = init { init().await; }
        Ok(f().await)
    }).await
}
