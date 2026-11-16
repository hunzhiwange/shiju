use crate::toast::{Toast, ToastKind, view};
fn keep_element(element: iced::Element<'_, ()>) {
    std::hint::black_box(element);
}
#[test]
fn view_returns_empty_element_without_active_toast() {
    let toast = None;

    keep_element(view::<()>(toast.as_ref(), Default::default(), None));
}

#[test]
fn view_renders_active_toast_for_each_kind() {
    for kind in [ToastKind::Success, ToastKind::Info, ToastKind::Warning, ToastKind::Error] {
        let toast = Some(Toast {
            localized_message: None,
            id: 7,
            kind,
            message: format!("toast {kind:?}"),
        });

        keep_element(view::<()>(toast.as_ref(), Default::default(), None));
    }
}
