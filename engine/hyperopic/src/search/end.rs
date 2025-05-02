use std::cmp::max;
use std::time::{Duration, Instant};

/// Represents some object which can determine whether a search should be
/// terminated given certain context about the current state. Implementations
/// are provided for Duration (caps the search based on time elapsed), for
/// usize which represents a maximum search depth and for a pair (Duration, usize)
/// which combines both checks.
pub trait SearchEndSignal {
    fn should_end_now(&self) -> bool;
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

#[derive(Clone, Debug)]
pub struct EmptyEndSignal;

impl SearchEndSignal for EmptyEndSignal {
    fn should_end_now(&self) -> bool {
        false
    }

    fn join(&self) -> () {
    }
}
