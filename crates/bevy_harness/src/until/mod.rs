//! Ticking until something holds.

mod ticked;

use core::time::Duration;

pub use self::ticked::Ticked;

/// Ticks an app (or apps) until `holds` holds, at most so long: failing,
/// at the test's line, if it never does.
pub struct Until<'a, T: Ticked, P: FnMut(&T::View) -> bool> {
    ticked: &'a mut T,
    holds: P,
}

impl<'a, T: Ticked, P: FnMut(&T::View) -> bool> Until<'a, T, P> {
    pub(crate) fn new(ticked: &'a mut T, holds: P) -> Self {
        Self { ticked, holds }
    }

    /// Ticks at most `limit` times; returns how many it took (none, if it
    /// held already).
    ///
    /// # Panics
    ///
    /// If it doesn't hold within `limit` ticks.
    #[track_caller]
    pub fn within_ticks(mut self, limit: u32) -> u32 {
        for ticks in 0..=limit {
            if (self.holds)(self.ticked.view()) {
                return ticks;
            }
            if ticks < limit {
                self.ticked.tick();
            }
        }
        panic!("it didn't hold within {limit} ticks");
    }

    /// Ticks for at most `length` of game time; returns how many ticks it
    /// took.
    ///
    /// # Panics
    ///
    /// If it doesn't hold by then.
    #[track_caller]
    pub fn within(self, length: Duration) -> u32 {
        let limit = self.ticked.ticks_in(length);
        self.within_ticks(limit)
    }

    /// Ticks for at most `seconds` of game time:
    /// [`within`](Self::within) in seconds.
    ///
    /// # Panics
    ///
    /// If it doesn't hold by then, or `seconds` is negative or not finite.
    #[track_caller]
    pub fn within_secs(self, seconds: f32) -> u32 {
        self.within(crate::time::seconds(seconds))
    }
}
