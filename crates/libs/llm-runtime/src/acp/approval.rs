//! A prompt owns its confirmation callback and cancels it before clearing the slot.
use acp_host_io::permissions::PermissionConfirmation;
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;

pub struct PromptApproval<'a> {
    slot: &'a Mutex<Option<PermissionConfirmation>>,
    cancellation: CancellationToken,
}
impl<'a> PromptApproval<'a> {
    pub fn bind(slot: &'a Mutex<Option<PermissionConfirmation>>, confirmation: Option<PermissionConfirmation>, cancellation: CancellationToken) -> Self {
        *slot.lock() = confirmation;
        Self { slot, cancellation }
    }
}
impl Drop for PromptApproval<'_> {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.slot.lock().take();
    }
}
