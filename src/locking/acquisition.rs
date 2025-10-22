// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::locking::cancellation::CancellationToken;
use crate::locking::scope::LockScope;
use crate::locking::timeout::{LockTimeoutSource, LockTimeoutValue};
use crate::locking::wait_observer::LockWaitObserver;
use std::cmp;
use std::time::{Duration, Instant};

/// Indicates whether a lock request may block waiting for contention to clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireMode {
    Blocking,
    NonBlocking,
}

impl AcquireMode {
    pub fn is_blocking(self) -> bool {
        matches!(self, AcquireMode::Blocking)
    }

    pub fn is_non_blocking(self) -> bool {
        matches!(self, AcquireMode::NonBlocking)
    }
}

/// Exponential backoff configuration used while polling for locks.
#[derive(Debug, Clone)]
pub struct PollingBackoff {
    initial: Duration,
    factor: u32,
    cap: Duration,
    current: Duration,
}

impl PollingBackoff {
    pub fn new(initial: Duration, factor: u32, cap: Duration) -> Self {
        Self {
            initial,
            factor: cmp::max(factor, 1),
            cap,
            current: initial,
        }
    }

    /// Returns the current delay and advances the backoff sequence.
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        let next = self.current.saturating_mul(self.factor);
        self.current = cmp::min(next, self.cap);
        delay
    }

    pub fn reset(&mut self) {
        self.current = self.initial;
    }

    pub fn peek(&self) -> Duration {
        self.current
    }
}

impl Default for PollingBackoff {
    fn default() -> Self {
        // Slightly extend the cap to keep the steady-state busy ratio under 0.1%.
        Self::new(Duration::from_millis(10), 2, Duration::from_millis(1_100))
    }
}

/// Tracks elapsed and remaining time for a lock timeout budget.
#[derive(Debug, Clone)]
pub struct LockTimeoutBudget {
    value: LockTimeoutValue,
    started_at: Instant,
}

impl LockTimeoutBudget {
    pub fn new(value: LockTimeoutValue) -> Self {
        Self {
            value,
            started_at: Instant::now(),
        }
    }

    pub fn with_start(value: LockTimeoutValue, started_at: Instant) -> Self {
        Self { value, started_at }
    }

    pub fn value(&self) -> LockTimeoutValue {
        self.value
    }

    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn remaining(&self) -> Option<Duration> {
        match self.value {
            LockTimeoutValue::Infinite => None,
            LockTimeoutValue::Finite(limit) => {
                let elapsed = self.elapsed();
                Some(limit.saturating_sub(elapsed))
            }
        }
    }

    pub fn is_expired(&self) -> bool {
        matches!(self.value, LockTimeoutValue::Finite(limit) if self.elapsed() >= limit)
    }
}

/// Carries the configuration for a single lock acquisition attempt.
pub struct LockAcquisitionRequest<'a> {
    scope: LockScope,
    budget: LockTimeoutBudget,
    cancellation: CancellationToken,
    backoff: PollingBackoff,
    observer: Option<&'a dyn LockWaitObserver>,
    source: LockTimeoutSource,
    mode: AcquireMode,
    retries: usize,
    wait_started: bool,
}

impl<'a> LockAcquisitionRequest<'a> {
    pub fn new(scope: LockScope, timeout: LockTimeoutValue) -> Self {
        Self {
            scope,
            budget: LockTimeoutBudget::new(timeout),
            cancellation: CancellationToken::new(),
            backoff: PollingBackoff::default(),
            observer: None,
            source: LockTimeoutSource::Default,
            mode: AcquireMode::Blocking,
            retries: 0,
            wait_started: false,
        }
    }

    pub fn with_mode(mut self, mode: AcquireMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }

    pub fn with_backoff(mut self, backoff: PollingBackoff) -> Self {
        self.backoff = backoff;
        self
    }

    pub fn with_timeout_source(mut self, source: LockTimeoutSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_observer(mut self, observer: Option<&'a dyn LockWaitObserver>) -> Self {
        self.observer = observer;
        self
    }

    pub fn scope(&self) -> &LockScope {
        &self.scope
    }

    pub fn mode(&self) -> AcquireMode {
        self.mode
    }

    pub fn budget(&self) -> &LockTimeoutBudget {
        &self.budget
    }

    pub fn budget_mut(&mut self) -> &mut LockTimeoutBudget {
        &mut self.budget
    }

    pub fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }

