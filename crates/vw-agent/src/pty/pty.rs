//! 终端领域的项目实例组合适配。
pub use agent_pty::{event, CreateInput, Error, Info, Size, Status, UpdateInput};
use agent_pty::{State, host::{BoxFuture, PtyHost}};
use crate::project::instance;
use std::sync::Arc;

struct ProjectPtyHost;
impl PtyHost for ProjectPtyHost {
    fn directory(&self) -> String { instance::directory() }
    fn state(&self) -> BoxFuture<Arc<State>> {
        instance::state(
            "pty",
            || async { State::default() },
            Some(|state: Arc<State>| async move {
                #[cfg(not(target_arch = "wasm32"))]
                state.dispose();
            }),
        )()
    }
}

fn install() {
    agent_pty::host::install(Arc::new(ProjectPtyHost));
}

pub async fn list() -> Vec<Info> {
    install();
    agent_pty::list().await
}

pub async fn get(id: &str) -> Option<Info> {
    install();
    agent_pty::get(id).await
}

pub async fn create(input: CreateInput) -> Result<Info, Error> {
    install();
    agent_pty::create(input).await
}

pub async fn update(id: &str, input: UpdateInput) -> Result<Option<Info>, Error> {
    install();
    agent_pty::update(id, input).await
}

pub async fn remove(id: &str) -> Result<bool, Error> {
    install();
    agent_pty::remove(id).await
}

pub async fn resize(id: &str, cols: u16, rows: u16) {
    install();
    agent_pty::resize(id, cols, rows).await
}

pub async fn write(id: &str, data: &str) {
    install();
    agent_pty::write(id, data).await
}

pub async fn read(id: &str, cursor: i64) -> Option<(String, usize)> {
    install();
    agent_pty::read(id, cursor).await
}
