//! 提示取消与工作线程回收边界。

use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_util::sync::CancellationToken;
use crate::{PromptEvent, PromptTermination};

/// 宿主按当前会话及截止时间回收进程；不扩大取消范围。
pub trait PromptCleanup: std::fmt::Debug + Send + Sync {
    fn reap(&self, cutoff: Option<std::time::Instant>) -> anyhow::Result<()>;
}

/// 显式、幂等的提示取消句柄。
#[derive(Debug, Clone)]
pub struct PromptCancelHandle {
    explicit: Arc<AtomicBool>,
    shell_cutoff: Arc<std::sync::Mutex<Option<std::time::Instant>>>,
    cleanup: Arc<dyn PromptCleanup>,
    token: CancellationToken,
    terminal: Arc<AtomicBool>,
    events: mpsc::Sender<PromptEvent>,
}

impl PromptCancelHandle {
    /// Bind the worker's terminal state and host cleanup to this exact prompt.
    pub fn new(
        token: CancellationToken,
        terminal: Arc<AtomicBool>,
        events: mpsc::Sender<PromptEvent>,
        cleanup: Arc<dyn PromptCleanup>,
    ) -> Self {
        Self {
            explicit: Arc::new(AtomicBool::new(false)),
            shell_cutoff: Arc::new(std::sync::Mutex::new(None)),
            token,
            terminal,
            events,
            cleanup,
        }
    }

    pub fn is_explicit(&self) -> bool {
        self.explicit.load(Ordering::Acquire)
    }

    /// 取消活动轮次；仅首次成功取消返回 `true`。
    pub fn cancel(&self) -> bool {
        self.explicit.store(true, Ordering::Release);
        self.token.cancel();
        let cleanup = self.reap_shell_processes();
        if self.terminal.swap(true, Ordering::AcqRel) {
            return false;
        }
        let termination = match cleanup {
            Ok(()) => PromptTermination::Cancelled,
            Err(error) => PromptTermination::Failed(error.to_string()),
        };
        let _ = self.events.send(PromptEvent::Terminated(termination));
        true
    }

    /// Stop consuming this prompt without treating transport loss as user stop.
    pub fn interrupt(&self) {
        self.token.cancel();
    }

    /// A retained stop handle must not cancel commands from a later prompt.
    pub fn finish_shell_range(&self) {
        self.shell_cutoff
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert_with(std::time::Instant::now);
    }

    pub fn reap_shell_processes(&self) -> anyhow::Result<()> {
        let cutoff =
            *self.shell_cutoff.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        self.cleanup.reap(cutoff)?;
        Ok(())
    }

    /// 返回取消是否已被请求。
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

/// 已启动提示轮次的事件接收端和取消句柄。
#[derive(Debug)]
pub struct PromptTurn {
    /// 按产生顺序接收事件，且最多接收一个终态。
    pub events: mpsc::Receiver<PromptEvent>,
    /// 可跨线程调用的幂等取消句柄。
    pub cancel: PromptCancelHandle,
    /// Joins execution and its cleanup, independently of early UI terminal events.
    pub worker: std::thread::JoinHandle<Result<(), String>>,
}
