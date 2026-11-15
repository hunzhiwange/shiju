//! Display-only USD conversion. Ledger amounts never pass through this service.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Duration};

pub const TARGET_CURRENCIES: [&str; 5] = ["CNY", "EUR", "JPY", "TWD", "TRY"];
pub const CACHE_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    #[default]
    USD,
    CNY,
    EUR,
    JPY,
    TWD,
    TRY,
}
impl Currency {
    pub fn code(self) -> &'static str {
        match self {
            Self::USD => "USD",
            Self::CNY => "CNY",
            Self::EUR => "EUR",
            Self::JPY => "JPY",
            Self::TWD => "TWD",
            Self::TRY => "TRY",
        }
    }
    fn fallback(self) -> f64 {
        match self {
            Self::USD => 1.0,
            Self::CNY => 6.8,
            Self::EUR => 0.87,
            Self::JPY => 158.0,
            Self::TWD => 32.0,
            Self::TRY => 48.0,
        }
    }
    fn symbol(self) -> &'static str {
        match self {
            Self::USD => "$",
            Self::CNY | Self::JPY => "¥",
            Self::EUR => "€",
            Self::TWD => "NT$",
            Self::TRY => "₺",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateMode {
    #[default]
    Manual,
    Auto,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExchangeSettings {
    pub currency: Currency,
    pub mode: RateMode,
    pub manual_rates: BTreeMap<String, f64>,
    pub cache: Option<CachedRates>,
    /// Fixed diagnostic; never contains upstream response bodies or URLs.
    pub refresh_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRates {
    pub rates: BTreeMap<String, f64>,
    pub fetched_at: i64,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyDisplay {
    pub currency: Currency,
    pub rate: f64,
    pub source: String,
    pub stale: bool,
    pub degraded: bool,
    pub error: Option<String>,
}
fn valid(rate: f64) -> bool {
    rate.is_finite() && rate > 0.0
}
fn pick(raw: impl IntoIterator<Item = (String, f64)>) -> BTreeMap<String, f64> {
    raw.into_iter()
        .filter_map(|(code, rate)| {
            let code = code.to_ascii_uppercase();
            (TARGET_CURRENCIES.contains(&code.as_str()) && valid(rate)).then_some((code, rate))
        })
        .collect()
}
pub fn parse_open_er_api(body: &str) -> Option<BTreeMap<String, f64>> {
    #[derive(Deserialize)]
    struct Response {
        result: String,
        rates: BTreeMap<String, f64>,
        base_code: Option<String>,
    }
    let value: Response = serde_json::from_str(body).ok()?;
    if value.result != "success" || value.base_code.as_deref().is_some_and(|base| base != "USD") {
        return None;
    }
    let rates = pick(value.rates);
    (!rates.is_empty()).then_some(rates)
}
pub fn parse_floatrates(body: &str) -> Option<BTreeMap<String, f64>> {
    let value: BTreeMap<String, serde_json::Value> = serde_json::from_str(body).ok()?;
    let rates = pick(value.into_iter().filter_map(|(code, entry)| {
        let raw = entry.get("rate")?;
        Some((code, raw.as_f64().or_else(|| raw.as_str()?.trim().parse().ok())?))
    }));
    (!rates.is_empty()).then_some(rates)
}
impl ExchangeSettings {
    pub fn needs_refresh(&self, now_ms: i64) -> bool {
        self.mode == RateMode::Auto
            && self.cache.as_ref().is_none_or(|cache| {
                cache.fetched_at <= 0
                    || now_ms < cache.fetched_at
                    || now_ms.saturating_sub(cache.fetched_at) >= CACHE_MAX_AGE_MS
            })
    }
    pub fn resolve(&self, now_ms: i64) -> MoneyDisplay {
        let mut display = MoneyDisplay {
            currency: self.currency,
            rate: 1.0,
            source: "USD".into(),
            stale: false,
            degraded: false,
            error: None,
        };
        if self.currency == Currency::USD {
            return display;
        }
        let code = self.currency.code();
        if self.mode == RateMode::Auto {
            display.error = self.refresh_error.clone();
            if let Some(cache) = &self.cache {
                if let Some(rate) = cache.rates.get(code).copied().filter(|rate| valid(*rate)) {
                    display.rate = rate;
                    display.source = cache.source.clone();
                    display.stale = self.needs_refresh(now_ms);
                    display.degraded = display.stale
                        || display.error.is_some()
                        || cache.source == "floatrates.com";
                    return display;
                }
            }
            display.degraded = true;
        }
        if let Some(rate) = self.manual_rates.get(code).copied().filter(|rate| valid(*rate)) {
            display.rate = rate;
            display.source = "manual".into();
        } else {
            display.rate = self.currency.fallback();
            display.source = "default".into();
            display.degraded = true;
        }
        display
    }
}
/// Uses fixed sources only. The host supplies its configured HTTP client.
pub async fn fetch_usd_rates(client: &reqwest::Client, now_ms: i64) -> Result<CachedRates, String> {
    if now_ms <= 0 {
        return Err("Invalid exchange-rate timestamp".into());
    }
    type Parser = fn(&str) -> Option<BTreeMap<String, f64>>;
    let sources: [(&str, &str, Parser); 2] = [
        ("open.er-api.com", "https://open.er-api.com/v6/latest/USD", parse_open_er_api),
        ("floatrates.com", "https://www.floatrates.com/daily/usd.json", parse_floatrates),
    ];
    for (source, url, parser) in sources {
        let Ok(response) = client.get(url).timeout(REQUEST_TIMEOUT).send().await else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let Ok(body) = response.text().await else {
            continue;
        };
        if let Some(rates) = parser(&body) {
            return Ok(CachedRates { rates, fetched_at: now_ms, source: source.into() });
        }
    }
    Err("Exchange-rate sources unavailable".into())
}
/// Formatting consumes an amount by value and cannot mutate the USD ledger.
pub fn format_money(usd: f64, digits: usize, display: &MoneyDisplay) -> String {
    let value = usd * display.rate;
    if !usd.is_finite() || !valid(display.rate) || !value.is_finite() {
        return "--".into();
    }
    let digits = digits.min(12);
    let shown = if display.currency == Currency::USD {
        digits
    } else {
        digits.saturating_sub(if display.currency == Currency::JPY { 2 } else { 0 }).min(2)
    };
    let fixed = format!("{value:.shown$}");
    if shown < digits && value != 0.0 && fixed.parse::<f64>().ok() == Some(0.0) {
        let precision = (1.0 - value.abs().log10().floor()).max(0.0).min(324.0) as usize;
        return format!("{}{value:.precision$}", display.currency.symbol());
    }
    let (whole, fraction) =
        fixed.split_once('.').map_or((fixed.as_str(), None), |(w, f)| (w, Some(f)));
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && ch != '-' && (whole.len() - i).is_multiple_of(3) && !grouped.ends_with('-') {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if let Some(fraction) = fraction {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    format!("{}{grouped}", display.currency.symbol())
}
