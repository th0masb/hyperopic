use std::cmp::{max, min};
use std::time::Duration;

const DEFAULT_MIN_COMPUTE_TIME_MS: u64 = 50;
const DEFAULT_INC_ONLY_THRESHOLD_MILLIS: u64 = 500;
const DEFAULT_INC_USED_UNDER_THRESHOLD: f64 = 0.8;

#[derive(Debug, Clone)]
pub struct TimeAllocator {
    /// Given the number of moves played return the expected value of moves
    /// still to play.
    half_moves_remaining: fn(usize) -> f64,
    /// Any time added to computing a move which is not spent thinking
    pub latency: Duration,
    pub min_compute_time: Duration,
    pub increment_only_threshold: Duration,
    pub inc_used_under_threshold: f64
}

impl Default for TimeAllocator {
    fn default() -> Self {
        TimeAllocator {
            half_moves_remaining: expected_half_moves_remaining,
            latency: Duration::ZERO,
            min_compute_time: Duration::from_millis(DEFAULT_MIN_COMPUTE_TIME_MS),
            increment_only_threshold: Duration::from_millis(DEFAULT_INC_ONLY_THRESHOLD_MILLIS),
            inc_used_under_threshold: DEFAULT_INC_USED_UNDER_THRESHOLD,
        }
    }
}

impl TimeAllocator {
    // TODO Pass in position so we can reduce time thinking if there is a clear capture for example
    pub fn allocate(
        &self,
        half_moves_played: usize,
        remaining_time: Duration,
        increment: Duration,
    ) -> Duration {
        // Idea is if we are under some threshold left on the clock we will only use the increment
        // time to think. We leave some of the inc on the table to try and gain some time back on
        // the clock.
        if remaining_time < self.increment_only_threshold && increment > Duration::ZERO {
            let inc = increment - min(increment, self.latency);
            let inc = inc.as_millis() as f64;
            let inc = (inc * self.inc_used_under_threshold).round() as u64;
            return max(self.min_compute_time, Duration::from_millis(inc));
        }
        let remaining_after_latency = remaining_time - min(remaining_time, self.latency);
        // Divide by two because we need to think for half of the remaining moves
        let exp_remaining = (self.half_moves_remaining)(half_moves_played) / 2f64;
        let estimated_no_inc =
            ((remaining_after_latency.as_millis() as f64) / exp_remaining).round() as u64;
        let estimated = Duration::from_millis(estimated_no_inc) + increment;
        max(estimated, self.min_compute_time)
    }
}

/// https://chess.stackexchange.com/questions/2506/what-is-the-average-length-of-a-game-of-chess
fn expected_half_moves_remaining(moves_played: usize) -> f64 {
    let k = moves_played as f64;
    59.3 + (72830f64 - 2330f64 * k) / (2644f64 + k * (10f64 + k))
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::timing::TimeAllocator;

    fn dummy_half_moves_remaining(moves_played: usize) -> f64 {
        moves_played as f64
    }

    #[test]
    fn remaining_less_than_increment_threshold() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(500),
            latency: Duration::from_millis(200),
            increment_only_threshold: Duration::from_millis(5000),
            inc_used_under_threshold: 0.8
        };
        assert_eq!(
            Duration::from_millis(640),
            timing.allocate(20, Duration::from_millis(4999), Duration::from_millis(1000))
        )
    }

    #[test]
    fn remaining_less_than_latency() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(1100),
            latency: Duration::from_millis(200),
            increment_only_threshold: Duration::from_millis(100),
            inc_used_under_threshold: 0.8
        };
        assert_eq!(
            Duration::from_millis(1100),
            timing.allocate(20, Duration::from_millis(100), Duration::from_millis(0))
        )
    }

    #[test]
    fn estimated_greater_than_min() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(1100),
            latency: Duration::from_millis(200),
            increment_only_threshold: Duration::from_millis(100),
            inc_used_under_threshold: 0.8
        };

        assert_eq!(
            Duration::from_millis(4979),
            timing.allocate(20, Duration::from_millis(40000), Duration::from_millis(999))
        );
    }

    #[test]
    fn estimated_less_than_min() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(1100),
            latency: Duration::from_millis(200),
            increment_only_threshold: Duration::from_millis(100),
            inc_used_under_threshold: 0.8
        };

        assert_eq!(
            Duration::from_millis(1100),
            timing.allocate(200, Duration::from_secs(10), Duration::from_millis(999))
        );
    }

    #[test]
    fn latency_larger_than_increment() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(100),
            latency: Duration::from_millis(200),
            increment_only_threshold: Duration::from_millis(5000),
            inc_used_under_threshold: 0.8
        };

        assert_eq!(
            Duration::from_millis(100),
            timing.allocate(200, Duration::from_secs(1), Duration::from_millis(100))
        );
    }
}
