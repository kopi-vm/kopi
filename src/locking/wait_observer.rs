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

use crate::indicator::{
    ProgressConfig, ProgressIndicator, ProgressRendererKind, ProgressStyle, StatusReporter,
};
use crate::locking::scope::LockScope;
use crate::locking::timeout::{LockTimeoutSource, LockTimeoutValue};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// Feedback bridge that renders lock wait events using a progress indicator.
pub(crate) struct LockFeedbackBridge {
    progress: Arc<Mutex<Box<dyn ProgressIndicator>>>,
    renderer_kind: ProgressRendererKind,
    timeout_source: LockTimeoutSource,
    state: Mutex<BridgeState>,
}

struct BridgeState {
    started_at: Option<Instant>,
    timeout: Option<LockTimeoutValue>,
    last_emit: Option<Instant>,
    spinner_started: bool,
    progress_emitted: bool,
}

impl BridgeState {
    fn new() -> Self {
        Self {
            started_at: None,
            timeout: None,
            last_emit: None,
            spinner_started: false,
            progress_emitted: false,
        }
    }
}

impl LockFeedbackBridge {
    pub(crate) fn for_handle(
        progress: Arc<Mutex<Box<dyn ProgressIndicator>>>,
        timeout_source: LockTimeoutSource,
    ) -> Self {
        let renderer_kind = progress
            .lock()
            .ok()
            .map(|indicator| indicator.renderer_kind())
            .unwrap_or(ProgressRendererKind::NonTty);

        Self {
            progress,
            renderer_kind,
            timeout_source,
            state: Mutex::new(BridgeState::new()),
        }
    }

    fn ensure_spinner_started(&self, state: &mut BridgeState) {
        if state.spinner_started || self.renderer_kind != ProgressRendererKind::Tty {
            return;
        }

        if let Ok(mut indicator) = self.progress.lock() {
            indicator.start(ProgressConfig::new(ProgressStyle::Status));
            state.spinner_started = true;
        }
    }

    fn emit_tty_message(&self, message: &str) {
        if let Ok(mut indicator) = self.progress.lock() {
            indicator.set_message(message.to_string());
        }
    }

    fn emit_line(&self, message: &str) {
        match self.renderer_kind {
            ProgressRendererKind::Tty => self.emit_tty_message(message),
            ProgressRendererKind::NonTty => {
                if let Ok(indicator) = self.progress.lock() {
                    let _ = indicator.println(message);
                }
            }
            ProgressRendererKind::Silent => {}
        }
    }

    fn emit_success(&self, message: &str) {
        match self.renderer_kind {
            ProgressRendererKind::Tty => {
                if let Ok(mut indicator) = self.progress.lock() {
                    indicator.set_message(message.to_string());
                    indicator.complete(Some(message.to_string()));
                }
            }
            ProgressRendererKind::NonTty => {
                if let Ok(indicator) = self.progress.lock() {
                    let _ = indicator.success(message);
                }
            }
            ProgressRendererKind::Silent => {}
        }
    }

    fn emit_error(&self, message: &str) {
        match self.renderer_kind {
            ProgressRendererKind::Tty => {
                if let Ok(mut indicator) = self.progress.lock() {
                    indicator.set_message(message.to_string());
                    indicator.error(message.to_string());
                }
            }
            ProgressRendererKind::NonTty => {
                if let Ok(mut indicator) = self.progress.lock() {
                    indicator.error(message.to_string());
                }
            }
            ProgressRendererKind::Silent => {}
        }
    }

    fn initial_message(&self, scope: &LockScope, timeout: LockTimeoutValue) -> String {
        let scope_label = scope.label();
        let timeout_label = timeout.to_string();
        let source_label = self.timeout_source.to_string();
        format!(
            "Waiting for lock on {scope_label} (timeout: {timeout_label}, source {source_label}) — \
             Ctrl-C to cancel; override with --lock-timeout."
        )
    }

