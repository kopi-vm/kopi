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

//! Observer interfaces for lock wait instrumentation.
//!
//! Lock wait observers decouple the `LockController` from user-facing feedback
//! so commands can surface contention information without duplicating polling
//! logic.

use crate::indicator::StatusReporter;
use crate::locking::scope::LockScope;
use crate::locking::timeout::{LockTimeoutSource, LockTimeoutValue};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Observer hooks for lock wait events.
pub trait LockWaitObserver: Send + Sync {
    fn on_wait_start(&self, _scope: &LockScope, _timeout: LockTimeoutValue) {}

    fn on_retry(
        &self,
        _scope: &LockScope,
        _attempt: usize,
        _elapsed: Duration,
        _remaining: Option<Duration>,
    ) {
    }

    fn on_acquired(&self, _scope: &LockScope, _waited: Duration) {}

    fn on_timeout(&self, _scope: &LockScope, _waited: Duration) {}

    fn on_cancelled(&self, _scope: &LockScope, _waited: Duration) {}
}

/// Observer implementation that performs no work.
#[derive(Debug, Default)]
pub struct NoopLockWaitObserver;

impl LockWaitObserver for NoopLockWaitObserver {}

/// Sink used by [`StatusReporterObserver`] to surface wait-state progress.
pub trait LockStatusSink: Send + Sync {
    fn step(&self, message: &str);
    fn success(&self, message: &str);
    fn error(&self, message: &str);
}

impl LockStatusSink for StatusReporter {
    fn step(&self, message: &str) {
        StatusReporter::step(self, message);
    }

    fn success(&self, message: &str) {
        StatusReporter::success(self, message);
    }

    fn error(&self, message: &str) {
        StatusReporter::error(self, message);
    }
}

/// Bridges lock wait events to a [`StatusReporter`] instance.
pub struct StatusReporterObserver<'a> {
    reporter: &'a dyn LockStatusSink,
    source: LockTimeoutSource,
    notified_contention: AtomicBool,
}

impl<'a> StatusReporterObserver<'a> {
    pub fn new(reporter: &'a dyn LockStatusSink, source: LockTimeoutSource) -> Self {
        Self {
            reporter,
            source,
            notified_contention: AtomicBool::new(false),
        }
    }
}

impl<'a> LockWaitObserver for StatusReporterObserver<'a> {
    fn on_wait_start(&self, scope: &LockScope, timeout: LockTimeoutValue) {
        let scope_label = scope.label();
        let timeout_label = timeout.to_string();
        let source_label = self.source.to_string();
        self.reporter.step(&format!(
            "Waiting for {scope_label} lock (timeout {timeout_label}, source {source_label})"
        ));
    }

    fn on_retry(
        &self,
        scope: &LockScope,
        attempt: usize,
        elapsed: Duration,
        remaining: Option<Duration>,
    ) {
        if !self.notified_contention.swap(true, Ordering::Relaxed) {
            let scope_label = scope.label();
            let waited = format_duration(elapsed);
            let remaining_text = remaining
                .map(format_duration)
                .map(|value| format!(" (~{value} remaining)"))
                .unwrap_or_default();
            self.reporter.step(&format!(
                "Lock contention detected for {scope_label}, waited {waited}{remaining_text}"
            ));
        } else if attempt % 10 == 0 {
            let scope_label = scope.label();
            let waited = format_duration(elapsed);
            self.reporter.step(&format!(
                "Still waiting for {scope_label} lock after {waited} (attempt {attempt})"
            ));
        }
    }

    fn on_acquired(&self, scope: &LockScope, waited: Duration) {
        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        self.reporter
            .success(&format!("Acquired {scope_label} lock after {waited_label}"));
    }

    fn on_timeout(&self, scope: &LockScope, waited: Duration) {
        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        self.reporter.error(&format!(
            "Timed out waiting for {scope_label} lock after {waited_label}"
        ));
    }

    fn on_cancelled(&self, scope: &LockScope, waited: Duration) {
        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        self.reporter.error(&format!(
            "Cancelled while waiting for {scope_label} lock after {waited_label}"
        ));
    }
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() >= 1 {
        format!("{:.1}s", duration.as_secs_f32())
    } else {
        format!("{:.0}ms", duration.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locking::timeout::LockTimeoutValue;
    use std::sync::Mutex;
    use std::time::Duration;

    #[test]
    fn reporter_observer_emits_progress_messages() {
        let sink = RecordingSink::default();
        let observer = StatusReporterObserver::new(&sink, LockTimeoutSource::Cli);
        let scope = LockScope::CacheWriter;

        observer.on_wait_start(&scope, LockTimeoutValue::from_secs(30));
        observer.on_retry(
            &scope,
            1,
            Duration::from_millis(120),
            Some(Duration::from_secs(10)),
        );
        observer.on_acquired(&scope, Duration::from_millis(250));

        let output = sink.messages();
        assert!(
            output
                .iter()
                .any(|line| line.contains("Waiting for cache writer lock"))
        );
        assert!(
            output
                .iter()
                .any(|line| line.contains("Lock contention detected for cache writer"))
        );
        assert!(
            output
                .iter()
                .any(|line| line.contains("Acquired cache writer lock after"))
        );
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<String>>,
    }

    impl RecordingSink {
        fn messages(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl LockStatusSink for RecordingSink {
        fn step(&self, message: &str) {
            self.events.lock().unwrap().push(message.to_string());
        }

        fn success(&self, message: &str) {
            self.events.lock().unwrap().push(message.to_string());
        }

        fn error(&self, message: &str) {
            self.events.lock().unwrap().push(message.to_string());
        }
    }
}
