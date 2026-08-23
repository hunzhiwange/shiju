//! 验证按键绑定的解析、别名归一化、格式化与匹配规则。
//! 覆盖 leader、组合修饰键、多组输入和空值，确保显示与匹配使用同一模型。

use super::keybind::{Info, matches, parse, to_string};

#[test]
fn parses_formats_and_matches_keybindings() {
    let parsed = parse("<leader> ctrl+shift+del");

    assert_eq!(parsed.len(), 1);
    assert!(parsed[0].leader);
    assert_eq!(parsed[0].name, "del");
    assert!(matches(Some(&parsed[0]), &parsed[0]));
    assert_eq!(
        to_string(Some(&Info {
            name: "delete".into(),
            ctrl: true,
            meta: false,
            shift: false,
            super_key: false,
            leader: false
        })),
        "ctrl+del"
    );
    assert!(parse("none").is_empty());
}

#[test]
fn formats_empty_leader_and_all_modifier_states() {
    let empty = Info {
        name: String::new(),
        ctrl: false,
        meta: false,
        shift: false,
        super_key: false,
        leader: false,
    };
    let leader = Info { leader: true, ..empty.clone() };
    let modified = Info {
        name: "x".into(),
        ctrl: true,
        meta: true,
        shift: true,
        super_key: true,
        leader: true,
    };

    assert_eq!(to_string(None), "");
    assert_eq!(to_string(Some(&empty)), "");
    assert_eq!(to_string(Some(&leader)), "<leader>");
    assert_eq!(to_string(Some(&modified)), "<leader> ctrl+alt+super+shift+x");
    assert!(!matches(None, &empty));
    assert!(!matches(Some(&leader), &empty));
}

#[test]
fn parse_normalizes_modifier_aliases_escape_and_multiple_combinations() {
    let parsed = parse("CTRL+OPTION+SUPER+SHIFT+ESC,meta+x");

    assert_eq!(
        parsed,
        vec![
            Info {
                name: "escape".into(),
                ctrl: true,
                meta: true,
                shift: true,
                super_key: true,
                leader: false,
            },
            Info {
                name: "x".into(),
                ctrl: false,
                meta: true,
                shift: false,
                super_key: false,
                leader: false,
            },
        ]
    );
}
