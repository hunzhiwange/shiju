//! 覆盖组合键配置的静态冲突检查、终端别名归一化与运行时匹配状态机。
//! 用例按上下文重叠关系验证允许与拒绝边界，并守住保留键的安全语义。

use super::KEY_CHORD_TIMEOUT;
use super::KeyChordMatch;
use super::KeyChordMatcher;
use super::KeymapContextSet;
use super::dispatch_binding;
use crate::key_hint;
use crate::key_hint::KeyBindingListExt;
use crate::keymap::RuntimeKeymap;
use crate::keymap::bindings::KeymapContext;
use crate::keymap::bindings::keymap_action_ids;
use crate::keymap::keymap_action_id;
use crate::keymap::user_bindings;
use codex_config::types::KeybindingSpec;
use codex_config::types::KeybindingsSpec;
use codex_config::types::TuiKeymap;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use pretty_assertions::assert_eq;
use std::collections::HashSet;
use tokio::time::Instant;

fn binding(spec: &str) -> KeybindingsSpec {
    // 为单个规范构造最小配置，减少拒绝场景中的无关绑定噪声。
    KeybindingsSpec::One(KeybindingSpec(spec.to_string()))
}

fn bindings(specs: &[&str]) -> KeybindingsSpec {
    // 多绑定 fixture 用于表达共享前缀、重复项和上下文交集。
    KeybindingsSpec::Many(
        specs
            .iter()
            .map(|spec| KeybindingSpec((*spec).to_string()))
            .collect(),
    )
}

#[test]
// 列表组合键不得占用恢复选择器的固定导航键，否则运行时行为会产生歧义。
fn rejects_fixed_resume_picker_keys_inside_list_chords() {
    for (chord, reserved_action) in [
        ("ctrl-c home", "cancel"),
        ("ctrl-t home", "resume_picker.toggle_transcript"),
        ("ctrl-e home", "resume_picker.toggle_expansion"),
        ("ctrl-o home", "resume_picker.toggle_density"),
    ] {
        let mut config = TuiKeymap::default();
        config.list.jump_top = Some(binding(chord));

        let error = RuntimeKeymap::from_config(&config)
            .expect_err("fixed picker actions must remain available");

        assert!(error.contains(reserved_action), "{error}");
    }
}

#[test]
// 审批上下文中的 Ctrl-C 必须保留取消语义，不能成为组合键的一部分。
fn rejects_ctrl_c_inside_approval_chords() {
    let mut config = TuiKeymap::default();
    config.approval.approve = Some(binding("ctrl-x ctrl-c"));

    let error =
        RuntimeKeymap::from_config(&config).expect_err("Ctrl+C must continue cancelling approvals");

    assert!(error.contains("cancel"), "{error}");
}

#[test]
// 相同按键序列应依据当前上下文解析为各自动作，并保持上下文隔离。
fn resolves_chords_for_actions_in_different_contexts() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(bindings(&["ctrl-t", "ctrl-x ctrl-t"]));
    config.composer.submit = Some(binding("ctrl-x enter"));
    config.list.jump_top = Some(binding("ctrl-x home"));

    let actual = RuntimeKeymap::from_config(&config).expect("valid chord keymap");

    assert_eq!(
        user_bindings(&actual.app.open_transcript),
        vec![key_hint::ctrl(KeyCode::Char('t'))]
    );
    for (context, action, expected) in [
        (
            "global",
            "open_transcript",
            vec!["ctrl-t".to_string(), "ctrl-x ctrl-t".to_string()],
        ),
        ("composer", "submit", vec!["ctrl-x enter".to_string()]),
        ("list", "jump_top", vec!["ctrl-x home".to_string()]),
    ] {
        let action = keymap_action_id(context, action).expect("known action");
        assert_eq!(
            actual.chords.configured_specs(action),
            Some(expected.as_slice())
        );
    }
}

