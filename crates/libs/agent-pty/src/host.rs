//! 由宿主提供当前项目上下文及其生命周期状态。
use std::{future::Future, pin::Pin, sync::{Arc, OnceLock}};
use crate::State;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait PtyHost: Send + Sync + 'static {
    fn directory(&self) -> String;
    fn state(&self) -> BoxFuture<Arc<State>>;
}

static HOST: OnceLock<Arc<dyn PtyHost>> = OnceLock::new();

/// 组合层在调用终端 API 前安装真实项目适配器。
pub fn install(host: Arc<dyn PtyHost>) {
    let _ = HOST.set(host);
}

fn host() -> &'static dyn PtyHost {
    HOST.get().expect("pty host must be installed before use").as_ref()
}

pub(crate) fn directory() -> String { host().directory() }
pub(crate) fn state() -> BoxFuture<Arc<State>> { host().state() }
