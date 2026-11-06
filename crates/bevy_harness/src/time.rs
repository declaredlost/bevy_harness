//! Seconds as a test writes them.

use core::time::Duration;

/// `seconds` as a [`Duration`].
///
/// # Panics
///
/// If `seconds` is negative or not finite: a test asking for that is
/// wrong, and waiting no time instead would let it pass.
#[track_caller]
pub(crate) fn seconds(seconds: f32) -> Duration {
    Duration::try_from_secs_f32(seconds)
        .unwrap_or_else(|_| panic!("{seconds} isn't a length of time: it has to be 0 or more"))
}
