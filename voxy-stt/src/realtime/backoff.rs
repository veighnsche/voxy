use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub base_delay: Duration,
    pub max_delay: Duration,
}

const JITTER_MIN_PERCENT: u32 = 50;
const JITTER_SPAN_PERCENT: u32 = 51; // 50..=100

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(5),
        }
    }
}

pub fn delay_for_attempt(policy: RetryPolicy, attempt: u32) -> Duration {
    let exp = attempt.min(8);
    let factor = 1u32 << exp;
    let delay = policy.base_delay.saturating_mul(factor);
    delay.min(policy.max_delay)
}

pub fn delay_for_attempt_with_jitter(
    policy: RetryPolicy,
    attempt: u32,
    jitter_seed: u64,
) -> Duration {
    let base = delay_for_attempt(policy, attempt);
    let jitter_percent = jitter_percent(jitter_seed);
    let jittered_ms =
        ((base.as_millis() * jitter_percent as u128) / 100).min(u64::MAX as u128) as u64;
    Duration::from_millis(jittered_ms).min(policy.max_delay)
}

fn jitter_percent(seed: u64) -> u32 {
    let bucket = (seed % JITTER_SPAN_PERCENT as u64) as u32;
    JITTER_MIN_PERCENT + bucket
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{delay_for_attempt, delay_for_attempt_with_jitter, RetryPolicy};

    #[test]
    fn delay_grows_exponentially_before_cap() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
        };

        assert_eq!(delay_for_attempt(policy, 0), Duration::from_millis(100));
        assert_eq!(delay_for_attempt(policy, 1), Duration::from_millis(200));
        assert_eq!(delay_for_attempt(policy, 2), Duration::from_millis(400));
        assert_eq!(delay_for_attempt(policy, 3), Duration::from_millis(800));
    }

    #[test]
    fn delay_caps_at_max_delay() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(700),
            max_delay: Duration::from_millis(2_000),
        };

        assert_eq!(delay_for_attempt(policy, 0), Duration::from_millis(700));
        assert_eq!(delay_for_attempt(policy, 1), Duration::from_millis(1_400));
        assert_eq!(delay_for_attempt(policy, 2), Duration::from_millis(2_000));
        assert_eq!(delay_for_attempt(policy, 9), Duration::from_millis(2_000));
    }

    #[test]
    fn jittered_delay_stays_within_half_to_full_base_delay() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
        };
        let base = delay_for_attempt(policy, 3);
        let jittered_min = delay_for_attempt_with_jitter(policy, 3, 0);
        let jittered_max = delay_for_attempt_with_jitter(policy, 3, 50);

        assert_eq!(base, Duration::from_millis(800));
        assert_eq!(jittered_min, Duration::from_millis(400));
        assert_eq!(jittered_max, Duration::from_millis(800));
    }

    #[test]
    fn jittered_delay_respects_policy_cap() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(1_000),
            max_delay: Duration::from_millis(1_500),
        };

        let jittered = delay_for_attempt_with_jitter(policy, 3, 50);
        assert_eq!(jittered, Duration::from_millis(1_500));
    }
}