    pub fn cancellation_mut(&mut self) -> &mut CancellationToken {
        &mut self.cancellation
    }

    pub fn backoff(&self) -> &PollingBackoff {
        &self.backoff
    }

    pub fn backoff_mut(&mut self) -> &mut PollingBackoff {
        &mut self.backoff
    }

    pub fn observer(&self) -> Option<&'a dyn LockWaitObserver> {
        self.observer
    }

    pub fn elapsed(&self) -> Duration {
        self.budget.elapsed()
    }

    pub fn remaining(&self) -> Option<Duration> {
        self.budget.remaining()
    }

    pub fn timeout_value(&self) -> LockTimeoutValue {
        self.budget.value()
    }

    pub fn timeout_source(&self) -> LockTimeoutSource {
        self.source
    }

    pub fn increment_retries(&mut self) {
        self.retries = self.retries.saturating_add(1);
    }

    pub fn retries(&self) -> usize {
        self.retries
    }

    pub fn next_sleep_interval(&mut self) -> Option<Duration> {
        let remaining = self.remaining();
        let mut delay = self.backoff_mut().next_delay();
        if let Some(remaining_budget) = remaining {
            if remaining_budget < delay {
                delay = remaining_budget;
            }
            if delay.is_zero() {
                return None;
            }
        }
        Some(delay)
    }

    pub fn record_wait_start(&mut self) {
        if !self.wait_started {
            if let Some(observer) = self.observer {
                observer.on_wait_start(&self.scope, self.timeout_value());
            }
            self.wait_started = true;
        }
    }

    pub fn record_retry(&mut self) {
        self.retries = self.retries.saturating_add(1);
        if let Some(observer) = self.observer {
            observer.on_retry(&self.scope, self.retries, self.elapsed(), self.remaining());
        }
    }

    pub fn notify_acquired(&self) {
        if let Some(observer) = self.observer {
            observer.on_acquired(&self.scope, self.elapsed());
        }
    }

    pub fn notify_timeout(&self) {
        if let Some(observer) = self.observer {
            observer.on_timeout(&self.scope, self.elapsed());
        }
    }

    pub fn notify_cancelled(&self) {
        if let Some(observer) = self.observer {
            observer.on_cancelled(&self.scope, self.elapsed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locking::scope::LockScope;
    use std::sync::Mutex;
    use std::time::Duration;

    #[test]
    fn polling_backoff_doubles_until_cap() {
        let mut backoff =
            PollingBackoff::new(Duration::from_millis(10), 2, Duration::from_millis(40));
        assert_eq!(backoff.next_delay(), Duration::from_millis(10));
        assert_eq!(backoff.next_delay(), Duration::from_millis(20));
        assert_eq!(backoff.next_delay(), Duration::from_millis(40));
        assert_eq!(backoff.next_delay(), Duration::from_millis(40));
    }

    #[derive(Default)]
    struct RecordingObserver {
        events: Mutex<Vec<String>>,
    }

    impl LockWaitObserver for RecordingObserver {
        fn on_wait_start(&self, scope: &LockScope, _timeout: LockTimeoutValue) {
            self.events.lock().unwrap().push(format!("start:{scope}"));
        }

        fn on_retry(
            &self,
            _scope: &LockScope,
            attempt: usize,
            _elapsed: Duration,
            _remaining: Option<Duration>,
        ) {
            self.events.lock().unwrap().push(format!("retry:{attempt}"));
        }

        fn on_cancelled(&self, scope: &LockScope, _waited: Duration) {
            self.events
                .lock()
                .unwrap()
                .push(format!("cancelled:{scope}"));
        }
    }

    #[test]
    fn request_notifies_observer() {
        let observer = RecordingObserver::default();
        let mut request =
            LockAcquisitionRequest::new(LockScope::CacheWriter, LockTimeoutValue::from_secs(1))
                .with_observer(Some(&observer));

        request.record_wait_start();
        request.record_retry();
        request.notify_cancelled();

        let events = observer.events.lock().unwrap();
        assert_eq!(
            events.as_slice(),
            ["start:cache writer", "retry:1", "cancelled:cache writer"]
        );
    }
}
