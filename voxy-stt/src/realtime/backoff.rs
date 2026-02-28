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
