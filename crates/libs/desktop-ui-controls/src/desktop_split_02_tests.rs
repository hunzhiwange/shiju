// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::toast::{Toast, ToastKind};
#[test]
fn toast_clone_preserves_every_kind_and_payload() {
    for (id, kind) in [ToastKind::Success, ToastKind::Info, ToastKind::Warning, ToastKind::Error]
        .into_iter()
        .enumerate()
    {
        let toast = Toast { localized_message: None, id, kind, message: format!("{kind:?}") };
        let cloned = toast.clone();

        assert_eq!(cloned.id, id);
        assert_eq!(cloned.kind, kind);
        assert_eq!(cloned.message, format!("{kind:?}"));
    }
}
}
