//! 终端状态测试，覆盖平台 Shell、UTF-8 安全截断、标签管理和尺寸恢复。
//! 尾部保留按字节限制时不能留下残缺字符，未知终端操作应保持无副作用。

#[test]
fn shell_display_and_all_are_stable() {
    let shells = crate::Shell::all();
    assert_eq!(crate::Shell::default(), crate::Shell::Bash);
    assert_eq!(<&'static str>::from(crate::Shell::Bash), "/bin/bash");
    assert_eq!(<&'static str>::from(crate::Shell::Zsh), "/bin/zsh");
    assert_eq!(shells, [crate::Shell::Bash, crate::Shell::Zsh]);
    assert_eq!(shells[0].to_string(), "bash");
    assert_eq!(shells[1].to_string(), "zsh");
    assert_eq!(crate::TerminalTheme::System.to_string(), "跟随系统");
}
