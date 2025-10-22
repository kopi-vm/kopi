use kopi::locking::PollingBackoff;
use std::time::Duration;

/// Long-running validation of polling backoff characteristics.
///
/// Calculates the estimated CPU utilisation for a given wait budget using the default
/// exponential backoff parameters. The test is ignored by default because the full
/// 5-minute profile is intended for manual execution when investigating contention
/// regressions.
#[test]
#[ignore = "Long-running stress test (~5 minutes). Set KOPI_STRESS_DURATION_SECS to override."]
fn lock_polling_backoff_stress_profile() {
    let target_secs = std::env::var("KOPI_STRESS_DURATION_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(300);

    let target_duration = Duration::from_secs(target_secs);
    let mut backoff = PollingBackoff::default();
    let mut polls: u64 = 0;
    let mut accumulated = Duration::ZERO;

    while accumulated < target_duration {
        let delay = backoff.next_delay();
        accumulated += delay;
        polls = polls.saturating_add(1);
    }

    // Assume each poll iteration consumes approximately 1 ms of busy CPU time.
    let busy_micros = polls.saturating_mul(1_000);
    let busy_estimate = Duration::from_micros(busy_micros);
    let busy_ratio = busy_estimate.as_secs_f64() / target_duration.as_secs_f64();

    assert!(
        busy_ratio <= 0.001,
        "Estimated busy ratio {busy_ratio:.4} exceeded 0.1% target (polls {polls}, total wait {:.1}s)",
        target_duration.as_secs_f64()
    );
}
