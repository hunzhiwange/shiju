// 覆盖权限规则的通配匹配、路径展开、配置解析与默认询问策略。

// 后出现的匹配规则拥有更高优先级，无匹配或未知动作统一回到 Ask。
use super::*;
use serde_json::json;
use std::sync::{LazyLock, Mutex};

static STATE_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test]
fn expand_home_handles_exact_prefixed_and_unrelated_patterns() {
    let home = home_dir().to_string_lossy().to_string();

    assert_eq!(expand_home("~"), home);
    assert_eq!(expand_home("$HOME"), home);
    assert_eq!(expand_home("~/Documents"), format!("{home}/Documents"));
    assert_eq!(expand_home("$HOME/cache"), format!("{home}/cache"));
    assert_eq!(expand_home("~other/Documents"), "~other/Documents");
    assert_eq!(expand_home("$HOME-cache"), "$HOME-cache");
    assert_eq!(expand_home("relative/path"), "relative/path");
}

#[test]
fn from_config_parses_simple_and_pattern_rules() {
    let home_pattern = expand_home("~/secret/**");
    let dollar_home_pattern = expand_home("$HOME/cache/**");
    let ruleset = from_config(&json!({
        "read": "allow",
        "edit": {
            "src/**": "ask",
            "~/secret/**": "deny",
            "$HOME/cache/**": "allow",
            "ignored": 42
        },
        "bad": true
    }));

    assert!(
        ruleset
            .iter()
            .any(|r| { r.permission == "read" && r.pattern == "*" && r.action == Action::Allow })
    );
    assert!(
        ruleset.iter().any(|r| {
            r.permission == "edit" && r.pattern == "src/**" && r.action == Action::Ask
        })
    );
    assert!(ruleset.iter().any(|r| {
        r.permission == "edit" && r.pattern == home_pattern && r.action == Action::Deny
    }));
    assert!(ruleset.iter().any(|r| {
        r.permission == "edit" && r.pattern == dollar_home_pattern && r.action == Action::Allow
    }));
    assert!(!ruleset.iter().any(|r| r.permission == "bad"));
}

// 请求 helper 集中构造一次性与永久授权候选，便于覆盖回复状态机。
fn request(patterns: Vec<&str>, always: Vec<&str>) -> Request {
    Request {
        id: String::new(),
        session_id: "session-1".to_string(),
        permission: "edit".to_string(),
        patterns: patterns.into_iter().map(str::to_string).collect(),
        metadata: Map::new(),
        always: always.into_iter().map(str::to_string).collect(),
        tool: Some(ToolInfo { message_id: "m1".to_string(), call_id: "c1".to_string() }),
    }
}

#[test]
fn request_and_reply_serde_preserve_the_public_shape() {
    let req = request(vec!["src/lib.rs"], vec!["src/**"]);
    let value = serde_json::to_value(&req).expect("request should serialize");

    assert_eq!(value["session_id"], "session-1");
    assert_eq!(value["permission"], "edit");
    assert_eq!(value["tool"]["message_id"], "m1");

    let decoded: Request = serde_json::from_value(value).expect("request should deserialize");
    assert_eq!(decoded.patterns, vec!["src/lib.rs".to_string()]);
    assert_eq!(decoded.always, vec!["src/**".to_string()]);
    assert_eq!(decoded.tool.expect("tool info should be present").call_id, "c1");

    assert_eq!(serde_json::to_value(Reply::Once).unwrap(), json!("once"));
    assert_eq!(serde_json::to_value(Reply::Always).unwrap(), json!("always"));
    assert_eq!(serde_json::to_value(Reply::Reject).unwrap(), json!("reject"));
}

#[test]
fn ask_allows_requests_without_patterns() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();

    assert!(ask(request(vec![], vec![]), &Vec::new()).is_ok());
    assert!(list().is_empty());

    reset();
}

#[test]
fn ask_preserves_a_supplied_id_for_pending_requests() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut req = request(vec!["other/file"], vec![]);
    req.id = "permission-known".to_string();

    let pending = ask(req, &Vec::new()).expect_err("unmatched request should be pending");
    let Error::Pending(pending) = pending else {
        panic!("expected pending request");
    };

    assert_eq!(pending.id, "permission-known");
    assert_eq!(list()[0].id, "permission-known");

    reset();
}

#[test]
fn ask_denial_reports_only_rules_for_the_requested_permission() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let ruleset = vec![
        Rule {
            permission: "edit".to_string(),
            pattern: "secret/**".to_string(),
            action: Action::Deny,
        },
        Rule {
            permission: "edit".to_string(),
            pattern: "public/**".to_string(),
            action: Action::Allow,
        },
        Rule { permission: "read".to_string(), pattern: "*".to_string(), action: Action::Deny },
    ];

    let denied = ask(request(vec!["secret/key"], vec![]), &ruleset)
        .expect_err("matching deny should reject the request");
    let Error::Denied(relevant) = denied else {
        panic!("expected denied request");
    };

    assert_eq!(relevant.len(), 2);
    assert!(relevant.iter().all(|rule| rule.permission == "edit"));
    assert!(list().is_empty());

    reset();
}

