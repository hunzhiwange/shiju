use super::HookMatcher;
use crate::hook_manifest::CodexHookEvent as E;

#[test]
fn codex_plugin_compat_t33_rejects_invalid_input() {
    assert_eq!(
        HookMatcher::new(E::PreToolUse, Some("[secret")).err().unwrap().to_string(),
        "invalid Codex hook matcher"
    );
    assert!(HookMatcher::new(E::Stop, Some("[")).unwrap().matches(None));
    for pattern in [None, Some(""), Some("*")] {
        assert!(HookMatcher::new(E::PreToolUse, pattern).unwrap().matches(None));
    }
    assert!(
        !HookMatcher::new(E::PreToolUse, Some("Write|Edit")).unwrap().matches(Some("Overwrite"))
    );
    for event in [
        E::SessionStart,
        E::SessionEnd,
        E::PreCompact,
        E::PostCompact,
        E::SubagentStart,
        E::SubagentStop,
        E::PermissionRequest,
    ] {
        let matcher = HookMatcher::new(event, Some("^auto$")).unwrap();
        assert!(matcher.matches(Some("auto")));
        assert!(!matcher.matches(Some("manual")));
    }
}
