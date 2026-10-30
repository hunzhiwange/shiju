use super::*;

#[test]
fn markdown_delivery_error_preserves_progress_without_secrets() {
    for kind in
        [DeliveryFailureKind::NotSent, DeliveryFailureKind::Rejected, DeliveryFailureKind::Unknown]
    {
        let error: anyhow::Error =
            ChannelDeliveryError { confirmed_parts: 1, total_parts: 4, failed_part_index: 1, kind }
                .into();
        let error = error.context("QQ delivery");
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(progress.confirmed_parts, 1);
        assert_eq!(progress.total_parts, 4);
        assert_eq!(progress.failed_part_index, 1);
        assert_eq!(progress.kind, kind);
        let display = format!("{error:#}");
        let debug = format!("{error:?}");
        for value in [&display, &debug] {
            assert!(!value.contains("https://"));
            assert!(!value.contains("token"));
            assert!(!value.contains("正文"));
        }
    }
}
