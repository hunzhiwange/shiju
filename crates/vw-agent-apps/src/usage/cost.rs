//! Decimal USD costs for reported usage. No model/provider inference occurs here.

use rust_decimal::Decimal;
use std::str::FromStr;
use vw_api_types::usage::{
    UsageCost, UsageCostSource, UsageInputSemantics, UsageReasoningSemantics, UsageTokens,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPricing {
    pub input_cost_per_million: Decimal,
    pub output_cost_per_million: Decimal,
    pub cache_read_cost_per_million: Decimal,
    /// Five-minute creation price; the one-hour price is 1.6 times this rate.
    pub cache_creation_cost_per_million: Decimal,
}

impl ModelPricing {
    pub fn from_strings(
        input: &str,
        output: &str,
        cache_read: &str,
        cache_creation: &str,
    ) -> Result<Self, String> {
        fn parse(value: &str) -> Result<Decimal, String> {
            let price = Decimal::from_str(value).map_err(|_| "Invalid decimal price".to_owned())?;
            if price < Decimal::ZERO {
                return Err("Usage prices must be nonnegative".into());
            }
            Ok(price)
        }
        Ok(Self {
            input_cost_per_million: parse(input)?,
            output_cost_per_million: parse(output)?,
            cache_read_cost_per_million: parse(cache_read)?,
            cache_creation_cost_per_million: parse(cache_creation)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostBreakdown {
    /// Components are base costs, before the provider multiplier.
    pub input_cost: Decimal,
    pub output_cost: Decimal,
    pub cache_read_cost: Decimal,
    pub cache_creation_cost: Decimal,
    pub total_cost: Decimal,
}

impl CostBreakdown {
    pub fn to_usage_cost(&self) -> UsageCost {
        UsageCost {
            source: UsageCostSource::Calculated,
            amount_usd: Some(self.total_cost.normalize().to_string()),
        }
    }
}

fn overflow() -> String {
    "Usage cost exceeds decimal range".into()
}

fn token_cost(tokens: u64, rate: Decimal) -> Result<Decimal, String> {
    Decimal::from(tokens)
        .checked_mul(rate)
        .and_then(|cost| cost.checked_div(Decimal::from(1_000_000)))
        .ok_or_else(overflow)
}

/// Missing prices, multiplier or primary counters yield unknown, never free.
/// Optional cache/reasoning counters charge only the quantities reported by the
/// source; the original counters remain unchanged, including their missingness.
/// Unknown semantics yield unknown when they affect the price. Callers must
/// preserve native costs and only use this result for calculated costs.
pub fn calculate(
    usage: &UsageTokens,
    pricing: Option<&ModelPricing>,
    multiplier: Option<Decimal>,
) -> Result<Option<CostBreakdown>, String> {
    let (Some(pricing), Some(multiplier)) = (pricing, multiplier) else {
        return Ok(None);
    };
    if multiplier < Decimal::ZERO
        || [
            pricing.input_cost_per_million,
            pricing.output_cost_per_million,
            pricing.cache_read_cost_per_million,
            pricing.cache_creation_cost_per_million,
        ]
        .into_iter()
        .any(|rate| rate < Decimal::ZERO)
    {
        return Err("Usage prices and multiplier must be nonnegative".into());
    }
    let (Some(input), Some(output)) = (usage.input, usage.output) else {
        return Ok(None);
    };
    let cache_read = usage.cache_read.unwrap_or(0);
    let cache_creation = usage.cache_creation.unwrap_or(0);
    if usage.cache_creation.is_none() && usage.cache_creation_1h.is_some_and(|n| n > 0) {
        return Ok(None);
    }
    let fresh_input = match usage.input_semantics {
        UsageInputSemantics::Fresh => input,
        UsageInputSemantics::Total => input.saturating_sub(cache_read).saturating_sub(cache_creation),
        UsageInputSemantics::Unknown if cache_read == 0 && cache_creation == 0 => input,
        UsageInputSemantics::Unknown => return Ok(None),
    };
    let reasoning = usage.reasoning.unwrap_or(0);
    let billable_output = match usage.reasoning_semantics {
        UsageReasoningSemantics::IncludedInOutput => Decimal::from(output),
        UsageReasoningSemantics::SeparateFromOutput => Decimal::from(output)
            .checked_add(Decimal::from(reasoning))
            .ok_or_else(overflow)?,
        UsageReasoningSemantics::Unknown if reasoning == 0 => Decimal::from(output),
        UsageReasoningSemantics::Unknown => return Ok(None),
    };
    // One-hour writes are a subset, never an additional creation bucket.
    let one_hour = usage.cache_creation_1h.unwrap_or(0).min(cache_creation);
    let weighted_creation = Decimal::from(one_hour)
        .checked_mul(Decimal::new(16, 1))
        .and_then(|tokens| tokens.checked_add(Decimal::from(cache_creation - one_hour)))
        .ok_or_else(overflow)?;
    let cache_creation_cost = weighted_creation
        .checked_mul(pricing.cache_creation_cost_per_million)
        .and_then(|cost| cost.checked_div(Decimal::from(1_000_000)))
        .ok_or_else(overflow)?;
    let input_cost = token_cost(fresh_input, pricing.input_cost_per_million)?;
    let output_cost = billable_output
        .checked_mul(pricing.output_cost_per_million)
        .and_then(|cost| cost.checked_div(Decimal::from(1_000_000)))
        .ok_or_else(overflow)?;
    let cache_read_cost = token_cost(cache_read, pricing.cache_read_cost_per_million)?;
    let total_cost = input_cost
        .checked_add(output_cost)
        .and_then(|cost| cost.checked_add(cache_read_cost))
        .and_then(|cost| cost.checked_add(cache_creation_cost))
        .and_then(|cost| cost.checked_mul(multiplier))
        .ok_or_else(overflow)?;
    Ok(Some(CostBreakdown {
        input_cost,
        output_cost,
        cache_read_cost,
        cache_creation_cost,
        total_cost,
    }))
}