#[test]
// 编辑器未命中局部组合键时，应继续尝试全局前缀而不是立即取消。
fn composer_chord_inherits_global_fallback() {
    let mut config = TuiKeymap::default();
    config.global.submit = Some(binding("ctrl-x enter"));

    let actual = RuntimeKeymap::from_config(&config).expect("valid global fallback chord");
    let action = keymap_action_id("composer", "submit").expect("known action");

    assert_eq!(
        actual.chords.configured_specs(action),
        Some(["ctrl-x enter".to_string()].as_slice())
    );
}

#[test]
// 共享前缀只要最终按键不同，就应形成两条可达的组合键路径。
fn allows_shared_prefixes_with_distinct_completions() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-x ctrl-t"));
    config.global.copy = Some(binding("ctrl-x ctrl-o"));

    RuntimeKeymap::from_config(&config).expect("prefix trie can branch on the second stroke");
}

#[test]
// Vim 模式允许可打印前缀，重复按键也应在所有上下文中稳定解析。
fn allows_plain_prefixes_in_vim_and_repeated_chords_in_all_contexts() {
    let mut config = TuiKeymap::default();
    config.vim_normal.move_line_start = Some(binding("g g"));
    config.vim_operator.motion_line_start = Some(binding("g g"));
    config.global.open_transcript = Some(binding("ctrl-t ctrl-t"));

    RuntimeKeymap::from_config(&config)
        .expect("Vim owns printable prefixes and all contexts support repeated chords");
}

#[test]
// 非 Vim 输入会把普通字符交给编辑器，因此不能截获它们作为组合键前缀。
fn rejects_plain_printable_prefixes_outside_vim() {
    let mut config = TuiKeymap::default();
    config.composer.submit = Some(binding("x enter"));

    let error = RuntimeKeymap::from_config(&config).expect_err("typing must not start a chord");

    assert!(error.contains("outside Vim"), "{error}");
    assert!(error.contains("ordinary text input"), "{error}");
}

#[test]
// 组合键前缀若覆盖同一上下文中的单键动作，会让单键无法及时分派，必须拒绝。
fn rejects_prefixes_that_shadow_overlapping_single_bindings() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-k ctrl-t"));

    let error = RuntimeKeymap::from_config(&config).expect_err("ctrl-k already edits text");

    assert!(error.contains("editor.kill_line_end"), "{error}");
    assert!(error.contains("ctrl-k ctrl-t"), "{error}");
}

#[test]
// 互不同时激活的上下文可以复用同一组合键，不应产生伪冲突。
fn permits_the_same_chord_in_non_overlapping_contexts() {
    let mut config = TuiKeymap::default();
    config.editor.move_line_start = Some(binding("ctrl-x home"));
    config.list.jump_top = Some(binding("ctrl-x home"));

    RuntimeKeymap::from_config(&config)
        .expect("editor and list contexts are never active together");
}

#[test]
// 上下文存在交集时的重复组合键无法唯一分派，配置校验必须报错。
fn rejects_duplicate_chords_in_overlapping_contexts() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-x ctrl-t"));
    config.composer.submit = Some(binding("ctrl-x ctrl-t"));

    let error =
        RuntimeKeymap::from_config(&config).expect_err("one active chord must identify one action");

    assert!(error.contains("global.open_transcript"), "{error}");
    assert!(error.contains("composer.submit"), "{error}");
}

#[test]
// 请求用户输入会同时使用聊天与列表上下文，因此跨两者的重复项也会冲突。
fn rejects_duplicate_chat_and_list_chords_used_by_request_user_input() {
    let mut config = TuiKeymap::default();
    config.chat.interrupt_turn = Some(binding("ctrl-x home"));
    config.list.jump_top = Some(binding("ctrl-x home"));

    let error =
        RuntimeKeymap::from_config(&config).expect_err("request input activates both contexts");

    assert!(error.contains("chat.interrupt_turn"), "{error}");
    assert!(error.contains("list.jump_top"), "{error}");
}

#[test]
// 终端对 Ctrl-/ 的等价编码应先归一化，再参与冲突和匹配判断。
fn normalizes_ctrl_slash_terminal_aliases() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-7 ctrl-t"));

    let error = RuntimeKeymap::from_config(&config)
        .expect_err("Ctrl-7 must remain the Ctrl-/ side-conversation alias");

    assert!(error.contains("global.toggle_side_conversation"), "{error}");
}

