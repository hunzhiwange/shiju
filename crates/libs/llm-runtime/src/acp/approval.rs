//! A prompt owns its confirmation callback and revokes it before clearing the slot.

use std::{sync::Arc, time::Duration};

use acp_host_io::permissions::PermissionConfirmation;
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;

const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

pub struct PromptApproval<'a> {
    slot: &'a Mutex<Option<PermissionConfirmation>>,
    installed: Option<PermissionConfirmation>,
    cancellation: CancellationToken,
}

impl<'a> PromptApproval<'a> {
    pub fn bind(
        slot: &'a Mutex<Option<PermissionConfirmation>>,
        confirmation: Option<PermissionConfirmation>,
        cancellation: CancellationToken,
    ) -> Self {
        let installed = confirmation.map(|confirmation| {
            let cancellation = cancellation.clone();
            let guarded: PermissionConfirmation = Arc::new(move |title, arguments| {
                let confirmation = Arc::clone(&confirmation);
                let cancellation = cancellation.clone();
                Box::pin(async move {
                    tokio::select! {
                        biased;
                        () = cancellation.cancelled() => false,
                        approved = tokio::time::timeout(APPROVAL_TIMEOUT, async {
                            confirmation(title, arguments).await
                        }) => {
                            approved.unwrap_or(false) && !cancellation.is_cancelled()
                        }
                    }
                })
            });
            guarded
        });
        *slot.lock() = installed.clone();
        Self { slot, installed, cancellation }
    }
}

impl Drop for PromptApproval<'_> {
    fn drop(&mut self) {
        self.cancellation.cancel();
        let mut slot = self.slot.lock();
        if let (Some(active), Some(installed)) = (slot.as_ref(), self.installed.as_ref())
            && Arc::ptr_eq(active, installed)
        {
            *slot = None;
        }
    }
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;
