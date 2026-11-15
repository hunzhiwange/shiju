//! A single send budget for one logical application request.

use std::{future::Future, time::Duration};
use tokio::time::{Instant, sleep_until};

const MAX_RETRIES: u8 = 10;
// Match the existing session recovery cap while keeping its budget independent.
const MAX_BACKOFF: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePhase {
    Connect,
    RequestSend,
    ResponseHeaders,
    ResponseBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureCode {
    Connection,
    HttpStatus(u16),
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayEvidence {
    /// Trusted transport or protocol evidence proves replay is safe.
    ProvenSafe,
    Unknown,
    Committed,
}

/// Redacted failure metadata. Status codes are independent of replay evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryFailure {
    pub phase: FailurePhase,
    pub code: FailureCode,
    pub replay: ReplayEvidence,
}

impl RetryFailure {
    fn permits_retry(self) -> bool {
        self.replay == ReplayEvidence::ProvenSafe
            && matches!(
                self.code,
                FailureCode::Connection | FailureCode::HttpStatus(429 | 502..=504)
            )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryStop {
    InvalidMaxRetries,
    Cancelled,
    DeadlineExceeded,
    Exhausted,
    InvalidAttemptSequence,
    Rejected(RetryFailure),
}

pub struct RetryBudget {
    attempts: u8,
    max_attempts: u8,
    initial_backoff: Duration,
    deadline: Instant,
    stopped: Option<RetryStop>,
}

impl RetryBudget {
    pub fn new(
        max_retries: u8,
        initial_backoff: Duration,
        deadline: Instant,
    ) -> Result<Self, RetryStop> {
        if max_retries > MAX_RETRIES {
            return Err(RetryStop::InvalidMaxRetries);
        }
        Ok(Self {
            attempts: 0,
            max_attempts: max_retries + 1,
            initial_backoff,
            deadline,
            stopped: None,
        })
    }

    /// Admit a send after candidate and authorization preflight. The first send
    /// has no failure; every subsequent send requires the preceding failure,
    /// even when the next candidate is a different provider. Send immediately
    /// after admission; the caller still owns transport cancellation and the
    /// same deadline for the in-flight request.
    pub async fn begin_attempt(
        &mut self,
        failure: Option<RetryFailure>,
        cancelled: impl Future<Output = ()>,
    ) -> Result<(), RetryStop> {
        if let Some(stop) = self.stopped {
            return Err(stop);
        }
        let ready = async {
            let now = Instant::now();
            let delay = self.next_delay(failure, now)?;
            if !delay.is_zero() {
                sleep_until(now + delay).await;
            }
            if Instant::now() >= self.deadline {
                return Err(RetryStop::DeadlineExceeded);
            }
            Ok(())
        };
        let result = tokio::select! {
            biased;
            () = cancelled => Err(RetryStop::Cancelled),
            result = ready => result,
        };
        match result {
            Ok(()) => {
                self.attempts += 1;
                Ok(())
            }
            Err(stop) => {
                self.stopped = Some(stop);
                Err(stop)
            }
        }
    }

    fn next_delay(
        &self,
        failure: Option<RetryFailure>,
        now: Instant,
    ) -> Result<Duration, RetryStop> {
        let remaining = self.deadline.saturating_duration_since(now);
        if remaining.is_zero() {
            return Err(RetryStop::DeadlineExceeded);
        }
        let failure = match (self.attempts, failure) {
            (0, None) => return Ok(Duration::ZERO),
            (0, Some(_)) | (_, None) => return Err(RetryStop::InvalidAttemptSequence),
            (_, Some(failure)) => failure,
        };
        if !failure.permits_retry() {
            return Err(RetryStop::Rejected(failure));
        }
        if self.attempts >= self.max_attempts {
            return Err(RetryStop::Exhausted);
        }
        let delay = self.initial_backoff.saturating_mul(1 << (self.attempts - 1)).min(MAX_BACKOFF);
        if delay >= remaining {
            return Err(RetryStop::DeadlineExceeded);
        }
        Ok(delay)
    }
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod retry_tests;