    fn progress_message(
        &self,
        scope: &LockScope,
        elapsed: Duration,
        remaining: Option<Duration>,
    ) -> String {
        let scope_label = scope.label();
        let waited = format_duration(elapsed);
        let remaining_text = remaining
            .map(format_duration)
            .map(|value| format!(" (~{value} remaining)"))
            .unwrap_or_default();
        format!("Waiting for lock on {scope_label} — elapsed {waited}{remaining_text}")
    }

    fn success_message(&self, scope: &LockScope, waited: Duration) -> String {
        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        format!("Lock acquired after {waited_label}; continuing {scope_label} work.")
    }

    fn timeout_message(&self, scope: &LockScope, waited: Duration) -> String {
        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        format!(
            "Could not acquire {scope_label} lock after {waited_label}; retry with --lock-timeout \
             or adjust configuration."
        )
    }

    fn cancelled_message(&self, scope: &LockScope, waited: Duration) -> String {
        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        format!(
            "Cancelled wait for {scope_label} lock after {waited_label}; rerun when the resource is free."
        )
    }
}

impl LockWaitObserver for LockFeedbackBridge {
    fn on_wait_start(&self, scope: &LockScope, timeout: LockTimeoutValue) {
        let now = Instant::now();
        {
            let mut state = self.state.lock().unwrap();
            state.started_at = Some(now);
            state.timeout = Some(timeout);
            state.last_emit = Some(now);
            self.ensure_spinner_started(&mut state);
        }

        let message = self.initial_message(scope, timeout);
        self.emit_line(&message);
    }

    fn on_retry(
        &self,
        scope: &LockScope,
        _attempt: usize,
        elapsed: Duration,
        remaining: Option<Duration>,
    ) {
        let now = Instant::now();
        let mut state = self.state.lock().unwrap();
        let interval = match self.renderer_kind {
            ProgressRendererKind::Tty => Duration::from_secs(1),
            ProgressRendererKind::NonTty => Duration::from_secs(5),
            ProgressRendererKind::Silent => Duration::MAX,
        };

        let emit = !state.progress_emitted
            || state
                .last_emit
                .map(|last| now.duration_since(last) >= interval)
                .unwrap_or(true);

        if emit && self.renderer_kind != ProgressRendererKind::Silent {
            state.last_emit = Some(now);
            state.progress_emitted = true;
            drop(state);

            let message = self.progress_message(scope, elapsed, remaining);
            self.emit_line(&message);
        }
    }

    fn on_acquired(&self, scope: &LockScope, waited: Duration) {
        let message = self.success_message(scope, waited);
        self.emit_success(&message);
    }

    fn on_timeout(&self, scope: &LockScope, waited: Duration) {
        let message = self.timeout_message(scope, waited);
        self.emit_error(&message);
    }

    fn on_cancelled(&self, scope: &LockScope, waited: Duration) {
        let message = self.cancelled_message(scope, waited);
        self.emit_error(&message);
    }
}

/// Sink used by [`StatusReporterObserver`] to surface wait-state progress.
pub trait LockStatusSink: Send + Sync {
    fn step(&self, message: &str);
    fn success(&self, message: &str);
    fn error(&self, message: &str);
    fn progress_handle(&self) -> Option<Arc<Mutex<Box<dyn ProgressIndicator>>>> {
        None
    }
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

    fn progress_handle(&self) -> Option<Arc<Mutex<Box<dyn ProgressIndicator>>>> {
        Some(self.progress_handle())
    }
}

/// Bridges lock wait events to the appropriate user feedback mechanism.
pub struct StatusReporterObserver<'a> {
    reporter: &'a dyn LockStatusSink,
    source: LockTimeoutSource,
    notified_contention: AtomicBool,
    bridge: Option<LockFeedbackBridge>,
}

impl<'a> StatusReporterObserver<'a> {
    pub fn new(reporter: &'a dyn LockStatusSink, source: LockTimeoutSource) -> Self {
        let bridge = reporter
            .progress_handle()
            .map(|handle| LockFeedbackBridge::for_handle(handle, source));

        Self {
            reporter,
            source,
            notified_contention: AtomicBool::new(false),
            bridge,
        }
    }
}

