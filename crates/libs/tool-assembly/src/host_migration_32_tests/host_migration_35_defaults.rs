//! Concrete default domain tools use the production domain constructor.
use agent_security::SecurityPolicy;
use std::sync::Arc;
#[test]
fn default_tools_include_apply_patch_once() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = crate::factory::default_tools::<super::host::Host>(security);
    assert_eq!(tools.iter().filter(|tool| tool.name() == "apply_patch").count(), 1);
}
#[test]
fn default_tools_names() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = crate::factory::default_tools::<super::host::Host>(security);
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"shell"));
    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"file_edit"));
    assert!(names.contains(&"file_write"));
    assert!(names.contains(&"apply_patch"));
    assert!(names.contains(&"ls"));
    assert!(names.contains(&"glob"));
    assert!(names.contains(&"grep"));
    assert!(!names.contains(&"lsp"));
    assert!(!names.contains(&"glob_search"));
    assert!(!names.contains(&"content_search"));
}
#[test]
fn apply_patch_spec_uses_v2_aliases() {
    let security = Arc::new(SecurityPolicy::default());
    let spec = crate::factory::default_tools::<super::host::Host>(security)
        .into_iter()
        .find(|tool| tool.name() == "apply_patch")
        .expect("apply_patch tool should exist")
        .spec();

    assert_eq!(spec.id, "apply_patch");
    assert!(!spec.aliases.iter().any(|alias| alias == "edit"));
}
#[test]
fn default_tools_specs_expose_bash_alias() {
    let security = Arc::new(SecurityPolicy::default());
    let spec = crate::factory::default_tools::<super::host::Host>(security)
        .into_iter()
        .find(|tool| tool.name() == "shell")
        .expect("shell tool should exist")
        .spec();

    assert_eq!(spec.id, "bash");
    assert!(spec.aliases.iter().any(|alias| alias == "shell"));
}
