use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use crate::search::search::Context;

/// Represents some object which can determine whether a search should be
/// terminated given certain context about the current state. Implementations
/// are provided for Duration (caps the search based on time elapsed), for
/// usize which represents a maximum search depth and for a pair (Duration, usize)
/// which combines both checks.
pub trait SearchEnd {
    fn should_end(&self, ctx: &Context) -> bool;
}

impl SearchEnd for Duration {
    fn should_end(&self, ctx: &Context) -> bool {
        ctx.start.elapsed() > *self
    }
}

impl SearchEnd for usize {
    fn should_end(&self, ctx: &Context) -> bool {
        ctx.depth as usize > *self
    }
}

impl SearchEnd for (Duration, usize) {
    fn should_end(&self, ctx: &Context) -> bool {
        self.0.should_end(ctx) || self.1.should_end(ctx)
    }
}

impl SearchEnd for (Duration, Arc<AtomicBool>) {
    fn should_end(&self, ctx: &Context) -> bool {
        self.0.should_end(ctx) || self.1.load(Ordering::Acquire)
    }
}