#[test]
// 归一化后相同的别名不能绕过重复组合键检查。
fn rejects_alias_equivalent_duplicate_chords() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-x ctrl-/"));
    config.global.copy = Some(binding("ctrl-x ctrl-7"));
    config.global.toggle_side_conversation = Some(KeybindingsSpec::Many(Vec::new()));

    let error = RuntimeKeymap::from_config(&config).expect_err("aliases must identify one action");

    assert!(error.contains("global.open_transcript"), "{error}");
    assert!(error.contains("global.copy"), "{error}");
}

#[test]
// 裸 Escape 负责取消待定状态，不能同时充当普通组合键按键。
fn rejects_plain_escape_inside_a_chord() {
    let mut config = TuiKeymap::default();
    config.list.cancel = Some(binding("ctrl-x esc"));

    let error = RuntimeKeymap::from_config(&config).expect_err("Esc cancels pending chords");

    assert!(error.contains("reserved for cancelling"), "{error}");
}

#[test]
// 主界面的中断保留键必须保持立即生效，禁止被组合键延迟消费。
fn rejects_reserved_interrupt_keys_inside_a_main_chord() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-x ctrl-c"));

    let error = RuntimeKeymap::from_config(&config).expect_err("ctrl-c must remain available");

    assert!(error.contains("fixed.interrupt_or_quit"), "{error}");
}

#[cfg(unix)]
#[test]
// Unix 挂起键在任意按键位置都应保留作业控制语义。
fn rejects_unix_suspend_key_in_any_chord_stroke() {
    for chord in ["ctrl-z ctrl-t", "ctrl-x ctrl-z"] {
        let mut config = TuiKeymap::default();
        config.list.jump_top = Some(binding(chord));

        let error = RuntimeKeymap::from_config(&config)
            .expect_err("the Unix suspend key must not be consumed");

        assert!(error.contains("suspending"), "{error}");
    }
}

#[cfg(windows)]
#[test]
// Windows 的 AltGr 前缀可能代表字符输入，除 Vim 外不能作为组合键起点。
fn rejects_windows_altgr_prefixes_outside_vim() {
    let mut config = TuiKeymap::default();
    config.composer.submit = Some(binding("ctrl-alt-x enter"));

    let error =
        RuntimeKeymap::from_config(&config).expect_err("AltGr may be ordinary Windows text input");

    assert!(error.contains("AltGr"), "{error}");
}

#[test]
// 动作清单必须拥有唯一分派令牌，确保运行时不靠上下文猜测目标动作。
fn dispatch_tokens_are_unique_and_cover_the_action_inventory() {
    let mut tokens = HashSet::new();
    let actions = keymap_action_ids().collect::<Vec<_>>();

    for action in &actions {
        assert!(
            tokens.insert(dispatch_binding(*action).expect("action inventory must fit token band")),
            "duplicate token for {}",
            action.config_path()
        );
    }

    assert_eq!(tokens.len(), actions.len());
}

#[test]
// 只用于物理键分派的令牌不应误清除正在等待后续按键的状态。
fn physical_dispatch_tokens_do_not_clear_pending_chords() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-x ctrl-t"));
    let keymap = RuntimeKeymap::from_config(&config).expect("valid chord keymap");
    let contexts = KeymapContextSet::new(KeymapContext::Global);
    let now = Instant::now();
    let mut matcher = KeyChordMatcher::default();

    assert!(matches!(
        matcher.advance(
            key_event(key_hint::ctrl(KeyCode::Char('x'))),
            &keymap.chords,
            contexts,
            now,
        ),
        KeyChordMatch::Pending(_)
    ));

    let action = keymap_action_id("global", "open_transcript").expect("known keymap action");
    let token = dispatch_binding(action).expect("internal dispatch token");
    assert_eq!(
        matcher.advance(key_event(token), &keymap.chords, contexts, now),
        KeyChordMatch::Ignored
    );
    assert!(matcher.is_pending());
    assert!(matches!(
        matcher.advance(
            key_event(key_hint::ctrl(KeyCode::Char('t'))),
            &keymap.chords,
            contexts,
            now,
        ),
        KeyChordMatch::Completed(_)
    ));
}

