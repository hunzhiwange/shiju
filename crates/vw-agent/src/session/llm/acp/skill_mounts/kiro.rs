//! kiro directory contract awaits its dedicated adapter acceptance.
pub(super) fn rejection() -> &'static str {
    "skill directory mounting is unverified; adapter acceptance required"
}
#[cfg(test)]
#[path = "kiro_tests.rs"]
mod tests;
