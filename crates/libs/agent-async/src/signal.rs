//! 提供一次性触发、可等待的异步信号。
//! 信号用于任务间的轻量协调，触发后等待方可以立即继续。

pub struct Signal {
    tx: tokio::sync::watch::Sender<bool>,
    rx: tokio::sync::watch::Receiver<bool>,
}

impl Signal {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        Self { tx, rx }
    }

    pub fn trigger(&self) {
        let _ = self.tx.send(true);
    }

    pub async fn wait(&mut self) {
        if *self.rx.borrow() {
            return;
        }
        while self.rx.changed().await.is_ok() {
            if *self.rx.borrow() {
                break;
            }
        }
    }

    #[cfg(test)]
    pub(super) fn test_sender(&self) -> tokio::sync::watch::Sender<bool> {
        self.tx.clone()
    }
}
