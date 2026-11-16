use crate::toast::ToastKind;
#[test]
fn toast_helpers_store_kind_message_and_increment_id() {
    let mut active_toast = None;
    let mut next_toast_id = 3;

    let _ = crate::toast::show_toast(
        &mut active_toast,
        &mut next_toast_id,
        ToastKind::Success,
        "saved".into(),
    );
    assert_eq!(active_toast.as_ref().map(|toast| toast.id), Some(3));
    assert_eq!(active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Success));
    assert_eq!(active_toast.as_ref().map(|toast| toast.message.as_str()), Some("saved"));

    let _ = crate::toast::show_toast(
        &mut active_toast,
        &mut next_toast_id,
        ToastKind::Error,
        "failed".into(),
    );
    assert_eq!(active_toast.as_ref().map(|toast| toast.id), Some(4));
    assert_eq!(active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Error));

    let _ = crate::toast::show_toast(
        &mut active_toast,
        &mut next_toast_id,
        ToastKind::Info,
        "noted".into(),
    );
    assert_eq!(active_toast.as_ref().map(|toast| toast.id), Some(5));
    assert_eq!(active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Info));

    let _ = crate::toast::show_toast(
        &mut active_toast,
        &mut next_toast_id,
        ToastKind::Warning,
        "careful".into(),
    );
    assert_eq!(active_toast.as_ref().map(|toast| toast.id), Some(6));
    assert_eq!(active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Warning));
    assert_eq!(next_toast_id, 7);
}

#[test]
fn desktop_integration_29_preserves_expected_behavior() {
    let mut active_toast = None;
    let mut next_toast_id = 0;
    crate::toast::show_toast(
        &mut active_toast,
        &mut next_toast_id,
        ToastKind::Success,
        "业务已完成".into(),
    );
    let original = active_toast.clone().unwrap();
    let next_id = next_toast_id;
    for result in [Err("通知权限未获允许".to_owned()), Err("通知权限未获允许".to_owned()), Ok(())]
    {
        if let Err(error) = result {
            assert!(
                crate::toast::show_supplemental_error(&mut active_toast, &mut next_toast_id, error)
                    .is_none()
            );
        }
        let toast = active_toast.as_ref().unwrap();
        assert_eq!(toast.id, original.id);
        assert_eq!(toast.kind, ToastKind::Success);
        assert_eq!(toast.message, original.message);
        assert_eq!(next_toast_id, next_id);
    }
}
#[test]
fn desktop_integration_29_reports_failure_when_no_toast_is_visible() {
    let mut active_toast = None;
    let mut next_id = 0;
    assert!(
        crate::toast::show_supplemental_error(
            &mut active_toast,
            &mut next_id,
            "permission denied".into()
        )
        .is_some()
    );
    let toast = active_toast.as_ref().unwrap();
    assert_eq!(toast.kind, ToastKind::Error);
    assert!(toast.message.contains("permission denied"));
}
