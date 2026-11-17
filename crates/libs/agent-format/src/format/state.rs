//! 格式化子系统的实例状态与数据模型。
//!
//! 该模块保存格式化器静态配置、启用探测缓存以及对外暴露的状态 DTO，
//! 让探测和运行时逻辑共享同一份项目级状态。

use agent_config::runtime as config;
use crate::host as instance;
use agent_log as log;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

pub(super) static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("format".to_string()));
        m
    }))
});

/// 对外展示的格式化器状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatterStatus {
    /// 格式化器名称。
    pub name: String,
    /// 该格式化器声明支持的文件扩展名。
    pub extensions: Vec<String>,
    /// 当前项目中是否启用。
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub(super) struct FormatterInfo {
    pub(super) name: String,
    pub(super) command: Vec<String>,
    pub(super) environment: HashMap<String, String>,
    pub(super) extensions: Vec<String>,
    pub(super) enabled: EnabledCheck,
}

#[derive(Debug, Clone)]
pub(super) enum EnabledCheck {
    #[cfg(test)]
    Always,
    Which(&'static str),
    #[cfg(test)]
    FileUpAny(&'static [&'static str]),
    Prettier,
    Oxfmt,
    Biome,
    ClangFormat,
    Ruff,
    UvFormat,
    Pint,
    Ocamlformat,
    RLangAir,
}

/// 格式化子系统的项目级状态。
#[derive(Debug, Default)]
pub struct State {
    pub(super) enabled: Mutex<HashMap<String, bool>>,
    pub(super) formatters: HashMap<String, FormatterInfo>,
}

pub(super) fn instance_state()
-> impl Fn() -> crate::host::BoxFuture<Arc<State>> + Send + Sync + 'static {
    || instance::state()
}

pub(crate) async fn load_state() -> State {
    let _cfg = config::get().await;
    let formatters = super::builtins::builtin_formatters();

    State { enabled: Mutex::new(HashMap::new()), formatters }
}

impl State {
    /// 供宿主项目状态缓存惰性初始化使用。
    pub async fn load() -> Self {
        load_state().await
    }
}
