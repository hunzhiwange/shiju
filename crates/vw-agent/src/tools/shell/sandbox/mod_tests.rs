//! 验证沙箱执行前对受支持命令包装器及其选项的剥离。

use super::strip_wrapper_tokens;

fn tokens(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn wrapper_stripping_handles_remaining_supported_option_forms() {
    let cases = [
        (tokens(&["env", "-i", "--", "rg", "needle"]), tokens(&["rg", "needle"])),
        (
            tokens(&["timeout", "--signal", "TERM", "--kill-after", "2s", "--", "rg", "needle"]),
            tokens(&["rg", "needle"]),
        ),
        (tokens(&["nice", "--adjustment", "7", "rg"]), tokens(&["rg"])),
        (tokens(&["zsh", "-f", "--", "rg"]), tokens(&["rg"])),
        (tokens(&["time", "-p", "--", "rg"]), tokens(&["rg"])),
    ];

    for (input, expected) in cases {
        assert_eq!(strip_wrapper_tokens(input), expected);
    }
}
