use std::cmp::max;
use std::time::{Duration, Instant, SystemTime};

/// A type which can be used to stop a search gracefully at any time.
pub trait SearchEndSignal {
    /// The returned flag indicates to the search whether it should immediately stop
    fn should_end_now(&self) -> bool;
    /// Blocks the calling thread until the stop condition is reached
    fn join(&self) -> ();
}

impl SearchEndSignal for Instant {
    fn should_end_now(&self) -> bool {
        self <= &Instant::now()
    }

    fn join(&self) -> () {
        std::thread::sleep(max(Duration::ZERO, *self - Instant::now()));
    }
}

impl SearchEndSignal for SystemTime {
    fn should_end_now(&self) -> bool {
        self <= &SystemTime::now()
    }

    fn join(&self) -> () {
        let wait = self.duration_since(SystemTime::now()).unwrap_or(Duration::ZERO);
        std::thread::sleep(max(Duration::ZERO, wait));
    }
}

#[derive(Clone, Debug)]
pub struct EmptyEndSignal;

impl SearchEndSignal for EmptyEndSignal {
    fn should_end_now(&self) -> bool {
        false
    }

    fn join(&self) -> () {}
}
