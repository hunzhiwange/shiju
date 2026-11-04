use super::VideoEditTemplateOptions;
use serde_json::json;

#[test]
fn video_edit_options_defaults_to_five_and_round_trips() {
    let options: VideoEditTemplateOptions = serde_json::from_value(json!({})).unwrap();
    assert_eq!(options.max_parallel(), 5);
    for value in 1..=5 {
        let options = VideoEditTemplateOptions::new(value).unwrap();
        let template = json!({"workflow": {"video_edit": options}});
        assert_eq!(template["workflow"]["video_edit"]["max_parallel"], value);
        assert_eq!(
            serde_json::from_value::<VideoEditTemplateOptions>(
                template["workflow"]["video_edit"].clone()
            )
            .unwrap(),
            options
        );
    }
}

#[test]
fn video_edit_options_rejects_invalid_or_unknown_parameters() {
    for value in [json!(0), json!(6), json!(-1), json!(256), json!("5"), json!(1.5), json!(null)] {
        assert!(
            serde_json::from_value::<VideoEditTemplateOptions>(json!({"max_parallel": value}))
                .is_err()
        );
    }
    assert!(VideoEditTemplateOptions::new(0).is_err());
    assert!(VideoEditTemplateOptions::new(6).is_err());
    assert!(
        serde_json::from_value::<VideoEditTemplateOptions>(json!({"segment_seconds": 60})).is_err()
    );
}
