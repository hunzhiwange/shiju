use super::*;
use gateway_usage_price::{ExchangeSettings, MoneyDisplay};

fn state(currency: Currency, rate: f64) -> State {
    State {
        result: Some(UsageRatesState {
            settings: ExchangeSettings { currency, ..Default::default() },
            display: MoneyDisplay {
                currency, rate, source: "manual".into(), stale: false,
                degraded: false, error: None,
            },
        }),
        ..Default::default()
    }
}

#[test]
fn usage_replica_07_decimal_amount_and_unknown_rate() {
    assert_eq!(amount(&state(Currency::USD, 1.0), None, "0.123456"), "0.123456 USD");
    assert_eq!(amount(&state(Currency::CNY, 7.25), None, "0.1"), "≈ 0.7250 CNY（原始 0.1 USD）");
    assert_eq!(amount(&state(Currency::JPY, 0.0), None, "1"), "1 USD（汇率未知）");
    assert_eq!(amount(&state(Currency::EUR, 0.9), None, "unknown"), "unknown USD（换算不可用）");
}

#[test]
fn usage_replica_07_restore_saved_draft() {
    let mut state = state(Currency::CNY, 7.25);
    state.currency = Currency::EUR;
    state.mode = RateMode::Auto;
    state.rate = "2".into();
    state.restore();
    assert_eq!(state.currency, Currency::CNY);
    assert_eq!(state.mode, RateMode::Manual);
    assert_eq!(state.rate, "7.25");
    assert!(state.result.is_some());
}

#[test]
fn usage_replica_07_preferences_persist_and_limit() {
    let directory = std::env::temp_dir().join(format!("usage-replica-07-{}-{}", std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap()));
    let path = directory.join("display.json");
    let mut preferences = Preferences::default();
    preferences.area = true;
    preferences.toggle(TodayItem::Requests);
    preferences.toggle(TodayItem::Tokens);
    assert_eq!(preferences.today, vec![TodayItem::Cost, TodayItem::Requests]);
    preferences.write(&path).unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), preferences);
    preferences.toggle(TodayItem::Cost);
    preferences.toggle(TodayItem::Requests);
    preferences.write(&path).unwrap();
    assert!(Preferences::read(&path).unwrap().today.is_empty());
    // A failed write must leave the previously saved file intact.
    assert!(preferences.write(&path.join("invalid.json")).is_err());
    assert_eq!(Preferences::read(&path).unwrap(), preferences);
    std::fs::remove_dir_all(directory).unwrap();
}
