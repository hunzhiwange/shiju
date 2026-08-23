// 验证命令权限前缀按已知参数数量截断，并保留未知命令的渐进匹配。
use super::*;

#[test]
fn prefix_keeps_progressive_command_parts() {
    assert_eq!(
        prefix(&["cargo", "clippy", "--all-targets"]),
        vec!["cargo", "cargo clippy", "cargo clippy --all-targets"]
    );
}

#[test]
fn prefix_handles_empty_input() {
    assert!(prefix(&[]).is_empty());
}

#[test]
fn prefix_stops_when_known_arity_is_reached() {
    assert_eq!(prefix(&["ls", "-la"]), vec!["ls"]);
}

#[test]
fn prefix_continues_progressive_parts_for_higher_arity_parent_commands() {
    assert_eq!(
        prefix(&["docker", "compose", "up", "-d"]),
        vec!["docker", "docker compose", "docker compose up", "docker compose up -d"]
    );
    assert_eq!(
        prefix(&["npm", "run", "build", "--", "--prod"]),
        vec!["npm", "npm run", "npm run build", "npm run build --", "npm run build -- --prod"]
    );
}

#[test]
fn prefix_returns_all_progressive_parts_for_unknown_command() {
    assert_eq!(
        prefix(&["unknown-tool", "sub", "arg"]),
        vec!["unknown-tool", "unknown-tool sub", "unknown-tool sub arg"]
    );
}

#[test]
fn prefix_with_single_token_arity_one_breaks_immediately() {
    assert_eq!(prefix(&["ls"]), vec!["ls"]);
}

#[test]
fn prefix_with_single_token_arity_two_returns_one_part() {
    assert_eq!(prefix(&["cargo"]), vec!["cargo"]);
}

#[test]
fn prefix_with_known_parent_and_unknown_subcommand_continues_through_all_tokens() {
    assert_eq!(
        prefix(&["npm", "install", "foo", "--save"]),
        vec!["npm", "npm install", "npm install foo", "npm install foo --save"]
    );
}

#[test]
fn prefix_with_long_unknown_chain_returns_all_progressive_parts() {
    assert_eq!(
        prefix(&["a", "b", "c", "d", "e"]),
        vec!["a", "a b", "a b c", "a b c d", "a b c d e"]
    );
}