#[test]
// 匹配器应积累完整序列并仅分派当前激活上下文对应的动作。
fn matching_chords_dispatches_the_active_context_action() {
    let mut config = TuiKeymap::default();
    config.global.open_transcript = Some(binding("ctrl-t ctrl-t"));
    config.editor.move_line_start = Some(binding("ctrl-x home"));
    config.list.jump_top = Some(binding("ctrl-x home"));
    let runtime = RuntimeKeymap::from_config(&config).expect("non-overlapping chord contexts");
    let now = Instant::now();

    for (context, prefix, completion, target, other) in [
        (
            KeymapContext::Global,
            key_event(key_hint::ctrl(KeyCode::Char('t'))),
            key_event(key_hint::ctrl(KeyCode::Char('t'))),
            runtime.app.open_transcript.as_slice(),
            runtime.app.copy.as_slice(),
        ),
        (
            KeymapContext::Editor,
            key_event(key_hint::ctrl(KeyCode::Char('x'))),
            KeyEvent::new(KeyCode::Home, crossterm::event::KeyModifiers::NONE),
            runtime.editor.move_line_start.as_slice(),
            runtime.list.jump_top.as_slice(),
        ),
        (
            KeymapContext::List,
            key_event(key_hint::ctrl(KeyCode::Char('x'))),
            KeyEvent::new(KeyCode::Home, crossterm::event::KeyModifiers::NONE),
            runtime.list.jump_top.as_slice(),
            runtime.editor.move_line_start.as_slice(),
        ),
    ] {
        let mut matcher = KeyChordMatcher::default();
        assert!(matches!(
            matcher.advance(prefix, &runtime.chords, KeymapContextSet::new(context), now,),
            KeyChordMatch::Pending(_)
        ));
        let KeyChordMatch::Completed(dispatch_event) = matcher.advance(
            completion,
            &runtime.chords,
            KeymapContextSet::new(context),
            now,
        ) else {
            panic!("second stroke must complete the active context chord");
        };
        assert!(target.is_pressed(dispatch_event));
        assert!(!other.is_pressed(dispatch_event));
    }
}

#[test]
// Escape、上下文切换和超时都必须可靠撤销待定组合键，避免迟到动作泄漏。
fn matcher_cancels_on_escape_context_change_and_timeout() {
    let mut config = TuiKeymap::default();
    config.list.jump_top = Some(binding("ctrl-x ctrl-t"));
    let runtime = RuntimeKeymap::from_config(&config).expect("valid list chord");
    let list = KeymapContextSet::new(KeymapContext::List);
    let pager = KeymapContextSet::new(KeymapContext::Pager);
    let prefix = key_event(key_hint::ctrl(KeyCode::Char('x')));
    let now = Instant::now();

    let mut matcher = KeyChordMatcher::default();
    assert!(matches!(
        matcher.advance(prefix, &runtime.chords, list, now),
        KeyChordMatch::Pending(_)
    ));
    assert_eq!(
        matcher.advance(
            KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE),
            &runtime.chords,
            list,
            now,
        ),
        KeyChordMatch::Cancelled
    );

    assert!(matches!(
        matcher.advance(prefix, &runtime.chords, list, now),
        KeyChordMatch::Pending(_)
    ));
    assert!(matcher.expire(pager, now));

    assert!(matches!(
        matcher.advance(prefix, &runtime.chords, list, now),
        KeyChordMatch::Pending(_)
    ));
    assert!(matcher.expire(list, now + KEY_CHORD_TIMEOUT));
}

fn key_event(binding: crate::key_hint::KeyBinding) -> KeyEvent {
    // 将配置层按键转换为运行时事件，统一匹配器用例的事件属性。
    let (code, modifiers) = binding.parts();
    KeyEvent::new(code, modifiers)
}
