//! Opening thresholds for one provider's production circuit.

use std::time::Instant;
use vw_api_types::agent_apps_high_availability::{CircuitBreakerPolicy, FailoverHealthState};

/// Result of a production attempt, classified by the authorized routing caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutcome {
    Success,
    ProviderFailure,
    /// Cancellation, business rejection, or another non-provider outcome.
    Neutral,
}

#[derive(Debug, Default)]
pub struct CircuitBreaker {
    opened_at: Option<Instant>,
    consecutive_failures: u32,
    total_requests: u64,
    failed_requests: u64,
}

impl CircuitBreaker {
    pub fn state(&self) -> FailoverHealthState {
        if self.opened_at.is_some() {
            FailoverHealthState::Open
        } else {
            FailoverHealthState::Closed
        }
    }

    /// Record a result against the validated policy snapshot for the attempt.
    pub fn record(&mut self, outcome: AttemptOutcome, policy: &CircuitBreakerPolicy, now: Instant) {
        // Attempts already in flight cannot extend the open interval.
        if self.opened_at.is_some() {
            return;
        }
        match outcome {
            AttemptOutcome::Success => {
                self.consecutive_failures = 0;
                self.total_requests = self.total_requests.saturating_add(1);
            }
            AttemptOutcome::ProviderFailure => {
                self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                self.total_requests = self.total_requests.saturating_add(1);
                self.failed_requests = self.failed_requests.saturating_add(1);

                if self.consecutive_failures >= policy.failure_threshold
                    || (self.total_requests >= u64::from(policy.min_requests)
                        && self.failed_requests as f64 / self.total_requests as f64
                            >= policy.error_rate_threshold)
                {
                    self.opened_at = Some(now);
                }
            }
            AttemptOutcome::Neutral => {}
        }
    }
}

#[cfg(test)]
#[path = "circuit_breaker_tests.rs"]
mod circuit_breaker_tests;
