use super::*;

#[test]
fn exhausted_and_invalid_state_never_grant_a_call() {
    let directory = std::env::temp_dir().join(format!("vw-budget-test-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let state = directory.join("state");
    File::create(state.with_extension("lock")).unwrap();
    std::fs::write(&state, "attempt test\nmodels 24\ntools 32\n").unwrap();
    assert!(claim(&state, "model", "test").is_err());
    assert!(claim(&state, "tool", "test").is_err());
    std::fs::write(&state, "corrupt").unwrap();
    assert!(claim(&state, "tool", "test").is_err());
    std::fs::remove_dir_all(directory).unwrap();
}