impl<'a> LockWaitObserver for StatusReporterObserver<'a> {
    fn on_wait_start(&self, scope: &LockScope, timeout: LockTimeoutValue) {
        if let Some(bridge) = &self.bridge {
            bridge.on_wait_start(scope, timeout);
            return;
        }

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
        if let Some(bridge) = &self.bridge {
            bridge.on_retry(scope, attempt, elapsed, remaining);
            return;
        }

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
        if let Some(bridge) = &self.bridge {
            bridge.on_acquired(scope, waited);
            return;
        }

        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        self.reporter
            .success(&format!("Acquired {scope_label} lock after {waited_label}"));
    }

    fn on_timeout(&self, scope: &LockScope, waited: Duration) {
        if let Some(bridge) = &self.bridge {
            bridge.on_timeout(scope, waited);
            return;
        }

        let scope_label = scope.label();
        let waited_label = format_duration(waited);
        self.reporter.error(&format!(
            "Timed out waiting for {scope_label} lock after {waited_label}"
        ));
    }

    fn on_cancelled(&self, scope: &LockScope, waited: Duration) {
        if let Some(bridge) = &self.bridge {
            bridge.on_cancelled(scope, waited);
            return;
        }

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
    use crate::indicator::ProgressConfig;
    use crate::locking::timeout::LockTimeoutValue;
    use std::sync::Mutex;

    #[test]
    fn reporter_observer_uses_progress_bridge_when_available() {
        let indicator = Box::new(RecordingIndicator::new());
        let reporter = StatusReporter::with_indicator(indicator);
        let observer = StatusReporterObserver::new(&reporter, LockTimeoutSource::Cli);
        let scope = LockScope::CacheWriter;

        observer.on_wait_start(&scope, LockTimeoutValue::from_secs(30));
        observer.on_retry(
            &scope,
            1,
            Duration::from_millis(1_200),
            Some(Duration::from_secs(10)),
        );
        observer.on_acquired(&scope, Duration::from_secs(2));

        let (output, errors) = RecordingIndicator::take_messages();
        assert!(errors.is_empty());
        assert!(
            output
                .iter()
                .any(|line| line.contains("Waiting for lock on cache writer"))
        );
        assert!(output.iter().any(|line| line.contains("elapsed 1.2s")));
        assert!(
            output
                .iter()
                .any(|line| line.contains("Lock acquired after 2.0s"))
        );
    }

    #[test]
    fn reporter_observer_falls_back_to_text_sink() {
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

    struct RecordingIndicator;

    impl RecordingIndicator {
        fn new() -> Self {
            RecordingIndicator
        }

        fn take_messages() -> (Vec<String>, Vec<String>) {
            let output = OUTPUT.lock().unwrap().drain(..).collect();
            let errors = ERROR.lock().unwrap().drain(..).collect();
            (output, errors)
        }
    }

    static OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ERROR: Mutex<Vec<String>> = Mutex::new(Vec::new());

    impl ProgressIndicator for RecordingIndicator {
        fn start(&mut self, _config: ProgressConfig) {}

        fn update(&mut self, _current: u64, _total: Option<u64>) {}

        fn set_message(&mut self, message: String) {
            OUTPUT.lock().unwrap().push(message);
        }

        fn complete(&mut self, message: Option<String>) {
            if let Some(msg) = message {
                OUTPUT.lock().unwrap().push(msg);
            }
        }

        fn success(&self, message: &str) -> std::io::Result<()> {
            OUTPUT.lock().unwrap().push(message.to_string());
            Ok(())
        }

        fn error(&mut self, message: String) {
            ERROR.lock().unwrap().push(message);
        }

        fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
            Box::new(RecordingIndicator::new())
        }

        fn suspend(&self, f: &mut dyn FnMut()) {
            f();
        }

        fn println(&self, message: &str) -> std::io::Result<()> {
            OUTPUT.lock().unwrap().push(message.to_string());
            Ok(())
        }

        fn renderer_kind(&self) -> ProgressRendererKind {
            ProgressRendererKind::NonTty
        }
    }
}