#[test]
fn ask_allows_denies_and_lists_pending_requests() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();

    let allow = vec![Rule {
        permission: "edit".to_string(),
        pattern: "src/**".to_string(),
        action: Action::Allow,
    }];
    assert!(ask(request(vec!["src/lib.rs"], vec![]), &allow).is_ok());

    let deny = vec![Rule {
        permission: "edit".to_string(),
        pattern: "secret/**".to_string(),
        action: Action::Deny,
    }];
    let err = ask(request(vec!["secret/key"], vec![]), &deny).unwrap_err();
    assert!(matches!(err, Error::Denied(_)));

    let pending = ask(request(vec!["other/file"], vec!["other/**"]), &Vec::new()).unwrap_err();
    let Error::Pending(req) = pending else {
        panic!("expected pending request");
    };
    assert!(!req.id.is_empty());
    assert_eq!(list().len(), 1);

    reset();
    assert!(list().is_empty());
}

#[test]
fn reply_handles_once_reject_corrected_always_and_unknown() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();

    let pending = ask(request(vec!["other/file"], vec!["other/**"]), &Vec::new()).unwrap_err();
    let Error::Pending(req) = pending else {
        panic!("expected pending request");
    };
    let once = reply(&req.id, Reply::Once, None).unwrap().expect("request should exist");
    assert_eq!(once.patterns, vec!["other/file".to_string()]);
    assert!(reply(&req.id, Reply::Once, None).unwrap().is_none());

    let pending = ask(request(vec!["again/file"], vec!["again/**"]), &Vec::new()).unwrap_err();
    let Error::Pending(req) = pending else {
        panic!("expected pending request");
    };
    let corrected = reply(&req.id, Reply::Reject, Some("use another file".to_string()))
        .expect_err("reject with message should return corrected");
    assert!(matches!(corrected, Error::Corrected(_)));

    let pending = ask(request(vec!["always/file"], vec!["always/**"]), &Vec::new()).unwrap_err();
    let Error::Pending(req) = pending else {
        panic!("expected pending request");
    };
    assert!(reply(&req.id, Reply::Always, None).unwrap().is_some());
    assert!(ask(request(vec!["always/next"], vec![]), &Vec::new()).is_ok());

    reset();
}

#[test]
fn reply_reject_without_feedback_removes_the_pending_request() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut request = request(vec!["other/file"], vec![]);
    request.id = "permission-reject".to_string();

    let pending = ask(request, &Vec::new()).unwrap_err();
    let Error::Pending(req) = pending else {
        panic!("expected pending request");
    };

    let rejected = reply(&req.id, Reply::Reject, None).expect_err("reject should return an error");
    assert!(matches!(rejected, Error::Rejected));
    assert!(list().is_empty());

    reset();
}

#[test]
fn always_approval_overrides_rules_until_reset() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut permission_request = request(vec!["approved/first"], vec!["approved/**", "shared/**"]);
    permission_request.id = "permission-always".to_string();

    let pending = ask(permission_request, &Vec::new()).unwrap_err();
    let Error::Pending(req) = pending else {
        panic!("expected pending request");
    };
    reply(&req.id, Reply::Always, None).expect("always reply should succeed");

    let approved = approved_rules();
    assert_eq!(approved.len(), 2);
    assert!(approved.iter().all(|rule| rule.permission == "edit"));
    assert!(approved.iter().all(|rule| rule.action == Action::Allow));

    let deny = vec![Rule {
        permission: "edit".to_string(),
        pattern: "approved/**".to_string(),
        action: Action::Deny,
    }];
    assert!(ask(request(vec!["approved/next"], vec![]), &deny).is_ok());

    reset();
    assert!(matches!(ask(request(vec!["approved/next"], vec![]), &deny), Err(Error::Denied(_))));

    reset();
}

#[test]
fn error_display_includes_rejection_denial_and_pending_messages() {
    let denied = Error::Denied(vec![Rule {
        permission: "bash".to_string(),
        pattern: "rm *".to_string(),
        action: Action::Deny,
    }]);
    assert!(denied.to_string().contains("prevents"));
    assert!(Error::Rejected.to_string().contains("rejected"));
    assert!(Error::Corrected("try read".to_string()).to_string().contains("try read"));
    assert!(Error::Pending(request(vec!["x"], vec![])).to_string().contains("approval"));
}

#[test]
fn poisoned_permission_state_fails_closed_and_recovers_for_other_tests() {
    let _guard = STATE_TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    reset();

    let poison = std::thread::spawn(|| {
        let _state = STATE.lock().expect("permission state should initially be available");
        panic!("poison permission state for fail-closed coverage");
    });
    assert!(poison.join().is_err());

    assert!(list().is_empty());
    assert!(approved_rules().is_empty());
    assert!(matches!(ask(request(vec!["unmatched"], vec![]), &Vec::new()), Err(Error::Rejected)));
    assert!(matches!(reply("missing", Reply::Once, None), Err(Error::Rejected)));

    STATE.clear_poison();
    reset();
}
