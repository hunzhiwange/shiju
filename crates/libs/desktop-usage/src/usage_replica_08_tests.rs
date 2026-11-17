use super::*;

fn price(value: &str) -> ModelPricingInfo {
    ModelPricingInfo {
        model_id: "example-model".into(), display_name: "示例".into(),
        input_cost_per_million: value.into(), output_cost_per_million: "0".into(),
        cache_read_cost_per_million: "0".into(), cache_creation_cost_per_million: "0".into(),
    }
}

#[test]
fn usage_replica_08_decimal_prices() {
    for value in ["0", "0.075", "4.00", "123456789.123456789"] { assert!(validate(&price(value)).is_ok()); }
    for value in ["-1", "NaN", "1e3", "", "1..2", " 1"] { assert!(validate(&price(value)).is_err()); }
}

#[test]
fn usage_replica_08_invalid_save_retains_editor() {
    let mut state = State::default();
    state.editor = Some(price("-1"));
    let _ = update(&mut state, None, true, Err("offline".into()), Event::Save);
    assert_eq!(state.editor.as_ref().unwrap().input_cost_per_million, "-1");
    assert!(state.error.is_some());
    assert!(!state.busy);
}

#[test]
fn usage_replica_08_delete_requires_matching_confirmation() {
    let mut state = State::default();
    state.pending_delete = Some("one".into());
    let _ = update(&mut state, None, true, Err("offline".into()), Event::ConfirmDelete("two".into()));
    assert_eq!(state.pending_delete.as_deref(), Some("one"));
    assert!(!state.busy);
}

fn identity() -> Identity {
    Identity { endpoint: desktop_settings_account::types::Scope { gateway: "http://example.invalid".into(), tenant: Default::default() }, project: "project".into(), host: "remote".into() }
}

#[test]
fn usage_replica_08_failed_write_retains_original_and_draft() {
    let id = identity();
    let mut state = State::default();
    state.open = true;
    state.scope = Some(id.clone());
    state.busy = true;
    state.prices.push(StoredModelPricing { price: price("1"), source: "fixture".into(), is_custom: true });
    state.editor = Some(price("2"));
    state.pending_delete = Some("example-model".into());
    let _ = update(&mut state, Some(id.clone()), true, Err("offline".into()), Event::Written(0, id, Err("write failed".into())));
    assert_eq!(state.prices[0].price.input_cost_per_million, "1");
    assert_eq!(state.editor.as_ref().unwrap().input_cost_per_million, "2");
    assert_eq!(state.pending_delete.as_deref(), Some("example-model"));
    assert!(!state.busy);
}
