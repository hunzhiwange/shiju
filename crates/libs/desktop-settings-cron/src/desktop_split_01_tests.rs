#[test]
fn desktop_schedule_draft_preserves_date_boundaries() {
    let rule = crate::state::CronScheduleDraft {
        start_date: Some(None),
        end_date: Some(Some("2028-12-31".parse().unwrap())),
        ..Default::default()
    }
    .request()
    .unwrap();
    assert_eq!(rule.start_date, Some(None));
    assert_eq!(rule.end_date, Some(Some("2028-12-31".into())));
}
