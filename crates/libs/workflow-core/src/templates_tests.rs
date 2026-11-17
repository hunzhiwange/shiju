use super::*;
#[test]
fn video_edit_template_without_options_defaults_to_five_parallel_tasks() {
 let no_options = workflow_template_contract::DEFAULT_YAML.replace("  video_edit:\n    max_parallel: 5\n", "");
 assert_eq!(validate_template_yaml(&no_options).unwrap().max_parallel(), 5);
}
