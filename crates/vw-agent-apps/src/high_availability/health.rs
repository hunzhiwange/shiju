//! Execution-host health for authorized production provider attempts.

use super::circuit_breaker::CircuitBreaker;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::{CircuitBreakerPolicy, FailoverHealthState},
    id::{ProjectScope, ProviderId},
};

/// Built from the verified route and provider resolved by the production caller.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct HealthKey {
    pub project: ProjectScope,
    pub target_host_id: String,
    pub app_id: AppId,
    pub provider_id: ProviderId,
    pub account: HealthAccount,
}

/// Credential-free identity of the account binding used for this attempt.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum HealthAccount {
    Unbound,
    Bound { id: String, generation: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthSnapshot {
    pub state: FailoverHealthState,
    pub retry_after_ms: Option<u64>,
}

impl HealthSnapshot {
    pub(super) const UNKNOWN: Self =
        Self { state: FailoverHealthState::Unknown, retry_after_ms: None };
}

/// Bounded, in-memory circuit ownership. A new execution-host instance starts
/// with no observations.
pub struct HealthRegistry {
    circuits: Mutex<HashMap<HealthKey, Arc<CircuitBreaker>>>,
    capacity: NonZeroUsize,
    observation_ttl: Duration,
}

impl HealthRegistry {
    pub fn new(capacity: NonZeroUsize, observation_ttl: Duration) -> Self {
        Self { circuits: Mutex::default(), capacity, observation_ttl }
    }

    /// Acquire the circuit for a verified current route before admitting an attempt.
    /// At capacity, a new key is rejected unless an idle, unobserved closed circuit
    /// can be reclaimed. Live and recovering circuits keep their admission state.
    pub fn circuit(&self, key: &HealthKey, now: Instant) -> Option<Arc<CircuitBreaker>> {
        let mut circuits = self.circuits.lock().expect("health registry lock poisoned");
        if let Some(circuit) = circuits.get(key) {
            return Some(Arc::clone(circuit));
        }
        if circuits.len() == self.capacity.get() {
            circuits.retain(|_, circuit| {
                Arc::strong_count(circuit) > 1 || !circuit.is_evictable(now, self.observation_ttl)
            });
            if circuits.len() == self.capacity.get() {
                return None;
            }
        }
        let circuit = Arc::new(CircuitBreaker::default());
        circuits.insert(key.clone(), Arc::clone(&circuit));
        Some(circuit)
    }

    /// Read recent production evidence without admitting a probe or creating a circuit.
    pub fn snapshot(
        &self,
        key: &HealthKey,
        policy: &CircuitBreakerPolicy,
        now: Instant,
    ) -> HealthSnapshot {
        self.circuits
            .lock()
            .expect("health registry lock poisoned")
            .get(key)
            .map_or(HealthSnapshot::UNKNOWN, |circuit| {
                circuit.snapshot(policy, now, self.observation_ttl)
            })
    }

    /// Reopen admission for exactly this key and discard its previous observations.
    /// In-flight results admitted before the reset are ignored.
    pub fn reset(&self, key: &HealthKey) -> bool {
        let circuits = self.circuits.lock().expect("health registry lock poisoned");
        let Some(circuit) = circuits.get(key) else {
            return false;
        };
        circuit.reset();
        true
    }

    /// Detach the circuit for a removed or replaced member. Outstanding attempts
    /// can only finish on the detached circuit, even if the key is used again.
    pub fn remove(&self, key: &HealthKey) -> bool {
        self.circuits.lock().expect("health registry lock poisoned").remove(key).is_some()
    }
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod health_tests;
