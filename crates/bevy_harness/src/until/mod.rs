//! Ticking until something holds.

mod ticked;

use core::time::Duration;

pub use self::ticked::Ticked;
use crate::time;

/// Ticks an app (or apps) until `holds` holds, at most so long: failing,
/// at the test's line, if it never does.
///
/// `holds` is asked of the app itself, by `&mut`, so everything a test
/// asks with works in it too: [`query`](crate::TestApp::query) registers
/// what it names, which is why it's `&mut`. It's asked before each tick,
/// and once after the last.
pub struct Until<'a, T: Ticked, P: FnMut(&mut T) -> bool> {
    ticked: &'a mut T,
    holds: P,
    /// What `holds` checks, for the failure to say.
    what: Option<&'a str>,
}

impl<'a, T: Ticked, P: FnMut(&mut T) -> bool> Until<'a, T, P> {
    pub(crate) fn new(ticked: &'a mut T, holds: P) -> Self {
        Self {
            ticked,
            holds,
            what: None,
        }
    }

    /// Says what it waits for, for a failure to say: `"the foe is dead"`.
    #[must_use]
    pub fn describe(mut self, what: &'a str) -> Self {
        self.what = Some(what);
        self
    }

    /// Ticks at most `limit` times; returns how many it took.
    ///
    /// None, if it held already.
    ///
    /// # Panics
    ///
    /// If it doesn't hold within `limit` ticks, saying how much game time
    /// that was, at what rate, and what it waited for.
    #[track_caller]
    pub fn within_ticks(mut self, limit: u32) -> u32 {
        for ticks in 0..=limit {
            if (self.holds)(self.ticked) {
                return ticks;
            }
            if ticks < limit {
                self.ticked.tick();
            }
        }
        let step = self.ticked.step();
        let what = self
            .what
            .map(|what| format!(": {what}"))
            .unwrap_or_default();
        panic!(
            "it didn't hold within {limit} ticks ({:?} of game time, at {} Hz){what}",
            step.saturating_mul(limit),
            time::hz(step)
        );
    }

    /// Ticks for at most `length` of game time; returns how many it took.
    ///
    /// `length` is rounded up to whole ticks, so it ticks at least once
    /// for any `length` more than none.
    ///
    /// # Panics
    ///
    /// If it doesn't hold by then.
    #[track_caller]
    pub fn within(self, length: Duration) -> u32 {
        let limit = time::covering(length, self.ticked.step());
        self.within_ticks(limit)
    }

    /// Ticks for at most `seconds` of game time.
    ///
    /// [`within`](Self::within) in seconds.
    ///
    /// # Panics
    ///
    /// If it doesn't hold by then, or `seconds` is negative or not finite.
    #[track_caller]
    pub fn within_secs(self, seconds: f32) -> u32 {
        self.within(time::seconds(seconds))
    }
}
