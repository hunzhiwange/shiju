//! 运行时日志文件写入工具。
//!
//! 原生入口通过本模块把追踪事件镜像到本地文件，并按级别与小时目录分流。

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;
use tracing::{Level, Metadata};
use tracing_subscriber::fmt::MakeWriter;

const RUNTIME_LOG_DIR_NAME: &str = "runtime_log";

#[derive(Clone, Debug)]
/// 为 `tracing` 按事件级别创建延迟打开的文件写入器。
pub struct RuntimeLogMakeWriter {
    root: Arc<PathBuf>,
}

impl RuntimeLogMakeWriter {
    /// 使用给定根目录创建写入器工厂；实际日志文件会在首次写入时打开。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: Arc::new(root.into()) }
    }

    fn writer_for_level(&self, level: Level) -> RuntimeLogWriter {
        RuntimeLogWriter::new(runtime_log_file_path(&self.root, level))
    }
}

impl<'a> MakeWriter<'a> for RuntimeLogMakeWriter {
    type Writer = RuntimeLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.writer_for_level(Level::INFO)
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        self.writer_for_level(*meta.level())
    }
}

#[derive(Debug)]
/// 指向单个级别日志文件的延迟写入器。
pub struct RuntimeLogWriter {
    path: PathBuf,
    file: Option<File>,
}

impl RuntimeLogWriter {
    fn new(path: PathBuf) -> Self {
        Self { path, file: None }
    }

    fn file(&mut self) -> io::Result<&mut File> {
        if self.file.is_none() {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent)?;
            }
            self.file = Some(OpenOptions::new().create(true).append(true).open(&self.path)?);
        }

        Ok(self.file.as_mut().expect("file should be initialized"))
    }
}

impl Write for RuntimeLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file()?.flush()
    }
}

/// 按配置目录、用户主目录和产品默认目录的优先级解析运行时日志根目录。
pub fn default_runtime_log_dir() -> PathBuf {
    env_path("VIBEWINDOW_CONFIG_DIR")
        .map(|path| path.join(RUNTIME_LOG_DIR_NAME))
        .or_else(|| home_dir().map(|home| default_home_runtime_log_dir(&home)))
        .unwrap_or_else(|| {
            PathBuf::from(vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME)
                .join(RUNTIME_LOG_DIR_NAME)
        })
}

/// 创建运行时日志目录及缺失的父目录。
pub fn ensure_runtime_log_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

fn runtime_log_file_path(root: &Path, level: Level) -> PathBuf {
    runtime_log_file_path_for_hour(root, &Local::now().format("%Y-%m-%d-%H").to_string(), level)
}

fn runtime_log_file_path_for_hour(root: &Path, hour: &str, level: Level) -> PathBuf {
    root.join(hour).join(level_file_name(level))
}

fn level_file_name(level: Level) -> &'static str {
    match level {
        Level::ERROR => "error.log",
        Level::WARN => "warn.log",
        Level::INFO => "info.log",
        Level::DEBUG => "debug.log",
        Level::TRACE => "trace.log",
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).and_then(|value| {
        let path = PathBuf::from(value);
        (!path.as_os_str().is_empty()).then_some(path)
    })
}

fn home_dir() -> Option<PathBuf> {
    env_path("HOME").or_else(|| env_path("USERPROFILE"))
}

fn default_home_runtime_log_dir(home: &Path) -> PathBuf {
    home.join(vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME).join(RUNTIME_LOG_DIR_NAME)
}

#[cfg(test)]
#[path = "runtime_log_tests.rs"]
mod runtime_log_tests;
