    use super::*;

    // 验证 `action_required_title_keeps_security_surface_visible` 场景的触发条件与预期结果。
    #[test]
    fn action_required_title_keeps_security_surface_visible() {
        assert_eq!(
            build_action_required_title_text(
                ACTION_REQUIRED_PREVIEW_PREFIX,
                [ActionRequiredTitleItem::Spinner, ActionRequiredTitleItem::Approval],
                &[],
                |item| (item == ActionRequiredTitleItem::Approval).then(|| "approval".to_string()),
            ),
            "[ ! ] Action Required | approval"
        );
    }
