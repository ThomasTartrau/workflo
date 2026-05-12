use rand::Rng;
use time::Duration;

pub fn retry_delay(hard_try_count: i32) -> Duration {
    let exponent = hard_try_count.max(0) as u32;
    let base_ms = (2i64.saturating_pow(exponent) * 1000).min(3_600_000);
    let half = base_ms / 2;
    let jitter = if half > 0 {
        rand::thread_rng().gen_range(0..=half)
    } else {
        0
    };
    Duration::milliseconds(half + jitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_backoff_bounds() {
        for _ in 0..100 {
            let d = retry_delay(0);
            assert!(d >= Duration::milliseconds(500));
            assert!(d <= Duration::milliseconds(1000));
        }
        for _ in 0..100 {
            let d = retry_delay(1);
            assert!(d >= Duration::milliseconds(1000));
            assert!(d <= Duration::milliseconds(2000));
        }
        for _ in 0..100 {
            let d = retry_delay(3);
            assert!(d >= Duration::milliseconds(4000));
            assert!(d <= Duration::milliseconds(8000));
        }
    }

    #[test]
    fn capped_at_one_hour() {
        for _ in 0..100 {
            let d = retry_delay(20);
            assert!(d >= Duration::milliseconds(1_800_000));
            assert!(d <= Duration::milliseconds(3_600_000));
        }
    }

    #[test]
    fn negative_count_treated_as_zero() {
        for _ in 0..100 {
            let d = retry_delay(-1);
            assert!(d >= Duration::milliseconds(500));
            assert!(d <= Duration::milliseconds(1000));
        }
    }
}
