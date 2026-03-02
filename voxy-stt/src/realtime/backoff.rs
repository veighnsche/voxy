use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub base_delay: Duration,
    pub max_delay: Duration,
}

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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{delay_for_attempt, RetryPolicy};

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
}
