//! Opening and recovery rules for one provider's production circuit.

use super::health::HealthSnapshot;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
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
    state: Mutex<CircuitState>,
}

#[derive(Debug, Default)]
struct CircuitState {
    phase: CircuitPhase,
    generation: u64,
    consecutive_failures: u32,
    total_requests: u64,
    failed_requests: u64,
    observed_at: Option<Instant>,
}

#[derive(Debug, Default)]
enum CircuitPhase {
    #[default]
    Closed,
    Open {
        opened_at: Instant,
    },
    HalfOpen {
        successes: u32,
        probe_in_flight: bool,
    },
}

impl CircuitBreaker {
    pub fn state(&self) -> FailoverHealthState {
        self.state.lock().expect("circuit state lock poisoned").phase.health_state()
    }

    pub(super) fn snapshot(
        &self,
        policy: &CircuitBreakerPolicy,
        now: Instant,
        observation_ttl: Duration,
    ) -> HealthSnapshot {
        let state = self.state.lock().expect("circuit state lock poisoned");
        if !state.has_fresh_observation(now, observation_ttl) {
            return HealthSnapshot::UNKNOWN;
        }
        let retry_after_ms = match state.phase {
            CircuitPhase::Open { opened_at } => {
                let cooldown = Duration::from_secs(u64::from(policy.cooldown_secs));
                let remaining = cooldown.saturating_sub(now.saturating_duration_since(opened_at));
                Some(remaining.as_millis() as u64)
            }
            _ => None,
        };
        HealthSnapshot { state: state.phase.health_state(), retry_after_ms }
    }

    pub(super) fn reset(&self) {
        let mut state = self.state.lock().expect("circuit state lock poisoned");
        *state = CircuitState {
            generation: state.generation.wrapping_add(1),
            ..CircuitState::default()
        };
    }

    pub(super) fn is_evictable(&self, now: Instant, observation_ttl: Duration) -> bool {
        let state = self.state.lock().expect("circuit state lock poisoned");
        matches!(state.phase, CircuitPhase::Closed)
            && !state.has_fresh_observation(now, observation_ttl)
    }

    /// Admit a production attempt with the validated policy snapshot.
    ///
    /// Once the cooldown elapses, only one half-open attempt may be in flight.
    pub fn allow_request(
        &self,
        policy: &CircuitBreakerPolicy,
        now: Instant,
    ) -> Option<CircuitAttempt<'_>> {
        let mut state = self.state.lock().expect("circuit state lock poisoned");
        let is_probe = match &mut state.phase {
            CircuitPhase::Closed => false,
            CircuitPhase::Open { opened_at } => {
                if now.checked_duration_since(*opened_at)?
                    < Duration::from_secs(u64::from(policy.cooldown_secs))
                {
                    return None;
                }
                state.phase = CircuitPhase::HalfOpen { successes: 0, probe_in_flight: true };
                true
            }
            CircuitPhase::HalfOpen { probe_in_flight, .. } => {
                if *probe_in_flight {
                    return None;
                }
                *probe_in_flight = true;
                true
            }
        };
        Some(CircuitAttempt {
            breaker: self,
            generation: state.generation,
            policy: *policy,
            is_probe,
        })
    }

    /// Record a closed-circuit result when the caller has exclusive access.
    pub fn record(&mut self, outcome: AttemptOutcome, policy: &CircuitBreakerPolicy, now: Instant) {
        let state = self.state.get_mut().expect("circuit state lock poisoned");
        if matches!(state.phase, CircuitPhase::Closed) {
            state.record_closed(outcome, policy, now);
        }
    }
}

impl CircuitPhase {
    fn health_state(&self) -> FailoverHealthState {
        match self {
            Self::Closed => FailoverHealthState::Closed,
            Self::Open { .. } => FailoverHealthState::Open,
            Self::HalfOpen { .. } => FailoverHealthState::HalfOpen,
        }
    }
}

impl CircuitState {
    fn has_fresh_observation(&self, now: Instant, observation_ttl: Duration) -> bool {
        self.observed_at.is_some_and(|at| now.saturating_duration_since(at) < observation_ttl)
    }

    fn record_closed(
        &mut self,
        outcome: AttemptOutcome,
        policy: &CircuitBreakerPolicy,
        now: Instant,
    ) {
        if outcome != AttemptOutcome::Neutral {
            self.observed_at = self.observed_at.max(Some(now));
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
                    // Invalidate other attempts admitted before the circuit opened.
                    self.generation = self.generation.wrapping_add(1);
                    self.phase = CircuitPhase::Open { opened_at: now };
                }
            }
            AttemptOutcome::Neutral => {}
        }
    }
}

/// Admission for a single production attempt. Dropping it releases any probe slot.
#[derive(Debug)]
pub struct CircuitAttempt<'a> {
    breaker: &'a CircuitBreaker,
    generation: u64,
    policy: CircuitBreakerPolicy,
    is_probe: bool,
}

impl CircuitAttempt<'_> {
    /// Consume the admission and apply its outcome once.
    pub fn record(mut self, outcome: AttemptOutcome, now: Instant) {
        let was_probe = std::mem::take(&mut self.is_probe);
        let mut state = self.breaker.state.lock().expect("circuit state lock poisoned");
        if state.generation != self.generation {
            return;
        }
        match (&mut state.phase, was_probe) {
            (CircuitPhase::Closed, false) => state.record_closed(outcome, &self.policy, now),
            (CircuitPhase::HalfOpen { successes, probe_in_flight }, true) => {
                *probe_in_flight = false;
                match outcome {
                    AttemptOutcome::Success => {
                        *successes += 1;
                        let recovered = *successes >= self.policy.success_threshold;
                        state.observed_at = state.observed_at.max(Some(now));
                        if recovered {
                            *state = CircuitState {
                                generation: state.generation,
                                observed_at: state.observed_at,
                                ..CircuitState::default()
                            };
                        }
                    }
                    AttemptOutcome::ProviderFailure => {
                        state.observed_at = state.observed_at.max(Some(now));
                        state.phase = CircuitPhase::Open { opened_at: now };
                    }
                    AttemptOutcome::Neutral => {}
                }
            }
            _ => {}
        }
    }
}

impl Drop for CircuitAttempt<'_> {
    fn drop(&mut self) {
        if !self.is_probe {
            return;
        }
        let mut state = self.breaker.state.lock().expect("circuit state lock poisoned");
        if state.generation == self.generation
            && let CircuitPhase::HalfOpen { probe_in_flight, .. } = &mut state.phase
        {
            *probe_in_flight = false;
        }
    }
}

#[cfg(test)]
#[path = "circuit_breaker_tests.rs"]
mod circuit_breaker_tests;
