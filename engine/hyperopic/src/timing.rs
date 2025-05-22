use std::cmp::{max, min};
use std::time::Duration;

const DEFAULT_MIN_COMPUTE_TIME_MS: u64 = 50;
const DEFAULT_MIN_CLOCK_TIME_MILLIS: u64 = 250;
const DEFAULT_LATENCY_MILLIS: u64 = 5;

#[derive(Debug, Clone)]
pub struct TimeAllocator {
    /// Given the number of moves played return the expected value of moves
    /// still to play.
    half_moves_remaining: fn(usize) -> f64,
    /// Any time added to computing a move which is not spent thinking
    latency: Duration,
    min_compute_time: Duration,
    min_clock_time: Duration,
}

impl Default for TimeAllocator {
    fn default() -> Self {
        TimeAllocator {
            half_moves_remaining: expected_half_moves_remaining,
            latency: Duration::from_millis(DEFAULT_LATENCY_MILLIS),
            min_compute_time: Duration::from_millis(DEFAULT_MIN_COMPUTE_TIME_MS),
            min_clock_time: Duration::from_millis(DEFAULT_MIN_CLOCK_TIME_MILLIS),
        }
    }
}

impl TimeAllocator {
    pub fn with_latency(latency: Duration) -> Self {
        TimeAllocator { latency, ..Default::default() }
    }

    // TODO Pass in position so we can reduce time thinking if there is a clear capture for example
    pub fn allocate(
        &self,
        half_moves_played: usize,
        remaining_time: Duration,
        increment: Duration,
    ) -> Duration {
        let min_remaining_after_thinking = min(remaining_time, self.min_clock_time + self.latency);
        let usable_thinking_time = remaining_time - min_remaining_after_thinking;

        max(
            self.min_compute_time,
            if usable_thinking_time <= increment {
                usable_thinking_time
            } else {
                // Otherwise we think for the increment and then a little more
                let thinking_time_after_increment = usable_thinking_time - increment;
                let exp_remaining = (self.half_moves_remaining)(half_moves_played) / 2f64;
                let extra_time = ((thinking_time_after_increment.as_millis() as f64)
                    / exp_remaining)
                    .round() as u64;
                increment + Duration::from_millis(extra_time)
            },
        )
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
            min_clock_time: Duration::from_millis(250),
        };
        assert_eq!(
            Duration::from_millis(1355),
            timing.allocate(20, Duration::from_millis(4999), Duration::from_millis(1000))
        )
    }

    #[test]
    fn remaining_less_than_latency() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(1100),
            latency: Duration::from_millis(200),
            min_clock_time: Duration::from_millis(250),
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
            min_clock_time: Duration::from_millis(250),
        };

        assert_eq!(
            Duration::from_millis(4854),
            timing.allocate(20, Duration::from_millis(40000), Duration::from_millis(999))
        );
    }

    #[test]
    fn estimated_less_than_min() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(1100),
            latency: Duration::from_millis(200),
            min_clock_time: Duration::from_millis(250),
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
            min_clock_time: Duration::from_millis(250),
        };

        assert_eq!(
            Duration::from_millis(105),
            timing.allocate(200, Duration::from_secs(1), Duration::from_millis(100))
        );
    }

    #[test]
    fn increment_larger_than_remaining_time() {
        let timing = TimeAllocator {
            half_moves_remaining: dummy_half_moves_remaining,
            min_compute_time: Duration::from_millis(50),
            latency: Duration::from_millis(5),
            min_clock_time: Duration::from_millis(250),
        };
        assert_eq!(
            Duration::from_millis(749),
            timing.allocate(224, Duration::from_millis(1004), Duration::from_millis(1000))
        );
    }
}
