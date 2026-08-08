use super::parse_shortcut;
use muda::accelerator::{Code, Modifiers};

#[test]
fn parse_shortcut_maps_command_and_shift_combinations() {
    let redo = parse_shortcut("⇧⌘Z").expect("redo");
    assert_eq!(redo.key(), Code::KeyZ);
    assert!(redo.modifiers().contains(Modifiers::SUPER));
    assert!(redo.modifiers().contains(Modifiers::SHIFT));

    let save_all = parse_shortcut("⌥⌘S").expect("save all");
    assert_eq!(save_all.key(), Code::KeyS);
    assert!(save_all.modifiers().contains(Modifiers::SUPER));
    assert!(save_all.modifiers().contains(Modifiers::ALT));

    let reload = parse_shortcut("⌘R").expect("reload");
    assert_eq!(reload.key(), Code::KeyR);
    assert!(reload.modifiers().contains(Modifiers::SUPER));
}

#[test]
fn parse_shortcut_rejects_unknown_keys() {
    assert!(parse_shortcut("⌘Q").is_none());
    assert!(parse_shortcut("").is_none());
}

#[test]
fn window_menu_registers_as_system_windows_menu() {
    // 回归：窗口菜单必须在 init_for_nsapp 后调用 set_as_windows_menu_for_nsapp，
    // 否则 macOS 不会注入「填充 / 居中 / 移动与调整大小 / 全屏幕平铺」。
    let src = include_str!("macos.rs");
    assert!(src.contains("set_as_windows_menu_for_nsapp"));
    assert!(src.contains("set_as_help_menu_for_nsapp"));
    assert!(src.contains("bring_all_to_front"));
}
