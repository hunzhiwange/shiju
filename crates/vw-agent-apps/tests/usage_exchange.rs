use std::collections::BTreeMap;
use vw_agent_apps::usage::exchange::{
    CACHE_MAX_AGE_MS, CachedRates, Currency, ExchangeSettings, RateMode, format_money,
    parse_floatrates, parse_open_er_api,
};

#[test]
fn usage17_exchange_failed_primary_uses_cached_or_fallback_rate() {
    let mut settings = ExchangeSettings {
        currency: Currency::CNY,
        mode: RateMode::Auto,
        manual_rates: BTreeMap::from([("CNY".into(), 7.0)]),
        cache: Some(CachedRates {
            rates: BTreeMap::from([("CNY".into(), 6.9)]),
            fetched_at: 1,
            source: "floatrates.com".into(),
        }),
        refresh_error: Some("Exchange-rate sources unavailable".into()),
    };
    let display = settings.resolve(CACHE_MAX_AGE_MS + 1);
    assert_eq!(display.rate, 6.9);
    assert!(display.stale);
    assert!(display.degraded);
    assert!(display.error.is_some());
    settings.cache = None;
    assert_eq!(settings.resolve(2).rate, 7.0);
    settings.manual_rates.clear();
    assert_eq!(settings.resolve(2).rate, 6.8);
}

#[test]
fn usage17_exchange_parsers_reject_invalid_and_accept_both_source_shapes() {
    let rates = parse_open_er_api(
        r#"{"result":"success","base_code":"USD","rates":{"CNY":7,"EUR":0,"JPY":-1,"GBP":1}}"#,
    )
    .unwrap();
    assert_eq!(rates, BTreeMap::from([("CNY".into(), 7.0)]));
    assert!(parse_open_er_api(r#"{"result":"error","rates":{"CNY":7}}"#).is_none());
    assert!(
        parse_open_er_api(r#"{"result":"success","base_code":"EUR","rates":{"CNY":7}}"#).is_none()
    );
    let rates = parse_floatrates(
        r#"{"cny":{"rate":"7.1"},"jpy":{"rate":158},"try":{"rate":"NaN"},"eur":{"rate":"-1"}}"#,
    )
    .unwrap();
    assert_eq!(rates.len(), 2);
    assert_eq!(rates["CNY"], 7.1);
}

#[test]
fn usage17_exchange_manual_mode_never_requests_network_and_usd_stays_one() {
    let mut settings = ExchangeSettings::default();
    assert!(!settings.needs_refresh(i64::MAX));
    settings.mode = RateMode::Auto;
    assert!(settings.needs_refresh(1));
    assert_eq!(settings.resolve(1).rate, 1.0);
    for currency in [Currency::CNY, Currency::EUR, Currency::JPY, Currency::TWD, Currency::TRY] {
        settings.currency = currency;
        assert!(settings.resolve(1).rate > 0.0);
    }
    settings.currency = Currency::CNY;
    let usd = 0.0005;
    assert_ne!(format_money(usd, 4, &settings.resolve(1)), "¥0.00");
    assert_eq!(usd, 0.0005);
    assert_eq!(format_money(f64::NAN, 2, &settings.resolve(1)), "--");
}
